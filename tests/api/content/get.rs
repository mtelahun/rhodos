use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn authenticated_create_content_form() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let response = state.get_content_form().await;

    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn create_content_form_components() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_user).await;

    // Act
    let html = state.get_content_form_html().await;

    // Assert
    println!("html page=\n{}", html);
    assert!(
        html.contains("<textarea name=\"content\""),
        "content creation page contains textarea for post"
    );
    assert!(
        html.contains(r#"<button type="cancel">Cancel</button>"#),
        "content creation page contains cancel button"
    );
    assert!(
        html.contains(r#"<button type="submit">Post</button>"#),
        "content creation page contains submit button"
    );
}
