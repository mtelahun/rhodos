use std::sync::Arc;

use axum::{
    extract::{Host, State},
    http::StatusCode,
    Form,
};
use sea_orm::{DatabaseConnection, EntityTrait, Set};
use serde::Deserialize;
use uuid::Uuid;

use super::{get_db_from_host, AppState};
use crate::{
    domain::{user_email::UserEmail, NewUser, UserName},
    entities::{prelude::*, user},
};

#[derive(Debug, Deserialize)]
pub struct InputUser {
    name: String,
    email: String,
}

#[tracing::instrument(
    name = "Add a New User",
    skip(form, state),
    fields(
        request_id = %Uuid::new_v4(),
        user_email = %form.email,
        user_name = %form.name
    )
)]
pub async fn create(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
    Form(form): Form<InputUser>,
) -> StatusCode {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();

    let new_user = parse_user(&form)
        .map_err(|_| StatusCode::BAD_REQUEST)
        .unwrap();

    insert_user(&conn, &new_user).await;

    StatusCode::OK
}

fn parse_user(form: &InputUser) -> Result<NewUser, String> {
    let name = UserName::parse(form.name.clone())?;
    let email = UserEmail::parse(form.email.clone())?;
    Ok(NewUser { name, email })
}

async fn insert_user(conn: &DatabaseConnection, new_user: &NewUser) {
    let data = user::ActiveModel {
        name: Set(new_user.name.as_ref().to_string()),
        email: Set(Some(new_user.email.as_ref().to_string())),
        ..Default::default()
    };
    let _ = User::insert(data).exec(conn).await;
}
