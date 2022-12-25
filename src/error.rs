use axum::{
    http::StatusCode,
    response::{IntoResponse, Redirect},
};

#[derive(thiserror::Error)]
pub enum RhodosError {
    #[error("An unexpected error occurred")]
    Unexpected(#[source] anyhow::Error),
}

impl std::fmt::Debug for RhodosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for RhodosError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Unexpected(e) => {
                tracing::error!("{}", e.to_string());
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    }
}

#[derive(thiserror::Error)]
pub enum TenantMapError {
    #[error("Tenant database not found: {0:?}")]
    NotFound(String),
    #[error("Unexpected error encountered: {0:?}")]
    UnexpectedError(String),
}

impl std::fmt::Debug for TenantMapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

#[derive(thiserror::Error)]
#[error("No valid session detected")]
pub struct SessionError(anyhow::Error);

impl std::fmt::Debug for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for SessionError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Session error: {}", self.0.to_string());
        Redirect::to("/login").into_response()
    }
}

pub fn error_chain_fmt(
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
