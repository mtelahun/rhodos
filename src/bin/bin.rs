extern crate slog;
extern crate slog_async;
extern crate slog_term;

use docopt::Docopt;
use serde::Deserialize;
use slog::{Drain, Level, Logger};
use slog::{info, o};

const USAGE: &'static str = "
Usage: rhodos [options]

Options: -h, --help             Show this usage screen.
         -v, --version          Show version.
         -l, --log-level=<crit,error,warning,info,debug>  Set log-level filter [default: info].
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_log_level: Option<LogLevel>,
}

#[derive(Debug, Deserialize)]
enum LogLevel { Crit, Error, Warning, Info, Debug }

fn main() {

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

    info!(logger, "Application Started");
}
