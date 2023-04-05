use axum::{http::StatusCode, response::IntoResponse};

use crate::error::error_chain_fmt;

pub mod apps;

#[derive(thiserror::Error)]
pub enum ApiError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
    #[error("{0}")]
    ValidationError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::UnexpectedError(e) => {
                tracing::info!("unexpected error");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
            }
            Self::ValidationError(s) => {
                tracing::info!("validation error {s:?}");
                (StatusCode::BAD_REQUEST, s).into_response()
            }
        }
    }
}

impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
