# rpc — Rust Price Control

Control devices based on the current electricity spot price. Devices can switch
on/off automatically or trigger custom scripts according to user-defined rules.

> This project is currently in beta testing phase

## Overview

rpc retrieves daily electricity spot prices and evaluates them against each
device’s configuration. Based on the current conditions, it can:

- Control Telldus smart switches
- Toggle virtual devices
- Execute custom scripts

The goal is to automate energy usage so devices run when power is cheap and
stay off when it is expensive.

The application is cross-platform and can run:

- As a normal binary
- As a system service
- In Docker (image not yet provided)

A local web UI provides:

- Price graph for today and tomorrow
- Device list with current state

All configuration is handled through a single config pricecontrol.toml in the
current path of the systems default config path.

## Spot Price API and Currency Support

The application supports user-specified spot-price APIs. Any API returning
hourly or more frequent prices can be used as long as the required fields are
mapped in the configuration.

It also supports user-specified currencies provided by those APIs such as:

- SEK
- NOK
- DKK
- EUR

Additional currencies can be used as long as the selected API provides values
in that unit.

## Device Modes

### Price Mode

The device reacts to the current spot price using its configured `price` value.
The program decides whether the device should be on or off based on the setting.

### Ratio Mode

A ratio between `0.0` and `1.0` defines how large a portion of the day the device
should be active. The application automatically chooses the cheapest hours of
the day based on the given ratio.

Example:
`ratio = 0.25` → device active during the cheapest 25% of hours.

Both modes support Telldus devices and virtual devices with script triggers.

## Telldus Support

When a Telldus Tellstick is used and a valid API token is provided, the
application can:

- Control Telldus smart switches
- Automatically list available Telldus devices

This eliminates the need to manually copy device IDs.

## Features

- User-specified electricity spot-price APIs
- User-specified currencies (SEK, NOK, DKK, EUR, etc.)
- Price Mode and Ratio Mode
- Telldus smart switch integration
- Automatic Telldus device discovery
- Virtual devices
- Script triggers for mode events
- Local web dashboard with price graphs
- Cross-platform operation
- Can run as a system service
- Single TOML configuration

## Running

`cargo run --release`

Or build and run:

`cargo build --release`
`./target/release/rpc`

A systemd unit or similar can be used for service mode.
A Dockerfile will be provided later.

## Web UI

Open the UI after startup:

`http://localhost:8088`

Shows the price graph and device states.

## Roadmap

- Official Docker image
- Support for additional smart-home systems
- Extended rule engine
- Optional notifications

## Contributing

Issue reports are welcome. Pull requests are not handled at this moment.

## License

This software is not yet licensed.
