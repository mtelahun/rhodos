use crate::domain::UserEmail;

pub struct EmailClient {
    _sender: UserEmail,
}

impl EmailClient {
    pub async fn send_email(
        &self,
        _receipient: UserEmail,
        _subject: &str,
        _html_content: &str,
        _text_content: &str,
    ) -> Result<(), String> {
        todo!()
    }
}
