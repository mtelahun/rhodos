use anyhow::{anyhow, Context};
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use axum_macros::debug_handler;
use axum_sessions::extractors::ReadableSession;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::entities::prelude::User;
use crate::error::error_chain_fmt;
use crate::routes::{get_db_from_host, AppState};

#[tracing::instrument(name = "Admin dashboard", skip(host, state, session))]
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

    let user_id: i64 = session.get::<i64>("user_id").unwrap_or(0);
    let user_name = if user_id != 0 {
        get_username(user_id, &conn)
            .await
            .map_err(AdminError::UnexpectedError)?
    } else {
        return Err(AdminError::SessionError(anyhow!(
            "unable to find user id in session store"
        )));
    };

    Ok(Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Admin dashboard</title>
    </head>
    <body>
        <p>Welcome {user_name}!</p>
    </body>
</html>"#
    )))
}

#[tracing::instrument(name = "Get username", skip(conn))]
async fn get_username(user_id: i64, conn: &DatabaseConnection) -> Result<String, anyhow::Error> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .context("Failed to retrieve a user record.")?
        .unwrap();

    Ok(model.name)
}

#[derive(thiserror::Error)]
pub enum AdminError {
    #[error("session creation failed")]
    SessionError(#[source] anyhow::Error),
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
