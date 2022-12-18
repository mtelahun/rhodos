use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000005_create_instance"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
CREATE TABLE instance (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    domain VARCHAR NOT NULL,
    db_name VARCHAR NOT NULL,
    db_host VARCHAR,
    db_port SMALLINT,
    db_user VARCHAR,
    db_password VARCHAR,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let sql = r#"SELECT rhodos_manage_updated_at('instance');"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    // Define how to rollback this migration
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = "DROP TABLE instance;";
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
