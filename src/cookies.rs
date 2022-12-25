use std::fmt;

use cookie::{time::Duration, Cookie, SameSite};
use once_cell::sync::OnceCell;
use tower_cookies::{Cookies, Key};

pub static FLASH_KEY: OnceCell<Key> = OnceCell::new();

pub const FLASH_COOKIE: &str = "_flash";

#[derive(Debug)]
pub enum FlashCookieType {
    LoginOk,
    LogoutOk,
    InvalidCreds,
    PasswordResetOk,
    PasswordResetCurrent,
    PasswordResetEmpty,
    PasswordResetMismatch,
    SessionSetupError,
}

impl fmt::Display for FlashCookieType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FlashCookieType::LoginOk => write!(f, "login_ok"),
            FlashCookieType::LogoutOk => write!(f, "logout_ok"),
            FlashCookieType::InvalidCreds => write!(f, "invalid_creds"),
            FlashCookieType::PasswordResetOk => write!(f, "password_reset_ok"),
            FlashCookieType::PasswordResetCurrent => write!(f, "password_reset_fail_current"),
            FlashCookieType::PasswordResetEmpty => write!(f, "password_reset_fail_empty"),
            FlashCookieType::PasswordResetMismatch => write!(f, "password_reset_fail_mismatch"),
            FlashCookieType::SessionSetupError => write!(f, "session_setup_error"),
        }
    }
}

pub fn set_flash_cookie(cookies: &Cookies, fct: FlashCookieType, domain: &str) {
    let key = FLASH_KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    private_cookies.add(
        Cookie::build(FLASH_COOKIE, fct.to_string())
            .max_age(Duration::seconds(1))
            .http_only(true)
            .same_site(SameSite::Lax)
            .domain(domain.to_string())
            .path("/")
            .secure(false)
            .finish(),
    );
}
