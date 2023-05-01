use oxide_auth::primitives::registrar::EncodedClient;
use serde::Deserialize;
use serde_json::Value;
use tokio_postgres::types::Json;
use tracing::debug;

use crate::helpers::{connect_to_db, spawn_app, TestState};

const READ_SCOPES: &[&str] = &[
    "read:accounts",
    "read:blocks",
    "read:bookmarks",
    "read:favourites",
    "read:filters",
    "read:follows",
    "read:lists",
    "read:mutes",
    "read:notifications",
    "read:search",
    "read:statuses",
];

const WRITE_SCOPES: &[&str] = &[
    "write:accounts",
    "write:blocks",
    "write:bookmarks",
    "write:conversations",
    "write:favourites",
    "write:filters",
    "write:follows",
    "write:media",
    "write:lists",
    "write:mutes",
    "write:notifications",
    "write:reports",
    "write:statuses",
];

const FOLLOW_SCOPES: &[&str] = &[
    "read:blocks",
    "write:blocks",
    "read:follows",
    "write:follows",
    "read:mutes",
    "write:mutes",
];

#[derive(Clone, Debug, Deserialize)]
struct Application {
    id: String,
    name: String,
    website: Option<String>,
    vapid_key: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[tokio::test]
async fn client_app_invalid_request() {
    // Arrange
    let state = spawn_app().await;
    let client = reqwest::Client::new();
    let invalid_cases = [
        (
            serde_json::json!({
                "redirect_uris": format!("{}/non-existant", &state.app_address),
                "scopes": "read",
                "website": &state.app_address.to_owned(),
            }),
            "missing client_name",
        ),
        (
            serde_json::json!({
                "client_name": "Test Harness",
                "scopes": "read",
                "website": &state.app_address.to_owned(),
            }),
            "missing redirect_uri",
        ),
    ];

    for (form, msg) in invalid_cases {
        // Act
        let url = format!("{}/api/v1/apps", &state.app_address);
        println!("Request URL: {}", url);
        let response = client
            .post(url)
            .form(&form)
            .send()
            .await
            .expect("request to client api failed");

        // Assert
        assert_eq!(
            response.status().as_u16(),
            422,
            "{} returns 422 Unprocessable Entity",
            msg
        )
    }
}

#[tokio::test]
async fn happy_path_client_app() {
    // Arrange
    let state = spawn_app().await;
    let cases = [
        (
            serde_json::json!({
                "client_name": "Test Harness1",
                "redirect_uris": format!("{}/home", &state.app_address),
                "website": "https://example.org",
            }),
            "missing scopes",
        ),
        (
            serde_json::json!({
                "client_name": "Test Harness2",
                "redirect_uris": format!("{}/home", &state.app_address),
                "scopes": "read",
            }),
            "missing website",
        ),
        (
            serde_json::json!({
                "client_name": "Test Harness3",
                "redirect_uris": format!("{}/home", &state.app_address),
            }),
            "missing all optional parameters",
        ),
        (
            serde_json::json!({
                "client_name": "Test Harness4",
                "redirect_uris": format!("{}/home", &state.app_address),
                "website": "https://example.org",
                "scopes": "read write push",
            }),
            "all optional parameters present",
        ),
    ];

    for (form, msg) in cases {
        // Act/Assert
        let _ = launch_request_check_response(&state, &form, &msg).await;
    }
}

#[tokio::test]
async fn duplicate_client_app_different_id() {
    // Arrange
    let state = spawn_app().await;
    let form = serde_json::json!({
        "client_name": "Test Harness",
        "redirect_uris": format!("{}/home", &state.app_address),
        "website": "https://example.org",
    });

    // Act/Assert - 1
    let application =
        launch_request_check_response(&state, &form, "client app registration #1").await;

    // Act/Assert - 2
    let application2 =
        launch_request_check_response(&state, &form, "client app registration #2").await;

    assert_ne!(
        application2.client_id, application.client_id,
        "the Ids of the two identical applications are different"
    );
}

#[tokio::test]
async fn client_app_default_scopes() {
    // Arrange
    let form = serde_json::json!({
        "client_name": "Test Harness",
        "redirect_uris": "https://example.org/authorization",
    });

    // Act / Assert
    common_client_app_scopes(&form, READ_SCOPES).await;
}

#[tokio::test]
async fn client_app_read_scopes() {
    // Arrange
    let form = serde_json::json!({
        "client_name": "Test Harness",
        "redirect_uris": "https://example.org/authorization",
        "scopes": "read",
    });

    // Act / Assert
    common_client_app_scopes(&form, READ_SCOPES).await;
}

#[tokio::test]
async fn client_app_write_scopes() {
    // Arrange
    let form = serde_json::json!({
        "client_name": "Test Harness",
        "redirect_uris": "https://example.org/authorization",
        "scopes": "write",
    });

    // Act / Assert
    common_client_app_scopes(&form, WRITE_SCOPES).await;
}

#[tokio::test]
async fn client_app_follow_scopes() {
    // Arrange
    let form = serde_json::json!({
        "client_name": "Test Harness",
        "redirect_uris": "https://example.org/authorization",
        "scopes": "follow",
    });

    // Act / Assert
    common_client_app_scopes(&form, FOLLOW_SCOPES).await;
}

async fn common_client_app_scopes(form: &Value, check_scopes: &[&str]) {
    // Arrange
    let state = spawn_app().await;

    // Act
    let response = state.register_client_app(form).await;

    // Assert - 1
    assert_eq!(
        response.status().as_u16(),
        200,
        "{} returns 200 Ok",
        "successful registration of client with default scope"
    );

    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(
            r#"SELECT id, name, encoded_client
                FROM "client_app" 
                WHERE name=$1 
                ORDER BY id desc 
                LIMIT 1;"#,
            &[&form.get("client_name").unwrap().as_str()],
        )
        .await
        .expect("query to fetch row failed");
    assert!(!row.is_empty(), "one record has been created");

    // Assert - 2
    let value = row[0].get::<_, Json<EncodedClient>>(2);
    let str_scopes = value.0.default_scope.to_string();
    debug!("scopes list: {}", str_scopes);

    let vec_str_scopes: Vec<&str> = str_scopes.split(char::is_whitespace).collect();
    assert_eq!(
        vec_str_scopes.len(),
        check_scopes.len(),
        "the number of scopes in the client's default scope match the number in the check list"
    );

    for s in check_scopes {
        assert!(str_scopes.contains(s), "tested scope includes {}", s);
    }
}

async fn launch_request_check_response(state: &TestState, form: &Value, msg: &str) -> Application {
    // Act
    let response = state.register_client_app(&form).await;

    // Assert - 1
    assert_eq!(response.status().as_u16(), 200, "{} returns 200 Ok", msg);

    // Assert - 2
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(
            r#"SELECT id, client_id, name, website
                FROM "client_app" 
                WHERE name=$1 
                ORDER BY id desc 
                LIMIT 1;"#,
            &[&form.get("client_name").unwrap().as_str()],
        )
        .await
        .expect("query to fetch row failed");

    assert!(!row.is_empty(), "one record has been created");
    let client_id: &str = row[0].get(1);
    let name: &str = row[0].get(2);
    let website: &str = row[0].get(3);

    // Assert - 3
    let res1 = response.text().await.expect("failed to read response");
    println!("response: {res1}");
    let app: Application = serde_json::from_str(res1.as_str()).unwrap();
    let application = app.clone();

    assert_eq!(application.name, name, "application name is correct");
    assert!(!application
        .client_id
        .clone()
        .unwrap_or(String::from(""))
        .is_empty());
    assert_eq!(
        application.client_id.unwrap_or(String::from("")),
        client_id,
        "client Id is correct"
    );
    assert!(!application
        .client_secret
        .clone()
        .unwrap_or(String::from(""))
        .is_empty());
    if form.get("website").is_some() {
        assert_eq!(
            application.website.unwrap_or(String::from("")),
            website,
            "application website is correct"
        );
    }
    assert!(!application.id.is_empty());
    assert!(!application.vapid_key.is_empty());

    app
}
