use anyhow::Result;
use log::{debug, info, warn};
use std::thread;
use std::time::Duration as TimeDuration;
use time::{Date, Duration, OffsetDateTime};

use crate::{config, device_model, price, structs, telldus};

/// Spawn a thread that loops just to get tomorrow's data at a lower tick rate.
pub fn get_tomorrow_thread(config: structs::Config) {
    thread::spawn(move || loop {
        let tomorrow = make_tomorrow(&config);
        if let Err(err) = price::read_price_data(tomorrow) {
            debug!("Failed to download tomorrow’s data: {}", err);
        }
        thread::sleep(TimeDuration::from_secs(3600));
    });
}

/// Check args for cli
pub fn check_args(args: &[String], config_result: &Result<structs::Config, structs::ConfigError>) {
    if args.contains(&"-h".into()) || args.contains(&"--help".into()) {
        println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        println!("Usage: {} [OPTION]\n", env!("CARGO_PKG_NAME"));
        println!("    --telldus-list        List Telldus devices (requires config file)");
        println!("    --generate-config     Create a default config file");
        println!("-h  --help                This help");
        println!("-v  --version             Version information");
        std::process::exit(0);
    }

    if args.contains(&"-v".into()) || args.contains(&"--version".into()) {
        println!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!("Written by {} 2025", env!("CARGO_PKG_AUTHORS"));
        println!("Licenced under GPL-3.0");
        std::process::exit(0);
    }

    if args.contains(&"--generate-config".into()) {
        match config::generate_config() {
            Ok(_) => {
                println!("Creating a default config file (pricecontrol.toml) in the current directory. You can put it in {:?}", dirs::config_dir().unwrap());
            }
            Err(e) => {
                println!("Creating config file failed: {}", e);
            }
        }
        std::process::exit(0);
    }

    if args.contains(&"--telldus-list".into()) {
        let config = match config_result {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("The argument --telldus-list needs a working config file. Error loading config: {e}");
                std::process::exit(2);
            }
        };

        match telldus::telldus_list(config) {
            Ok(output) => println!("{}", output),
            Err(e) => {
                eprintln!("Error listing Telldus devices: {e}");
                std::process::exit(3);
            }
        }
        std::process::exit(0);
    }

    println!("Unknown argument(s): {:?}", args);
    std::process::exit(1);
}

/// Make a today-instance
pub fn make_today(config: &structs::Config) -> structs::Day {
    let today: Date = OffsetDateTime::now_local().unwrap().date();
    let today_str = format!(
        "{}/{:02}-{:02}",
        today.year(),
        today.month() as u8,
        today.day()
    );
    structs::Day {
        date: today,
        url: format!("{}{}_{}.json", config.api, today_str, config.area),
        file: format!(
            "{}-{:02}-{:02}_{}.json",
            today.year(),
            today.month() as u8,
            today.day(),
            config.area
        ),
    }
}

/// Make a tomorrow-instance
pub fn make_tomorrow(config: &structs::Config) -> structs::Day {
    let tomorrow: Date = OffsetDateTime::now_local().unwrap().date() + Duration::days(1);
    let tomorrow_str = format!(
        "{}/{:02}-{:02}",
        tomorrow.year(),
        tomorrow.month() as u8,
        tomorrow.day()
    );
    structs::Day {
        date: tomorrow,
        url: format!("{}{}_{}.json", config.api, tomorrow_str, config.area),
        file: format!(
            "{}-{:02}-{:02}_{}.json",
            tomorrow.year(),
            tomorrow.month() as u8,
            tomorrow.day(),
            config.area
        ),
    }
}

/// The main loop
pub fn logic_loop(
    today_spot_prices: &serde_json::Value,
    tomorrow_spot_prices: &serde_json::Value,
    mut devices: device_model::Devices,
    config: &structs::Config,
) -> Result<device_model::Devices, device_model::ActionError> {
    let price = price::current_price(today_spot_prices, &config.currency);

    let avg_price = price::average_price(today_spot_prices, &config.currency).unwrap();

    if let Some(p) = price {
        debug!("Current spot price: {:.2} {}", p, &config.currency);
        debug!(
            "With fees and VAT:  {:.2} {}",
            price::total_price(p, config),
            &config.currency
        );
    } else {
        warn!("No current price found.");
    }

    debug!("Average spot price: {:.2} {}", avg_price, &config.currency);

    for device in devices.device.iter_mut() {
        if device.mode == device_model::Mode::Price {
            device.today_trigger_price = device.price;
            device.tomorrow_trigger_price = device.price;
            if (device.state != device_model::State::On
                && device.today_trigger_price > price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price > price.unwrap_or_default())
            {
                info!(
                    "{}: {:?} mode - Changing state to On",
                    device.name, device.mode
                );
                device.state = device.switch_on(config)?;
            } else if (device.state != device_model::State::Off
                && device.today_trigger_price < price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price < price.unwrap_or_default())
            {
                info!(
                    "{}: {:?} mode - Changing state to Off",
                    device.name, device.mode,
                );
                device.state = device.switch_on(config)?;
            }
        }

        if device.mode == device_model::Mode::Ratio {
            device.today_trigger_price =
                price::ratio_price(today_spot_prices, &config.currency, device.ratio).unwrap();
            device.tomorrow_trigger_price =
                price::ratio_price(tomorrow_spot_prices, &config.currency, device.ratio)
                    .unwrap_or(0.0);
            if (device.state != device_model::State::On
                && device.today_trigger_price > price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price > price.unwrap_or_default())
            {
                info!(
                    "{}: {:?} mode - Changing state to On",
                    device.name, device.mode
                );
                device.state = device.switch_on(config)?;
            } else if (device.state != device_model::State::Off
                && device.today_trigger_price < price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price < price.unwrap_or_default())
            {
                info!(
                    "{}: {:?} mode - Changing state to Off",
                    device.name, device.mode
                );
                device.state = device.switch_off(config)?;
            }
        }

        debug!(
            "Device: {},\tMode: {:?},\tRatio: {},\tPrice: {:.2} - {:?} - Ratio price: {:.2}",
            device.name,
            device.mode,
            device.ratio,
            device.price,
            device.state,
            device.today_trigger_price
        );
    }

    Ok(devices)
}
