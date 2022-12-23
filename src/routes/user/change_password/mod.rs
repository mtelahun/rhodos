use anyhow;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

use crate::error::error_chain_fmt;

pub mod get;
pub mod post;

#[derive(thiserror::Error)]
pub enum ResetError {
    #[error("new password is an empty string")]
    ConfirmPasswordFail(String),
    #[error("current password does not match")]
    CurrentPasswordFail(String),
    #[error("new password is an empty string")]
    EmptyPasswordFail(String),
    #[error("does this user have a session?")]
    NoSession(#[source] anyhow::Error),
    #[error("an unexpected error occurred")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for ResetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for ResetError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::ConfirmPasswordFail(s) => {
                tracing::error!("{s}");
                Redirect::to("/user/change-password").into_response()
            }
            Self::CurrentPasswordFail(s) => {
                tracing::error!("{s}");
                Redirect::to("/user/change-password").into_response()
            }
            Self::EmptyPasswordFail(s) => {
                tracing::error!("{s}");
                Redirect::to("/user/change-password").into_response()
            }
            Self::NoSession(e) => {
                tracing::error!("session not found: {}", e.to_string());
                (StatusCode::from_u16(303).unwrap(), Redirect::to("/login")).into_response()
            }
            Self::UnexpectedError(e) => {
                tracing::error!("an unexpected error occurred during password change: {e}");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}
