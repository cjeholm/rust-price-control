use log::{debug, info, warn};
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tiny_http::{Response, Server};
use urlencoding::decode;

use crate::actions;
use crate::structs;
use crate::telldus;

pub fn run_server(
    data: Arc<Mutex<structs::AppState>>,
    config: &structs::Config,
    mut devices: structs::Devices,
) {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

    let addr = format!("0.0.0.0:{}", config.webui_port);
    let server = Server::http(&addr).unwrap();

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        debug!("Incoming request: {}", url);

        match (request.method(), url.as_str()) {
            (tiny_http::Method::Post, path) if path.starts_with("/switchon/") => {
                if config.webui_toggle {
                    let name_encoded = path.trim_start_matches("/switchon/");
                    let name =
                        decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                    for device in devices.device.iter_mut() {
                        if device.name == name {
                            info!("User switching On device {}", name);
                            let _ = actions::change_state(config, device, structs::State::On);
                        }
                    }

                    let _ = request.respond(
                        Response::from_string(format!(
                            r#"{{"status":"ok","action":"on","name":"{}"}}"#,
                            name
                        ))
                        .with_header(
                            "Content-Type: application/json"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        )
                        .with_status_code(200),
                    );
                } else {
                    let name = path.trim_start_matches("/switchon/");
                    warn!("Disabled: User switching On device {}", name);

                    let _ = request.respond(
                        Response::from_string(format!(
                            r#"{{"status":"forbidden","action":"on","name":"{}"}}"#,
                            name
                        ))
                        .with_header(
                            "Content-Type: application/json"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        )
                        .with_status_code(403),
                    );
                }
            }

            (tiny_http::Method::Post, path) if path.starts_with("/switchoff/") => {
                if config.webui_toggle {
                    let name_encoded = path.trim_start_matches("/switchoff/");
                    let name =
                        decode(name_encoded).unwrap_or_else(|_| name_encoded.to_string().into());

                    for device in devices.device.iter_mut() {
                        if device.name == name {
                            info!("User switching Off {}", name);
                            let _ = actions::change_state(config, device, structs::State::Off);
                        }
                    }

                    let _ = request.respond(
                        Response::from_string(format!(
                            r#"{{"status":"ok","action":"off","name":"{}"}}"#,
                            name
                        ))
                        .with_header(
                            "Content-Type: application/json"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        )
                        .with_status_code(200),
                    );
                } else {
                    let name = path.trim_start_matches("/switchoff/");
                    warn!("Disabled: User switching Off device {}", name);

                    let _ = request.respond(
                        Response::from_string(format!(
                            r#"{{"status":"forbidden","action":"off","name":"{}"}}"#,
                            name
                        ))
                        .with_header(
                            "Content-Type: application/json"
                                .parse::<tiny_http::Header>()
                                .unwrap(),
                        )
                        .with_status_code(403),
                    );
                }
            }

            (_, "/health") => {
                let _ = request.respond(Response::from_string("OK").with_status_code(200));
            }

            (_, "/listdevices") => {
                let state = data.lock().unwrap();
                let json = telldus::telldus_list(config).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/data") => {
                let state = data.lock().unwrap();
                let json = serde_json::to_string(&*state).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/config") => {
                let state = data.lock().unwrap();
                let json = serde_json::to_string(&state.config).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/devices") => {
                let state = data.lock().unwrap();
                let json = serde_json::to_string(&state.devices).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/today") => {
                let state = data.lock().unwrap();
                let json = serde_json::to_string(&state.todays_spot_prices).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/tomorrow") => {
                let state = data.lock().unwrap();
                let json = serde_json::to_string(&state.tomorrows_spot_prices).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/pricecontrol.js") => {
                let html = if DEBUG {
                    fs::read_to_string("static/pricecontrol.js").unwrap()
                } else {
                    include_str!("../static/pricecontrol.js").to_string()
                };

                let _ = request.respond(
                    Response::from_string(html).with_header(
                        "Content-Type: text/html"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/listdevices.htm") => {
                let html = if DEBUG {
                    fs::read_to_string("static/listdevices.htm").unwrap()
                } else {
                    include_str!("../static/listdevices.htm").to_string()
                };

                let _ = request.respond(
                    Response::from_string(html).with_header(
                        "Content-Type: text/html"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            (_, "/") => {
                let raw_html = if DEBUG {
                    fs::read_to_string("static/index.html").unwrap()
                } else {
                    include_str!("../static/index.html").to_string()
                };

                let html = raw_html
                    .replace("{{PROJECT_NAME}}", env!("CARGO_PKG_NAME"))
                    .replace("{{PROJECT_VERSION}}", env!("CARGO_PKG_VERSION"))
                    .replace("{{PROJECT_AUTHORS}}", env!("CARGO_PKG_AUTHORS"));

                let _ = request.respond(
                    Response::from_string(html).with_header(
                        "Content-Type: text/html"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            _ => {
                let _ = request.respond(Response::from_string("Not found").with_status_code(404));
            }
        }
    }
}
