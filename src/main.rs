use dirs;
use price::try_load_local;
use serde_json::Value;
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration as TimeDuration;

use anyhow::Result;
use env_logger::Env;
use log::{debug, info, warn};
use std::path::PathBuf;

mod actions;
mod config;
mod devices;
mod functions;
mod price;
mod structs;
mod telldus;
mod webserver;

/// MAIN
fn main() -> Result<()> {
    // env_logger::init();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    // Check args
    let args: Vec<String> = env::args().skip(1).collect();
    if !args.is_empty() {
        let _ = functions::check_args(&args);
    }

    // Set and read config file
    let mut config_path = dirs::config_dir().unwrap();
    config_path.push("pricecontrol.toml");

    let final_path = if config_path.exists() {
        config_path
    } else {
        PathBuf::from("pricecontrol.toml")
    };

    let config = config::read_config_from_file(&final_path);
    let mut devices = devices::read_devices_from_file(&final_path)?;

    // Check for --telldus-list here when config exists
    if args.contains(&"--telldus-list".to_string()) {
        println!("{}", telldus::telldus_list(&config).unwrap());
        std::process::exit(0);
    }

    info!("Config file: {}", final_path.display());

    // Check tmp dir
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

    // Spawn a thread that loops just to get tomorrow's data
    // at a lower tick rate.
    let config_thread = config.clone();
    thread::spawn(move || loop {
        let tomorrow = functions::make_tomorrow(&config_thread);
        if let Err(err) = price::read_price_data(tomorrow) {
            debug!("Failed to download tomorrow’s data: {}", err);
        }
        thread::sleep(TimeDuration::from_secs(3600));
    });

    // Start your webserver in a background thread
    info!(
        "Starting the web UI on http://127.0.0.1:{}",
        config.webui_port
    );
    thread::spawn(move || {
        webserver::run_server(server_data, &server_config);
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
        let tomorrows_spot_prices = match try_load_local(&tomorrow) {
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
