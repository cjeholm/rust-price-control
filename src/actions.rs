// use anyhow::Result;
use log::{debug, error, info, warn};
use std::process::Command;
use std::thread;

use crate::structs;
use crate::telldus;

/// Change the state of a device and run its actions
pub fn change_state(
    config: &structs::Config,
    device: &structs::Device,
    action: structs::State,
) -> Result<structs::State, structs::ActionError> {
    if device.telldus {
        info!(
            "{}: Telldus switching {:?} device {}",
            device.name, action, device.telldus_id
        );
        let telldus_reply = match telldus::telldus_switch(config, device, &action) {
            Ok(reply) => reply,
            Err(e) => {
                error!("Telldus: {:?}", e);
                "Telldus error".to_string()
            }
        };
        debug!("Telldus reply: {telldus_reply:?}");
    };
    if (action == structs::State::On && !device.script_on.is_empty())
        || (action == structs::State::Off && !device.script_off.is_empty())
    {
        let _ = action_run_script(device, &action);
    };
    Ok(action)
}

/// Run a custom On or Off script
fn action_run_script(device: &structs::Device, action: &structs::State) -> Result<(), structs::ActionError> {
    let script: String = match action {
        structs::State::On => {
            info!("{}: Executing On script: {}", device.name, device.script_on);
            device.script_on.clone()
        }
        structs::State::Off => {
            info!(
                "{}: Executing Off script: {}",
                device.name, device.script_off
            );
            device.script_off.clone()
        }
        _ => {
            warn!("Weird state switch");
            return Err(structs::ActionError::WrongState(action.clone()));
        }
    };

    // Move the script string into the closure
    thread::spawn(move || {
        let _ = Command::new("bash").arg(script).spawn();
    });

    Ok(())
}
