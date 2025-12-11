use reqwest::blocking::Client;
use std::error::Error;
use std::time::Duration;

use crate::structs::{self};

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
