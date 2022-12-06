use once_cell::sync::Lazy;
use std::net::TcpListener;

use librhodos::telemetry::{get_subscriber, init_subscriber};
use librhodos::{
    migration::{self, DbUri},
    serve, settings,
};
use secrecy::Secret;

// Ensure that the `tracing` stack is only initialized once
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "test=debug,tower_http=debug".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

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
    // Initialize tracing stack
    Lazy::force(&TRACING);

    let global_config = settings::Settings::new()
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e.to_string());
            return;
        })
        .unwrap();
    let db_name = global_config.database.db_name.to_string();
    let db_uri = DbUri {
        full: Secret::from(format!(
            "postgres://postgres:password@localhost:5432/{}",
            db_name
        )),
        path: Secret::from("postgres://postgres:password@localhost:5432".to_string()),
        db_name: db_name,
    };
    let _ = migration::initialize_and_migrate_database(&db_uri)
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
