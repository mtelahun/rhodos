use std::{sync::{Arc}, collections::HashMap, process};

use axum::{
    routing::get,
    Router, Extension, http::StatusCode
};
use sea_orm::{DatabaseConnection, EntityTrait, ColumnTrait, QueryFilter, Database};

pub mod index;
pub mod test;

use index::index;
use test::proxy;
use tokio::sync::RwLock;

use crate::{entities::instance, settings::Settings};
use crate::entities::prelude::*;

#[derive(Clone)]
pub struct TenantData {
    domain: String,
    db: DatabaseConnection,
}

pub struct AppState {
    domain: String,
    rhodos_db: Option<DatabaseConnection>,
    host_db_map: Arc<RwLock<HashMap<String, TenantData>>>,
}

pub async fn create_routes(db_url: &str, global_config: &Settings) -> Router {

    let db = match Database::connect(db_url)
        .await {
            Ok(conn) => conn,
            Err(_) => {
                eprintln!("unable to connect to database {}", global_config.database.db_name);
                process::exit(1)
            },
        };
    let shared_state = Arc::new(
        AppState{
            domain: global_config.server.domain.clone(),
            rhodos_db: Some(db), 
            host_db_map: Arc::new(RwLock::new(HashMap::new())) 
        }
    );

    Router::new()
        .route("/", get(index))
        .route("/proxy", get(proxy))
        .layer(Extension(shared_state))
}

pub async fn get_db_from_host(
    host: &String, 
    state: &Arc<AppState>
) -> Result<DatabaseConnection, StatusCode> {

    let split = host.split(':');
    let mut key = "".to_string();
    for i in split {
        key = i.to_string();
        break;
    }

    if key == state.domain {
        if let Some(dbconn) = &state.rhodos_db {
            println!("This is the main DB!");
            return Ok(dbconn.clone())
        }
    }
    let res = map_get(&key, &state).await;
    match res {
        Ok(td) => return Ok(td.db),
        Err(e) => return Err(e),
    }
}

async fn map_get(
    key: &String,
    //db: &DatabaseConnection,
    state: &Arc<AppState>,
) -> Result<TenantData, StatusCode> {
    println!("in map_get()");

    {
        println!("  reading...");
        let db_map = state.host_db_map.read().await;
        let found_tenant = db_map.get(key);

        if let Some(value) = found_tenant {
            println!("found key: {}", value.domain);
            return Ok(value.clone())
        }
    }
    println!("did NOT find key: {}", key);
    let db_opt = &state.rhodos_db;
    if let Some(db) = db_opt {
        let instance = Instance::find()
            .filter(instance::Column::Domain.eq(key.clone()))
            .one(db)
            .await;
        if let Ok(opt) = instance {
            if let Some(inst) = opt {
                let db_url = make_db_uri(&inst);
                let res = map_set(&inst.domain, &db_url, &state).await;
                if let Ok(td) = res {
                    
                    return Ok(td)
                }
            }
        }
    } else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
     
    Err(StatusCode::NOT_FOUND)
}

async fn map_set(
    domain: &String,
    db_url: &String,
    state: &Arc<AppState>,
) -> Result<TenantData, String> {
    println!("in map_set(): db_url = {}", db_url);
    let db = match Database::connect(db_url)
        .await {
            Ok(conn) => conn,
            Err(e) => return Err(e.to_string()),
        };

    println!("found tenant: {}", domain);
    let td = TenantData{ domain: domain.clone(), db: db };

    println!("inserting...");
    let _ = &state.host_db_map.write().await.insert(domain.clone(), td.clone());
    println!("insert done");

    Ok(td)
}

fn make_db_uri(inst: &instance::Model) -> String {

    println!("in make_db_uri()");
    let mut db_host: String = "".to_string();
    let mut db_port: u16 = 0;
    let mut db_user: String = "".to_string();
    let mut db_pass: String = "".to_string();
    let mut user_part: String = "".to_string();
    let mut host_part: String = "".to_string();

    if let Some(du) = inst.db_user.clone() {
        db_user = du;
    }
    if let Some(dpa) = inst.db_password.clone() {
        db_pass = dpa;
    }
    if db_user.len() > 0 {
        user_part = format!("{}:{}", db_user, db_pass);
    }

    if let Some(dh) = inst.db_host.clone() {
        db_host = dh;
    }
    if let Some(dpo) = inst.db_port {
        db_port = dpo as u16;
    }
    if db_host.len() > 0 {
        if user_part.len() > 0 {
            host_part = "@".to_string();
        }
        host_part = format!("{}{}", host_part, db_host);
        if db_port > 0 {
            host_part = format!("{}:{}", host_part, db_port);
        }
    }


    let res = format!("postgres://{}{}/{}", user_part, host_part, inst.db_name.clone());
    println!("db_uri = {}", res);
    res

}
