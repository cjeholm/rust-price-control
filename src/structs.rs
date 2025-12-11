use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use thiserror::Error;
use time::Date;

use crate::device_model;

#[derive(Debug)]
pub struct Day {
    pub date: Date,
    pub url: String,
    pub file: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error while reading config: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("I/O error while reading devices: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse devices: {0}")]
    Parse(#[from] toml::de::Error),
}

/// The program config from config file
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub api: String,
    pub area: String,
    pub currency: String,
    pub interval: u64,
    pub webui_port: u64,

    #[serde(default)]
    pub webui_toggle: bool,

    pub grid_fee: f64,
    pub energy_tax: f64,
    pub variable_costs: f64,
    pub spot_fee: f64,
    pub cert_fee: f64,
    pub vat: f64,

    #[serde(default)]
    pub telldus_ip: String,
    #[serde(default)]
    pub telldus_token: String,
}

/// Shared state for the webui
#[derive(Clone, Serialize)]
pub struct AppState {
    pub config: Config,
    pub devices: device_model::Devices,
    pub todays_spot_prices: Value,    // store the JSON array directly
    pub tomorrows_spot_prices: Value, // store the JSON array directly
}
