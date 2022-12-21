use async_session::async_trait;
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_sessions::extractors::{ReadableSession, WritableSession};
use futures::future::Ready;

const USER_ID_KEY: &'static str = "user_id";

pub struct TypedReadableSession(ReadableSession);

impl TypedReadableSession {
    pub fn get_user_id(&self) -> Result<Option<i64>, serde_json::Error> {
        Ok(self.0.get(USER_ID_KEY))
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for TypedReadableSession
where
    S: Send + Sync,
{
    type Rejection = <ReadableSession as FromRequestParts<S>>::Rejection;

    // type Future = Ready<Result<TypedReadableSession, Self::Rejection>>;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Ok(TypedReadableSession(()))
    }
}

pub struct TypedWritableSession(WritableSession);

impl TypedWritableSession {
    pub fn regenerate(&mut self) {
        self.0.regenerate();
    }

    pub fn insert_user_id(&mut self, user_id: i64) -> Result<(), serde_json::Error> {
        self.0.insert(USER_ID_KEY, user_id)
    }

    pub fn get_user_id(&self) -> Result<Option<i64>, serde_json::Error> {
        Ok(self.0.get(USER_ID_KEY))
    }
}
