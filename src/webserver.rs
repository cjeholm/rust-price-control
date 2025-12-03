use log::debug;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tiny_http::{Response, Server};

use crate::structs;
use crate::telldus;

pub fn run_server(data: Arc<Mutex<structs::AppState>>, config: &structs::Config) {
    #[cfg(debug_assertions)]
    const DEBUG: bool = true;
    #[cfg(not(debug_assertions))]
    const DEBUG: bool = false;

    let addr = format!("0.0.0.0:{}", config.webui_port);
    let server = Server::http(&addr).unwrap();

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        debug!("Incoming request: {}", url);

        match url.as_str() {
            "/listdevices" => {
                let state = data.lock().unwrap();
                let json = telldus::telldus_list(&config).unwrap();
                drop(state);

                let _ = request.respond(
                    Response::from_string(json).with_header(
                        "Content-Type: application/json"
                            .parse::<tiny_http::Header>()
                            .unwrap(),
                    ),
                );
            }

            "/data" => {
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

            "/config" => {
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

            "/devices" => {
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

            "/today" => {
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

            "/tomorrow" => {
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

            "/pricecontrol.js" => {
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

            "/listdevices.htm" => {
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

            "/" => {
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
