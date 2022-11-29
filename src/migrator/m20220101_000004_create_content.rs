
use sea_orm::{Statement, ConnectionTrait};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000004_create_content"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        let sql = r#"
CREATE TABLE content (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    publisher_id BIGINT NOT NULL,
    cw VARCHAR,
    body VARCHAR,
    published BOOLEAN,
    published_at TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_publisher
        FOREIGN KEY(publisher_id)
            REFERENCES account
)"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager
            .get_connection()
            .execute(stmt)
            .await {
                Ok(_) => { },
                Err(e) => { return Err(e) }
            }
        let sql = r#"SELECT rhodos_manage_updated_at('content')"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager
            .get_connection()
            .execute(stmt)
            .await {
                Ok(_) => Ok(()),
                Err(e) => { Err(e) }
            }
    }

    // Define how to rollback this migration
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = "DROP TABLE ACCOUNT;";
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager
            .get_connection()
            .execute(stmt)
            .await {
                Ok(_) => Ok(()),
                Err(e) => { Err(e) }
            }
    }
}
