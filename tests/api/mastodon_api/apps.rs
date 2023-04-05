use serde::Deserialize;
use serde_json::Value;

use crate::helpers::{connect_to_db, spawn_app, TestState};

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

async fn launch_request_check_response(state: &TestState, form: &Value, msg: &str) -> Application {
    // Act
    let response = state.register_client_app(&form).await;

    // Assert - 1
    assert_eq!(response.status().as_u16(), 200, "{} returns 200 Ok", msg);

    // Assert - 2
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(
            r#"SELECT id, client_id, client_secret, name, website 
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
    let client_secret: &str = row[0].get(2);
    let name: &str = row[0].get(3);
    let website: &str = row[0].get(4);

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
    assert_eq!(
        application.client_secret.unwrap_or(String::from("")),
        client_secret,
        "client secret is correct"
    );
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
