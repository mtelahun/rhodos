use sea_orm::Database;
use slog::{Logger, info};

pub mod db;
pub mod entities;
pub mod migration;
pub mod migrator;
mod routes;

pub async fn run(db_url: &str, logger: &Logger) {
    let db = Database::connect(db_url).await.unwrap();

    let app = routes::create_routes(db);

    let listen_addr = "0.0.0.0:5000";
    info!(logger, "Listening on {}", listen_addr);
    axum::Server::bind(&listen_addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap()

}

#[cfg(test)]
mod tests {

}
