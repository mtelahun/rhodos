use fake::{faker::internet::en::SafeEmail, Fake};
use librhodos::{domain::UserEmail, email_client::EmailClient, smtp_client::SmtpMailer};
use secrecy::Secret;
use uuid::Uuid;

#[tokio::test]
async fn send_email() {
    // Arrange
    let client = reqwest::Client::new();

    // Act
    let smtp_subject = Uuid::new_v4().to_string();
    let plain = "This is the test email body".to_string();
    let html = "<h1>This is the test email body</h1>".to_string();
    let smtp_mailer = SmtpMailer::new(
        "localhost",
        1025,
        "smtp",
        Secret::from("password".to_string()),
    );
    let email_client = EmailClient::new(UserEmail::parse(SafeEmail().fake()).unwrap());
    let _ = email_client
        .send_email(
            UserEmail::parse(SafeEmail().fake()).unwrap(),
            &smtp_subject,
            &plain,
            &html,
            &smtp_mailer,
        )
        .await
        .expect("Failed to send email");

    // Assert

    // Check MailHog for the sent email
    let response = client
        .get(&format!(
            "http://localhost:8025/api/v2/search?kind=containing&query={}",
            smtp_subject
        ))
        .send()
        .await
        .expect("Failed to execute mailhog request");

    assert_eq!(
        response.status().as_u16(),
        200,
        "query of mailhog queue returns 200 Ok",
    );
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains(smtp_subject.as_str()),
        "The response contains the email body"
    );
}
