use crate::structs;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

/// Loads the devices
pub fn read_devices_from_file(path: &PathBuf) -> Result<structs::Devices> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}
