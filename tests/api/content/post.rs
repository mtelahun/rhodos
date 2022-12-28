use chrono::Utc;

use crate::{
    content::generate_random_data,
    helpers::{assert_is_redirect_to, connect_to_db, spawn_app},
};

#[tokio::test]
pub async fn invalid_json_is_bad_request_422() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    let invalid_cases = vec![(
        serde_json::json!({
            "content": {}
        }),
        "missing content",
    )];

    for (case, desc) in invalid_cases {
        // Act
        let response = state.post_content(&case).await;

        // Assert
        assert_eq!(
            response.status().as_u16(),
            422,
            "{} returns 422 Bad Request",
            desc
        );
    }
}

#[tokio::test]
pub async fn logical_error_in_json_field_is_bad_request_400() {
    // Arrange
    let state = spawn_app().await;
    let msg = generate_random_data(501);
    state.login_as(&state.test_user_user).await;
    let invalid_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "",
                }
            }),
            "missing text",
        ),
        (
            serde_json::json!({
                "content": {
                    "text": msg,
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
            response.status().as_u16(),
            400,
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
    let account_id = state.test_user_superadmin.account_id;
    // Login
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let msg = generate_random_data(500);
    let body = serde_json::json!({
        "content": {
            "text": msg,
        }
    });
    let response = state.post_content(&body).await;

    // Assert
    assert_eq!(
        response.status().as_u16(),
        200,
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
    // Login
    state.login_as(&state.test_user_superadmin).await;
    // Sabotage the database
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"ALTER TABLE content DROP COLUMN "body";"#, &[])
        .await
        .expect("query to alter content table failed");

    // Act
    let body = serde_json::json!({
        "content": {
            "text": "This is a test.",
        }
    });
    let response = state.post_content(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}

#[tokio::test]
async fn request_missing_authorization_redirect_303() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = serde_json::json!({
        "content": {
            "text": "This is a random thought.",
        }
    });
    let response = state.post_content(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}
