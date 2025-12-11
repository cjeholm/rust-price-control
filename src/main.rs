use serde_json::Value;
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as TimeDuration;

use anyhow::Result;
use env_logger::Env;
use log::{error, info, warn};

mod config;
mod device_model;
mod functions;
mod price;
mod structs;
mod telldus;
mod webui;

/// MAIN
fn main() -> Result<()> {
    // env_logger::init();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let config_path = config::config_path();
    let config_result = config::read_config_from_file(&config_path);

    let args: Vec<String> = env::args().skip(1).collect();
    if !args.is_empty() {
        functions::check_args(&args, &config_result);
    }

    // Here we check for a bad config, not sooner; we want the cli args to always work.
    let config = match config_result {
        Ok(config) => config,
        Err(structs::ConfigError::Io(e)) => {
            error!("Could not find the config file {:?}: {}", config_path, e);
            panic!("{}", e);
        }
        Err(structs::ConfigError::Parse(e)) => {
            error!("Errors in the config file. {}", e);
            panic!("{}", e);
        }
    };

    let mut devices = match config::read_devices_from_file(&config_path) {
        Ok(devices) => devices,
        Err(e) => {
            error!("Could not read devices from config file. {}", e);
            panic!("{}", e);
        }
    };

    info!("Config file: {}", config_path.display());

    let tmp = env::temp_dir();
    info!("Temp dir: {}", tmp.display());

    // Async variables for the web ui
    let asyncdata = Arc::new(Mutex::new(structs::AppState {
        config: (config.clone()),
        devices: (devices.clone()),
        todays_spot_prices: Value::Array(vec![]), // initially empty
        tomorrows_spot_prices: Value::Array(vec![]), // initially empty
    }));
    let server_data = asyncdata.clone();
    let server_config = config.clone();
    let server_devices = devices.clone();

    functions::get_tomorrow_thread(config.clone());

    // Start webserver in a background thread
    info!(
        "Starting the web UI on http://127.0.0.1:{}",
        config.webui_port
    );
    thread::spawn(move || {
        webui::run_server(server_data, &server_config, server_devices);
    });

    // LOOP
    loop {
        // Today
        let today = functions::make_today(&config);
        let todays_spot_prices = match price::read_price_data(today) {
            Ok(data) => data,
            Err(err) => {
                warn!("Failed to read today’s data: {}", err);
                thread::sleep(TimeDuration::from_secs(config.interval));
                continue;
            }
        };

        // Tomorrows prices for the webui async
        let tomorrow = functions::make_tomorrow(&config);
        let tomorrows_spot_prices = match price::try_load_local(&tomorrow) {
            Ok(data) => data,
            Err(_) => serde_json::json!({}),
        };

        // let updated_devices = functions::logic_loop(&todays_spot_prices, devices, &config)?;
        match functions::logic_loop(
            &todays_spot_prices,
            &tomorrows_spot_prices,
            devices.clone(),
            &config,
        ) {
            Ok(updated_devices) => devices = updated_devices,
            Err(e) => warn!("{e}"),
        }

        // The async var for the webui
        {
            let mut state = asyncdata.lock().unwrap();
            state.config = config.clone();
            state.devices = devices.clone();
            state.todays_spot_prices = todays_spot_prices.clone(); // JSON Value
            state.tomorrows_spot_prices = tomorrows_spot_prices.clone(); // JSON Value
        }

        thread::sleep(TimeDuration::from_secs(config.interval));
    }
}
