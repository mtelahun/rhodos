use std::sync::Arc;

use axum::{
    extract::{Host, State},
    http::StatusCode,
    Form,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
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
) -> StatusCode {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();

    let new_user = parse_user(&form)
        .map_err(|_| StatusCode::BAD_REQUEST)
        .unwrap();
    let user_id = match insert_user(&conn, &new_user).await {
        Ok(user_id) => user_id,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let token = generate_confirmation_token();
    if store_token(user_id, &token, &conn).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }
    if send_confirmation_email(new_user, &state, &token)
        .await
        .is_err()
    {
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

#[tracing::instrument(
    name = "Send a confirmation email to the new user",
    skip(new_user, state)
)]
pub async fn send_confirmation_email(
    new_user: NewUser,
    state: &Arc<AppState>,
    token: &str,
) -> Result<(), String> {
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

async fn insert_user(conn: &DatabaseConnection, new_user: &NewUser) -> Result<i64, String> {
    let data = user::ActiveModel {
        name: Set(new_user.name.as_ref().to_string()),
        email: Set(Some(new_user.email.as_ref().to_string())),
        confirmed: Set(false),
        ..Default::default()
    };
    let res = User::insert(data)
        .exec(conn)
        .await
        .map_err(|e| e.to_string())?;

    Ok(res.last_insert_id)
}

fn parse_user(form: &InputUser) -> Result<NewUser, String> {
    let name = UserName::parse(form.name.clone())?;
    let email = UserEmail::parse(form.email.clone())?;
    Ok(NewUser { name, email })
}

#[tracing::instrument(name = "Store new use confirmation token", skip(user_id, token, db))]
pub async fn store_token(user_id: i64, token: &str, db: &DatabaseConnection) -> Result<(), String> {
    let model = user_token::ActiveModel {
        user_id: Set(user_id),
        token: Set(token.to_string()),
        ..Default::default()
    };
    let _ = UserToken::insert(model)
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
