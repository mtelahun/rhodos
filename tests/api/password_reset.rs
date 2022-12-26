use secrecy::ExposeSecret;

use crate::helpers::{assert_is_redirect_to, connect_to_db, spawn_app};

#[tokio::test]
async fn authenticated_password_reset_form() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let response = state.get_password_reset().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn authenticated_password_reset() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = serde_json::json!({
        "current_password": "foo",
        "password": "foo",
        "confirm_password": "foo",
    });
    let response = state.post_password_reset(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn password_reset_invalid_form_bad_request_400() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let invalid_cases = vec![
        (
            serde_json::json!({
                "password": "foo",
                "confirm_password": "foo",
            }),
            "no current password",
        ),
        (
            serde_json::json!({
                "current_password": "foo",
                "confirm_password": "foo",
            }),
            "no password",
        ),
        (
            serde_json::json!({
                "current_password": "foo",
                "password": "foo",
            }),
            "no confirm password",
        ),
        (
            serde_json::json!({
                "confirm_password": "foo",
            }),
            "no current password or password",
        ),
        (
            serde_json::json!({
                "current_password": "foo",
            }),
            "no password or confirm password",
        ),
        (
            serde_json::json!({
                "password": "foo",
            }),
            "no current password or confirm password",
        ),
        (serde_json::json!({}), "empty body"),
    ];
    for (case, msg) in invalid_cases {
        let response = state.post_password_reset(&case).await;
        // Assert
        assert_eq!(
            response.status().as_u16(),
            400,
            "{} returns status 422 Bad Request",
            msg
        )
    }
}

#[tokio::test]
async fn password_reset_empty_password() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let body = serde_json::json!({
        "current_password": &state.test_user_superadmin.password.expose_secret(),
        "password": "",
        "confirm_password": "bar",
    });
    let response = state.post_password_reset(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/user/change-password");

    // Assert- 2 follow the redirect
    let html_page = state.get_password_reset_html().await;
    assert!(
        html_page.contains(&format!("<p><i>You didn't specify a new password</i></p>")),
        "reset page shows empty password message"
    );
}

#[tokio::test]
async fn password_reset_wrong_current_password() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let body = serde_json::json!({
        "current_password": "foo",
        "password": "foo",
        "confirm_password": "foo",
    });
    let response = state.post_password_reset(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/user/change-password");

    // Assert- 2 follow the redirect
    let html_page = state.get_password_reset_html().await;
    assert!(
        html_page.contains(&format!(
            "<p><i>Your current password does not match</i></p>"
        )),
        "reset page shows wrong current password message"
    );
}

#[tokio::test]
async fn password_reset_confirm_password_mismatch() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let body = serde_json::json!({
        "current_password": &state.test_user_superadmin.password.expose_secret(),
        "password": "foo",
        "confirm_password": "bar",
    });
    let response = state.post_password_reset(&body).await;

    // Assert
    assert_is_redirect_to(&response, "/user/change-password");

    // Assert- 2 follow the redirect
    let html_page = state.get_password_reset_html().await;
    assert!(
        html_page.contains(&format!(
            "<p><i>The new password and the confirmation do not match</i></p>"
        )),
        "reset page shows password mismatch message"
    );
}

#[tokio::test]
async fn happy_path_password_reset_redirect_303() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let body = serde_json::json!({
        "current_password": &state.test_user_superadmin.password.expose_secret(),
        "password": "foo",
        "confirm_password": "foo",
    });
    let response = state.post_password_reset(&body).await;
    // Assert- 1
    assert_is_redirect_to(&response, "/user/change-password");

    // Assert- 2 follow the redirect
    let response = state.get_password_reset().await;
    // axum-login notices that the session is no-longer valid and re-directs us to /login
    assert_is_redirect_to(&response, "/login");
    let html_page = state.get_login_html().await;
    assert!(
        html_page.contains(&format!(
            "<p><i>Your password has been successfully updated. Please log in again.</i></p>"
        )),
        "reset page shows success message"
    );

    // Assert- 3 attempt to login with the new password
    let body = serde_json::json!({
        "username": &state.test_user_superadmin.username,
        "password": "foo"
    });
    let response = state.post_login(&body).await;
    assert_is_redirect_to(&response, "/home");
}

#[tokio::test]
async fn password_reset_fails_if_fatal_db_err_00() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Sabotage the database
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"ALTER TABLE "user" DROP COLUMN "password";"#, &[])
        .await
        .expect("query to alter content table failed");

    // Act
    let body = serde_json::json!({
        "current_password": &state.test_user_superadmin.password.expose_secret(),
        "password": "foo",
        "confirm_password": "foo",
    });
    let response = state.post_password_reset(&body).await;
    assert_eq!(
        response.status().as_u16(),
        500,
        "fatal database error returns status 500 Internal server error"
    )
}

#[tokio::test]
async fn get_password_reset_contains_post_link() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let response = state.get_password_reset_html().await;

    // Assert
    assert!(
        response
            .to_lowercase()
            .contains(r#"<form action="/user/change-password" method="post">"#),
        "form post links to correct route/path"
    );
}
