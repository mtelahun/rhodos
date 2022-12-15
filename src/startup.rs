use axum::Router;
use docopt::Docopt;
use secrecy::Secret;
use serde::Deserialize;
use std::net::TcpListener;

use crate::{
    get_router,
    migration::{self, DbUri},
    settings::Settings,
};

#[derive(Debug, Deserialize)]
pub struct Args {
    pub flag_database: Vec<String>,
    pub flag_init_db: Option<bool>,
}

const USAGE: &str = "
Usage: rhodos [options]
       rhodos [options] [--init-db [--database URI...]]
       rhodos (--help | --version)

Options: -d --database URI      URI of database to initialize
         -h, --help             Show this usage screen.
         -i, --init-db          Initialize database
         -m, --migration        Migrate database
         -v, --version          Show version.
";

pub fn process_command_line() -> Args {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    args
}

pub async fn initialize_database(args: &Args, global_config: &Settings) -> Result<(), String> {
    tracing::info!("Database initialization started");
    let mut uri_list: Vec<DbUri> = vec![];
    if args.flag_database.is_empty() {
        uri_list.push(DbUri {
            full: global_config.database.connection_string(),
            path: global_config.database.connection_string_no_db(),
            db_name: global_config.database.db_name.clone(),
        });
    } else {
        for u in args.flag_database.clone() {
            let tmp = u.to_string();
            let vec: Vec<&str> = tmp.split('/').collect();
            let server_part = vec[0].to_string();
            let db_part = vec[1].to_string();
            uri_list.push(DbUri {
                full: Secret::from(format!("postgres://{}", u)),
                path: Secret::from(format!("postgres://{}", server_part)),
                db_name: db_part,
            });
        }
    }

    if uri_list.is_empty() {
        tracing::error!("No databases to initialize!")
    };
    for uri in uri_list {
        migration::initialize_and_migrate_database(&uri).await?;
    }

    Ok(())
}

pub async fn build(
    global_config: &Settings,
    bind_address: Option<String>,
) -> (Router, TcpListener) {
    let router = get_router(global_config)
        .await
        .map_err(|e| {
            eprintln!("{}", e);
        })
        .unwrap();

    let mut addr = format!("0.0.0.0:{}", global_config.server.port);
    if bind_address.is_some() {
        addr = bind_address.unwrap();
    }
    let listener = TcpListener::bind(addr)
        .map_err(|e| {
            eprintln!("unable to parse local address: {}", e);
        })
        .unwrap();

    (router, listener)
}
