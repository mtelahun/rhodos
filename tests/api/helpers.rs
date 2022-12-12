use librhodos::startup;
use once_cell::sync::Lazy;
use tokio_postgres::NoTls;
use uuid::Uuid;

use librhodos::telemetry::{get_subscriber, init_subscriber};
use librhodos::{
    migration::{self, DbUri},
    serve, settings,
};
use secrecy::Secret;

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

pub struct TestState {
    pub app_address: String,
    pub db_name: String,
    pub port: u16,
}

impl TestState {
    pub async fn post_user(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/user", self.app_address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_confirmation_links(&self, recepient: &String) -> ConfirmationLinks {
        let client = reqwest::Client::new();
        let response = client
            .get(&format!(
                "http://localhost:8025/api/v2/search?kind=to&query={}",
                recepient
            ))
            .send()
            .await
            .expect("Failed to execute mailhog request");

        assert_eq!(
            response.status().as_u16(),
            200,
            "query of mailhog queue returns 200 Ok",
        );

        let body: serde_json::Value =
            serde_json::from_str(response.text().await.unwrap().as_str()).unwrap();
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "localhost");
            println!("confirmation link={}", confirmation_link);
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        use mail_parser::*;
        let raw = body["items"][0]["Raw"]["Data"].as_str().unwrap();
        let message = Message::parse(raw.as_bytes()).unwrap();
        let html = get_link(&message.body_html(0).unwrap());
        let mut text = get_link(&message.body_text(0).unwrap());
        text.set_port(Some(self.port)).unwrap();

        ConfirmationLinks { html, text }
    }
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

pub async fn connect_to_db(db_name: &str) -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(
        &format!(
            "host=localhost user=postgres password=password dbname={}",
            db_name
        ),
        NoTls,
    )
    .await
    .expect("Unable to connect to test database");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e)
        }
    });

    client
}

pub async fn spawn_app() -> TestState {
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

    let (router, listener) =
        startup::build(&mut global_config, Some("0.0.0.0:0".to_string())).await;
    let port = listener.local_addr().unwrap().port();

    let _ = tokio::spawn(serve(router, listener));

    TestState {
        app_address: format!("http://localhost:{}", port),
        db_name: global_config.database.db_name,
        port: port,
    }
}
