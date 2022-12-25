use crate::helpers::spawn_app;

#[tokio::test]
async fn root_index_contains_login_link() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let page = state
        .api_client
        .get(&format!("{}/", &state.app_address))
        .send()
        .await
        .expect("Failed to get index (/)")
        .text()
        .await
        .expect("Failed to get html for index (/)");

    // Assert
    println!("html=\n{}", page);
    assert!(
        page.contains(r#"<p><a href="/login">Please login</a></p>"#),
        "The home page contains a 'Login' link"
    );
}
