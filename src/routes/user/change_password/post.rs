use axum::{
    extract::{Host, State},
    response::Redirect,
    Form,
};
use axum_sessions::extractors::ReadableSession;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use tower_cookies::Cookies;

use super::ResetError;
use crate::{
    authentication::{change_password, AuthError},
    cookies::{set_flash_cookie, FlashCookieType},
    error::user_id_from_session_r,
    routes::{get_db_from_host, AppState},
};

#[derive(Debug, Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    password: Secret<String>,
    confirm_password: Secret<String>,
}

pub async fn change(
    Host(host): Host,
    State(state): State<AppState>,
    cookies: Cookies,
    session: ReadableSession,
    Form(form): Form<FormData>,
) -> Result<Redirect, ResetError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| ResetError::UnexpectedError(e.into()))?;

    // These are obvious errors
    if form.password.expose_secret().is_empty() {
        set_flash_cookie(&cookies, FlashCookieType::PasswordResetEmpty, &state.domain);
        return Err(ResetError::EmptyPasswordFail(
            "empty password string".to_string(),
        ));
    } else if form.confirm_password.expose_secret() != form.password.expose_secret() {
        set_flash_cookie(
            &cookies,
            FlashCookieType::PasswordResetMismatch,
            &state.domain,
        );
        return Err(ResetError::ConfirmPasswordFail(
            "new password mismatch".to_string(),
        ));
    }

    let user_id = user_id_from_session_r(&session).await?;

    change_password(user_id, form.current_password, form.password, &conn)
        .await
        .map_err(|e| match e {
            AuthError::CurrentPasswordFail(_) => {
                set_flash_cookie(
                    &cookies,
                    FlashCookieType::PasswordResetCurrent,
                    &state.domain,
                );
                ResetError::CurrentPasswordFail(e.to_string())
            }
            _ => ResetError::UnexpectedError(e.into()),
        })?;

    set_flash_cookie(&cookies, FlashCookieType::PasswordResetOk, &state.domain);
    Ok(Redirect::to("/user/change-password"))
}
