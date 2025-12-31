use log::debug;
use rusqlite::{params, Connection};
use std::env;
use std::path::PathBuf;

use crate::{device_model, structs};

pub fn open_conn() -> rusqlite::Result<Connection> {
    let dir = env::temp_dir();
    let db_path: PathBuf = dir.join("pricecontrol.db");
    let db_conn = Connection::open(&db_path)?;
    db_conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    db_conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(db_conn)
}

pub fn init_db(db_conn: &Connection) -> rusqlite::Result<()> {
    db_conn.execute_batch(include_str!("../static/schema.sql"))?;
    let path: String = db_conn.query_row("PRAGMA database_list;", [], |row| row.get(2))?;
    debug!("Database initialized at {}", path);
    Ok(())
}

pub fn save_config(db_conn: &Connection, config: &structs::Config) -> rusqlite::Result<()> {
    db_conn.execute(
        r#"
        INSERT OR REPLACE INTO config (
            id,
            api,
            area,
            currency,
            interval,
            webui_port,
            webui_toggle,
            grid_fee,
            energy_tax,
            variable_costs,
            spot_fee,
            cert_fee,
            vat,
            telldus_ip,
            telldus_token
        ) VALUES (
            1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
        )
        "#,
        params![
            config.api,
            config.area,
            config.currency,
            config.interval as i64,
            config.webui_port as i64,
            config.webui_toggle as i64,
            config.grid_fee,
            config.energy_tax,
            config.variable_costs,
            config.spot_fee,
            config.cert_fee,
            config.vat,
            config.telldus_ip,
            config.telldus_token
        ],
    )?;
    Ok(())
}

pub fn load_config(db_conn: &Connection) -> rusqlite::Result<structs::Config> {
    db_conn.query_row(
        r#"
        SELECT
            api,
            area,
            currency,
            interval,
            webui_port,
            webui_toggle,
            grid_fee,
            energy_tax,
            variable_costs,
            spot_fee,
            cert_fee,
            vat,
            telldus_ip,
            telldus_token
        FROM config
        WHERE id = 1
        "#,
        [],
        |row| {
            Ok(structs::Config {
                api: row.get("api")?,
                area: row.get("area")?,
                currency: row.get("currency")?,
                interval: row.get::<_, i64>("interval")? as u64,
                webui_port: row.get::<_, i64>("webui_port")? as u64,
                webui_toggle: row.get::<_, i64>("webui_toggle")? != 0,

                grid_fee: row.get("grid_fee")?,
                energy_tax: row.get("energy_tax")?,
                variable_costs: row.get("variable_costs")?,
                spot_fee: row.get("spot_fee")?,
                cert_fee: row.get("cert_fee")?,
                vat: row.get("vat")?,

                telldus_ip: row.get("telldus_ip")?,
                telldus_token: row.get("telldus_token")?,
            })
        },
    )
}

pub fn save_devices(
    db_conn: &mut Connection,
    devices: &device_model::Devices,
) -> rusqlite::Result<()> {
    let tx = db_conn.transaction()?; // atomic update

    // clear existing devices
    tx.execute("DELETE FROM devices", [])?;

    for d in &devices.device {
        tx.execute(
            r#"
            INSERT INTO devices (
                name,
                mode,
                ratio,
                price,
                today_trigger_price,
                tomorrow_trigger_price,
                state,
                force_update,
                telldus,
                telldus_id,
                script_on,
                script_off
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                d.name,
                format!("{:?}", d.mode), // enum -> string
                d.ratio,
                d.price,
                d.today_trigger_price,
                d.tomorrow_trigger_price,
                format!("{:?}", d.state), // enum -> string
                d.force_update as i64,
                d.telldus as i64,
                d.telldus_id,
                d.script_on,
                d.script_off,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn load_devices(db_conn: &Connection) -> rusqlite::Result<device_model::Devices> {
    let mut stmt = db_conn.prepare(
        r#"
        SELECT
            name,
            mode,
            ratio,
            price,
            today_trigger_price,
            tomorrow_trigger_price,
            state,
            force_update,
            telldus,
            telldus_id,
            script_on,
            script_off
        FROM devices
        "#,
    )?;

    let device_iter = stmt.query_map([], |row| {
        // read strings and convert to enums
        let mode = match row.get::<_, String>("mode")?.as_str() {
            "Price" => device_model::Mode::Price,
            "Ratio" => device_model::Mode::Ratio,
            "On" => device_model::Mode::On,
            "Off" => device_model::Mode::Off,
            _ => device_model::Mode::Unknown,
        };

        let state = match row.get::<_, String>("state")?.as_str() {
            "On" => device_model::State::On,
            "Off" => device_model::State::Off,
            _ => device_model::State::Unknown,
        };

        Ok(device_model::Device {
            name: row.get("name")?,
            mode,
            ratio: row.get("ratio")?,
            price: row.get("price")?,
            today_trigger_price: row.get("today_trigger_price")?,
            tomorrow_trigger_price: row.get("tomorrow_trigger_price")?,
            state,
            force_update: row.get::<_, i64>("force_update")? != 0,
            telldus: row.get::<_, i64>("telldus")? != 0,
            telldus_id: row.get("telldus_id")?,
            script_on: row.get("script_on")?,
            script_off: row.get("script_off")?,
        })
    })?;

    let devices = device_iter.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(device_model::Devices { device: devices })
}

pub fn update_devices(
    db_conn: &mut Connection,
    devices: &device_model::Devices,
) -> rusqlite::Result<()> {
    let tx = db_conn.transaction()?; // atomic update

    for d in &devices.device {
        tx.execute(
            r#"
            INSERT INTO devices (
                name,
                mode,
                ratio,
                price,
                today_trigger_price,
                tomorrow_trigger_price,
                state,
                force_update,
                telldus,
                telldus_id,
                script_on,
                script_off
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
                mode = excluded.mode,
                ratio = excluded.ratio,
                price = excluded.price,
                today_trigger_price = excluded.today_trigger_price,
                tomorrow_trigger_price = excluded.tomorrow_trigger_price,
                state = excluded.state,
                force_update = excluded.force_update,
                telldus = excluded.telldus,
                telldus_id = excluded.telldus_id,
                script_on = excluded.script_on,
                script_off = excluded.script_off
            "#,
            params![
                d.name,
                format!("{:?}", d.mode), // enum -> string
                d.ratio,
                d.price,
                d.today_trigger_price,
                d.tomorrow_trigger_price,
                format!("{:?}", d.state), // enum -> string
                d.force_update as i64,
                d.telldus as i64,
                d.telldus_id,
                d.script_on,
                d.script_off,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn save_todays_prices(db_conn: &Connection, json_content: String) -> rusqlite::Result<()> {
    db_conn.execute(
        r#"
        INSERT INTO todays_prices(id, json)
        VALUES (1, ?)
        ON CONFLICT(id) DO UPDATE SET json = excluded.json
        "#,
        params![json_content],
    )?;
    Ok(())
}

pub fn load_todays_prices(db_conn: &Connection) -> rusqlite::Result<Option<String>> {
    let result = db_conn.query_row("SELECT json FROM todays_prices WHERE id = 1", [], |row| {
        row.get::<_, String>(0)
    });

    match result {
        Ok(json) => Ok(Some(json)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn save_tomorrows_prices(db_conn: &Connection, json_content: String) -> rusqlite::Result<()> {
    db_conn.execute(
        r#"
        INSERT INTO tomorrows_prices(id, json)
        VALUES (1, ?)
        ON CONFLICT(id) DO UPDATE SET json = excluded.json
        "#,
        params![json_content],
    )?;
    Ok(())
}

pub fn load_tomorrows_prices(db_conn: &Connection) -> rusqlite::Result<Option<String>> {
    let result = db_conn.query_row("SELECT json FROM tomorrows_prices WHERE id = 1", [], |row| {
        row.get::<_, String>(0)
    });

    match result {
        Ok(json) => Ok(Some(json)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
