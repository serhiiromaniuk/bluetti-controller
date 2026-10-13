//! The subcommands. Each one reaches a station only through
//! [`bluetti_controller::transport`].

use bluetti_controller::config::{BleConfig, StationConfig};
use bluetti_controller::protocol::crypto::handshake::StaticKeys;
use clap::Subcommand;

mod detect;
mod scan;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List Bluetti stations in range: name, RSSI, model, encryption.
    Scan {
        /// Discovery time in seconds.
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=120))]
        timeout: u64,
    },
    /// Connect to one station (key exchange when encrypted): model, protocol,
    /// encryption flag, device type and firmware, then disconnect.
    Detect {
        /// A station name from the config, or a MAC address.
        station: String,
    },
}

pub async fn run(
    command: Command,
    ble: &BleConfig,
    stations: &[StationConfig],
    keys: Option<&StaticKeys>,
    anonymise: bool,
) -> anyhow::Result<()> {
    match command {
        Command::Scan { timeout } => scan::run(ble, timeout, anonymise).await,
        Command::Detect { station } => detect::run(ble, stations, keys, &station, anonymise).await,
    }
}
