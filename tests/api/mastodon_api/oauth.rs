use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use secrecy::ExposeSecret;

use crate::helpers::spawn_app;

#[tokio::test]
async fn oauth_invalid_request() {
    // Arrange
    let state = spawn_app().await;
    let client = reqwest::Client::new();
    let invalid_cases = [
        (
            serde_json::json!({
                "client_secret": "bar"
            }),
            "missing client",
        ),
        (
            serde_json::json!({
                "client_id": "foo",
            }),
            "missing secret",
        ),
        (serde_json::json!({}), "both user and secret missing"),
    ];

    for (body, msg) in invalid_cases {
        // Act
        let response = client
            .post(&format!(
                "{}/api/v1/accounts/verify_credentials",
                &state.app_address
            ))
            .json(&body)
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .expect("Failed to get authorization");

        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "{} returns bad request error",
            msg
        )
    }
}

#[tokio::test]
async fn happy_path_oauth_get_authorization() {
    // Arrange
    let state = spawn_app().await;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "client_id": &state.test_user_user.username,
        "client_secret": &state.test_user_user.password.expose_secret()
    });

    // Act -1
    let response = client
        .post(&format!("{}/user/authorize", &state.app_address))
        .json(&body)
        .header(CONTENT_TYPE, "application/json")
        .send()
        .await
        .expect("Failed to get authorization");

    // Assert -1
    assert_eq!(
        response.status().as_u16(),
        200,
        "successfull authentication returns 200 OK"
    );

    // Act -2
    let token = response.text().await.expect("Failed to extract token");

    // Assert -2
    println!("token={}", token);
    assert!(false);

    let _response = client
        .get(&format!("{}/home", &state.app_address))
        .header(AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .expect("Failed to get home page");
}
