use secrecy::ExposeSecret;

use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn logout_clears_session() {
    // Arrange
    let state = spawn_app().await;
    let body = serde_json::json!({
        "username": &state.test_user.username,
        "password": &state.test_user.password.expose_secret()
    });
    let response = state.post_login(&body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
    // follow redirect to admin dashboard
    let html_page = state.get_admin_dashboard_html().await;
    assert!(
        html_page.contains(&format!("Welcome {}", state.test_user.name)),
        "admin dashboard contains the user welcome message"
    );

    // Act
    let response = state.post_logout().await;
    assert_is_redirect_to(&response, "/login");

    // Assert
    let html_page = state.get_login_html().await;
    assert!(
        html_page.contains("<p><i>You have successfully logged out</i></p>"),
        "login page contains successfull logout message"
    );
    let response = state.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
