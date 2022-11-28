use sea_orm_migration::prelude::*;

mod m20220101_000001_create_trigger;
mod m20220101_000002_create_trigger;
mod m20220101_000003_create_account;
mod m20220101_000004_create_content;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_trigger::Migration),
            Box::new(m20220101_000002_create_trigger::Migration),
            Box::new(m20220101_000003_create_account::Migration),
            Box::new(m20220101_000004_create_content::Migration),
        ]
    }
}
