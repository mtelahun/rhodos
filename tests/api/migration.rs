use librhodos::migrator::Migrator;
use sea_orm::Database;
use sea_orm_migration::{MigratorTrait, SchemaManager};

use crate::helpers::{connect_to_db, spawn_app};

#[tokio::test]
async fn rollback_all_migrations_and_reapply() {
    // Arrange
    let state = spawn_app().await;
    let db = match Database::connect(state.global_config.database.connection_options()).await {
        Ok(conn) => conn,
        _ => {
            panic!("couldn't get database connection")
        }
    };
    let client = connect_to_db(&state.db_name.clone()).await;
    client
        .execute(r#"DELETE FROM account;"#, &[])
        .await
        .expect("query to delete all rows from account table failed");
    client
        .execute(r#"DELETE FROM "user";"#, &[])
        .await
        .expect("query to delete all rows from 'user' table failed");

    // Act
    Migrator::refresh(&db).await.expect("Migration failed");

    // Assert
    let schema_manager = SchemaManager::new(&db);
    assert!(schema_manager
        .has_table("account")
        .await
        .expect("database does not have 'account' table"));
    assert!(schema_manager
        .has_table("content")
        .await
        .expect("database does not have 'content' table"));
    assert!(schema_manager
        .has_table("instance")
        .await
        .expect("database does not have 'instance' table"));
    assert!(schema_manager
        .has_table("microblog")
        .await
        .expect("database does not have 'microblog' table"));
    assert!(schema_manager
        .has_table("user")
        .await
        .expect("database does not have 'user' table"));
    assert!(schema_manager
        .has_table("user_token")
        .await
        .expect("database does not have 'user_token' table"));
    assert!(schema_manager
        .has_table("client_app")
        .await
        .expect("database does not have 'client_app' table"));
}
