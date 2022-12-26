use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn authenticated_admin_dashboard() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let response = state.get_admin_dashboard().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn happy_path_admin_role_can_access_admin_dashboard() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let html = state.get_admin_dashboard_html().await;
    println!("html=\n{}", html);

    // Assert
    assert!(
        html.contains(&format!("Welcome {}", &state.test_user_superadmin.name)),
        "user with role admin can access admin dashboard"
    );
}

#[tokio::test]
async fn user_role_cannot_access_admin_dashboard() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_user).await;

    // Act
    let response = state.get_admin_dashboard().await;
    let html = state.get_admin_dashboard_html().await;
    println!("html=\n{}", html);

    // Assert
    assert_eq!(
        response.status().as_u16(),
        303,
        "user attemp to access admin area returns 303 redirect"
    );
}
