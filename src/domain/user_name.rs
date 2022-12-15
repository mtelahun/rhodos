use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
pub struct UserName(String);

impl UserName {
    pub fn parse(s: String) -> Result<UserName, String> {
        let is_empty = s.trim().is_empty();
        let is_long = s.graphemes(true).count() > 256;

        if !(is_empty || is_long) {
            return Ok(Self(s));
        }

        Err("One of the criteria is not satisfied".to_string())
    }
}

impl AsRef<str> for UserName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::user_name::UserName;

    #[test]
    fn long_name_256_is_valid() {
        let name = "a".repeat(256);
        assert!(
            !UserName::parse(name).is_err(),
            "usernames up to 256 chars are valid"
        );
    }

    #[test]
    fn long_name_257_is_invalid() {
        let name = "a".repeat(257);
        assert!(
            UserName::parse(name).is_err(),
            "usernames longer than 256 chars are NOT valid"
        );
    }

    #[test]
    fn empty_name_is_invalid() {
        assert!(
            UserName::parse("   ".to_string()).is_err(),
            "Empty values for user names are NOT valid"
        )
    }

    #[test]
    fn valid_name_parsed_successfully() {
        let name = UserName::parse("Mike Henke".to_string()).unwrap();
        assert_eq!(
            name.as_ref(),
            "Mike Henke",
            "valid string is parsed successfully"
        );
    }
}
