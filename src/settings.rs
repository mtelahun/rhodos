use std::fmt;

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub domain: String,
    pub port: u16,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_password: String,
    pub db_name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: Server,
    pub database: Database,
    pub env: ENV,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let env = std::env::var("RHODOS_ENV").unwrap_or_else(|_| "Prod".into());
        let mut config_path = CONFIG_FILE_PATH.to_string();
        if env != "Prod" {
            config_path = format!("{}-{}.toml", CONFIG_FILE_PREFIX, env);
        }
        let builder = Config::builder()
            .set_default("env", env)?
            .set_default("server.port", "8080")?
            .set_default("server.log_level", "info")?
            .set_default("database.db_host", "")?
            .set_default("database.db_port", 5432)?
            .set_default("database.db_user", "")?
            .set_default("database.db_password", "")?
            .set_default("database.db_name", "prod")?
            .add_source(File::with_name(CONFIG_FILE_PATH))
            .add_source(File::with_name(&config_path).required(false))
            .add_source(Environment::with_prefix("rhodos").separator("_"))
            .build()?;

        builder.try_deserialize()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum ENV {
    Dev,
    Test,
    Prod,
}

impl fmt::Display for ENV {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ENV::Dev => write!(f, "Dev"),
            ENV::Test => write!(f, "Test"),
            ENV::Prod => write!(f, "Prod"),
        }
    }
}

const CONFIG_FILE_PREFIX: &str = "./config/rhodos";
const CONFIG_FILE_PATH: &str = "./config/rhodos.ini";
