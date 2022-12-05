extern crate slog;
extern crate slog_async;
extern crate slog_term;

use docopt::Docopt;
use dotenvy::dotenv;
use serde::Deserialize;
use slog::{debug, info, o};
use slog::{Drain, Level, Logger};
use std::env;
use std::net::TcpListener;
use std::process::ExitCode;

use librhodos::migration::{self, DbUri};
use librhodos::settings;
use librhodos::{get_router, serve};

const ENV_DBUSER: &str = "DB_USER";
const ENV_DBPASS: &str = "DB_PASSWORD";
const USAGE: &str = "
Usage: rhodos [options]
       rhodos [options] [--init-db [--database URI...]]
       rhodos (--help | --version)

Options: -d --database URI      URI of database to initialize
         -h, --help             Show this usage screen.
         -i, --init-db          Initialize database
         -l, --log-level=<crit,error,warning,info,debug>  Set log-level filter.
         -m, --migration        Migrate database
         -v, --version          Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_database: Vec<String>,
    flag_init_db: Option<bool>,
    flag_log_level: Option<LogLevel>,
}

#[derive(Debug, Deserialize)]
enum LogLevel {
    Crit,
    Error,
    Warning,
    Info,
    Debug,
}

#[tokio::main]
async fn main() -> ExitCode {
    let mut global_config = settings::Settings::new()
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e);
            ExitCode::FAILURE
        })
        .unwrap();

    let db_name = global_config.database.db_name.to_string();
    let db_host = global_config.database.db_host.to_string();
    let db_port = global_config.database.db_port.to_string();
    let mut db_user = global_config.database.db_user.to_string();
    let mut db_pass = global_config.database.db_password.to_string();

    // Get database username and password from .env
    dotenv().ok();
    let mut user_part = "".to_string();
    let mut host_part = "".to_string();

    // Figure out database uri
    if !(env::var(ENV_DBUSER).unwrap_or_else(|_| "".to_string())).is_empty() {
        db_user = env::var(ENV_DBUSER).unwrap();
    }
    if !(env::var(ENV_DBPASS).unwrap_or_else(|_| "".to_string())).is_empty() {
        db_pass = env::var(ENV_DBPASS).unwrap();
    }
    if !db_user.is_empty() {
        user_part = format!("{}:{}", db_user, db_pass);
    }
    if !db_host.is_empty() {
        host_part = format!("@{}", db_host);
        if !db_port.is_empty() {
            host_part = format!("{}:{}", host_part, db_port);
        }
    }
    let server_url = format!("postgres://{}{}", user_part, host_part);
    let db_url = format!("{}/{}", server_url, db_name);

    // Process command line arguments
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    // Over-ride log-level global_config from command-line
    let mut tmp = "";
    let log_level = global_config.server.log_level.to_lowercase().to_owned();
    match args.flag_log_level {
        Some(LogLevel::Crit) => tmp = "critical",
        Some(LogLevel::Error) => tmp = "error",
        Some(LogLevel::Warning) => tmp = "warning",
        Some(LogLevel::Info) => tmp = "info",
        Some(LogLevel::Debug) => tmp = "debug",
        None => {}
    };
    if !tmp.is_empty() && tmp != log_level {
        global_config.server.log_level = tmp.to_string();
    };

    // Set log-level for logger
    let log_level = global_config
        .server
        .log_level
        .to_string()
        .to_lowercase()
        .as_str()
        .to_owned();
    let filter_level = match log_level.as_str() {
        "debug" => Level::Debug,
        "warning" => Level::Warning,
        "error" => Level::Error,
        "critical" => Level::Critical,
        _ => Level::Info,
    };

    // Create a drain hierarchy
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    // Get root logger
    let logger: Logger = Logger::root(
        drain.filter_level(filter_level).fuse(),
        o!(
            "version" => env!("CARGO_PKG_VERSION"),
            "env" => global_config.env.to_string(),
        ),
    );

    if args.flag_init_db.is_some() {
        info!(logger, "Database initialization started");
        let mut uri_list: Vec<DbUri> = vec![];
        if args.flag_database.is_empty() {
            uri_list.push(DbUri {
                full: format!("{}/{}", server_url, db_name),
                path: server_url.clone(),
                db_name: db_name.clone(),
            });
        } else {
            for u in args.flag_database {
                let tmp = u.to_string();
                let vec: Vec<&str> = tmp.split('/').collect();
                let server_part = vec[0].to_string();
                let db_part = vec[1].to_string();
                uri_list.push(DbUri {
                    full: format!("postgres://{}", u),
                    path: format!("postgres://{}", server_part),
                    db_name: db_part,
                });
            }
        }

        for uri in uri_list {
            let _ = migration::initialize_and_migrate_database(&uri, &logger)
                .await
                .map_err(|err_str| {
                    eprintln!("{}", err_str);
                });
        }
        return ExitCode::SUCCESS;
    }

    info!(logger, "Application Started");
    debug!(logger, "database url: {}", db_url);
    let router = get_router(&db_url, &global_config)
        .await
        .map_err(|e| {
            eprintln!("{}", e);
        })
        .unwrap();
    let listener = TcpListener::bind(format!("0.0.0.0:{}", global_config.server.port))
        .map_err(|e| {
            eprintln!("unable to parse local address: {}", e);
        })
        .unwrap();
    tokio::join!(serve(router, listener));

    info!(logger, "Application Stopped");
    ExitCode::SUCCESS
}
