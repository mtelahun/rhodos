use std::sync::Arc;

use axum::{extract::Host, http::StatusCode, Extension};
use axum_login::extractors::AuthContext;
use argon2:: {
    Algorithm, Params,
    password_hash::{
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
    },
    Argon2
};
use sea_orm::EntityTrait;
use crate::entities::prelude::*;
use super::{xauth::{self, TestUser}, AppState, get_db_from_host};


pub async fn login_handler(
    mut auth: AuthContext<xauth::TestUser, xauth::TestUserStore>,
    Host(host): Host,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<(), StatusCode> {
    let hst = host.to_string();
    let db = get_db_from_host(&hst, &state).await;
    let db_user = User::find_by_id(1)
        .one(&db.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    
    // Verify password here
    
    if let Ok(Some(db_user)) = db_user {
        let user: TestUser = TestUser { id: db_user.id, password: db_user.password.unwrap() };
        auth.login(&user).await.unwrap();
    }

    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn logout_handler(mut auth: AuthContext<xauth::TestUser, xauth::TestUserStore>) {
    dbg!("Logging out user: {}", &auth.current_user);
    auth.logout().await;
}

fn hash_password(password: String) -> Result<String, StatusCode> {
    let params = Params::new(15000, 2, 1, None)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
    let salt = SaltString::generate(&mut rand::thread_rng());
    let hasher = Argon2::new(
        Algorithm::Argon2id,
        argon2::Version::V0x13, 
        params.unwrap(),
    );
    let hash = hasher.hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();
    
    Ok(hash)
 }

fn verify_password(password: String, hash: String) -> Result<(), StatusCode> {
    // let mut expected_hash = Secret::new(    
    //     "$argon2id$v=19$m=15000,t=2,p=1$gZiV/M1gPc22ElAH/Jh1Hw$\
    //     CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno".to_string()
    // );
    let expected_hash = PasswordHash::new(&hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    
    Argon2::default().verify_password(password.as_bytes(), &expected_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
