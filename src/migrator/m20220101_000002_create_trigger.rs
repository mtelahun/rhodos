// src/migrator/m20220602_000001_create_bakery_table.rs (create new file)

use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000002_create_trigger"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create updated_at column for each new table
        let sql = r#"
CREATE OR REPLACE FUNCTION rhodos_set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END
$$ LANGUAGE plpgsql;"#;
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(e),
        }
    }

    // Define how to rollback this migration
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = "DROP FUNCTION rhodos_set_updated_at";
        let stmt = Statement::from_string(manager.get_database_backend(), sql.to_owned());
        match manager.get_connection().execute(stmt).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
