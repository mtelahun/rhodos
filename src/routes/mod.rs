use axum::{
    routing::get,
    Router, Extension
};
use sea_orm::DatabaseConnection;

pub mod index;

use index::index;

pub fn create_routes(db: DatabaseConnection) -> Router {
    Router::new()
        .route("/", get(index))
        .layer(Extension(db))
}