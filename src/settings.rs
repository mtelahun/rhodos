use config::{Config, ConfigError, Environment, File};
use dotenvy::dotenv;
use sea_orm::ConnectOptions;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::{env, fmt, path::PathBuf, time::Duration};
use tracing::log;

use crate::domain::UserEmail;

use super::APP_NAME;

const ENV_DBPASS: &str = "DB_PASSWORD";

pub fn override_db_password(global_config: &mut Settings) {
    // Get database password from .env
    dotenv().ok();

    if env::var(ENV_DBPASS).is_ok() && !env::var(ENV_DBPASS).unwrap().is_empty() {
        global_config.database.db_password = Secret::from(env::var(ENV_DBPASS).unwrap());
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub domain: String,
    pub port: u16,
    pub log_level: String,
    pub base_url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Database {
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_password: Secret<String>,
    pub db_name: String,
    pub ssl_mode: SslMode,
}

impl Database {
    pub fn connection_options(&self) -> ConnectOptions {
        #[allow(unused_assignments)]
        let mut ssl = "".to_string();
        let mut url = self.connection_string_no_db().expose_secret().clone();
        if self.ssl_mode == SslMode::require {
            ssl = "?sslmode=require".to_string();
            url = format!("{}/{}{}", url, self.db_name, ssl);
        } else {
            url = format!("{}/{}", url, self.db_name);
        }
        let mut opt = ConnectOptions::new(url);
        self.set_opts(&mut opt);

        opt.to_owned()
    }

    pub fn connection_options_no_db(&self, include_options: bool) -> ConnectOptions {
        let mut ssl = "".to_string();
        if include_options && self.ssl_mode == SslMode::require {
            ssl = "?sslmode=require".to_string();
        }
        let mut opt = ConnectOptions::new(format!(
            "{}/{}",
            self.connection_string_no_db().expose_secret(),
            ssl
        ));
        self.set_opts(&mut opt);

        opt.to_owned()
    }

    fn set_opts(&self, opt: &mut ConnectOptions) {
        opt.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(8))
            .max_lifetime(Duration::from_secs(8))
            .sqlx_logging(true)
            .sqlx_logging_level(log::LevelFilter::Info);
    }

    pub fn connection_string(&self) -> Secret<String> {
        let mut ssl = "".to_string();
        if self.ssl_mode == SslMode::require {
            ssl = "?sslmode=require".to_string();
        }
        Secret::from(format!(
            "{}/{}{}",
            self.connection_string_no_db().expose_secret(),
            self.db_name,
            ssl
        ))
    }

    pub fn connection_string_no_db(&self) -> Secret<String> {
        Secret::from(format!(
            "postgres://{}:{}@{}:{}",
            self.db_user,
            self.db_password.expose_secret(),
            self.db_host,
            self.db_port
        ))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmailOutgoing {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: Secret<String>,
    pub smtp_sender: UserEmail,
    pub disable_ssl: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub server: Server,
    pub database: Database,
    pub email_outgoing: EmailOutgoing,
    pub env: Env,
}

const CONFIG_PREFIX: &str = "config";

impl Settings {
    pub fn new(base_path: Option<&str>, base_name: Option<&str>) -> Result<Self, ConfigError> {
        // Config file: defaults to ./CONFIG_PREFIX/APP_NAME
        let config_dir = match base_path {
            Some(path) => PathBuf::from(path),
            None => std::env::current_dir()
                .expect("Failed to determine current directory")
                .join(CONFIG_PREFIX),
        };
        let app_name = match base_name {
            Some(name) => name,
            None => APP_NAME,
        };
        let env: Env = std::env::var("APP_ENV")
            .unwrap_or_else(|_| "prod".into())
            .try_into()
            .expect("Failed to parse the APP_ENV environment variable");
        let config_path = config_dir.join(app_name);
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
            .set_default("database.ssl_mode", "disable")?
            .add_source(File::from(config_path))
            .add_source(File::from(env_config_path).required(false))
            .add_source(Environment::with_prefix(APP_NAME).separator("__"))
            .build()?;

        builder.try_deserialize()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum SslMode {
    #[default]
    disable,
    require,
}

impl TryFrom<String> for SslMode {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "disable" => Ok(Self::disable),
            "require" => Ok(Self::require),
            other => Err(format!("{} is not a supported ssl_mode", other)),
        }
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
            other => Err(format!("{} is not a supported environment", other)),
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
