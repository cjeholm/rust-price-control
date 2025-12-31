CREATE TABLE IF NOT EXISTS config (
    id INTEGER PRIMARY KEY CHECK (id = 1),

    api TEXT NOT NULL,
    area TEXT NOT NULL,
    currency TEXT NOT NULL,
    interval INTEGER NOT NULL,
    webui_port INTEGER NOT NULL,
    webui_toggle INTEGER NOT NULL,

    grid_fee REAL NOT NULL,
    energy_tax REAL NOT NULL,
    variable_costs REAL NOT NULL,
    spot_fee REAL NOT NULL,
    cert_fee REAL NOT NULL,
    vat REAL NOT NULL,

    telldus_ip TEXT NOT NULL,
    telldus_token TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS devices (
    name TEXT PRIMARY KEY,
    mode TEXT NOT NULL DEFAULT 'Unknown',
    ratio REAL NOT NULL DEFAULT 0,
    price REAL NOT NULL DEFAULT 0,
    today_trigger_price REAL NOT NULL DEFAULT 0,
    tomorrow_trigger_price REAL NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'Unknown',
    force_update INTEGER NOT NULL DEFAULT 0,
    telldus INTEGER NOT NULL DEFAULT 0,
    telldus_id TEXT NOT NULL DEFAULT '',
    script_on TEXT NOT NULL DEFAULT '',
    script_off TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS todays_prices (
    id INTEGER PRIMARY KEY,
    json TEXT NOT NULL DEFAULT ''
);

CREATE TABLE IF NOT EXISTS tomorrows_prices (
    id INTEGER PRIMARY KEY,
    json TEXT NOT NULL DEFAULT ''
);

