use anyhow::anyhow;
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Form,
};
use axum_sessions::extractors::WritableSession;
use secrecy::Secret;
use serde::Deserialize;
use tower_cookies::Cookies;

use crate::{
    authentication::{validate_credentials, AuthError, Credentials},
    cookies::{set_flash_cookie, FlashCookieType},
    error::{error_chain_fmt, TenantMapError},
    routes::{get_db_from_host, AppState},
};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[tracing::instrument(
    skip(host, state, form),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn login(
    Host(host): Host,
    State(state): State<AppState>,
    mut session: WritableSession,
    cookies: Cookies,
    Form(form): Form<FormData>,
) -> Result<Redirect, LoginError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => LoginError::UnexpectedError(anyhow!(s)),
        TenantMapError::UnexpectedError(s) => LoginError::UnexpectedError(anyhow!(s)),
    })?;

    let credentials = Credentials {
        username: form.username,
        password: form.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));

    let user_id = validate_credentials(credentials, &conn)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(_) => {
                set_flash_cookie(&cookies, FlashCookieType::InvalidCreds);
                LoginError::AuthError(e.into())
            }
            _ => LoginError::UnexpectedError(e.into()),
        })?;
    session.regenerate();
    session.insert("user_id", user_id).map_err(|e| {
        set_flash_cookie(&cookies, FlashCookieType::SessionSetupError);
        LoginError::UnexpectedError(anyhow!(e))
    })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    Ok(Redirect::to("/admin/dashboard"))
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),
    #[error("An unexpected error occurred.")]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::AuthError(_) => {
                tracing::error!("failed to authenticate user");
                // let cookie = Cookie::build("_flash", "invalid_creds")
                //     .max_age(Duration::seconds(0))
                //     .finish();
                // eprintln!("\n\nconstructed: {}", cookie.to_string());
                // eprintln!("header value: {:?}", HeaderValue::from_str(&cookie.to_string()).unwrap());
                (
                    StatusCode::from_u16(303).unwrap(),
                    // [(header::SET_COOKIE, HeaderValue::from_str(&cookie.to_string()).unwrap())],
                    Redirect::to("/login"),
                )
                    .into_response()
            }
            Self::UnexpectedError(e) => {
                tracing::error!("an unexpected error occured");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
            }
        }
    }
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
