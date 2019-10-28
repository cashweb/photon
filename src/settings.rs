use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;

use crate::CLI_ARGS;

pub const DEFAULT_BIND: &str = "[::1]:50051";
pub const DEFAULT_BANNER: &str = "Welcome to Photon!";
pub const DEFAULT_DONATION_ADDRESS: &str = "";

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub bind: String,
    pub banner: String,
    pub donation_address: String,
}

impl Settings {
    /// Fetch settings from CLI, then ENV, then settings file, then defaults.
    pub fn fetch() -> Result<Self, ConfigError> {
        let mut settings = Config::new();

        // Default settings
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

        settings.try_into()
    }
}
