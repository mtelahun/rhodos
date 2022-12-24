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
