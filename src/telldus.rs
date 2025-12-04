use reqwest::blocking::Client;
use std::error::Error;
use std::time::Duration;

use crate::structs::{self, ActionError};

/// List the Telldus devices
pub fn telldus_list(config: &structs::Config) -> Result<String, Box<dyn Error>> {
    let command_request = format!("http://{}/api/devices/list", config.telldus_ip);

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    let res = client
        .get(&command_request)
        .header("Authorization", config.telldus_token.clone())
        .send()?;

    let body = res.text()?;
    Ok(body)
}

/// Switch a Telldus device on or off
pub fn telldus_switch(
    config: &structs::Config,
    device: &structs::Device,
    action: &structs::State,
) -> Result<String, ActionError> {
    let command_request = match action {
        structs::State::On => {
            format!("http://{}/api/device/turnOn", config.telldus_ip)
        }
        structs::State::Off => {
            format!("http://{}/api/device/turnOff", config.telldus_ip)
        }
        _ => {
            return Err(ActionError::WrongState(action.clone()));
        }
    };

    let client = Client::builder().timeout(Duration::from_secs(5)).build()?;

    let res = client
        .get(&command_request)
        .header("Authorization", config.telldus_token.clone())
        .query(&[("id", device.telldus_id.clone())])
        .send()?;

    let body = res.text()?;
    Ok(body)
}
