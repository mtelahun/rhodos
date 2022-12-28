use crate::helpers::spawn_app;

#[tokio::test]
async fn home_dashboard_contains_logout_link() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_superadmin).await;

    // Act
    let page = state.get_home_dashboard_html().await;

    // Assert
    assert!(
        page.contains(r#"<form name="logout_form" action="/user/logout" method="post">"#),
        "The home page contains a 'Logout' link"
    );
}

#[tokio::test]
async fn home_dashboard_contains_create_content_link() {
    // Arrange
    let state = spawn_app().await;
    state.login_as(&state.test_user_user).await;

    // Act
    let page = state.get_home_dashboard_html().await;

    // Assert
    println!("html page=\n{}", page);
    assert!(
        page.contains(r#"<a href="/content/form">Post Content</a>"#),
        "The home page contains a 'new post' link"
    );
}
