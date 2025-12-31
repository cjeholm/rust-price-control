use anyhow::Result;
use log::{debug, info, warn};
use std::thread;
use std::time::Duration as TimeDuration;
use time::{Date, Duration, OffsetDateTime};

use crate::{config, device_model, price, structs, telldus};

/// Spawn a thread that loops just to get tomorrow's data at a lower tick rate.
pub fn get_tomorrow_thread(config: structs::Config) {
    thread::spawn(move || loop {
        let date: Date = OffsetDateTime::now_local().unwrap().date() + Duration::days(1);
        let tomorrow = make_day(&config, date);
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
pub fn make_day(config: &structs::Config, date: Date) -> structs::Day {
    // let date: Date = OffsetDateTime::now_local().unwrap().date();
    let today_str = format!(
        "{}/{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    );
    structs::Day {
        date,
        url: format!("{}{}_{}.json", config.api, today_str, config.area),
        file: format!(
            "{}-{:02}-{:02}_{}.json",
            date.year(),
            date.month() as u8,
            date.day(),
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
        // Mode Price
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
                device.state = device.switch_off(config)?;
            }
        }

        // Mode Ratio
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

        // Mode ON
        if device.mode == device_model::Mode::On {
            device.today_trigger_price = 9999.9;
            device.tomorrow_trigger_price = 9999.9;
            if device.state != device_model::State::On || device.force_update {
                info!(
                    "{}: {:?} mode - Changing state to On",
                    device.name, device.mode
                );
                device.state = device.switch_on(config)?;
            };
        }

        // Mode OFF
        if device.mode == device_model::Mode::Off {
            device.today_trigger_price = -9999.9;
            device.tomorrow_trigger_price = -9999.9;
            if device.state != device_model::State::Off || device.force_update  {
                info!(
                    "{}: {:?} mode - Changing state to Off",
                    device.name, device.mode
                );
                device.state = device.switch_off(config)?;
            };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_day_formats_url_and_file_correctly() {
        let config = structs::Config {
            api: "https://example.com/".to_string(),
            area: "SE8".to_string(),
            currency: "SEK".to_string(),
            interval: 10,
            webui_port: 8088,
            webui_toggle: false,
            grid_fee: 0.3,
            energy_tax: 0.4,
            variable_costs: 0.04,
            spot_fee: 0.1,
            cert_fee: 0.01,
            vat: 0.25,
            telldus_ip: "127.0.0.1".to_string(),
            telldus_token: "foobar".to_string(),
        };
        let date = Date::from_calendar_date(2025, time::Month::March, 7).unwrap();
        let day = make_day(&config, date);
        assert_eq!(day.date, date);
        assert_eq!(day.url, "https://example.com/2025/03-07_SE8.json");
        assert_eq!(day.file, "2025-03-07_SE8.json");
    }
}
