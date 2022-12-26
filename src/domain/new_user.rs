use secrecy::{ExposeSecret, Secret};

use super::{user_email::UserEmail, user_name::UserName, UserRole};

#[derive(Debug, Default, Clone)]
pub struct AppUser {
    pub id: Option<i64>,
    pub name: UserName,
    pub email: UserEmail,
    pub password: Option<Secret<String>>,
    pub role: UserRole,
}

impl AppUser {
    pub fn get_password_hash_as_string(&self) -> String {
        if let Some(hash) = self.password.clone() {
            hash.expose_secret().to_owned()
        } else {
            "".to_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::Secret;

    use super::AppUser;

    #[test]
    fn no_hash_is_empty_string() {
        let secret = Secret::from("foobar".to_string());
        let user = AppUser {
            password: Some(secret),
            ..Default::default()
        };
        assert_eq!(
            user.get_password_hash_as_string(),
            "foobar",
            "unwrapped hash is same as original"
        );

        let user = AppUser {
            password: None,
            ..Default::default()
        };
        assert_eq!(
            user.get_password_hash_as_string(),
            "",
            "when unwrapped a hash with no value is an empty string"
        )
    }
}
