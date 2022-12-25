use anyhow::Context;
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::IntoResponse,
    Form,
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, DbErr, EntityTrait, Set, TransactionTrait};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

use super::super::{generate_random_key, get_db_from_host, AppState};
use crate::{
    domain::{user_email::UserEmail, NewUser, UserName, UserRole},
    email_client::EmailClient,
    entities::{prelude::*, user, user_token},
    error::{error_chain_fmt, TenantMapError},
    smtp_client::SmtpMailer,
};

#[derive(Debug, Deserialize)]
pub struct InputUser {
    name: String,
    email: String,
    password: String,
    role: String,
}

#[tracing::instrument(
    name = "Add a New User",
    skip(form, state),
    fields(
        request_id = %Uuid::new_v4(),
        user_email = %form.email,
        user_name = %form.name,
    )
)]
pub async fn create(
    Host(host): Host,
    State(state): State<AppState>,
    Form(form): Form<InputUser>,
) -> Result<(), UserError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => UserError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => UserError::UnexpectedError(anyhow::anyhow!(s)),
    })?;

    let new_user = parse_user(&form)?;
    let token = generate_random_key(25);

    let new_user2 = new_user.clone();
    let token2 = token.clone();
    conn.transaction::<_, (), UserError>(|txn| {
        Box::pin(async move {
            let user_id = insert_user(txn, &new_user2)
                .await
                .context("Failed to insert a new user into the database")?;

            store_token(user_id, &token2, txn)
                .await
                .context("Failed to store the new user confirmation token in the database")?;
            Ok(())
        })
    })
    .await
    .context("Encountered a database transaction error")?;

    if let Err(e) = send_confirmation_email(&new_user, &state, &token).await {
        return Err(UserError::ValidationError(e.to_string()));
    }

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to the new user",
    skip(new_user, state)
)]
pub async fn send_confirmation_email(
    new_user: &NewUser,
    state: &AppState,
    token: &str,
) -> Result<(), EmailTokenError> {
    let confirmation_link = format!(
        "{}/user/confirm?confirmation_token={}",
        &state.global_config.server.base_url, token
    );
    let plain = format!(
        "Welcome to Rhodos!\n Visit {} to confirm your account.",
        confirmation_link
    );
    let html = format!(
        r#"Welcome to Rhodos!<br />Click <a href="{}">here</a> to confirm your account."#,
        confirmation_link
    );
    let smtp_mailer = SmtpMailer::new(
        &state.global_config.email_outgoing.smtp_host.clone(),
        state.global_config.email_outgoing.smtp_port,
        &state.global_config.email_outgoing.smtp_user.clone(),
        state.global_config.email_outgoing.smtp_password.clone(),
    );
    let email_client = EmailClient::new(state.global_config.email_outgoing.smtp_sender.clone());
    email_client
        .send_email(
            &new_user.email,
            &"Please confirm your email".to_string(),
            &plain,
            &html,
            &smtp_mailer,
        )
        .await?;

    Ok(())
}

async fn insert_user(conn: &DatabaseTransaction, new_user: &NewUser) -> Result<i64, DbErr> {
    let data = user::ActiveModel {
        name: Set(new_user.name.as_ref().to_string()),
        email: Set(new_user.email.as_ref().to_string()),
        password: Set(new_user.password.expose_secret().clone()),
        role: Set(new_user.role.to_string()),
        confirmed: Set(false),
        ..Default::default()
    };
    let res = User::insert(data).exec(conn).await?;

    Ok(res.last_insert_id)
}

fn parse_user(form: &InputUser) -> Result<NewUser, String> {
    let name = UserName::parse(form.name.clone())?;
    let email = UserEmail::parse(form.email.clone())?;
    let password = Secret::from(form.password.clone());
    let role = UserRole::try_from(form.role.clone())?;
    Ok(NewUser {
        name,
        email,
        password,
        role,
    })
}

#[tracing::instrument(name = "Store new user confirmation token", skip(user_id, token, db))]
pub async fn store_token(
    user_id: i64,
    token: &str,
    db: &DatabaseTransaction,
) -> Result<(), StoreTokenError> {
    user_token::ActiveModel {
        user_id: Set(user_id),
        token: Set(token.to_string()),
        ..Default::default()
    }
    .save(db)
    .await?;

    Ok(())
}

#[derive(thiserror::Error)]
pub enum UserError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("{0}")]
    ValidationError(String),
    #[error("{0}")]
    AuthorizationError(String),
}

impl IntoResponse for UserError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::AuthorizationError(s) => {
                tracing::info!("authorization denied: {s:?}");
                (StatusCode::UNAUTHORIZED, s)
            }
            Self::UnexpectedError(e) => {
                tracing::info!("an unexpected error occured");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
            }
            Self::ValidationError(s) => {
                tracing::info!("unable to validate user supplied data: {s:?}");
                (StatusCode::BAD_REQUEST, s)
            }
        };
        (status, err_msg).into_response()
    }
}

impl From<String> for UserError {
    fn from(s: String) -> Self {
        Self::ValidationError(s)
    }
}

impl std::fmt::Debug for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
#[error("Failed to send a confirmation email: {0:?}")]
pub struct EmailTokenError(String);

impl From<String> for EmailTokenError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Debug for EmailTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A mail error was encountered while trying to\
            send a new user confirmation email.\nCaused by:\n\t{}",
            self.0
        )
    }
}

#[derive(thiserror::Error)]
#[error("Failed to store a new user confirmation token: {0:?}")]
pub struct StoreTokenError(#[from] DbErr);

impl std::fmt::Debug for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
