use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

pub mod health_check;
pub mod index;
pub mod test;
pub mod user;

use health_check::health_check;
use index::index;

use crate::entities::instance;
use crate::{entities::prelude::*, settings::Settings};

#[derive(Clone, Debug)]
pub struct TenantData {
    domain: String,
    db: DatabaseConnection,
}

#[derive(Debug)]
pub struct AppState {
    domain: String,
    rhodos_db: Option<DatabaseConnection>,
    global_config: Settings,
    host_db_map: Arc<RwLock<HashMap<String, TenantData>>>,
}

pub async fn create_routes(global_config: &Settings) -> Result<Router, String> {
    let db = match Database::connect(global_config.database.connection_options()).await {
        Ok(conn) => conn,
        Err(e) => {
            return Err(format!(
                "create_routes: unable to connect to database {}: {}",
                global_config.database.db_name, e,
            ))
        }
    };
    let shared_state = Arc::new(AppState {
        domain: global_config.server.domain.clone(),
        rhodos_db: Some(db),
        global_config: global_config.clone(),
        host_db_map: Arc::new(RwLock::new(HashMap::new())),
    });

    let router = Router::new()
        .route("/", get(index))
        .route("/health_check", get(health_check))
        .route("/user", post(user::create))
        .layer(TraceLayer::new_for_http())
        .with_state(shared_state);

    Ok(router)
}

pub async fn get_db_from_host(
    host: &str,
    state: &Arc<AppState>,
) -> Result<DatabaseConnection, StatusCode> {
    let mut split = host.split(':');
    let mut key = "".to_string();
    if let Some(i) = split.next() {
        key = i.to_string();
    }

    if key == state.domain {
        if let Some(dbconn) = &state.rhodos_db {
            return Ok(dbconn.clone());
        }
    }
    let res = map_get(&key, state).await;
    match res {
        Ok(td) => Ok(td.db),
        Err(e) => Err(e),
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
            return Ok(value.clone());
        }
    }
    println!("did NOT find key: {}", key);
    let db_opt = &state.rhodos_db;
    if let Some(db) = db_opt {
        let instance = Instance::find()
            .filter(instance::Column::Domain.eq(key.clone()))
            .one(db)
            .await;
        if let Ok(Some(inst)) = instance {
            let db_url = make_db_uri(&inst);
            let res = map_set(&inst.domain, &db_url, state).await;
            if let Ok(td) = res {
                return Ok(td);
            }
        }
    } else {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Err(StatusCode::NOT_FOUND)
}

async fn map_set(
    domain: &String,
    db_url: &String,
    state: &Arc<AppState>,
) -> Result<TenantData, String> {
    println!("in map_set(): db_url = {}", db_url);
    let db = match Database::connect(db_url).await {
        Ok(conn) => conn,
        Err(e) => return Err(e.to_string()),
    };

    println!("found tenant: {}", domain);
    let td = TenantData {
        domain: domain.clone(),
        db,
    };

    println!("inserting...");
    let _ = &state
        .host_db_map
        .write()
        .await
        .insert(domain.clone(), td.clone());
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
    if !db_user.is_empty() {
        user_part = format!("{}:{}", db_user, db_pass);
    }

    if let Some(dh) = inst.db_host.clone() {
        db_host = dh;
    }
    if let Some(dpo) = inst.db_port {
        db_port = dpo as u16;
    }
    if !db_host.is_empty() {
        if !user_part.is_empty() {
            host_part = "@".to_string();
        }
        host_part = format!("{}{}", host_part, db_host);
        if db_port > 0 {
            host_part = format!("{}:{}", host_part, db_port);
        }
    }

    let res = format!(
        "postgres://{}{}/{}",
        user_part,
        host_part,
        inst.db_name.clone()
    );
    println!("db_uri = {}", res);
    res
}
