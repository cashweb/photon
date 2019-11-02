use config::{Config, ConfigError, Environment, File};
use dirs::home_dir;
use serde::Deserialize;

use crate::CLI_ARGS;

pub const DEFAULT_BITCOIN_RPC: &str = "localhost:18332";
pub const DEFAULT_BITCOIN_USER: &str = "";
pub const DEFAULT_BITCOIN_PASSWORD: &str = "";
pub const DEFAULT_BIND: &str = "[::1]:50051";
pub const DEFAULT_BANNER: &str = "Welcome to Photon!";
pub const DEFAULT_DONATION_ADDRESS: &str = "";
pub const DEFAULT_DB_PATH: &str = ".photon/db";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bitcoin_rpc: String,
    pub bitcoin_user: String,
    pub bitcoin_password: String,
    pub db_path: String,
    pub bind: String,
    pub banner: String,
    pub donation_address: String,
}

impl Settings {
    /// Fetch settings from CLI, then ENV, then settings file, then defaults.
    pub fn fetch() -> Result<Self, ConfigError> {
        let mut settings = Config::new();

        // Get home directory
        let home_dir = match dirs::home_dir() {
            Some(some) => some,
            None => return Err(ConfigError::Message("no home directory".to_string())),
        };

        // Default settings
        settings.set_default("bitcoin_rpc", DEFAULT_BITCOIN_RPC)?;
        settings.set_default("bitcoin_user", DEFAULT_BITCOIN_USER)?;
        settings.set_default("bitcoin_password", DEFAULT_BITCOIN_PASSWORD)?;
        settings.set_default("bind", DEFAULT_BIND)?;
        settings.set_default("banner", DEFAULT_BANNER)?;
        let mut default_db = home_dir.clone();
        default_db.push(DEFAULT_DB_PATH);
        settings.set_default("db_path", default_db.to_str())?;
        settings.set_default("donation_address", DEFAULT_DONATION_ADDRESS)?;

        // Merge with settings file
        settings.merge(File::with_name("config").required(false))?;

        // Merge with enviromental variables
        settings.merge(Environment::with_prefix("photon"))?;

        // Merge with command line arguments
        if let Some(bind) = CLI_ARGS.value_of("bind") {
            settings.set("bind", bind)?;
        }
        if let Some(banner) = CLI_ARGS.value_of("banner") {
            settings.set("banner", banner)?;
        }
        if let Some(bitcoin_rpc) = CLI_ARGS.value_of("bitcoin-rpc") {
            settings.set("bitcoin_rpc", bitcoin_rpc)?;
        }
        if let Some(bitcoin_user) = CLI_ARGS.value_of("bitcoin-user") {
            settings.set("bitcoin_user", bitcoin_user)?;
        }
        if let Some(bitcoin_password) = CLI_ARGS.value_of("bitcoin-password") {
            settings.set("bitcoin_password", bitcoin_password)?;
        }
        if let Some(db_path) = CLI_ARGS.value_of("db-path") {
            settings.set("db_path", db_path)?;
        }
        if let Some(address) = CLI_ARGS.value_of("donation-address") {
            settings.set("donation_address", address)?;
        }
        settings.try_into()
    }
}
