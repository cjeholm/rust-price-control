use crate::device_model;
use crate::structs;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
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

/// Loads the devices
pub fn read_devices_from_file(
    path: &PathBuf,
) -> Result<device_model::Devices, structs::DeviceError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

/// Return a static embedded file for release builds
fn read_static(path: &str, embedded: &str, debug: bool) -> Result<String, std::io::Error> {
    if debug {
        fs::read_to_string(path)
    } else {
        Ok(embedded.to_string())
    }
}

/// Generate a new default config
pub fn generate_config() -> std::io::Result<()> {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

    let new_config = read_static(
        "static/config.example",
        include_str!("../static/config.example"),
        DEBUG,
    )?;
    write_new_file_from_string("pricecontrol.toml", &new_config)
}

// Write the new config file, dont overwrite
fn write_new_file_from_string(filename: &str, contents: &str) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(filename)?;

    file.write_all(contents.as_bytes())
}
