use settings::Settings;
use slog::{info, Logger};

pub mod db;
pub mod entities;
pub mod migration;
pub mod migrator;
mod routes;
pub mod settings;

pub async fn run(db_url: &str, logger: &Logger, global_config: &Settings) -> Result<(), String> {
    let app = routes::create_routes(db_url, global_config).await;

    let listen_addr = "0.0.0.0:5000";
    info!(logger, "Listening on {}", listen_addr);
    axum::Server::bind(&listen_addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {}
