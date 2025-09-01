use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;

/// Configuration values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Configuration settings.
    #[serde(rename = "config")]
    pub settings: Settings,
    /// Encryption configuration
    pub encryption: EncryptionConfig,
}

/// General settings for configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    /// Refresh rate of the configuration file.
    #[serde(with = "humantime_serde")]
    pub refresh_rate: Duration,
    pub base_dir: String,
}

/// Encryption configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EncryptionConfig {
    /// Public keys listing
    pub public_keys: Vec<String>,
}

impl Config {
    /// Parses the config file and returns the values.
    pub fn parse(path: &Path) -> Result<Config, Box<dyn Error>> {
        let config_str = read_to_string(path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn get_base_dir(&self) -> String {
        self.settings.base_dir.clone()
    }
}
