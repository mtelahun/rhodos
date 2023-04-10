use anyhow::{anyhow, Context};
use oxide_auth::primitives::{
    registrar::{Argon2, Client, RegisteredUrl},
    scope::Scope,
};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use secrecy::{ExposeSecret, Secret};

use crate::{
    domain::{AppUser, ClientId, ClientSecret, UserEmail, UserName, UserRole},
    entities::{
        client_app,
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
