use anyhow::Context;
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use sea_orm::{EntityTrait, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::NewUser,
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
    skip(state, body),
    fields(
        request_id = %Uuid::new_v4(),
        username = tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn create(
    Extension(user): Extension<NewUser>,
    Host(host): Host,
    State(state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<(), ContentError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => ContentError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => ContentError::UnexpectedError(anyhow::anyhow!(s)),
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

#[derive(thiserror::Error)]
pub enum ContentError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("{0}")]
    ValidationError(String),
}

impl IntoResponse for ContentError {
    fn into_response(self) -> axum::response::Response {
        match self {
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
