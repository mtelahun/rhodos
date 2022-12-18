use std::sync::Arc;

use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    extract::{Host, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    entities::{content, user},
    error::{error_chain_fmt, TenantMapError},
    telemetry::spawn_blocking_with_tracing,
};

use super::{get_db_from_host, AppState};
use crate::entities::prelude::*;

const MAX_POST_CHARS: usize = 500;

#[derive(Debug, Deserialize)]
pub struct BodyData {
    pub content: NewPost,
}

#[derive(Debug, Deserialize)]
pub struct NewPost {
    publisher_id: i64,
    text: String,
}
#[tracing::instrument(
    name = "Post a microblog",
    skip(host, state, headers, body),
    fields(
        request_id = %Uuid::new_v4(),
        username = tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn create(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<(), ContentError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => ContentError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => ContentError::UnexpectedError(anyhow::anyhow!(s)),
    })?;

    let credentials = basic_authentication(&headers).map_err(ContentError::AuthError)?;
    let _user_id = validate_credentials(credentials, &conn).await?;
    let account_id = body.content.publisher_id;
    let new_content = body.content.text;
    if new_content.is_empty() || new_content.len() > MAX_POST_CHARS || account_id <= 0 {
        tracing::error!("Content creation attempted, but a field is invalid");
        return Err(ContentError::ValidationError("empty content".to_string()));
    }

    let data = content::ActiveModel {
        publisher_id: Set(account_id),
        body: Set(Some(new_content)),
        ..Default::default()
    };
    let _ = Content::insert(data)
        .exec(&conn)
        .await
        .context("failed to post new content")?;

    Ok(())
}

struct Credentials {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, conn))]
async fn validate_credentials(
    credentials: Credentials,
    conn: &DatabaseConnection,
) -> Result<i64, ContentError> {
    // Use dummy hashed password so that non-existent users go through
    // a dummy password verification step. This ensures both invalid
    // passwords and non-existent users take the same amount of time to verify.
    let mut user_id = None;
    let mut db_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_uid, stored_hash)) = get_stored_credentials(&credentials.username, conn)
        .await
        .map_err(|e| ContentError::UnexpectedError(anyhow!(e)))?
    {
        user_id = Some(stored_uid);
        db_password_hash = stored_hash;
    }

    let parsed_hash = PasswordHash::new(db_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(|e| ContentError::UnexpectedError(anyhow!(e)))?;
    // hashing is likely to block for some time; yield
    let str_hash = Secret::from(parsed_hash.serialize().to_string());
    spawn_blocking_with_tracing(move || verify_password_hash(str_hash, credentials.password))
        .await
        .context("Failed to spawn blocking hash verifier")
        .map_err(|e| ContentError::UnexpectedError(anyhow!(e)))??;

    user_id.ok_or_else(|| ContentError::AuthError(anyhow!("invalid credentials")))
}

#[tracing::instrument(name = "Verify password hash", skip(expected_hash, password))]
fn verify_password_hash(
    expected_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), ContentError> {
    let parsed_hash = PasswordHash::new(expected_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(|e| ContentError::UnexpectedError(anyhow!(e)))?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
        .context("invalid credentials")
        .map_err(|_| ContentError::AuthError(anyhow!("invalid credentials")))?;

    Ok(())
}

#[tracing::instrument(name = "Get stored credentials", skip(username, conn))]
async fn get_stored_credentials(
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

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header is missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid Utf8 string.")?;
    let base64encoded_segment = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme is not 'Basic'.")?;
    let decoded_bytes = base64::decode(base64encoded_segment)
        .context("Failed to base64-decode 'Basic' credentials.")?;
    let decoded_credentials = String::from_utf8(decoded_bytes)
        .context("The decoded credential string is not valid Utf8.")?;

    // Split into two segments
    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A username must be provided"))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| anyhow::anyhow!("A password must be proveded"))?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}

#[derive(thiserror::Error)]
pub enum ContentError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("{0}")]
    ValidationError(String),
}

impl IntoResponse for ContentError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::AuthError(_) => {
                tracing::error!("failed to autheticate poster");
                (
                    StatusCode::UNAUTHORIZED,
                    [("WWW-Authenticate", "Basic realm=\"publish\"")],
                )
                    .into_response()
            }
            Self::UnexpectedError(e) => {
                tracing::info!("an unexpected error occured");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
            }
            Self::ValidationError(s) => {
                tracing::info!("a validation error occurred: {s:?}");
                (StatusCode::BAD_REQUEST, s).into_response()
            }
        }
    }
}

impl From<String> for ContentError {
    fn from(s: String) -> Self {
        Self::ValidationError(s)
    }
}

impl std::fmt::Debug for ContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
