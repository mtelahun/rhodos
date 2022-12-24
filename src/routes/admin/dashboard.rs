use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use axum_macros::debug_handler;
use axum_sessions::extractors::ReadableSession;

use crate::{
    error::{error_chain_fmt, user_id_from_session_r, SessionError},
    orm,
    routes::{get_db_from_host, AppState},
};

#[tracing::instrument(name = "Admin dashboard", skip(state, session))]
#[debug_handler]
pub async fn admin_dashboard(
    session: ReadableSession,
    Host(host): Host,
    State(state): State<AppState>,
) -> Result<Html<String>, AdminError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| AdminError::UnexpectedError(e.into()))?;

    let user_id = user_id_from_session_r(&session).await?;
    let model = orm::get_user_model_by_id(user_id, &conn)
        .await
        .map_err(|e| AdminError::UnexpectedError(e.into()))?;
    let user_name = model.name;

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {user_name}!</p>
        <p>Actions:</p>
        <ol>
            <li><a href="/user/change-password">Change your password</a></li>
            <li>
                <form name="logoutForm" action="/user/logout" method="post">
                    <input type="submit" value="Logout" />
                </form>
            </li>
            </ol>
    </body>
</html>"#
    )))
}

#[derive(thiserror::Error)]
pub enum AdminError {
    #[error("session creation failed")]
    SessionError(#[from] SessionError),
    #[error("an unexpected error occurred")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for AdminError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::SessionError(e) => {
                tracing::error!("failed to instantiate session: {}", e.to_string());
                (StatusCode::from_u16(303).unwrap(), Redirect::to("/login")).into_response()
            }
            Self::UnexpectedError(e) => {
                tracing::error!("an unexpected error occurred during session creation");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}")).into_response()
            }
        }
    }
}
