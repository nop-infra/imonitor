#![warn(clippy::unwrap_used, clippy::expect_used)]

/// Configuration file parser.
pub mod config;

/// Device struct
pub mod device;

/// Get idevice provider from Device
pub mod provider;

/// Use idevice services
pub mod services;

///// File encryption from memory buffer to disk.
//pub mod encrypt;

/// Environment variable for setting the configuration file path.
pub const CONFIG_ENV: &str = "CONFIG";
