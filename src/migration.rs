use sea_orm::{Database, DbBackend, Statement, ConnectionTrait, ConnectOptions, DatabaseConnection};
use sea_orm_migration::prelude::*;
use slog::{
    Logger,
    debug,
    info,
};
use std::time::Duration;

use super::migrator::Migrator;

pub async fn init(
    server_url: &String,
    db_name: &String,
    logger: &Logger
) -> Result<(), DbErr> {
    let db = Database::connect(server_url).await?;
    let _db = &match db.get_database_backend() {
        DbBackend::Postgres => {
            info!(logger, "Creating database: {}", db_name);
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!(r#"CREATE DATABASE "{}";"#, db_name),
            ))
            .await?;

            let mut opt =
                ConnectOptions::new(format!("{}/{}", server_url, db_name).into()).to_owned();
            opt.connect_timeout(Duration::from_secs(5));
            debug!(logger, "Attempting connection to: {}", opt.get_url());
            Database::connect(opt)
                .await?;
            debug!(logger, "Connection succeeded!");
        }
        _ => panic!("Expected db engine: postgresql!"),
    };

    Ok(())
}

pub async fn migrate(
    db: &DatabaseConnection,
    logger: &Logger
) -> Result<(), DbErr> {
    info!(logger, "Starting migration");
    Migrator::refresh(&db).await?;

    let schema_manager = SchemaManager::new(&db);
    assert!(schema_manager.has_table("account").await?);

    Ok(())
}
