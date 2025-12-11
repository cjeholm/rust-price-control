use log::{debug, error, info, warn};
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tiny_http::StatusCode;
use tiny_http::{Response, Server};
use urlencoding::decode;

use crate::telldus;
use crate::{device_model, structs};

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

pub fn run_server(
    data: Arc<Mutex<structs::AppState>>,
    config: &structs::Config,
    mut devices: device_model::Devices,
) {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

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
                let (json, status) = match telldus::telldus_list(config) {
                    Ok(json) => (json, StatusCode(200)),
                    Err(e) => {
                        error!("Telldus request failed: {e}. Check telldus ip address and token.");
                        (r#"{ "device": [] }"#.to_string(), StatusCode(500))
                    }
                };

                respond_json(request, json, status);
            }

            // ---------------- shared state helpers ----------------
            (_, "/data") => {
                let state = match data.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("State mutex poisoned: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                        continue;
                    }
                };

                match serde_json::to_string(&*state) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

            (_, "/config") => {
                let state = match data.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("State mutex poisoned: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                        continue;
                    }
                };

                match serde_json::to_string(&state.config) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

            (_, "/devices") => {
                let state = match data.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("State mutex poisoned: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                        continue;
                    }
                };

                match serde_json::to_string(&state.devices) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

            (_, "/today") => {
                let state = match data.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("State mutex poisoned: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                        continue;
                    }
                };

                match serde_json::to_string(&state.todays_spot_prices) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

            (_, "/tomorrow") => {
                let state = match data.lock() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("State mutex poisoned: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                        continue;
                    }
                };

                match serde_json::to_string(&state.tomorrows_spot_prices) {
                    Ok(json) => respond_json(request, json, StatusCode(200)),
                    Err(e) => {
                        error!("JSON serialize failed: {e}");
                        respond_json(request, "{}".to_string(), StatusCode(500));
                    }
                }
            }

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
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User switching On device {}", name);
                        if let Err(e) = device.switch_on(config) {
                            error!("Failed to switch on {}: {e}", name);
                        }
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"on","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
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
                for device in devices.device.iter_mut() {
                    if device.name == name {
                        info!("User switching Off device {}", name);
                        if let Err(e) = device.switch_off(config) {
                            error!("Failed to switch off {}: {e}", name);
                        }
                        found = true;
                    }
                }

                if found {
                    respond_json(
                        request,
                        format!(r#"{{"status":"ok","action":"off","name":"{}"}}"#, name),
                        StatusCode(200),
                    );
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
