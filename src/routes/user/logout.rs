use axum::{
    extract::{Host, State},
    response::Redirect,
};
use axum_sessions::extractors::WritableSession;
use tower_cookies::Cookies;

use crate::{
    cookies::set_flash_cookie,
    error::RhodosError,
    routes::{get_db_from_host, AppState},
};

#[tracing::instrument(
    name = "Logout"
    skip(host, state, session, cookies),
)]
pub async fn logout(
    Host(host): Host,
    State(state): State<AppState>,
    mut session: WritableSession,
    cookies: Cookies,
) -> Result<Redirect, RhodosError> {
    let hst = host.to_string();
    let _conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| RhodosError::Unexpected(anyhow::anyhow!(e)))?;

    session.destroy();
    set_flash_cookie(&cookies, "logout_ok");
    Ok(Redirect::to("/login"))
}
