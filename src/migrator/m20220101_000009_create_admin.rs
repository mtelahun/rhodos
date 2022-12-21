use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000009_create_admin"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
INSERT INTO "user" (name, email, password, confirmed)
    VALUES (
        'Administrator',
        'admin',
        '$argon2id$v=19$m=15000,t=2,p=1$laqlSNfx8l3DlrLRQTWgzA$qE19C+eq4KraG7HVuu9hpBR0ItqMUgeqgz5G4EPxb3E',
        TRUE
    )
;"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(e),
        }
    }

    // Define how to rollback this migration
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"DELETE FROM "user" WHERE email='admin';"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
