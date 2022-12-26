use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn logout_clears_session() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;
    // follow redirect to home
    let html_page = state.get_home_dashboard_html().await;
    assert!(
        html_page.contains(&format!("Welcome {}", state.test_user_superadmin.name)),
        "home dashboard contains the user welcome message"
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
