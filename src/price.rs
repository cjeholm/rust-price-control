use anyhow::{anyhow, Result};
use log::{debug, info};
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::Write;
use std::time::Duration as TimeDuration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::structs;

/// Download json from url
fn download_prices(url: &str) -> Result<Value> {
    let client = Client::builder()
        .timeout(TimeDuration::from_secs(10))
        .build()?;
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }

    let json = response.json::<Value>()?;

    if !json.is_array() {
        return Err(anyhow!("Expected array at root of JSON, got: {}", json));
    }

    Ok(json)
}

/// Get local json data
fn load_prices_from_file(file: String) -> Result<Value> {
    let mut path = env::temp_dir();
    path.push(file);
    let json = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&json)?;

    if !value.is_array() {
        return Err(anyhow!("Expected array at root of JSON, got: {}", value));
    }

    Ok(value)
}

/// Save json data to local file
fn save_prices_to_file(value: &Value, file: &str) -> Result<()> {
    let mut path = env::temp_dir();
    path.push(file);

    let mut tmp_path = env::temp_dir();
    tmp_path.push(file);

    let mut file = File::create(&tmp_path)?;
    let pretty = serde_json::to_string_pretty(value)?;
    file.write_all(pretty.as_bytes())?;

    std::fs::rename(tmp_path, path)?;
    Ok(())
}

pub fn read_price_data(day: structs::Day) -> Result<Value> {
    match try_load_local(&day) {
        Ok(data) => Ok(data),
        Err(_) => try_download_and_save(&day),
    }
}

pub fn try_load_local(day: &structs::Day) -> Result<Value> {
    match load_prices_from_file(day.file.clone()) {
        Ok(data) => {
            debug!("Reading local file: {}", day.file);
            Ok(data)
        }
        Err(err) => Err(err),
    }
}

fn try_download_and_save(day: &structs::Day) -> Result<Value> {
    debug!("Attempting download {}", day.url);
    let data = download_prices(&day.url)?;
    let _ = save_prices_to_file(&data, &day.file);
    info!("Prices for {} downloaded", day.date);
    Ok(data)
}

/// Return the current price
pub fn current_price(json: &Value, currency: &str) -> Option<f64> {
    json.as_array()?
        .iter()
        .find_map(|obj| extract_valid_price(obj, currency))
}

/// Return the average price
pub fn average_price(json: &Value, currency: &str) -> Option<f64> {
    let array = json.as_array()?;

    let prices: Vec<f64> = array
        .iter()
        .filter_map(|obj| obj.get(currency)?.as_f64())
        .collect();

    if prices.is_empty() {
        None
    } else {
        Some(prices.iter().sum::<f64>() / prices.len() as f64)
    }
}

/// Return the n'th sorted price for Ratio mode
pub fn ratio_price(json: &Value, currency: &str, ratio: f64) -> Option<f64> {
    let array = json.as_array()?;
    let safe_ratio = ratio.clamp(0.0, 1.0);
    let index = ((array.len() - 1) as f64 * safe_ratio).floor() as usize;

    let mut prices: Vec<f64> = array
        .iter()
        .filter_map(|obj| obj.as_object()?.get(currency)?.as_f64())
        .collect();

    prices.sort_by(|a, b| a.partial_cmp(b).unwrap());
    prices.get(index).copied()
}

fn extract_valid_price(obj: &Value, currency: &str) -> Option<f64> {
    let price = obj.get(currency)?.as_f64()?;
    let start_str = obj.get("time_start")?.as_str()?;
    let end_str = obj.get("time_end")?.as_str()?;

    let start = parse_local_datetime(start_str)?;
    let end = parse_local_datetime(end_str)?;

    let now = OffsetDateTime::now_local().ok()?;

    if now >= start && now < end {
        Some(price)
    } else {
        None
    }
}

/// Total price incl fees and vat
pub fn total_price(spot: f64, config: &structs::Config) -> f64 {
    let total = spot
        + config.grid_fee
        + config.energy_tax
        + config.variable_costs
        + config.spot_fee
        + config.cert_fee;

    total * (1.0 + config.vat)
}

/// Parse RFC3339 timestamp into local OffsetDateTime
pub fn parse_local_datetime(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).ok()
}
