use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

use crate::CLI_ARGS;

pub const DEFAULT_BITCOIN_RPC: &str = "localhost:18332";
pub const DEFAULT_BITCOIN_USER: &str = "";
pub const DEFAULT_BITCOIN_PASSWORD: &str = "";
pub const DEFAULT_BIND: &str = "[::1]:50051";
pub const DEFAULT_BANNER: &str = "Welcome to Photon!";
pub const DEFAULT_DONATION_ADDRESS: &str = "";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bitcoin_rpc: String,
    pub bitcoin_user: String,
    pub bitcoin_password: String,
    pub bind: String,
    pub banner: String,
    pub donation_address: String,
}

impl Settings {
    /// Fetch settings from CLI, then ENV, then settings file, then defaults.
    pub fn fetch() -> Result<Self, ConfigError> {
        let mut settings = Config::new();

        // Default settings
        settings.set("bitcoin_rpc", DEFAULT_BITCOIN_RPC)?;
        settings.set("bitcoin_user", DEFAULT_BITCOIN_USER)?;
        settings.set("bitcoin_password", DEFAULT_BITCOIN_PASSWORD)?;
        settings.set("bind", DEFAULT_BIND)?;
        settings.set("banner", DEFAULT_BANNER)?;
        settings.set("donation_address", DEFAULT_DONATION_ADDRESS)?;

        // Merge with settings file
        settings.merge(File::with_name("config").required(false))?;

        // Merge with enviromental variables
        settings.merge(Environment::with_prefix("photon"))?;

        // Merge with command line arguments
        if let Some(bind) = CLI_ARGS.value_of("bind") {
            settings.set("bind", bind).unwrap();
        }
        if let Some(banner) = CLI_ARGS.value_of("banner") {
            settings.set("banner", banner).unwrap();
        }
        if let Some(address) = CLI_ARGS.value_of("donation-address") {
            settings.set("donation_address", address).unwrap();
        }
        if let Some(bitcoin_rpc) = CLI_ARGS.value_of("bitcoin-rpc") {
            settings.set("bitcoin_rpc", bitcoin_rpc).unwrap();
        }
        if let Some(bitcoin_user) = CLI_ARGS.value_of("bitcoin-user") {
            settings.set("bitcoin_user", bitcoin_user).unwrap();
        }
        if let Some(bitcoin_password) = CLI_ARGS.value_of("bitcoin-password") {
            settings.set("bitcoin_password", bitcoin_password).unwrap();
        }
        settings.try_into()
    }
}
