use axum::response::IntoResponse;
use reqwest::StatusCode;
use sea_orm::DbErr;

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
