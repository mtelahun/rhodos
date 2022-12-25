use axum::{
    extract::{Host, State},
    response::Html,
    Extension,
};
use tower_cookies::Cookies;

use super::ResetError;
use crate::{
    cookies::{FlashCookieType, FLASH_COOKIE, FLASH_KEY},
    domain::NewUser,
    routes::{get_db_from_host, AppState},
};

#[tracing::instrument(
    name = "Reset password form"
    skip(state)
)]
pub async fn password_reset(
    Host(host): Host,
    State(state): State<AppState>,
    Extension(user): Extension<NewUser>,
    cookies: Cookies,
) -> Result<Html<String>, ResetError> {
    let hst = host.to_string();
    let _conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| ResetError::UnexpectedError(e.into()))?;

    let feedback_html = get_feedback_html(&cookies);
    let html_string = format!(
        r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta http-equiv="content-type" content="text/html; charset=utf-8">
    <title>Login</title>
</head>
<body>
    {feedback_html}
    <form action="/user/change-password" method="post">
        <label>Curren Password
            <input type="password" placeholder="Enter your current password" name="current_password">
        </label>
        <label>Password
            <input type="password" placeholder="New password" name="password">
        </label>
        <label>Password
            <input type="password" placeholder="Confirm your new password" name="confirm_password">
        </label>
        <button type="submit">Change password</button>
    </form>
</body>
</html>"#
    );
    Ok(Html(html_string))
}

fn get_feedback_html(cookies: &Cookies) -> String {
    let key = FLASH_KEY.get().unwrap();
    let private_cookies = cookies.private(key);
    let mut feedback_html = "";
    if let Some(c) = private_cookies.get(FLASH_COOKIE) {
        if c.name() == FLASH_COOKIE {
            if c.value() == FlashCookieType::PasswordResetMismatch.to_string() {
                feedback_html = "<p><i>The new password and the confirmation do not match</i></p>"
            } else if c.value() == FlashCookieType::PasswordResetCurrent.to_string() {
                feedback_html = "<p><i>Your current password does not match</i></p>"
            } else if c.value() == FlashCookieType::PasswordResetEmpty.to_string() {
                feedback_html = "<p><i>You didn't specify a new password</i></p>"
            }
        }
    }

    feedback_html.to_string()
}
