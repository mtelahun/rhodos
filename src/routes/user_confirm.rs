use anyhow::{anyhow, Context};
use axum::{
    extract::{Host, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use super::{get_db_from_host, AppState};
use crate::{
    entities::{prelude::*, user, user_token},
    error::error_chain_fmt,
};

#[derive(Debug, Deserialize)]
pub struct QueryParameters {
    confirmation_token: Option<String>,
}

#[tracing::instrument(
    name = "Confirm Email Registration",
    skip(state),
    fields(
        request_id = %Uuid::new_v4(),
    )
)]
pub async fn confirm(
    Host(host): Host,
    State(state): State<AppState>,
    Query(query_params): Query<QueryParameters>,
) -> Result<(), TokenError> {
    let hst = host.to_string();
    let db = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| TokenError::UnexpectedError(anyhow!(e)))?;

    let database_token = get_token(query_params.confirmation_token, &db)
        .await?
        .unwrap();

    // Transaction: find the token, update the user, remove token
    db.transaction::<_, (), TokenError>(|txn| {
        Box::pin(async move {
            // Update the user record and remove the token so it can't be
            // used again.
            let user_model = user::ActiveModel {
                id: Set(database_token.user_id),
                confirmed: Set(true),
                ..Default::default()
            };
            User::update(user_model)
                .filter(user::Column::Id.eq(database_token.user_id))
                .exec(txn)
                .await
                .map_err(|e| TokenError::UnexpectedError(anyhow!(e)))?;
            UserToken::delete_by_id(database_token.id)
                .exec(txn)
                .await
                .map_err(|e| TokenError::UnexpectedError(anyhow!(e)))?;
            Ok(())
        })
    })
    .await
    .context("transaction error")?;

    Ok(())
}

#[tracing::instrument(name = "Get user from token", skip(request_token, conn))]
async fn get_token(
    request_token: Option<String>,
    conn: &DatabaseConnection,
) -> Result<Option<user_token::Model>, TokenError> {
    if request_token.is_none() {
        return Err(TokenError::BadRequest(
            "empty confirmation token".to_string(),
        ));
    }
    let request_token = request_token.unwrap();
    let database_token = UserToken::find()
        .filter(user_token::Column::Token.eq(request_token))
        .one(conn)
        .await
        .map_err(|e| TokenError::UnexpectedError(anyhow::anyhow!(e)))?;
    if database_token.is_none() {
        return Err(TokenError::NotFound(
            "invalid confirmation token".to_string(),
        ));
    }

    Ok(database_token)
}

#[derive(thiserror::Error)]
pub enum TokenError {
    #[error("{0}")]
    BadRequest(String),
    #[error("There is no token associated with the user: {0:?}")]
    NotFound(String),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl IntoResponse for TokenError {
    fn into_response(self) -> axum::response::Response {
        let (status, err_msg) = match self {
            Self::BadRequest(s) => {
                tracing::info!("no query parameter: {s:?}");
                (StatusCode::BAD_REQUEST, s)
            }
            Self::UnexpectedError(e) => {
                tracing::info!("an unexpected error occured");
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))
            }
            Self::NotFound(s) => {
                tracing::info!("there is no user associated with the token: {s:?}");
                (StatusCode::UNAUTHORIZED, s)
            }
        };
        (status, err_msg).into_response()
    }
}
