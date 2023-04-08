use std::ops::Deref;

use anyhow::anyhow;
use axum::{
    async_trait,
    body::HttpBody,
    extract::{rejection::FormRejection, FromRequest, Host, State},
    http::{Request, StatusCode},
    BoxError, Json,
};
use secrecy::ExposeSecret;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

use crate::{
    orm::register_confidential_client,
    routes::{get_db_from_host, AppState},
};

use super::ApiError;

#[derive(Debug, Deserialize)]
pub struct FormData {
    client_name: String,
    redirect_uris: String,
    scopes: Option<String>,
    website: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Application {
    id: String,
    name: String,
    website: Option<String>,
    vapid_key: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

pub async fn create_app(
    Host(host): Host,
    State(state): State<AppState>,
    MastoForm(form): MastoForm<FormData>,
) -> Result<Json<Application>, ApiError> {
    let hst = host.to_string();
    let conn = get_db_from_host(&hst, &state)
        .await
        .map_err(|e| ApiError::UnexpectedError(anyhow!(e)))?;

    let (client_id, client_secret) = register_confidential_client(
        &form.client_name,
        &form.website.clone().unwrap_or(String::from("")),
        &form.redirect_uris,
        &form.scopes.unwrap_or(String::from("")),
        &conn,
    )
    .await
    .map_err(|e| ApiError::UnexpectedError(anyhow!(e)))?;

    let res = Application {
        id: String::from("123456"), // Unsure what purpose this serves in Mastodon
        name: form.client_name,
        website: form.website,
        vapid_key: String::from("not_implemented_yet"),
        client_id: Some(client_id.to_string()),
        client_secret: Some(client_secret.expose_secret().to_string()),
    };

    Ok(Json(res))
}

/// Custom Wrapper around Axum's Form extractor. Returns 422 when required form
/// fields are missing
///
#[derive(Debug, Clone, Copy, Default)]
pub struct MastoForm<T>(pub T);

impl<T> Deref for MastoForm<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<T, S, B> FromRequest<S, B> for MastoForm<T>
where
    axum::Form<T>: FromRequest<S, B, Rejection = FormRejection>,
    T: DeserializeOwned,
    B: HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<Value>);

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Form::<T>::from_request(req, state).await {
            Ok(value) => Ok(Self(value.0)),
            // convert the error from `axum::Json` into whatever we want
            Err(rejection) => {
                let payload = serde_json::json!({
                    "error": rejection.to_string(),
                    "origin": "custom_extractor",
                });

                Err((StatusCode::UNPROCESSABLE_ENTITY, axum::Json(payload)))
            }
        }
    }
}
