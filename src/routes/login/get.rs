use axum::response::Html;
use tower_cookies::Cookies;

use crate::cookies::{FlashCookieType, FLASH_COOKIE, FLASH_KEY};

pub async fn login_form(cookies: Cookies) -> Html<String> {
    let error_html = get_error_html(&cookies);
    let html_string = format!(
        r#"
<!DOCTYPE html>
<html lang="en">

<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>

<body>
    {error_html}
    <form action="/login" method="post">
        <label>Username
            <input type="text" placeholder="Enter Username" name="username">
        </label>
        <label>Password
            <input type="password" placeholder="Enter Password" name="password">
        </label>
        <button type="submit">Login</button>
    </form>
</body>

</html>"#
    );
    Html(html_string)
}

fn get_error_html(cookies: &Cookies) -> String {
    let key = FLASH_KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    let mut error_html = "";
    if let Some(c) = private_cookies.get(FLASH_COOKIE) {
        if c.name() == FLASH_COOKIE {
            if c.value() == FlashCookieType::InvalidCreds.to_string() {
                error_html = "<p><i>Either the username or password was incorrect</i></p>"
            } else if c.value() == FlashCookieType::LogoutOk.to_string() {
                error_html = "<p><i>You have successfully logged out</i></p>";
            } else if c.value() == FlashCookieType::SessionSetupError.to_string() {
                error_html = "<p><i>There was an error setting up your session</i></p>";
            } else if c.value() == FlashCookieType::PasswordResetOk.to_string() {
                error_html = "<p><i>Your password has been successfully updated. Please log in again.</i></p>"
            }
        }
    }

    error_html.to_string()
}
