use axum::{
    extract::{Host, State},
    response::Redirect,
};
use tower_cookies::Cookies;

use crate::{
    cookies::{set_flash_cookie, FlashCookieType},
    error::RhodosError,
    routes::{get_db_from_host, AppState},
    session_state::AuthContext,
};

#[tracing::instrument(
    name = "Logout"
    skip(host, state, cookies),
)]
pub async fn logout(
    Host(host): Host,
    State(state): State<AppState>,
    // mut session: WritableSession,
    mut auth: AuthContext,
    cookies: Cookies,
) -> Result<Redirect, RhodosError> {
    let hst = host.to_string();
    let _conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| RhodosError::Unexpected(anyhow::anyhow!(e)))?;

    auth.logout().await;

    // session.destroy();
    set_flash_cookie(&cookies, FlashCookieType::LogoutOk, &state.domain);
    Ok(Redirect::to("/login"))
}
