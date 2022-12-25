use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn error_flash_message_set_on_failure() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = serde_json::json!({
        "username": "random-username",
        "password": "random-password"
    });
    let response = state.post_login(&body).await;

    // Assert
    assert_eq!(
        response.status().as_u16(),
        303,
        "random credentials return 303 redirect"
    );
    assert_is_redirect_to(&response, "/login");

    // Act - 2
    // We are depending on the cookie set in the previous call to
    // be propagated by reqwest when we make the following GET
    let html_page = state.get_login_html().await;
    assert!(
        html_page.contains(r#"<p><i>Either the username or password was incorrect</i></p>"#),
        "Authentication failure notice is in body of response"
    );

    // Act - 3
    // The cookie should not be set on subsequent re-loads
    // XXX - sleep is needed here because it fails without it (cookie max-age = 1 second)
    std::thread::sleep(std::time::Duration::from_secs(1));
    let html_page = state.get_login_html().await;
    assert!(
        !html_page.contains(r#"<p><i>Either the username or password was incorrect</i></p>"#),
        "Authentication failure notice is NOT in body of response"
    );
}

#[tokio::test]
async fn redirect_to_home_after_login_ok_200() {
    // Arrange
    let state = spawn_app().await;

    // Act
    state.login_as(&state.test_user_superadmin).await;

    // Assert - follow redirect
    let html_page = state.get_home_dashboard_html().await;
    assert!(
        html_page.contains(&format!("Welcome {}", state.test_user_superadmin.name)),
        "home page shows welcome to user"
    );
}

#[tokio::test]
async fn seed_user_super_admin_login_ok_200() {
    // Arrange
    let state = spawn_app().await;

    // Act- 1
    let body = serde_json::json!({
        "username": "admin",
        "password": "rhodos"
    });
    let response = state.post_login(&body).await;

    // Assert- 1
    assert_is_redirect_to(&response, "/home");

    // Act- 2 - follow redirect
    let html_page = state.get_admin_dashboard_html().await;
    assert!(
        html_page.contains(&format!("Welcome Administrator")),
        "home page shows welcome to user"
    );
}
