extern crate slog;
extern crate slog_async;
extern crate slog_term;

use docopt::Docopt;
use librhodos::telemetry::{get_subscriber, init_subscriber};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::net::TcpListener;
use std::process::ExitCode;

use librhodos::migration::{self, DbUri};
use librhodos::settings::{self, override_db_password};
use librhodos::APP_NAME;
use librhodos::{get_router, serve};

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
    let subscriber = get_subscriber(
        APP_NAME.into(),
        "rhodos=info,tower_http=info".into(),
        std::io::stdout,
    );
    init_subscriber(subscriber);

    let mut global_config = settings::Settings::new(None, None)
        .map_err(|e| {
            eprintln!("Failed to get settings: {}", e);
            ExitCode::FAILURE
        })
        .unwrap();

    override_db_password(&mut global_config);

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
    let _log_level = global_config
        .server
        .log_level
        .to_string()
        .to_lowercase()
        .as_str()
        .to_owned();

    if args.flag_init_db.is_some() {
        tracing::info!("Database initialization started");
        let mut uri_list: Vec<DbUri> = vec![];
        if args.flag_database.is_empty() {
            uri_list.push(DbUri {
                full: Secret::from(format!(
                    "{}/{}",
                    global_config.database.connection_string().expose_secret(),
                    global_config.database.db_name
                )),
                path: Secret::from(format!(
                    "{}/{}",
                    global_config
                        .database
                        .connection_string_no_db()
                        .expose_secret(),
                    global_config.database.db_name
                )),
                db_name: global_config.database.db_name.clone(),
            });
        } else {
            for u in args.flag_database {
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

        for uri in uri_list {
            let _ = migration::initialize_and_migrate_database(&uri)
                .await
                .map_err(|err_str| {
                    eprintln!("{}", err_str);
                });
        }
        return ExitCode::SUCCESS;
    }

    tracing::info!("Application Started");
    let router = get_router(&global_config)
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

    tracing::info!("Application Stopped");
    ExitCode::SUCCESS
}
