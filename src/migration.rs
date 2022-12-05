use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use slog::{debug, info, Logger};

use super::migrator::Migrator;

pub async fn init<'a>(server_url: &String, db_name: &String, logger: &Logger) -> Result<(), DbErr> {
    let db = Database::connect(server_url).await?;
    let _db = &match db.get_database_backend() {
        DbBackend::Postgres => {
            info!(logger, "Creating database: {}", db_name);
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!(r#"CREATE DATABASE "{}";"#, db_name),
            ))
            .await?;

            let url = format!("{}/{}", server_url, db_name);
            debug!(logger, "Attempting connection to: {}", url);
            Database::connect(url).await?;
            debug!(logger, "Connection succeeded!");
        }
        _ => panic!("Expected db engine: postgresql!"),
    };

    Ok(())
}

pub async fn migrate(db: &DatabaseConnection, logger: &Logger) -> Result<(), DbErr> {
    info!(logger, "Starting migration");
    Migrator::refresh(db).await?;

    let schema_manager = SchemaManager::new(db);
    assert!(schema_manager.has_table("account").await?);

    Ok(())
}
