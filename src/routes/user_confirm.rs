use std::sync::Arc;

use axum::{
    extract::{Host, Query, State},
    http::StatusCode,
};
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use super::{get_db_from_host, AppState};
use crate::entities::{prelude::*, user, user_token};

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
    State(state): State<Arc<AppState>>,
    Query(query_params): Query<QueryParameters>,
) -> StatusCode {
    let hst = host.to_string();
    let db = get_db_from_host(&hst, &state)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        .unwrap();

    let request_token = query_params.confirmation_token;
    if request_token.is_none() {
        return StatusCode::BAD_REQUEST;
    }

    // Transaction: find the token, update the user, remove token
    db.transaction::<_, (), DbErr>(|txn| {
        Box::pin(async move {
            let database_token = match UserToken::find()
                .filter(user_token::Column::Token.eq(request_token))
                .one(txn)
                .await
            {
                Ok(Some(t)) => t,
                _ => return Ok(()),
            };

            // Remove the token (so it can't be used again) and
            // update the user record.
            let user_model = user::ActiveModel {
                id: Set(database_token.user_id),
                confirmed: Set(true),
                ..Default::default()
            };
            User::update(user_model)
                .filter(user::Column::Id.eq(database_token.user_id))
                .exec(txn)
                .await?;
            UserToken::delete_by_id(database_token.id).exec(txn).await?;
            Ok(())
        })
    })
    .await
    .map_err(|_| StatusCode::UNAUTHORIZED)
    .unwrap();

    StatusCode::OK
}
