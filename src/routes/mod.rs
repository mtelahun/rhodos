use axum::{
    routing::{get, post},
    Router,
};
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

pub mod content;
pub mod health_check;
pub mod index;
pub mod test;
pub mod user;
pub mod user_confirm;

use health_check::health_check;
use index::index;

use crate::{entities::instance, error::TenantMapError};
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

pub async fn create_routes(
    db: DatabaseConnection,
    global_config: &Settings,
) -> Result<Router, String> {
    let shared_state = Arc::new(AppState {
        domain: global_config.server.domain.clone(),
        rhodos_db: Some(db),
        global_config: global_config.clone(),
        host_db_map: Arc::new(RwLock::new(HashMap::new())),
    });

    let router = Router::new()
        .route("/", get(index))
        .route("/content", post(content::create))
        .route("/health_check", get(health_check))
        .route("/user", post(user::create))
        .route("/user/confirm", get(user_confirm::confirm))
        .layer(TraceLayer::new_for_http())
        .with_state(shared_state);

    Ok(router)
}

pub async fn get_db_from_host(
    host: &str,
    state: &Arc<AppState>,
) -> Result<DatabaseConnection, TenantMapError> {
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

async fn map_get(key: &String, state: &Arc<AppState>) -> Result<TenantData, TenantMapError> {
    // Scope our RwLock
    {
        // Happy path: tenant is already in the Map
        let db_map = state.host_db_map.read().await;
        let found_tenant = db_map.get(key);

        if let Some(value) = found_tenant {
            return Ok(value.clone());
        }
    }

    // Tenant is not in the Map. Search for it in the instance table of the main db.
    //
    if state.rhodos_db.is_none() {
        return Err(TenantMapError::UnexpectedError(
            "no valid connection to main database".to_string(),
        ));
    }
    let db = state.rhodos_db.clone().unwrap();
    let instance = Instance::find()
        .filter(instance::Column::Domain.eq(key.clone()))
        .one(&db)
        .await
        .map_err(|e| TenantMapError::NotFound(e.to_string()))?
        .unwrap();
    let db_url = make_db_uri(&instance);
    let res = map_set(&instance.domain, &db_url, state).await?;
    assert_eq!(res.domain, instance.domain);

    Ok(res)
}

async fn map_set(
    domain: &str,
    db_url: &str,
    state: &Arc<AppState>,
) -> Result<TenantData, TenantMapError> {
    let db = Database::connect(db_url)
        .await
        .map_err(|e| TenantMapError::NotFound(e.to_string()))?;

    let td = TenantData {
        domain: domain.to_string(),
        db,
    };

    let _ = &state
        .host_db_map
        .write()
        .await
        .insert(domain.to_string(), td.clone());

    Ok(td)
}

fn make_db_uri(inst: &instance::Model) -> String {
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
