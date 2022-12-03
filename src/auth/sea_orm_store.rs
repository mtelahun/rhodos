use std::marker::PhantomData;

use async_trait::async_trait;
use axum_login::extractors::AuthContext;
use axum_login::secrecy::SecretVec;
use axum_login::{UserStore, AuthUser, RequireAuthorizationLayer};
use sea_orm::sea_query::Mode;
use sea_orm::{DatabaseConnection, EntityTrait};
use eyre::Result as AyreResult;

use crate::entities::user::Model;
//use crate::entities;

#[derive(Debug, Clone)]
struct MyUser(Model);

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Role {
    User,
    TenantAdmin,
    SuperAdmin,
}

impl AuthUser<Role> for MyUser {

    fn get_id(&self) -> String {
        format!("{}", self.0.id)
    }

    fn get_password_hash(&self) -> SecretVec<u8> {
        let opt = self.0.password.clone();
        if let Some(hash) = opt {
            return SecretVec::new(hash.clone().into())
        } else {
            return SecretVec::new("".to_string().into())
        }
    }

    fn get_role(&self) -> Option<Role> {
        //let opt = self.role;
        if self.0.role == "SuperAdmin".to_string() {
            return Some(Role::SuperAdmin)
        } else if self.0.role == "TenantAdmin".to_string() {
            return Some(Role::TenantAdmin)
        }
        Some(Role::User)
    }
}

type Auth = AuthContext<MyUser, SeaOrmStore<MyUser>, Role>;

type RequireAuth = RequireAuthorizationLayer<MyUser, Role>;

#[derive(Clone, Debug, Default)]
pub struct SeaOrmStore<User> {
    db: DatabaseConnection,
    _u: PhantomData<User>,
}

impl SeaOrmStore<MyUser> {

    fn new(db: &DatabaseConnection) -> Self {
        Self {
            db: db.clone(),
            _u: Default::default(),
        }
    }
 }

pub type PgSeaOrmStore<User, Role = ()> = SeaOrmStore<MyUser>;

#[async_trait]
impl<User, Role> UserStore<Role> for PgSeaOrmStore<MyUser, Role>
where
    Role: PartialOrd + PartialEq + Clone + Send + Sync + 'static,
    User: AuthUser<Role>,
{
    type User = User;

    async fn load_user(&self, user_id: &str) -> AyreResult<Option<User>> {
        
        let res = crate::entities::prelude::User::find_by_id(user_id.parse()?)
            .one(&self.db)
            .await?;

        Ok(Some(MyUser(res.unwrap())))
    }
}