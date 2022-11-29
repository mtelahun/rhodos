use sea_orm::Database;
use slog::{Logger, info};

pub mod settings;
pub mod db;
pub mod entities;
pub mod migration;
pub mod migrator;
mod routes;

pub async fn run(db_url: &str, logger: &Logger) -> Result<(), String> {
    let db = match Database::connect(db_url)
        .await {
            Ok(conn) => conn,
            Err(e) => return Err(e.to_string()),
        };

    let app = routes::create_routes(db);

    let listen_addr = "0.0.0.0:5000";
    info!(logger, "Listening on {}", listen_addr);
    axum::Server::bind(&listen_addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[cfg(test)]
mod tests {

}
