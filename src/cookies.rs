use cookie::{time::Duration, Cookie, SameSite};
use once_cell::sync::OnceCell;
use tower_cookies::{Cookies, Key};

pub static FLASH_KEY: OnceCell<Key> = OnceCell::new();

pub const FLASH_COOKIE: &str = "_flash";

pub fn set_flash_cookie(cookies: &Cookies, value: &str) {
    let key = FLASH_KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    private_cookies.add(
        Cookie::build(FLASH_COOKIE, value.to_string())
            .max_age(Duration::seconds(1))
            .http_only(true)
            .same_site(SameSite::Lax)
            .domain("localhost")
            .path("/")
            .secure(false)
            .finish(),
    );
}
