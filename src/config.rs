use crate::structs;
// use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use log::error;

/// Loads the general settings
pub fn read_config_from_file(path: &PathBuf) -> structs::Config {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|e| {
            error!("Could not read config: {path:?}");
            panic!("{e}");
        });

    toml::from_str(&contents)
        .unwrap_or_else(|e| {
            error!("Could not parse config: {path:?}");
            panic!("{e}");
        })
}
