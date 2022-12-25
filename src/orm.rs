use anyhow::{anyhow, Context};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use secrecy::Secret;

use crate::{
    domain::{NewUser, UserEmail, UserName, UserRole},
    entities::{
        prelude::*,
        user::{self, Model as UserModel},
    },
    error::error_chain_fmt,
};

#[tracing::instrument(name = "Get user model", skip(conn))]
pub async fn get_user_model_by_id(
    user_id: i64,
    conn: &DatabaseConnection,
) -> Result<UserModel, OrmError> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!("Failed to retrieve a user record: {}", e)))
        })?
        .unwrap();

    Ok(model)
}

#[tracing::instrument(name = "Get ORM model", skip(conn))]
pub async fn get_orm_model_by_id(
    user_id: i64,
    conn: &DatabaseConnection,
) -> Result<NewUser, OrmError> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(|e| {
            OrmError::UnexpectedError(anyhow!(format!("Failed to retrieve a user record: {}", e)))
        })?
        .unwrap();

    Ok(NewUser {
        id: Some(model.id),
        password: Some(Secret::from(model.password)),
        name: UserName::parse(model.name).unwrap_or_default(),
        email: UserEmail::parse(model.email).unwrap_or_default(),
        role: UserRole::try_from(model.role).unwrap_or_default(),
    })
}

#[tracing::instrument(name = "Get credential", skip(username, conn))]
pub async fn get_credential(
    username: &str,
    conn: &DatabaseConnection,
) -> Result<Option<(i64, Secret<String>)>, anyhow::Error> {
    let model = User::find()
        .filter(user::Column::Email.eq(username))
        .one(conn)
        .await
        .context("Failed to retrieve stored credentials")?
        .map(|m| (m.id, Secret::new(m.password)));

    Ok(model)
}

pub async fn update_credential(
    user_id: i64,
    orm_user: user::ActiveModel,
    conn: &DatabaseConnection,
) -> Result<i64, OrmError> {
    let res = User::update(orm_user)
        .filter(user::Column::Id.eq(user_id))
        .exec(conn)
        .await
        .context("Failed to update user record")?;

    Ok(res.id)
}

#[derive(thiserror::Error)]
pub enum OrmError {
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for OrmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
