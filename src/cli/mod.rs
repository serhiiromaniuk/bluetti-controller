//! The subcommands. Each one reaches a station only through
//! [`bluetti_controller::transport`].

use bluetti_controller::config::BleConfig;
use clap::Subcommand;

mod scan;

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List Bluetti stations in range: name, RSSI, model, encryption.
    Scan {
        /// Discovery time in seconds.
        #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u64).range(1..=120))]
        timeout: u64,
    },
}

pub async fn run(command: Command, ble: &BleConfig, anonymise: bool) -> anyhow::Result<()> {
    match command {
        Command::Scan { timeout } => scan::run(ble, timeout, anonymise).await,
    }
}
