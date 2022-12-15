use lettre::{message::MultiPart, Message};

use crate::{domain::UserEmail, smtp_client::SmtpMailer};

pub struct EmailClient {
    pub sender: UserEmail,
}

impl EmailClient {
    pub fn new(sender: UserEmail) -> Self {
        Self { sender }
    }

    pub async fn send_email(
        &self,
        to: &UserEmail,
        subject: &String,
        plain: &String,
        html: &String,
        smtp_mailer: &SmtpMailer,
    ) -> Result<(), String> {
        let email = Message::builder()
            .from(self.sender.as_ref().parse().unwrap())
            .to(to.as_ref().parse().unwrap())
            .subject(subject)
            .multipart(MultiPart::alternative_plain_html(
                plain.to_string(),
                html.to_string(),
            ))
            .map_err(|e| e.to_string())
            .unwrap();

        smtp_mailer.send(&email)
    }
}
