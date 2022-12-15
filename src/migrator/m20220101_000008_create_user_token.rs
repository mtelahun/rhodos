use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000008_create_user_token"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE "user_token" (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    token VARCHAR UNIQUE NOT NULL,
    user_id BIGINT NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_user
        FOREIGN KEY(user_id)
            REFERENCES "user"
);"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let sql = r#"SELECT rhodos_manage_updated_at('user_token');"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    // Define how to rollback this migration
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"DROP TABLE 'user_token';"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
