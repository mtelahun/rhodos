use fake::{
    faker::{internet::en::SafeEmail, name::raw::Name},
    locales::EN,
    Fake,
};

use crate::helpers::{connect_to_db, spawn_app};

#[tokio::test]
async fn add_user_valid_form_data_200() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = "name=Sonja%20Hemphill&email=sonja%40lowdelhi.example";
    let response = state.post_user(body.to_string()).await;

    // Assert
    assert_eq!(
        200,
        response.status().as_u16(),
        "valid form data return 200 OK"
    );
}

#[tokio::test]
async fn add_user_persists_new_user() {
    // Arrange
    let state = spawn_app().await;

    // Act
    let body = "name=Sonja%20Hemphill&email=sonja%40lowdelhi.example";
    let _ = state.post_user(body.to_string()).await;

    // Assert
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(r#"SELECT name, email, confirmed FROM "user";"#, &[])
        .await
        .expect("query to fetch row failed");

    assert!(!row.is_empty(), "one record has been created");
    let name: &str = row[0].get(0);
    let email: &str = row[0].get(1);
    let confirmed: bool = row[0].get(2);
    assert_eq!(name, "Sonja Hemphill");
    assert_eq!(email, "sonja@lowdelhi.example");
    assert!(!confirmed);
}

#[tokio::test]
async fn add_user_missing_form_data_400() {
    // Arrange
    let state = spawn_app().await;
    let test_cases = vec![
        ("name=Sonja%20Hemphill", "missing email"),
        ("email=sonja%40lowdelhi.example", "missing name"),
        ("", "missing name and email"),
    ];

    for (invalid_data, msg_err) in test_cases {
        // ACt
        let response = state.post_user(invalid_data.to_string()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "{} returns 400 Bad Client Request",
            msg_err,
        );
    }
}

#[tokio::test]
async fn add_user_sends_confirmation_with_link() {
    // Arrange
    let state = spawn_app().await;
    let user_email: String = SafeEmail().fake();
    let user_name: String = Name(EN).fake();

    // Act
    let post_body = format!("name={}&email={}", user_name, user_email);
    let response = state.post_user(post_body.to_string()).await;

    // Assert
    assert_eq!(
        200,
        response.status().as_u16(),
        "valid form data returns 200 OK"
    );

    let links = state.get_confirmation_links(&user_email).await;
    assert!(
        links.html.as_str().contains("confirmation_token"),
        "the html part contains part of the confirmation string"
    );
    assert!(
        links.text.as_str().contains("confirmation_token"),
        "the text part contains part of the confirmation string"
    );
    assert_eq!(
        links.html.as_str(),
        links.text.as_str(),
        "the links in the html and text parts are identical"
    );
}

#[tokio::test]
async fn add_user_token_fails_if_fatal_db_err() {
    // Arrange
    let state = spawn_app().await;
    let user_email: String = SafeEmail().fake();
    let user_name: String = Name(EN).fake();
    // Sabotage the database
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"ALTER TABLE user_token DROP COLUMN "token""#, &[])
        .await
        .expect("query to alter user_token table failed");

    // Act
    let post_body = format!("name={}&email={}", user_name, user_email);
    let response = state.post_user(post_body.to_string()).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}

#[tokio::test]
async fn insert_user_fails_if_fatal_db_err() {
    // Arrange
    let state = spawn_app().await;
    let user_email: String = SafeEmail().fake();
    let user_name: String = Name(EN).fake();
    // Sabotage the database
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"ALTER TABLE "user" DROP COLUMN "email""#, &[])
        .await
        .expect("query to alter user table failed");

    // Act
    let post_body = format!("name={}&email={}", user_name, user_email);
    let response = state.post_user(post_body.to_string()).await;

    // Assert
    assert_eq!(response.status().as_u16(), 500)
}
