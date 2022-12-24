use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use secrecy::ExposeSecret;
use uuid::Uuid;

use crate::helpers::{connect_to_db, spawn_app};

#[tokio::test]
pub async fn invalid_json_is_bad_request_422() {
    // Arrange
    let state = spawn_app().await;
    let invalid_cases = vec![
        (
            serde_json::json!({
                "content": {}
            }),
            "missing content",
        ),
        (
            serde_json::json!({
                "content": {
                    "text": "This is a post",
                }
            }),
            "missing publisher",
        ),
    ];

    for (case, desc) in invalid_cases {
        // Act
        let response = state.post_content(&case).await;

        // Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            "{} returns 400 Bad Request",
            desc
        );
    }
}

#[tokio::test]
pub async fn logical_error_in_json_field_is_bad_request_400() {
    // Arrange
    let state = spawn_app().await;
    let msg = generate_random_data(501);
    let invalid_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "",
                    "publisher_id": 0,
                }
            }),
            "missing text",
        ),
        (
            serde_json::json!({
                "content": {
                    "text": msg,
                    "publisher_id": 0,
                }
            }),
            "post greater than 500 chars",
        ),
    ];

    for (case, desc) in invalid_cases {
        // Act
        let response = state.post_content(&case).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "{} returns 400 Bad Request",
            desc
        );
    }
}

#[tokio::test]
pub async fn happy_path_less_than_501_chars_is_ok_200() {
    // Arrange
    let state = spawn_app().await;
    let client = connect_to_db(&state.db_name.clone()).await;
    let account_id = state.test_user.account_id;

    // Act
    let msg = generate_random_data(500);
    let body = serde_json::json!({
        "content": {
            "text": msg,
            "publisher_id": account_id,
        }
    });
    let response = state.post_content(&body).await;

    // Assert
    assert_eq!(
        200,
        response.status().as_u16(),
        "post data less than/equal to 500 chars returns 200 Ok"
    );

    // Retrive post and compare
    let row = client
        .query_one("SELECT publisher_id,body,updated_at FROM content;", &[])
        .await
        .expect("query to retrieve just added content failed");
    assert!(!row.is_empty());
    let publisher_id: i64 = row.get(0);
    let database_body: String = row.get(1);
    let timestamp: chrono::NaiveDateTime = row.get(2);
    assert_eq!(
        publisher_id, account_id,
        "the publisher is the just added user"
    );
    assert_eq!(database_body, msg, "the post contents match");
    assert!(
        Utc::now()
            .naive_utc()
            .signed_duration_since(timestamp)
            .num_minutes()
            < 1,
        "timestamp on the post is less than one minute old"
    );
}

#[tokio::test]
async fn post_content_fails_if_fatal_db_err() {
    // Arrange
    let state = spawn_app().await;
    let client = connect_to_db(&state.db_name.clone()).await;
    let account_id: i64 = state.test_user.account_id;
    // Sabotage the database
    client
        .execute(r#"ALTER TABLE content DROP COLUMN "body";"#, &[])
        .await
        .expect("query to alter content table failed");

    // Act
    let body = serde_json::json!({
        "content": {
            "text": "This is a test.",
            "publisher_id": account_id,
        }
    });
    let response = state.post_content(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}

#[tokio::test]
async fn request_missing_authorization_rejected_401() {
    // Arrange
    let state = spawn_app().await;
    let account_id = state.test_user.account_id;

    // Act
    let body = serde_json::json!({
        "content": {
            "text": "This is a random thought.",
            "publisher_id": account_id,
        }
    });
    let response = reqwest::Client::new()
        .post(&format!("{}/content", state.app_address))
        .json(&body)
        .send()
        .await
        .expect("Failed to execute request");

    // Assert
    assert_eq!(
        response.status().as_u16(),
        401,
        "missing creds return 401 Unauthorized"
    );
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#,
        "Basic authentication"
    )
}

#[tokio::test]
async fn non_existing_user_is_401_unauthorized() {
    // Arrange
    let state = spawn_app().await;
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/content", &state.app_address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "content": {
                "text": "Some text",
                "publisher_id": 1,
            }
        }))
        .send()
        .await
        .expect("Failed to post reqwest");

    // Assert
    assert_eq!(
        response.status().as_u16(),
        401,
        "The attempt to create content is rejected; it couldn't authenticate the user"
    );
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}

#[tokio::test]
async fn invalid_password_is_401_unauthorized() {
    // Arrange
    let state = spawn_app().await;
    let username = &state.test_user.username;
    let password = Uuid::new_v4().to_string();
    assert_ne!(
        password,
        state.test_user.password.expose_secret().to_string(),
        "random password does not equal actual password"
    );

    // Act
    let response = reqwest::Client::new()
        .post(&format!("{}/content", &state.app_address))
        .basic_auth(username, Some(password))
        .json(&serde_json::json!({
            "content": {
                "text": "Some text",
                "publisher_id": 1,
            }
        }))
        .send()
        .await
        .expect("Failed to post reqwest");

    // Assert
    assert_eq!(
        response.status().as_u16(),
        401,
        "The attempt to create content is rejected; it couldn't authenticate the user"
    );
    assert_eq!(
        response.headers()["WWW-Authenticate"],
        r#"Basic realm="publish""#
    );
}

fn generate_random_data(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}
