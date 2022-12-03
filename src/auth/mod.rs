use async_trait::async_trait;
use axum::Extension;
use axum::response::IntoResponse;
use axum_login::PostgresStore;
use axum_login::{AuthUser, secrecy::SecretVec, RequireAuthorizationLayer, UserStore};
use sea_orm::DatabaseConnection;
use sqlx::FromRow;
use sqlx::postgres::PgPoolOptions;


#[derive(Debug, Default, Clone, FromRow)]
struct User {
    id: i64,
    name: String,
    password_hash: String,
}

impl AuthUser for User {
    fn get_id(&self) -> String {
        format!("{}", self.id)
    }

    fn get_password_hash(&self) -> axum_login::secrecy::SecretVec<u8> {
        SecretVec::new(self.password_hash.clone().into())
    }
}

type AuthContext = axum_login::extractors::AuthContext<User, PostgresStore<User>>;
