use fake::{
    faker::{internet::en::SafeEmail, name::raw::Name},
    locales::EN,
    Fake,
};

use crate::helpers::{connect_to_db, spawn_app};

#[tokio::test]
async fn confirm_without_token_rejected_400() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = reqwest::get(&format!("{}/user/confirm", app.app_address))
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 400);
}

#[tokio::test]
async fn confirmation_link_works_200() {
    // Arrange
    let state = spawn_app().await;
    let user_email: String = SafeEmail().fake();
    let user_name: String = Name(EN).fake();

    let post_body = format!("name={}&email={}", user_name, user_email);
    let response = state.post_user(post_body.to_string()).await;

    assert_eq!(
        200,
        response.status().as_u16(),
        "valid form data returns 200 OK"
    );

    // Act
    let links = state.get_confirmation_links(&user_email).await;
    assert!(
        links.html.as_str().contains("_token"),
        "the html part contains part of the confirmation string"
    );
    assert!(
        links.text.as_str().contains("_token"),
        "the text part contains part of the confirmation string"
    );
    assert_eq!(
        links.html.as_str(),
        links.text.as_str(),
        "the links in the html and text parts are identical"
    );

    let response = reqwest::get(links.html).await.unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 200);
}

#[tokio::test]
async fn confirmation_link_confirms_added_user() {
    // Arrange
    let state = spawn_app().await;
    let user_email: String = SafeEmail().fake();
    let user_name: String = Name(EN).fake();

    // Act
    let post_body = format!("name={}&email={}", user_name, user_email);
    let response = state.post_user(post_body.to_string()).await;
    assert_eq!(
        200,
        response.status().as_u16(),
        "valid form data returns 200 OK"
    );
    let links = state.get_confirmation_links(&user_email).await;
    let _ = reqwest::get(links.html).await.unwrap();

    // Assert
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(r#"SELECT name, email, confirmed FROM "user";"#, &[])
        .await
        .expect("query to fetch user row failed");
    // check status of user
    assert!(!row.is_empty(), "one record has been created");
    let name: &str = row[0].get(0);
    let email: &str = row[0].get(1);
    let confirmed: bool = row[0].get(2);
    assert_eq!(name, user_name);
    assert_eq!(email, user_email);
    assert!(confirmed);
    // check status of token
    let row = client
        .query(r#"SELECT id, user_id, token FROM "user_token";"#, &[])
        .await
        .expect("query to fetch user_token row failed");
    assert!(row.is_empty(), "the token has been removed");
}

#[tokio::test]
async fn confirm_with_invalid_token_rejected_401() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let response = reqwest::get(&format!(
        "{}/user/confirm?confirmation_token=abcd",
        app.app_address
    ))
    .await
    .unwrap();

    // Assert
    assert_eq!(response.status().as_u16(), 401);
}
