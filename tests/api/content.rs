use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
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
        let response = state.content_post(&case).await;

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
        let response = state.content_post(&case).await;

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
pub async fn post_less_than_501_chars_is_ok_200() {
    // Arrange
    let state = spawn_app().await;
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(
            r#"INSERT INTO account(email) VALUES('test@mail.com');"#,
            &[],
        )
        .await
        .expect("query to add an account failed");
    let row = client
        .query_one("SELECT id FROM account WHERE email='test@mail.com';", &[])
        .await
        .expect("query to retrieve just added account failed");
    let account_id: i64 = row.get(0);

    // Act
    let msg = generate_random_data(500);
    let body = serde_json::json!({
        "content": {
            "text": msg,
            "publisher_id": account_id,
        }
    });
    let response = state.content_post(&body).await;

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
    client
        .execute(
            r#"INSERT INTO account(email) VALUES('test@mail.com');"#,
            &[],
        )
        .await
        .expect("query to add an account failed");
    let row = client
        .query_one("SELECT id FROM account WHERE email='test@mail.com';", &[])
        .await
        .expect("query to retrieve just added account failed");
    let account_id: i64 = row.get(0);
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
    let response = state.content_post(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}

#[tokio::test]
async fn request_missing_authorization_rejected_401() {
    // Arrange
    let state = spawn_app().await;
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(
            r#"INSERT INTO account(email) VALUES('test@mail.com');"#,
            &[],
        )
        .await
        .expect("query to add an account failed");
    let row = client
        .query_one("SELECT id FROM account WHERE email='test@mail.com';", &[])
        .await
        .expect("query to retrieve just added account failed");
    let account_id: i64 = row.get(0);

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

fn generate_random_data(len: usize) -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(len)
        .collect()
}
