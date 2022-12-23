use anyhow::Context;
use axum::{
    extract::{Host, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use sea_orm::{EntityTrait, Set};
use secrecy::Secret;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    entities::content,
    error::{error_chain_fmt, TenantMapError},
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
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BodyData>,
) -> Result<(), ContentError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => ContentError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => ContentError::UnexpectedError(anyhow::anyhow!(s)),
    })?;

    let credentials = basic_authentication(&headers).map_err(ContentError::AuthError)?;
    let _user_id = validate_credentials(credentials, &conn)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => ContentError::AuthError(e.into()),
            _ => ContentError::UnexpectedError(e.into()),
        })?;
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
