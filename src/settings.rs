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
    pub env: Env,
}

const APP: &str = "rhodos";
const CONFIG_PREFIX: &str = "config";

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let env: Env = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "prod".into())
            .try_into()
            .expect("Failed to parse the APP_ENV environment variable");
        let base_path = std::env::current_dir().expect("Failed to determine current directory");
        let config_dir = base_path.join(CONFIG_PREFIX);
        let config_path = config_dir.join(APP);
        let env_config_path = config_dir.join(env.as_str());
        let builder = Config::builder()
            .set_default("env", env.as_str())?
            .set_default("server.port", "8080")?
            .set_default("server.log_level", "info")?
            .set_default("database.db_host", "")?
            .set_default("database.db_port", 5432)?
            .set_default("database.db_user", "")?
            .set_default("database.db_password", "")?
            .set_default("database.db_name", "prod")?
            .add_source(File::from(config_path))
            .add_source(File::from(env_config_path).required(false))
            .add_source(Environment::with_prefix(APP).separator("__"))
            .build()?;

        builder.try_deserialize()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub enum Env {
    Dev,
    Test,
    Prod,
}

impl Env {
    pub fn as_str(&self) -> &'static str {
        match self {
            Env::Dev => "Dev",
            Env::Test => "Test",
            Env::Prod => "Prod",
        }
    }
}

impl TryFrom<String> for Env {

    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "dev" => Ok(Self::Dev),
            "test" => Ok(Self::Test),
            "prod" => Ok(Self::Prod),
            other => Err(format!("{} is not a supported environment", other))
        }
    }
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Env::Dev => write!(f, "Dev"),
            Env::Test => write!(f, "Test"),
            Env::Prod => write!(f, "Prod"),
        }
    }
}
