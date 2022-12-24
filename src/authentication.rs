use anyhow::{anyhow, Context};
use argon2::{
    password_hash::SaltString, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
};
use sea_orm::{DatabaseConnection, IntoActiveModel, Set};
use secrecy::{ExposeSecret, Secret};

use crate::{error::error_chain_fmt, orm, telemetry::spawn_blocking_with_tracing};

#[derive(Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Change password", skip(current_password, new_password, conn))]
pub async fn change_password(
    user_id: i64,
    current_password: Secret<String>,
    new_password: Secret<String>,
    conn: &DatabaseConnection,
) -> Result<(), AuthError> {
    let model = orm::get_user_model_by_id(user_id, conn)
        .await
        .context("Failed to get get username from id,")?;
    let username = model.email;

    let mut credentials = Credentials {
        username,
        password: current_password,
    };
    let current_password_matches = password_ok(credentials.clone(), conn).await?;
    if !current_password_matches {
        return Err(AuthError::CurrentPasswordFail(anyhow::anyhow!(
            "current password does not match"
        )));
    }

    credentials.password = new_password;
    let _res = update_credential(user_id, credentials, conn)
        .await
        .context("Failed to update new password")?;

    Ok(())
}

#[tracing::instrument(name = "Check current password matches", skip(credentials, conn))]
pub async fn password_ok(
    credentials: Credentials,
    conn: &DatabaseConnection,
) -> Result<bool, AuthError> {
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    match validate_credentials(credentials, conn).await {
        Ok(_) => Ok(true),
        Err(e) => match e {
            AuthError::InvalidCredentials(_) => Ok(false),
            _ => Err(AuthError::UnexpectedError(e.into())),
        },
    }
}

#[tracing::instrument(name = "Update credential", skip())]
pub async fn update_credential(
    user_id: i64,
    credentials: Credentials,
    conn: &DatabaseConnection,
) -> Result<i64, AuthError> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    let password_hash = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(credentials.password.expose_secret().as_bytes(), &salt)
    .unwrap()
    .to_string();

    let mut orm_user = orm::get_user_model_by_id(user_id, conn)
        .await
        .context("Failed to retrieve the user record.")?
        .into_active_model();
    orm_user.password = Set(password_hash);
    let user_id = orm::update_credential(user_id, orm_user, conn)
        .await
        .context("Failed to update user record")?;

    Ok(user_id)
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
    let dummy_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    let db_password_hash;
    if let Some((stored_uid, stored_hash)) = get_stored_credentials(&credentials.username, conn)
        .await
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))?
    {
        user_id = Some(stored_uid);
        db_password_hash = stored_hash;
    } else {
        db_password_hash = dummy_hash.clone();
        tracing::debug!("Comparing against dummy hash");
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
    let password = orm::get_credential(username, conn)
        .await
        .context("Failed to retrieve stored credentials")?;

    Ok(password)
}

#[derive(thiserror::Error)]
pub enum AuthError {
    #[error("current password does not match")]
    CurrentPasswordFail(#[source] anyhow::Error),
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}
