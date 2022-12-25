use axum::{response::Html, Extension};

use crate::domain::NewUser;

pub async fn home(Extension(user): Extension<NewUser>) -> Html<String> {
    let user_name = user.name;
    Html(format!(
        r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <meta http-equiv="content-type" content="text/html; charset=utf-8">
        <title>Home</title>
    </head>
    <body>
        <p>Welcome {}!</p>
        <p>Actions:</p>
        <ol>
            <li><a href="/user/change-password">Change your password</a></li>
            <li>
                <form name="logout_form" action="/user/logout" method="post">
                    <input type="submit" value="Logout" />
                </form>
            </li>
        </ol>
    </body>
</html>"#,
        user_name
    ))
}
