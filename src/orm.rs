use anyhow::{anyhow, Context};
use oxide_auth::primitives::{
    registrar::{Argon2, Client, RegisteredUrl},
    scope::Scope,
};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{AppUser, ClientId, ClientSecret, UserEmail, UserId, UserName, UserRole},
    entities::{
        client_app, client_authorization,
        prelude::*,
        user::{self, Model as UserModel},
    },
    error::error_chain_fmt,
    scopes::{FOLLOW_SCOPES, GLOBAL_FOLLOW, GLOBAL_READ, GLOBAL_WRITE, READ_SCOPES, WRITE_SCOPES},
};

#[tracing::instrument(name = "Get user model", skip(conn))]
pub async fn get_user_model_by_id(
    user_id: i64,
    conn: &DatabaseConnection,
) -> Result<UserModel, OrmError> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!("Failed to retrieve a user record: {}", e)))
        })?
        .unwrap();

    Ok(model)
}

#[tracing::instrument(name = "Get ORM model", skip(conn))]
pub async fn get_orm_model_by_id(
    user_id: i64,
    conn: &DatabaseConnection,
) -> Result<AppUser, OrmError> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!("Failed to retrieve a user record: {}", e)))
        })?
        .unwrap();

    Ok(AppUser {
        id: Some(model.id),
        password: Some(Secret::from(model.password)),
        name: UserName::parse(model.name).unwrap_or_default(),
        email: UserEmail::parse(model.email).unwrap_or_default(),
        role: UserRole::try_from(model.role).unwrap_or_default(),
    })
}

#[tracing::instrument(name = "Get credential", skip(username, conn))]
pub async fn get_credential(
    username: &str,
    conn: &DatabaseConnection,
) -> Result<Option<(i64, Secret<String>)>, anyhow::Error> {
    let model = User::find()
        .filter(user::Column::Email.eq(username))
        .one(conn)
        .await
        .context("Failed to retrieve stored credentials")?
        .map(|m| (m.id, Secret::new(m.password)));

    Ok(model)
}

pub async fn update_credential(
    user_id: i64,
    orm_user: user::ActiveModel,
    conn: &DatabaseConnection,
) -> Result<i64, OrmError> {
    let res = User::update(orm_user)
        .filter(user::Column::Id.eq(user_id))
        .exec(conn)
        .await
        .context("Failed to update user record")?;

    Ok(res.id)
}

#[tracing::instrument(name = "Register confidential client app", skip(conn))]
pub async fn register_confidential_client(
    client_name: &str,
    website: &str,
    uri: &str,
    default_scope: &str,
    conn: &DatabaseConnection,
) -> Result<(ClientId, Secret<ClientSecret>), OrmError> {
    let id = ClientId::new();
    let secret = Secret::from(ClientSecret::new());

    let mut scopes = String::from(default_scope);
    if scopes.is_empty() || scopes == GLOBAL_READ {
        scopes = READ_SCOPES.join(" ");
    } else if scopes == GLOBAL_WRITE {
        scopes = WRITE_SCOPES.join(" ");
    } else if scopes == GLOBAL_FOLLOW {
        scopes = FOLLOW_SCOPES.join(" ");
    }
    let client = Client::confidential(
        id.as_str(),
        RegisteredUrl::Semantic(uri.parse().unwrap()),
        scopes.parse::<Scope>().unwrap(),
        secret.expose_secret().as_str().as_bytes(),
    );
    let encoded_client = client.encode(&Argon2::default());

    tracing::debug!("Registering confidential client: {client_name}");
    let app = client_app::ActiveModel {
        client_id: ActiveValue::Set(id.to_string()),
        name: ActiveValue::Set(Some(client_name.to_string())),
        website: ActiveValue::Set(Some(website.to_string())),
        encoded_client: ActiveValue::Set(serde_json::json!(encoded_client)),
        ..Default::default()
    };
    let _ = ClientApp::insert(app)
        .exec(conn)
        .await
        .map_err(|e| OrmError::UnexpectedError(anyhow!(e)))?;

    Ok((id, secret.to_owned()))
}

#[derive(Debug, Deserialize, Serialize)]
struct Authorization {
    scope: Scope,
}

#[tracing::instrument(name = "Get authorization scope for client app", skip(conn))]
pub async fn get_client_authorization(
    user_id: UserId,
    client_id: ClientId,
    conn: &DatabaseConnection,
) -> Result<Scope, OrmError> {
    let user_id = Into::<i64>::into(user_id);
    let client_model = get_client_app_by_client_id(client_id, conn).await?;
    let client_authorization = ClientAuthorization::find()
        .filter(client_authorization::Column::UserId.eq(user_id))
        .filter(client_authorization::Column::ClientAppId.eq(client_model.id))
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!(
                "Failed to retrieve client authorization: {}",
                e
            )))
        })?
        .unwrap();

    let res = client_authorization.scope.parse::<Scope>().map_err(|e| {
        OrmError::UnexpectedError(anyhow!(format!(
            "failed to parse authorization scope: {}",
            e
        )))
    })?;

    Ok(res)
}

#[tracing::instrument(name = "Update authorization scope for client app", skip(conn))]
pub async fn update_client_authorization(
    user_id: UserId,
    client_id: ClientId,
    scope: Scope,
    conn: &DatabaseConnection,
) -> Result<bool, OrmError> {
    let user_id = Into::<i64>::into(user_id);
    let client_model = get_client_app_by_client_id(client_id, conn).await?;
    let client_authorization = ClientAuthorization::find()
        .filter(client_authorization::Column::UserId.eq(user_id))
        .filter(client_authorization::Column::ClientAppId.eq(client_model.id))
        .one(conn)
        .await;

    match client_authorization {
        Ok(Some(model)) => {
            update_existing_client_authorization(model, scope, user_id, client_id, conn).await
        }
        _ => create_new_client_authorization(scope, user_id, client_id, conn).await,
    }
}

#[tracing::instrument(name = "Get client app model", skip(conn))]
pub async fn get_client_app_by_client_id(
    client_id: ClientId,
    conn: &DatabaseConnection,
) -> Result<client_app::Model, OrmError> {
    let model = ClientApp::find()
        .filter(client_app::Column::ClientId.eq(client_id.to_string()))
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!("Failed to retrieve a client app: {}", e)))
        })?
        .unwrap();

    Ok(model)
}

#[tracing::instrument(name = "update an existing client authorization", skip(conn))]
async fn update_existing_client_authorization(
    model: client_authorization::Model,
    scope: Scope,
    user_id: i64,
    client_id: ClientId,
    conn: &DatabaseConnection,
) -> Result<bool, OrmError> {
    let authorization = client_authorization::ActiveModel {
        id: ActiveValue::Set(model.id),
        scope: ActiveValue::Set(scope.to_string()),
        user_id: ActiveValue::Unchanged(model.user_id),
        client_app_id: ActiveValue::Unchanged(model.client_app_id),
        ..Default::default()
    };
    let _ = ClientAuthorization::update(authorization)
        .exec(conn)
        .await
        .context("failed to update authorization scope")?;

    Ok(true)
}

#[tracing::instrument(name = "create a new client authorization", skip(conn))]
async fn create_new_client_authorization(
    scope: Scope,
    user_id: i64,
    client_id: ClientId,
    conn: &DatabaseConnection,
) -> Result<bool, OrmError> {
    let client_model = get_client_app_by_client_id(client_id, conn).await?;
    let authorization = client_authorization::ActiveModel {
        user_id: ActiveValue::Set(user_id),
        client_app_id: ActiveValue::Set(client_model.id),
        scope: ActiveValue::Set(scope.to_string()),
        ..Default::default()
    };
    let _ = ClientAuthorization::insert(authorization)
        .exec(conn)
        .await
        .context("failed to insert a new authorization scope")?;

    Ok(true)
}

#[derive(thiserror::Error)]
pub enum OrmError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for OrmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
