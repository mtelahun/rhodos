use diesel::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use std::env;

pub mod models;
pub mod schema;

const ENV_DBURL: &'static str = "DATABASE_URL";

pub fn connect() -> PgConnection {
    dotenv().ok();

    let db_url: String = env::var(ENV_DBURL)
        .expect("Missing environment variable DATABSE_URL");
    PgConnection::establish(&db_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", db_url))
}

pub fn browse_content(conn: &mut PgConnection, limit: i32) -> Vec<models::Content> {
    use schema::content::dsl::*;

    let mut lim: i32 = limit;
    if lim < 0 || lim > 500 {
        lim = 0;
    }
    content.filter(published.eq(true))
        .limit(i64::from(lim))
        .load::<models::Content>(conn)
        .expect("Error loading content")

}

pub fn create_content(
    conn: &mut PgConnection, new_content: &models::NewContent
) -> models::Content {
    use schema::content;

    diesel::insert_into(content::table)
        .values(new_content)
        .get_result(conn)
        .expect("Error saving new post")
}

pub fn publish_content(conn: &mut PgConnection, id: i64) -> models::Content {
    use schema::content::dsl::{content, published};
    diesel::update(content.find(id))
        .set(published.eq(true))
        .get_result::<models::Content>(conn)
        .unwrap()
}

pub fn delete_content(conn: &mut PgConnection, del_id: i64) -> Result<usize, diesel::result::Error> {
    use schema::content::dsl::*;
    diesel::delete(content.filter(id.eq(del_id)))
        .execute(conn)
}
