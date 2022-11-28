use chrono;
use sea_orm::{DatabaseConnection, Database, DbErr};
use slog::{Logger, debug, info};

pub async fn connect(
    url: &String,
    logger: &Logger
) -> Result<DatabaseConnection, DbErr> {

    let db = match Database::connect(url).await {
        Ok(db) => { db },
        Err(e) => { 
            info!(logger, "Unable to connect to database {}", url);
            return Err(e) 
        }
    };
    debug!(logger, "Connected to {}", url);
    Ok(db)
}

pub mod content {
    use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait};
    use super::super::entities::{prelude::*, *};

    pub async fn browse(
        db: &DatabaseConnection, 
        id: i64) -> Result<Vec<content::Model>, String> {
            let res = match Content::find_by_id(id).one(db)
                .await {
                    Ok(res) => { vec![res.unwrap()] },
                    Err(e) => { return Err(e.to_string()) }
                };

            Ok(res)
    }

    pub async fn create(
        db: &DatabaseConnection,
        new_content: content::ActiveModel
    ) -> Result<i64, String> {

        let res = match Content::insert(new_content).exec(db)
            .await {
                Ok(insert_result) => { insert_result.last_insert_id },
                Err(e) => { return Err(e.to_string()) }
            };
        
        Ok(res)
    }

    pub async fn publish(
        db: &DatabaseConnection,
        id: i64
    ) -> Result<bool, String> {

        let cnt = content::ActiveModel {
            id: sea_orm::ActiveValue::Set(id),
            published: sea_orm::ActiveValue::Set(Some(true)),
            published_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now().naive_utc())),
            ..Default::default()
        };
        match cnt.update(db).await {
            Ok(_) => { Ok(true) },
            Err(e) => { return Err(e.to_string()) }
        }
    }

}
