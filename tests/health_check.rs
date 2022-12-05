use std::net::TcpListener;

use librhodos::{
    migration::{self, DbUri},
    serve, settings,
};
use slog::{o, Drain, Level, Logger};

extern crate slog;
extern crate slog_async;
extern crate slog_term;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn spawn_app() -> String {
    let global_config = settings::Settings::new()
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e.to_string());
            return;
        })
        .unwrap();
    let db_name = global_config.database.db_name.to_string();
    let db_uri = DbUri {
        full: format!("postgres://postgres:password@localhost:5432/{}", db_name),
        path: "postgres://postgres:password@localhost:5432".to_string(),
        db_name: db_name,
    };
    // Create a drain hierarchy
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    // Get root logger
    let logger: Logger = Logger::root(
        drain.filter_level(Level::Debug).fuse(),
        o!(
            "version" => env!("CARGO_PKG_VERSION"),
            "env" => global_config.env.to_string(),
        ),
    );
    let _ = migration::initialize_and_migrate_database(&db_uri, &logger)
        .await
        .map_err(|err_str| {
            eprintln!("{}", err_str);
            return;
        });

    let router = librhodos::get_router(&db_uri.full, &global_config)
        .await
        .map_err(|e| {
            eprintln!("{}", e.to_string());
            return;
        })
        .unwrap();

    let listener = TcpListener::bind("0.0.0.0:0")
        .map_err(|e| {
            eprintln!("unable to parse local address: {}", e.to_string());
            return;
        })
        .unwrap();
    let port = listener.local_addr().unwrap().port();

    let _ = tokio::spawn(serve(router, listener));

    format!("http://localhost:{}", port)
}
