use axum::Router;
use settings::Settings;
use std::net::TcpListener;

pub mod db;
pub mod entities;
pub mod migration;
pub mod migrator;
pub mod routes;
pub mod settings;
pub mod telemetry;

pub const APP_NAME: &str = "rhodos";

pub async fn get_router(global_config: &Settings) -> Result<Router, String> {
    let app = routes::create_routes(global_config).await.unwrap();

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
