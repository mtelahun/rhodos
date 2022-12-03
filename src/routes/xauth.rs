use axum::async_trait;
use axum_login::{secrecy::SecretVec, AuthUser, UserStore};
use sea_orm::{DatabaseConnection, EntityTrait};
use crate::entities::prelude::User as DbUser;


type Result<T = ()> = std::result::Result<T, eyre::Error>;

#[derive(Debug, Clone)]
pub struct TestUser {
    pub id: i64,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum TestRole {
    User,
    TenantAdmin,
    SuperAdmin,
}

impl<Role> AuthUser<Role> for TestUser
where
    Role: PartialOrd + PartialEq + Clone + Send + Sync + 'static,
{
    fn get_id(&self) -> String {
        format!("{}", self.id)
    }

    fn get_password_hash(&self) -> axum_login::secrecy::SecretVec<u8> {
        SecretVec::new(self.password.clone().into())
    }
}

#[derive(Debug, Clone)]
pub struct TestUserStore {
    conn: DatabaseConnection,
}

impl TestUserStore {
    pub fn new(conn: &DatabaseConnection) -> Self {
        Self { conn: conn.clone() }
    }
}

#[async_trait]
impl<Role> UserStore<Role> for TestUserStore
where
    Role: PartialOrd + PartialEq + Clone + Send + Sync + 'static,
{
    type User = TestUser;

    async fn load_user(&self, user_id: &str) -> Result<Option<Self::User>> {
        let id = user_id.parse()?;
        let user = DbUser::find_by_id(id)
            .one(&self.conn)
            .await?;
        match user {
            Some(u) => Ok(Some(TestUser {
                id: u.id,
                password: u.password.unwrap(),
            })),
            None => Ok(None),
        }
    }
}

type AuthContext = axum_login::extractors::AuthContext<TestUser, TestUserStore>;
