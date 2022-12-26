use serde::Deserialize;
use validator::validate_email;

#[derive(Clone, Debug, Default, Eq, Deserialize, PartialEq, PartialOrd)]
pub struct UserEmail(String);

impl UserEmail {
    pub fn parse(s: String) -> Result<UserEmail, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{} is not a valid email", s))
        }
    }
}
impl AsRef<str> for UserEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::UserEmail;
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[test]
    fn empty_string_rejected() {
        let s = "  ".to_string();
        assert!(
            UserEmail::parse(s).is_err(),
            "an email cannot be an empty string"
        );
    }

    #[test]
    fn missing_at_symbol_rejected() {
        let s = "sonjalowdelhi.example".to_string();
        assert!(
            UserEmail::parse(s).is_err(),
            "an email must have an '@' symbol"
        );
    }

    #[test]
    fn missing_subject_rejected() {
        let s = "@lowdelhi.example".to_string();
        assert!(
            UserEmail::parse(s).is_err(),
            "an email must have a subject before the '@' sign"
        );
    }

    #[test]
    fn valid_emails_parsed_successfully() {
        let s = SafeEmail().fake();
        assert!(
            !UserEmail::parse(s).is_err(),
            "correctly formatted emails are parsed successfully"
        )
    }
}
