extern crate slog;
extern crate slog_async;
extern crate slog_term;

use async_std::task;
use docopt::Docopt;
use dotenvy::dotenv;
use serde::Deserialize;
use slog::{Drain, Level, Logger};
use slog::{info, o};
use std::env;
use std::process::ExitCode;

use librhodos::migration;
use librhodos::db;

const ENV_DBHOST: &'static str = "DB_HOST";
const ENV_DBNAME: &'static str = "DB_NAME";
const ENV_DBUSER: &'static str = "DB_USER";
const ENV_DBPASS: &'static str = "DB_PASSWORD";
const USAGE: &'static str = "
Usage: rhodos [options]

Options: -h, --help             Show this usage screen.
         -l, --log-level=<crit,error,warning,info,debug>  Set log-level filter [default: info].
         -i, --init-db          Initialize database
         -m, --migration        Migrate database
         -v, --version          Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_log_level: Option<LogLevel>,
    flag_init_db: Option<bool>,
}

#[derive(Debug, Deserialize)]
enum LogLevel { Crit, Error, Warning, Info, Debug }

fn main() -> ExitCode {

    dotenv().ok();
    let mut user_part = "".to_string();
    let mut host_part = "".to_string();
    let db_name = match env::var(ENV_DBNAME) {
        Ok(name) => { name },
        Err(err)=> { 
            eprintln!("DB_NAME: {}", err.to_string());
            return ExitCode::FAILURE;
    }
    };
    let db_host = env::var(ENV_DBHOST).or_else(|err| return Err(err)).unwrap();
    let db_user = env::var(ENV_DBUSER).or_else(|err| return Err(err)).unwrap();
    let db_pass = env::var(ENV_DBPASS).or_else(|err| return Err(err)).unwrap();
    if db_user.len() > 0 {
        user_part = format!("{}:{}", db_user, db_pass);
    }
    if db_host.len() > 0 {
        host_part = format!("@{}", db_host);
    }
    let server_url = format!("postgres://{}{}", user_part, host_part);
    let db_url = format!("{}/{}", server_url, db_name).to_string();

    // Process command line arguments
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    // Create a drain hierarchy
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    
    // Get root logger
    let filter_level: Level;
    match args.flag_log_level {
        Some(LogLevel::Crit) => filter_level = Level::Critical,
        Some(LogLevel::Error) => filter_level = Level::Error,
        Some(LogLevel::Warning) => filter_level = Level::Warning,
        Some(LogLevel::Debug) => filter_level = Level::Debug,
        _ => filter_level = Level::Info,
    }
    let logger: Logger = Logger::root(
        drain.filter_level(filter_level).fuse(),
        o!("version" => env!("CARGO_PKG_VERSION")),
    );

    if args.flag_init_db.is_some() {
        // Initialize DB
        let _res = match task::block_on(migration::init(&server_url, &db_name, &logger)) {
            Ok(_) => { },
            Err(err) => {
                eprintln!("Initialization of {} failed: {}", db_name, err.to_string());
                return ExitCode::FAILURE
            }
        };

        // Migrate DB
        let db = match task::block_on(db::connect(&db_url, &logger)) {
            Ok(d) => { d },
            Err(e) => {
                eprintln!("Unable to connect to {}: {}", db_url, e.to_string());
                return ExitCode::FAILURE
            }
        };
        match task::block_on(migration::migrate(&db, &logger)) {
            Ok(()) => { },
            Err(e) => {
                eprintln!("Migration of {} failed: {}", db_name, e.to_string());
                return ExitCode::FAILURE
            }
        }
    }

    info!(logger, "Application Started");

    return ExitCode::SUCCESS
}
