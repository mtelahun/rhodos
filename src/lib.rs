use axum::Router;
use sea_orm::{Database, DatabaseConnection};
use settings::Settings;
use std::net::TcpListener;

pub mod authentication;
pub mod db;
pub mod domain;
pub mod email_client;
pub mod entities;
pub mod error;
pub mod migration;
pub mod migrator;
pub mod routes;
// pub mod session_state;
pub mod settings;
pub mod smtp_client;
pub mod startup;
pub mod telemetry;

pub const APP_NAME: &str = "rhodos";

pub struct AppBaseUrl(pub String);

pub async fn get_database_connection(
    global_config: &Settings,
) -> Result<DatabaseConnection, String> {
    let db = match Database::connect(global_config.database.connection_options()).await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!(
                "create_routes: unable to connect to database {}: {}",
                global_config.database.db_name, e,
            ))
        }
    };

    Ok(db)
}

pub async fn get_router(global_config: &Settings) -> Result<Router, String> {
    let db = match Database::connect(global_config.database.connection_options()).await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!(
                "get_router: unable to connect to database {}: {}",
                global_config.database.db_name, e,
            ))
        }
    };

    let app = routes::create_routes(db, global_config).await.unwrap();

    Ok(app)
}

pub async fn serve(app: Router, listener: TcpListener) {
    axum::Server::from_tcp(listener)
        .map_err(|e| eprintln!("{}", e))
        .unwrap()
        .serve(app.into_make_service())
        .await
        .unwrap();
}
