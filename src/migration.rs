use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use secrecy::{ExposeSecret, Secret};

use super::db;
use super::migrator::Migrator;

#[derive(Debug)]
pub struct DbUri {
    pub full: Secret<String>,
    pub path: Secret<String>,
    pub db_name: String,
}

pub async fn init<'a>(server_url: &Secret<String>, db_name: &String) -> Result<(), DbErr> {
    let db = Database::connect(server_url.expose_secret()).await?;
    let _db = &match db.get_database_backend() {
        DbBackend::Postgres => {
            tracing::info!("Creating database: {}", db_name);
            db.execute(Statement::from_string(
                db.get_database_backend(),
                format!(r#"CREATE DATABASE "{}";"#, db_name),
            ))
            .await?;

            let url = Secret::from(format!("{}/{}", server_url.expose_secret(), db_name));
            tracing::debug!("Attempting connection to: postgres://****/{}", db_name);
            Database::connect(url.expose_secret()).await?;
            tracing::debug!("Connection succeeded!");
        }
        _ => panic!("Expected db engine: postgresql!"),
    };

    Ok(())
}

pub async fn migrate(db: &DatabaseConnection) -> Result<(), DbErr> {
    tracing::info!("Starting migration");
    Migrator::up(db, None).await?;

    let schema_manager = SchemaManager::new(db);
    assert!(schema_manager.has_table("account").await?);

    Ok(())
}

pub async fn initialize_and_migrate_database(uri: &DbUri) -> Result<(), String> {
    // Initialize DB
    tracing::debug!("Initializing database: postgres://****/{}", uri.db_name);

    let _ = init(&uri.path, &uri.db_name).await.map_err(|e| {
        Err::<(), std::string::String>(format!("Initialization of {} failed: {}", uri.db_name, e))
    });

    // Migrate DB
    let db_conn = db::connect(uri.full.expose_secret())
        .await
        .map_err(|e| {
            Err::<(), std::string::String>(format!(
                "Unable to connect to postgres://****/{}: {}",
                uri.db_name, e
            ))
        })
        .unwrap();
    let _ = migrate(&db_conn).await.map_err(|e| {
        Err::<(), std::string::String>(format!("Migration of {} failed: {}", uri.db_name, e))
    });

    tracing::info!("Database(s) initialization finished");
    Ok(())
}
