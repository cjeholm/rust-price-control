use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use thiserror::Error;

#[derive(Debug)]
pub struct Day {
    pub date: NaiveDate,
    pub url: String,
    pub file: String,
}

#[derive(Debug, Error)]
pub enum ActionError {
    #[error("Invalid state for telldus_switch")]
    WrongState(State),

    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("I/O error while reading config: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

// #[derive(Debug, Error)]
// pub enum DeviceError {
//     #[error("I/O error while reading devices: {0}")]
//     Io(#[from] io::Error),
//
//     #[error("Failed to parse devices: {0}")]
//     Parse(#[from] toml::de::Error),
// }

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

/// Devices in the device vector
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Device {
    pub name: String,
    #[serde(default)]
    pub mode: Mode,
    pub ratio: f64,
    pub price: f64,
    #[serde(default)]
    pub today_trigger_price: f64,
    #[serde(default)]
    pub tomorrow_trigger_price: f64,
    #[serde(default)]
    pub state: State,
    #[serde(default)]
    pub force_update: bool,
    #[serde(default)]
    pub telldus: bool,
    #[serde(default)]
    pub telldus_id: String,
    #[serde(default)]
    pub script_on: String,
    #[serde(default)]
    pub script_off: String,
}

/// Vector of devices from config file
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Devices {
    pub device: Vec<Device>,
}

/// State of devices
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub enum State {
    #[default]
    Unknown,
    On,
    Off,
    Testing,
}

/// Device modes
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub enum Mode {
    #[default]
    Unknown,
    Price,
    Ratio,
}

/// Shared state for the webui
#[derive(Clone, Serialize)]
pub struct AppState {
    pub config: Config,
    pub devices: Devices,
    pub todays_spot_prices: Value,    // store the JSON array directly
    pub tomorrows_spot_prices: Value, // store the JSON array directly
}
