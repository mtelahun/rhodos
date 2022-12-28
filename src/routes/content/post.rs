use anyhow::{anyhow, Context};
use axum::{
    extract::{Host, State},
    response::Redirect,
    Extension, Form, Json,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    domain::AppUser,
    entities::{account, content, prelude::*},
    error::TenantMapError,
    routes::{get_db_from_host, AppState},
};

use super::ContentError;

const MAX_POST_CHARS: usize = 500;

#[derive(Debug, Deserialize)]
pub struct BodyData {
    pub content: NewPost,
}

#[derive(Debug, Deserialize)]
pub struct NewPost {
    text: String,
}

#[tracing::instrument(
    name = "Post a microblog",
    skip(state, body),
    fields(
        request_id = %Uuid::new_v4(),
        username = tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn create(
    Extension(user): Extension<AppUser>,
    Host(host): Host,
    State(state): State<AppState>,
    Json(body): Json<BodyData>,
) -> Result<(), ContentError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => ContentError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => ContentError::UnexpectedError(anyhow::anyhow!(s)),
    })?;

    let account_id = process_content(&user, &body.content.text, &conn).await?;

    post_content(account_id, body.content.text, &conn).await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    content: String,
}

#[tracing::instrument(
    name = "Post a microblog form",
    skip(state, body),
    fields(
        username = tracing::field::Empty,
        user_id=tracing::field::Empty,
    )
)]
pub async fn new(
    Extension(user): Extension<AppUser>,
    Host(host): Host,
    State(state): State<AppState>,
    Form(body): Form<FormData>,
) -> Result<Redirect, ContentError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state).await.map_err(|e| match e {
        TenantMapError::NotFound(s) => ContentError::ValidationError(s),
        TenantMapError::UnexpectedError(s) => ContentError::UnexpectedError(anyhow::anyhow!(s)),
    })?;

    let account_id = process_content(&user, &body.content, &conn).await?;

    post_content(account_id, body.content, &conn).await?;

    Ok(Redirect::to("/home"))
}

#[tracing::instrument(name = "Process content", skip(content, conn))]
async fn process_content(
    user: &AppUser,
    content: &String,
    conn: &DatabaseConnection,
) -> Result<i64, ContentError> {
    let account = Account::find()
        .filter(account::Column::UserId.eq(user.id.unwrap()))
        .one(conn)
        .await
        .context("Unable to retrieve account associated with current user")?;
    let account_id = match account {
        Some(model) => model.id,
        None => {
            return Err(ContentError::UnexpectedError(anyhow!(
                "There is no account associated with current user"
            )))
        }
    };
    let new_content = content;
    if new_content.is_empty() || new_content.len() > MAX_POST_CHARS || account_id <= 0 {
        tracing::error!("Content creation attempted, but a field is invalid");
        return Err(ContentError::ValidationError(
            "empty content or too long".to_string(),
        ));
    }

    Ok(account_id)
}

#[tracing::instrument(
    name = "Post content"
    skip(content, conn),
)]
async fn post_content(
    account_id: i64,
    content: String,
    conn: &DatabaseConnection,
) -> Result<(), ContentError> {
    let data = content::ActiveModel {
        publisher_id: Set(account_id),
        body: Set(Some(content)),
        ..Default::default()
    };
    let _ = Content::insert(data)
        .exec(conn)
        .await
        .context("failed to post new content")?;

    Ok(())
}
