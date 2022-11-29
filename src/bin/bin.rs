extern crate slog;
extern crate slog_async;
extern crate slog_term;

use docopt::Docopt;
use dotenvy::dotenv;
use serde::Deserialize;
use slog::{Drain, Level, Logger};
use slog::{info, o};
use std::env;
use std::process::ExitCode;

use librhodos::migration;
use librhodos::db;
use librhodos::run;
use librhodos::settings;

const ENV_DBUSER: &'static str = "DB_USER";
const ENV_DBPASS: &'static str = "DB_PASSWORD";
const USAGE: &'static str = "
Usage: rhodos [options]

Options: -h, --help             Show this usage screen.
         -i, --init-db          Initialize database
         -l, --log-level=<crit,error,warning,info,debug>  Set log-level filter.
         -m, --migration        Migrate database
         -v, --version          Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_init_db: Option<bool>,
    flag_log_level: Option<LogLevel>,
}

#[derive(Debug, Deserialize)]
enum LogLevel { Crit, Error, Warning, Info, Debug }

#[tokio::main]
async fn main() -> ExitCode {

    let mut global_config: settings::Settings =
        settings::Settings::new().expect("unable to load global_configuration");

    let db_name = global_config.database.db_name.to_string();
    let db_host = global_config.database.db_host.to_string();
    let db_port = global_config.database.db_host.to_string();
    let mut db_user = global_config.database.db_user.to_string();
    let mut db_pass = global_config.database.db_password.to_string();

    // Get database username and password from .env
    dotenv().ok();
    let mut user_part = "".to_string();
    let mut host_part = "".to_string();
    
    // Figure out database uri
    if env::var(ENV_DBUSER).unwrap_or_else(|_| "".to_string()).len() > 0 {
        db_user = env::var(ENV_DBUSER).unwrap();
    }
    if env::var(ENV_DBPASS).unwrap_or_else(|_| "".to_string()).len() > 0 {
        db_pass = env::var(ENV_DBPASS).unwrap();
    }
    if db_user.len() > 0 {
        user_part = format!("{}:{}", db_user, db_pass);
    }
    if db_host.len() > 0 {
        host_part = format!("@{}", db_host);
        if db_port.len() > 0 {
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
        None => {},
    };
    if tmp.len() > 0 && tmp != log_level {
        global_config.server.log_level = tmp.to_string();
    };

    // Set log-level for logger
    let filter_level: Level;
    let log_level = global_config.server.log_level
        .to_string()
        .to_lowercase()
        .as_str()
        .to_owned();
    match log_level.as_str() {
        "debug" => filter_level = Level::Debug,
        "warning" => filter_level = Level::Warning,
        "error" => filter_level = Level::Error,
        "critical" => filter_level = Level::Critical,
        _ => filter_level = Level::Info,
    }

    // Create a drain hierarchy
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    
    // Get root logger
    let logger: Logger = Logger::root(
        drain.filter_level(filter_level).fuse(),
        o!("version" => env!("CARGO_PKG_VERSION")),
    );

    if args.flag_init_db.is_some() {
        // Initialize DB
        let _res = match migration::init(&server_url, &db_name, &logger).await {
            Ok(_) => { },
            Err(err) => {
                eprintln!("Initialization of {} failed: {}", db_name, err.to_string());
                return ExitCode::FAILURE
            }
        };

        // Migrate DB
        let db = match db::connect(&db_url, &logger).await {
            Ok(d) => { d },
            Err(e) => {
                eprintln!("Unable to connect to {}: {}", db_url, e.to_string());
                return ExitCode::FAILURE
            }
        };
        match migration::migrate(&db, &logger).await {
            Ok(_) => { },
            Err(e) => {
                eprintln!("Migration of {} failed: {}", db_name, e.to_string());
                return ExitCode::FAILURE
            }
        }
    }

    info!(logger, "Application Started");
    let res = run(&db_url, &logger).await;
    if let Err(e) = res {
        eprintln!("{}", e.to_string());
        return ExitCode::FAILURE;
    };

    info!(logger, "Application Stopped");
    return ExitCode::SUCCESS
}
