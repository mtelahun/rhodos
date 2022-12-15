use std::sync::Arc;

use anyhow::Context;
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::IntoResponse,
    Form,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set};
use serde::Deserialize;
use uuid::Uuid;

use super::{get_db_from_host, AppState};
use crate::{
    domain::{user_email::UserEmail, NewUser, UserName},
    email_client::EmailClient,
    entities::{prelude::*, user, user_token},
    smtp_client::SmtpMailer,
};

#[derive(Debug, Deserialize)]
pub struct InputUser {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Add a New User",
    skip(form, state),
    fields(
        request_id = %Uuid::new_v4(),
        user_email = %form.email,
        user_name = %form.name
    )
)]
pub async fn create(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
    Form(form): Form<InputUser>,
) -> Result<(), RhodosError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.unwrap();

    let new_user = parse_user(&form)?;

    let user_id = insert_user(&conn, &new_user)
        .await
        .context("Failed to insert a new user into the database")?;

    let token = generate_confirmation_token();

    store_token(user_id, &token, &conn)
        .await
        .context("Failed to store the new user confirmation token in the database")?;

    if let Err(e) = send_confirmation_email(new_user, &state, &token).await {
        return Err(RhodosError::ValidationError(e.to_string()));
    }

    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to the new user",
    skip(new_user, state)
)]
pub async fn send_confirmation_email(
    new_user: NewUser,
    state: &Arc<AppState>,
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
            new_user.email,
            &"Please confirm your email".to_string(),
            &plain,
            &html,
            &smtp_mailer,
        )
        .await?;

    Ok(())
}

fn generate_confirmation_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

async fn insert_user(conn: &DatabaseConnection, new_user: &NewUser) -> Result<i64, DbErr> {
    let data = user::ActiveModel {
        name: Set(new_user.name.as_ref().to_string()),
        email: Set(Some(new_user.email.as_ref().to_string())),
        confirmed: Set(false),
        ..Default::default()
    };
    let res = User::insert(data).exec(conn).await?;

    Ok(res.last_insert_id)
}

fn parse_user(form: &InputUser) -> Result<NewUser, String> {
    let name = UserName::parse(form.name.clone())?;
    let email = UserEmail::parse(form.email.clone())?;
    Ok(NewUser { name, email })
}

#[tracing::instrument(name = "Store new user confirmation token", skip(user_id, token, db))]
pub async fn store_token(
    user_id: i64,
    token: &str,
    db: &DatabaseConnection,
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
pub enum RhodosError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("{0}")]
    ValidationError(String),
}

impl IntoResponse for RhodosError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
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

impl From<String> for RhodosError {
    fn from(s: String) -> Self {
        Self::ValidationError(s)
    }
}

impl std::fmt::Debug for RhodosError {
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

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
