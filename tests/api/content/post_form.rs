use chrono::Utc;

use crate::helpers::{assert_is_redirect_to, connect_to_db, spawn_app};

use super::generate_random_data;

#[tokio::test]
pub async fn content_form_invalid_bad_request_422() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_user).await;

    let invalid_cases = vec![(serde_json::json!({}), "empty request")];

    for (case, desc) in invalid_cases {
        // Act
        let response = state.post_content_form(&case).await;

        // Assert
        let status = response.status().as_u16();
        assert!(
            400 <= status && status <= 499,
            "{} returns Bad Request",
            desc
        );
    }
}

#[tokio::test]
pub async fn content_form_logical_bad_request_400() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.user_admin).await;
    let msg = generate_random_data(501);
    let invalid_cases = vec![
        (
            serde_json::json!({
                "content": ""
            }),
            "missing text",
        ),
        (
            serde_json::json!({ "content": msg }),
            "post greater than 500 chars",
        ),
    ];

    for (case, desc) in invalid_cases {
        // Act
        let response = state.post_content_form(&case).await;

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
pub async fn happy_path_content_form_redirect_303() {
    // Arrange
    let state = spawn_app().await;
    let account_id = state.test_user_user.account_id;
    state.login_as(&state.test_user_user).await;

    // Act
    let msg = generate_random_data(500);
    let body = serde_json::json!({ "content": msg });
    let response = state.post_content_form(&body).await;

    // Assert
    assert_eq!(
        response.status().as_u16(),
        303,
        "form data equal to 500 chars returns 303 Redirect"
    );

    // Retrive post and compare
    let client = connect_to_db(&state.db_name.clone()).await;
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
        "the publisher is the correct account"
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
    state.login_as(&state.test_user_user).await;
    // Sabotage the database
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"ALTER TABLE content DROP COLUMN "body";"#, &[])
        .await
        .expect("query to alter content table failed");

    // Act
    let body = serde_json::json!({
        "content": "This is a test."
    });
    let response = state.post_content_form(&body).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}

#[tokio::test]
async fn content_form_missing_authorization_redirect_303() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = serde_json::json!({
        "content": "This is my rifle. There are many like it, but this one is mine."
    });
    let response = state.post_content_form(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}
