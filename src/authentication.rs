use anyhow::{anyhow, Context};
use argon2::{
    password_hash::SaltString, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set};
use secrecy::{ExposeSecret, Secret};

use crate::{
    entities::{prelude::*, user},
    telemetry::spawn_blocking_with_tracing,
};

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
    let username = get_username_by_id(user_id, conn)
        .await
        .context("Failed to get get username from id,")?;

    let mut credentials = Credentials {
        username,
        password: current_password,
    };
    let current_password_matches = current_password_ok(credentials.clone(), conn).await?;
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
pub async fn current_password_ok(
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

    let mut db_user = User::find_by_id(user_id)
        .one(conn)
        .await
        .context("Failed to retrieve the user record.")?
        .unwrap()
        .into_active_model();
    db_user.password = Set(password_hash);
    let res = User::update(db_user)
        .filter(user::Column::Id.eq(user_id))
        .exec(conn)
        .await
        .context("Failed to update user record")?;

    Ok(res.id)
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
    let mut db_password_hash = dummy_hash.clone();
    if let Some((stored_uid, stored_hash)) = get_stored_credentials(&credentials.username, conn)
        .await
        .map_err(|e| AuthError::UnexpectedError(anyhow!(e)))?
    {
        user_id = Some(stored_uid);
        db_password_hash = stored_hash;
    }
    if db_password_hash.expose_secret() == dummy_hash.expose_secret() {
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
    let model = User::find()
        .filter(user::Column::Email.eq(username))
        .one(conn)
        .await
        .context("Failed to retrieve stored credentials")?
        .map(|m| (m.id, Secret::new(m.password)));

    Ok(model)
}

#[tracing::instrument(name = "Get username", skip(conn))]
pub async fn get_username_by_id(
    user_id: i64,
    conn: &DatabaseConnection,
) -> Result<String, AuthError> {
    let model = User::find_by_id(user_id)
        .one(conn)
        .await
        .map_err(|e| {
            AuthError::UnexpectedError(anyhow!(format!("Failed to retrieve a user record: {}", e)))
        })?
        .unwrap();

    Ok(model.email)
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("current password does not match")]
    CurrentPasswordFail(#[source] anyhow::Error),
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
