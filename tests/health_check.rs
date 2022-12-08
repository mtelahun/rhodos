use once_cell::sync::Lazy;
use std::net::TcpListener;
use tokio_postgres::NoTls;
use uuid::Uuid;

use librhodos::telemetry::{get_subscriber, init_subscriber};
use librhodos::{
    migration::{self, DbUri},
    serve, settings,
};
use secrecy::Secret;

struct TestState {
    app_address: String,
    db_name: String,
}

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
    let state = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", state.app_address))
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn add_user_valid_form_data_200() {
    // Arrange
    let state = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=Sonja%20Hemphill&email=sonja%40lowdelhi.example";
    let response = client
        .post(&format!("{}/user", state.app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(
        200,
        response.status().as_u16(),
        "valid form data return 200 OK"
    );
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(r#"SELECT name, email FROM "user";"#, &[])
        .await
        .expect("query to fetch row failed");

    assert!(!row.is_empty(), "one record has been created");
    let name: &str = row[0].get(0);
    let email: &str = row[0].get(1);
    assert_eq!(name, "Sonja Hemphill");
    assert_eq!(email, "sonja@lowdelhi.example");
}

#[tokio::test]
async fn add_user_missing_form_data_400() {
    // Arrange
    let state = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=Sonja%20Hemphill", "missing email"),
        ("email=sonja%40lowdelhi.example", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_data, msg_err) in test_cases {
        // ACt
        let response = client
            .post(&format!("{}/user", state.app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_data)
            .send()
            .await
            .expect("Failed to execute request");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "{} returns 400 Bad Client Request",
            msg_err,
        );
    }
}

async fn connect_to_db(db_name: &str) -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host=localhost user=postgres password=password dbname={}",
            db_name
        ),
        NoTls,
    )
    .await
    .expect("Unable to connect test database");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e)
        }
    });

    client
}

async fn spawn_app() -> TestState {
    // Initialize tracing stack
    Lazy::force(&TRACING);

    let mut global_config = settings::Settings::new(None, None)
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e.to_string());
            return;
        })
        .unwrap();
    global_config.database.db_host = "localhost".to_string();
    global_config.database.db_port = 5432;
    global_config.database.db_user = "postgres".to_string();
    global_config.database.db_password = Secret::from("password".to_string());
    global_config.database.db_name = Uuid::new_v4().to_string();
    let db_uri = DbUri {
        full: global_config.database.connection_string(),
        path: global_config.database.connection_string_no_db(),

        // randomize db name for test isolation
        db_name: global_config.database.db_name.clone(),
    };
    let _ = migration::initialize_and_migrate_database(&db_uri)
        .await
        .map_err(|err_str| {
            eprintln!("{}", err_str);
            return;
        });

    let router = librhodos::get_router(&global_config)
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

    TestState {
        app_address: format!("http://localhost:{}", port),
        db_name: global_config.database.db_name,
    }
}
