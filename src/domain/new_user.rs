use secrecy::Secret;

use super::{user_email::UserEmail, user_name::UserName};

#[derive(Clone, Debug)]
pub struct NewUser {
    pub name: UserName,
    pub email: UserEmail,
    pub password: Secret<String>,
}
