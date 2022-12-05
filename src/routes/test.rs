use std::sync::Arc;

use crate::entities::prelude::*;
use crate::routes::get_db_from_host;
use axum::{extract::Host, http::StatusCode, Extension};
use axum_macros::debug_handler;
use sea_orm::EntityTrait;

use super::AppState;

#[debug_handler]
pub async fn proxy(
    Host(host): Host,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<String, StatusCode> {
    let hst = host.to_string();
    println!("hst = {}", hst);
    let db = get_db_from_host(&hst, &state).await;
    match db {
        Ok(db) => {
            let rs = Microblog::find_by_id(1).one(&db).await;
            if let Ok(opt) = rs {
                if let Some(mb) = opt {
                    return Ok(mb.name);
                }
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(e) => Err(e),
    }
}
