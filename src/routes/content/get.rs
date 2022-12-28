use axum::{
    extract::{Host, State},
    response::Html,
    Extension,
};
use serde::Serialize;

use crate::{
    domain::AppUser,
    routes::{get_db_from_host, AppState},
};

use super::ContentError;

#[derive(Debug, Serialize)]
pub struct FormData {
    pub content: String,
}

pub async fn form(
    Host(host): Host,
    State(state): State<AppState>,
    Extension(_user): Extension<AppUser>,
) -> Result<Html<&'static str>, ContentError> {
    let hst = host.to_string();
    let _conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| ContentError::UnexpectedError(e.into()))?;

    Ok(Html(include_str!("content.html")))
}
