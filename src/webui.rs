use log::{debug, error, info, warn};
use serde_json::Value;
use std::fs;
use tiny_http::StatusCode;
use tiny_http::{Response, Server};
use urlencoding::decode;

use crate::{db, device_model, functions};
use crate::{structs, telldus};

fn respond_json(request: tiny_http::Request, body: String, status: StatusCode) {
    let _ = request.respond(
        Response::from_string(body)
            .with_status_code(status)
            .with_header(
                "Content-Type: application/json"
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            ),
    );
}

fn respond_text(request: tiny_http::Request, body: &str, status: StatusCode, content_type: &str) {
    let _ = request.respond(
        Response::from_string(body)
            .with_status_code(status)
            .with_header(
                format!("Content-Type: {}", content_type)
                    .parse::<tiny_http::Header>()
                    .unwrap(),
            ),
    );
}

fn read_static(path: &str, embedded: &str, debug: bool) -> Result<String, std::io::Error> {
    if debug {
        fs::read_to_string(path)
    } else {
        Ok(embedded.to_string())
    }
}

pub fn run_server() {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

    // Connect to the database
    let mut db_conn = db::open_conn().expect("Failed to open DB connection");
    let config = db::load_config(&db_conn).expect("Failed to read config from DB");
    let config_clean = structs::Config{
        telldus_token: "<hidden>".to_string(),
        ..config.clone()
    };
    

    let addr = format!("0.0.0.0:{}", config.webui_port);
    let server = Server::http(&addr).expect("Failed to bind HTTP server");

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        debug!("Incoming request: {}", url);

        match (request.method(), url.as_str()) {
            // ---------------- health ----------------
            (_, "/health") => {
                respond_text(request, "OK", StatusCode(200), "text/plain");
            }

            // ---------------- listdevices ----------------
            (_, "/listdevices") => {
                let (json, status) = match telldus::telldus_list(&config) {
                    Ok(json) => (json, StatusCode(200)),
                    Err(e) => {
                        error!("Telldus request failed: {e}. Check telldus ip address and token.");
                        (r#"{ "device": [] }"#.to_string(), StatusCode(500))
                    }
                };

                respond_json(request, json, status);
            }

            (_, "/config") => match serde_json::to_string(&config_clean) {
                Ok(json) => respond_json(request, json, StatusCode(200)),
                Err(e) => {
                    error!("JSON serialize failed: {e}");
                    respond_json(request, "{}".to_string(), StatusCode(500));
                }
            },

            (_, "/devices") => {
                let devices = db::load_devices(&db_conn).expect("Failed to read devices from DB");
                match serde_json::to_string(&devices) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

            (_, "/today") => {
                let todays_spot_prices = db::load_todays_prices(&db_conn)
                    .expect("Error reading todays prices")
                    .unwrap_or_else(|| "{}".to_string());
                respond_json(request, todays_spot_prices, StatusCode(200));
            }

            (_, "/tomorrow") => {
                let tomorrows_spot_prices = db::load_tomorrows_prices(&db_conn)
                    .expect("Error reading tomorrows prices")
                    .unwrap_or_else(|| "{}".to_string());
                respond_json(request, tomorrows_spot_prices, StatusCode(200));
            }

            // (tiny_http::Method::Post, path) if path.starts_with("/switchon/") => {
            //     if !config.webui_toggle {
            //         warn!("Disabled: User switching On device");
            //         respond_json(
            //             request,
            //             r#"{"status":"forbidden"}"#.to_string(),
            //             StatusCode(403),
            //         );
            //         continue;
            //     }
            //
            //     let name_encoded = path.trim_start_matches("/switchon/");
            //     let name = decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());
            //
            //     let mut found = false;
            //     let mut devices =
            //         db::load_devices(&db_conn).expect("Failed to read devices from DB");
            //     for device in devices.device.iter_mut() {
            //         if device.name == name {
            //             info!("User switching On device {}", name);
            //             if let Err(e) = device.switch_on(&config) {
            //                 error!("Failed to switch on {}: {e}", name);
            //             }
            //             found = true;
            //         }
            //     }
            //
            //     if found {
            //         respond_json(
            //             request,
            //             format!(r#"{{"status":"ok","action":"on","name":"{}"}}"#, name),
            //             StatusCode(200),
            //         );
            //     } else {
            //         respond_json(
            //             request,
            //             format!(
            //                 r#"{{"status":"not_found","action":"on","name":"{}"}}"#,
            //                 name
            //             ),
            //             StatusCode(404),
            //         );
            //     }
            // }

            // Change mode to On
            (tiny_http::Method::Post, path) if path.starts_with("/switchon/") => {
                if !config.webui_toggle {
                    warn!("Disabled: User switching On device");
                    respond_json(
                        request,
                        r#"{"status":"forbidden"}"#.to_string(),
                        StatusCode(403),
                    );
                    continue;
                }

                let name_encoded = path.trim_start_matches("/switchon/");
                let name = decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                let mut found = false;
                let mut devices =
                    db::load_devices(&db_conn).expect("Failed to read devices from DB");
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User changing {} mode to On", name);
                        device.mode = device_model::Mode::On;
                        device.today_trigger_price = 9999.9;
                        device.tomorrow_trigger_price = 9999.9;
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"on","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
                    force_update(&mut db_conn, devices, &config);
                } else {
                    respond_json(
                        request,
                        format!(
                            r#"{{"status":"not_found","action":"on","name":"{}"}}"#,
                            name
                        ),
                        StatusCode(404),
                    );
                }
            }

            // Change mode to Off
            (tiny_http::Method::Post, path) if path.starts_with("/switchoff/") => {
                if !config.webui_toggle {
                    warn!("Disabled: User switching Off device");
                    respond_json(
                        request,
                        r#"{"status":"forbidden"}"#.to_string(),
                        StatusCode(403),
                    );
                    continue;
                }

                let name_encoded = path.trim_start_matches("/switchoff/");
                let name = decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                let mut found = false;
                let mut devices =
                    db::load_devices(&db_conn).expect("Failed to read devices from DB");
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User changing {} mode to Off", name);
                        device.mode = device_model::Mode::Off;
                        device.today_trigger_price = -9999.9;
                        device.tomorrow_trigger_price = -9999.9;
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"off","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
                    force_update(&mut db_conn, devices, &config);
                } else {
                    respond_json(
                        request,
                        format!(
                            r#"{{"status":"not_found","action":"off","name":"{}"}}"#,
                            name
                        ),
                        StatusCode(404),
                    );
                }
            }

            // Change mode to Price
            (tiny_http::Method::Post, path) if path.starts_with("/switchprice/") => {
                if !config.webui_toggle {
                    warn!("Disabled: User switching device mode");
                    respond_json(
                        request,
                        r#"{"status":"forbidden"}"#.to_string(),
                        StatusCode(403),
                    );
                    continue;
                }

                let name_encoded = path.trim_start_matches("/switchprice/");
                let name = decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                let mut found = false;
                let mut devices =
                    db::load_devices(&db_conn).expect("Failed to read devices from DB");
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User changing {} mode to Price", name);
                        device.mode = device_model::Mode::Price;
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"price","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
                    force_update(&mut db_conn, devices, &config);
                } else {
                    respond_json(
                        request,
                        format!(
                            r#"{{"status":"not_found","action":"price","name":"{}"}}"#,
                            name
                        ),
                        StatusCode(404),
                    );
                }
            }

            // Change mode to Ratio
            (tiny_http::Method::Post, path) if path.starts_with("/switchratio/") => {
                if !config.webui_toggle {
                    warn!("Disabled: User switching device mode");
                    respond_json(
                        request,
                        r#"{"status":"forbidden"}"#.to_string(),
                        StatusCode(403),
                    );
                    continue;
                }

                let name_encoded = path.trim_start_matches("/switchratio/");
                let name = decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                let mut found = false;
                let mut devices =
                    db::load_devices(&db_conn).expect("Failed to read devices from DB");
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User changing {} mode to Ratio", name);
                        device.mode = device_model::Mode::Ratio;
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"ratio","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
                    force_update(&mut db_conn, devices, &config);
                } else {
                    respond_json(
                        request,
                        format!(
                            r#"{{"status":"not_found","action":"ratio","name":"{}"}}"#,
                            name
                        ),
                        StatusCode(404),
                    );
                }
            }


            // ---------------- static files ----------------
            (_, "/pricecontrol.js") => {
                match read_static(
                    "static/pricecontrol.js",
                    include_str!("../static/pricecontrol.js"),
                    DEBUG,
                ) {
                    Ok(js) => respond_text(request, &js, StatusCode(200), "application/javascript"),
                    Err(e) => {
                        error!("Failed to load pricecontrol.js: {e}");
                        respond_text(request, "Internal error", StatusCode(500), "text/plain");
                    }
                }
            }

            (_, "/listdevices.htm") => {
                match read_static(
                    "static/listdevices.htm",
                    include_str!("../static/listdevices.htm"),
                    DEBUG,
                ) {
                    Ok(html) => respond_text(request, &html, StatusCode(200), "text/html"),
                    Err(e) => {
                        error!("Failed to load listdevices.htm: {e}");
                        respond_text(request, "Internal error", StatusCode(500), "text/plain");
                    }
                }
            }

            (_, "/") => {
                match read_static(
                    "static/index.html",
                    include_str!("../static/index.html"),
                    DEBUG,
                ) {
                    Ok(raw) => {
                        let html = raw
                            .replace("{{PROJECT_NAME}}", env!("CARGO_PKG_NAME"))
                            .replace("{{PROJECT_VERSION}}", env!("CARGO_PKG_VERSION"))
                            .replace("{{PROJECT_AUTHORS}}", env!("CARGO_PKG_AUTHORS"));

                        respond_text(request, &html, StatusCode(200), "text/html");
                    }
                    Err(e) => {
                        error!("Failed to load index.html: {e}");
                        respond_text(request, "Internal error", StatusCode(500), "text/plain");
                    }
                }
            }

            // ---------------- fallback ----------------
            _ => {
                respond_text(request, "Not found", StatusCode(404), "text/plain");
            }
        }
    }
}

fn force_update(
    db_conn: &mut rusqlite::Connection,
    devices: device_model::Devices,
    config: &structs::Config,
) {
    let today_spot_prices: Value = {
        let json_str = db::load_todays_prices(db_conn)
            .expect("Error reading todays prices")
            .expect("No prices found");
        serde_json::from_str(&json_str).expect("Invalid JSON in todays prices")
    };
    let tomorrow_spot_prices: Value = {
        let json_str = db::load_tomorrows_prices(db_conn)
            .expect("Error reading todays prices")
            .expect("No prices found");
        serde_json::from_str(&json_str).expect("Invalid JSON in todays prices")
    };
    let devices = functions::logic_loop(&today_spot_prices, &tomorrow_spot_prices, devices, config);
    let _ = db::update_devices(db_conn, &devices.unwrap());
}
