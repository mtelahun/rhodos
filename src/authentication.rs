use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use secrecy::{ExposeSecret, Secret};

use crate::{
    entities::{prelude::*, user},
    telemetry::spawn_blocking_with_tracing,
};

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, conn))]
pub async fn validate_credentials(
    credentials: Credentials,
    conn: &DatabaseConnection,
) -> Result<i64, AuthError> {
    // Use dummy hashed password so that non-existent users go through
    // a dummy password verification step. This ensures both invalid
    // passwords and non-existent users take the same amount of time to verify.
    let mut user_id = None;
    let mut db_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );
    if let Some((stored_uid, stored_hash)) = get_stored_credentials(&credentials.username, conn)
        .await
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))?
    {
        user_id = Some(stored_uid);
        db_password_hash = stored_hash;
    }

    let parsed_hash = PasswordHash::new(db_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))?;
    // hashing is likely to block for some time; yield
    let str_hash = Secret::from(parsed_hash.serialize().to_string());
    spawn_blocking_with_tracing(move || verify_password_hash(str_hash, credentials.password))
        .await
        .context("Failed to spawn blocking hash verifier")
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))??;

    user_id.ok_or_else(|| AuthError::InvalidCredentials(anyhow!("invalid credentials")))
}

#[tracing::instrument(name = "Verify password hash", skip(expected_hash, password))]
fn verify_password_hash(
    expected_hash: Secret<String>,
    password: Secret<String>,
) -> Result<(), AuthError> {
    let parsed_hash = PasswordHash::new(expected_hash.expose_secret())
        .context("Failed to parse hash in PHC string format")
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))?;
    Argon2::default()
        .verify_password(password.expose_secret().as_bytes(), &parsed_hash)
        .context("invalid credentials")
        .map_err(|_| AuthError::InvalidCredentials(anyhow!("invalid credentials")))?;

    Ok(())
}

#[tracing::instrument(name = "Get stored credentials", skip(username, conn))]
pub async fn get_stored_credentials(
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

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
