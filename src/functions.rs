use anyhow::Result;
use chrono::{Datelike, Duration, Local};
use log::{debug, warn};

use crate::{
    actions, price,
    structs::{self, ActionError},
};

/// Check cli args
pub fn check_args(args: &Vec<String>) {
    if args.contains(&"-h".to_string()) || args.contains(&"--help".to_string()) {
        println!("{}", env!("CARGO_PKG_DESCRIPTION"));
        println!("Usage: {} [OPTION]\n", env!("CARGO_PKG_NAME"));
        println!("    --telldus-list    List Telldus devices (requires config file)");
        println!("-h  --help            This help");
        println!("-v  --version         Version information");
        std::process::exit(0);
    } else if args.contains(&"-v".to_string()) || args.contains(&"--version".to_string()) {
        println!("{} {}\n", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        println!("Copyright (C) {}", env!("CARGO_PKG_AUTHORS"));
        std::process::exit(0);
    } else if args.contains(&"--telldus-list".to_string()) {
        // do the listing in main function where config exists.
        // we may be able to list help and stuff wothout it.
        return;
    } else {
        println!("Unknown argument: {:?}", args);
        std::process::exit(0);
    }
}

/// Make a today-instance
pub fn make_today(config: &structs::Config) -> structs::Day {
    let today = Local::now().date_naive();
    let today_str = format!("{}/{:02}-{:02}", today.year(), today.month(), today.day());
    structs::Day {
        date: today,
        url: format!("{}{}_{}.json", config.api, today_str, config.area),
        file: format!(
            "{}-{:02}-{:02}_{}.json",
            today.year(),
            today.month(),
            today.day(),
            config.area
        ),
    }
}

/// Make a tomorrow-instance
pub fn make_tomorrow(config: &structs::Config) -> structs::Day {
    let tomorrow = Local::now().date_naive() + Duration::days(1);
    let tomorrow_str = format!(
        "{}/{:02}-{:02}",
        tomorrow.year(),
        tomorrow.month(),
        tomorrow.day()
    );
    structs::Day {
        date: tomorrow,
        url: format!("{}{}_{}.json", config.api, tomorrow_str, config.area),
        file: format!(
            "{}-{:02}-{:02}_{}.json",
            tomorrow.year(),
            tomorrow.month(),
            tomorrow.day(),
            config.area
        ),
    }
}

// TODO! fix returns and error handling
/// The main loop
pub fn logic_loop(
    today_spot_prices: &serde_json::Value,
    tomorrow_spot_prices: &serde_json::Value,
    mut devices: structs::Devices,
    config: &structs::Config,
) -> Result<structs::Devices, ActionError> {
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
        if device.mode == structs::Mode::Price {
            device.today_trigger_price = device.price;
            device.tomorrow_trigger_price = device.price;
            if (device.state != structs::State::On
                && device.today_trigger_price > price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price > price.unwrap_or_default())
            {
                debug!("{} (Price mode) switching ON", device.name);
                device.state = actions::change_state(&config, &device, structs::State::On)?;
            } else if (device.state != structs::State::Off
                && device.today_trigger_price < price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price < price.unwrap_or_default())
            {
                debug!("{} (Price mode) switching OFF", device.name);
                device.state = actions::change_state(&config, &device, structs::State::Off)?;
            }
        }

        if device.mode == structs::Mode::Ratio {
            device.today_trigger_price =
                price::ratio_price(today_spot_prices, &config.currency, device.ratio).unwrap();
            device.tomorrow_trigger_price =
                price::ratio_price(tomorrow_spot_prices, &config.currency, device.ratio)
                    .unwrap_or(0.0);
            if (device.state != structs::State::On
                && device.today_trigger_price > price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price > price.unwrap_or_default())
            {
                debug!("{} (Ratio mode) switching ON", device.name);
                device.state = actions::change_state(&config, &device, structs::State::On)?;
            } else if (device.state != structs::State::Off
                && device.today_trigger_price < price.unwrap_or_default())
                || (device.force_update && device.today_trigger_price < price.unwrap_or_default())
            {
                debug!("{} (Ratio mode) switching OFF", device.name);
                device.state = actions::change_state(&config, &device, structs::State::Off)?;
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
