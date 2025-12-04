use crate::structs;
// use anyhow::Result;
// use log::error;
use std::fs;
use std::path::PathBuf;

/// Loads the general settings
pub fn read_config_from_file(path: &PathBuf) -> Result<structs::Config, structs::ConfigError> {
    let contents = fs::read_to_string(path)?;
    let config = toml::from_str::<structs::Config>(&contents)?;
    Ok(config)
}

/// Set the path for config
pub fn config_path() -> PathBuf {
    if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push("pricecontrol.toml");
        if config_dir.exists() {
            return config_dir;
        }
    }
    PathBuf::from("pricecontrol.toml")
}
