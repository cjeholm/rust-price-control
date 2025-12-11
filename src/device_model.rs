use log::{debug, error, info, warn};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::thread;
use std::time::Duration;
use thiserror::Error;

use crate::structs;

/// Vector of devices from config file
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Devices {
    pub device: Vec<Device>,
}

/// Devices in the device vector
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Device {
    pub name: String,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub ratio: f64,
    #[serde(default)]
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

impl Device {
    fn telldus_on(&self, config: &structs::Config) -> Result<String, ActionError> {
        let command_request = format!("http://{}/api/device/turnOn", config.telldus_ip);
        self.telldus_action(command_request, config)
    }

    fn telldus_off(&self, config: &structs::Config) -> Result<String, ActionError> {
        let command_request = format!("http://{}/api/device/turnOff", config.telldus_ip);
        self.telldus_action(command_request, config)
    }

    pub fn switch_on(&self, config: &structs::Config) -> Result<State, ActionError> {
        self.change_state(config, State::On)
    }

    pub fn switch_off(&self, config: &structs::Config) -> Result<State, ActionError> {
        self.change_state(config, State::Off)
    }

    fn change_state(&self, config: &structs::Config, action: State) -> Result<State, ActionError> {
        if self.telldus {
            info!(
                "{}: Telldus switching {:?} device {}",
                self.name, action, self.telldus_id
            );

            let reply = match action {
                State::On => self.telldus_on(config),
                State::Off => self.telldus_off(config),
                _ => Err(ActionError::WrongState(action.clone())),
            };

            match reply {
                Ok(r) => debug!("Telldus reply: {r:?}"),
                Err(e) => error!("Telldus: {e:?}"),
            }
        }

        match action {
            State::On if !self.script_on.is_empty() => {
                if let Err(e) = self.action_script_on() {
                    error!("Script failed: {e}");
                }
            }
            State::Off if !self.script_off.is_empty() => {
                if let Err(e) = self.action_script_off() {
                    error!("Script failed: {e}");
                }
            }
            _ => {}
        }

        Ok(action)
    }

    fn telldus_action(
        &self,
        command_request: String,
        config: &structs::Config,
    ) -> Result<String, ActionError> {
        let client = Client::builder().timeout(Duration::from_secs(5)).build()?;
        let res = client
            .get(&command_request)
            .header("Authorization", &config.telldus_token)
            .query(&[("id", &self.telldus_id)])
            .send()?;
        let body = res.text()?;
        Ok(body)
    }

    fn action_script_on(&self) -> Result<(), ActionError> {
        self.run_script(&State::On)
    }

    fn action_script_off(&self) -> Result<(), ActionError> {
        self.run_script(&State::Off)
    }

    /// Run a custom On or Off script
    fn run_script(&self, action: &State) -> Result<(), ActionError> {
        let script: String = match action {
            State::On => {
                info!("{}: Executing On script: {}", self.name, self.script_on);
                self.script_on.clone()
            }
            State::Off => {
                info!("{}: Executing Off script: {}", self.name, self.script_off);
                self.script_off.clone()
            }
            _ => {
                warn!("Weird state switch");
                return Err(ActionError::WrongState(action.clone()));
            }
        };

        // Move the script string into the closure
        thread::spawn(move || {
            #[cfg(unix)]
            let _ = Command::new("sh").arg(script).spawn();

            #[cfg(windows)]
            let _ = Command::new("cmd").arg("/C").arg(script).spawn();
        });

        Ok(())
    }
}

/// State of devices
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub enum State {
    #[default]
    Unknown,
    On,
    Off,
}

/// Device modes
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub enum Mode {
    #[default]
    Unknown,
    Price,
    Ratio,
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
