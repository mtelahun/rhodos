use chrono::NaiveDateTime;
use diesel::prelude::*;

use super::schema::{ account, content };

#[derive(Queryable)]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub password: Option<String>,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = account)]
pub struct NewAccount<'a> {
    pub email: &'a str,
    pub password: &'a str,
}

#[derive(Queryable)]
#[diesel(belongs_to(Account))]
pub struct Content {
    pub id: i64,
    pub publisher_id: i64,
    pub cw: Option<String>,
    pub body: Option<String>,
    pub published: Option<bool>,
    pub published_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = content)]
pub struct NewContent<'a> {
    pub publisher_id: i64,
    pub cw: &'a str,
    pub body: &'a str,
}
