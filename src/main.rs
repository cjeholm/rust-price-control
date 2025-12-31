use std::env;
use std::thread;
use std::time::Duration as TimeDuration;
use time::{Date, Duration, OffsetDateTime};

use anyhow::Result;
use env_logger::Env;
use log::{error, info, warn};

mod config;
mod db;
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

    // Connect to the database
    let mut db_conn = db::open_conn()?;
    db::init_db(&db_conn)?;
    db::save_config(&db_conn, &config)?;
    db::save_devices(&mut db_conn, &devices)?;
    db::save_todays_prices(&db_conn, "".to_string())?;
    db::save_tomorrows_prices(&db_conn, "".to_string())?;

    functions::get_tomorrow_thread(config.clone());

    // Start webserver in a background thread
    info!(
        "Starting the web UI on http://127.0.0.1:{}",
        config.webui_port
    );
    thread::spawn(move || {
        webui::run_server();
    });

    // LOOP
    loop {
        // Today
        let date: Date = OffsetDateTime::now_local().unwrap().date();
        let today = functions::make_day(&config, date);
        let todays_spot_prices = match price::read_price_data(today) {
            Ok(data) => data,
            Err(err) => {
                warn!("Failed to read today’s data: {}", err);
                thread::sleep(TimeDuration::from_secs(config.interval));
                continue;
            }
        };

        // Tomorrows prices for the webui async
        let date: Date = OffsetDateTime::now_local().unwrap().date() + Duration::days(1);
        let tomorrow = functions::make_day(&config, date);
        let tomorrows_spot_prices = match price::try_load_local(&tomorrow) {
            Ok(data) => data,
            Err(_) => serde_json::json!({}),
        };

        // Run the logic that iterates over devices
        match functions::logic_loop(
            &todays_spot_prices,
            &tomorrows_spot_prices,
            db::load_devices(&db_conn)?,
            &config,
        ) {
            Ok(updated_devices) => devices = updated_devices,
            Err(e) => warn!("{e}"),
        }

        // Update the database
        db::update_devices(&mut db_conn, &devices)?;
        db::save_todays_prices(&db_conn, todays_spot_prices.to_string())?;
        db::save_tomorrows_prices(&db_conn, tomorrows_spot_prices.to_string())?;

        thread::sleep(TimeDuration::from_secs(config.interval));
    }
}
