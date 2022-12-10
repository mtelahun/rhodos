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
    let client = connect_to_db(&state.db_name).await;
    let row = client
        .query(r#"SELECT name, email FROM "user";"#, &[])
        .await
        .expect("query to fetch row failed");

    assert!(!row.is_empty(), "one record has been created");
    let name: &str = row[0].get(0);
    let email: &str = row[0].get(1);
    assert_eq!(name, "Sonja Hemphill");
    assert_eq!(email, "sonja@lowdelhi.example");
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
