//! `bluetti-controller` command line: read-only station tools (`ROADMAP.md`
//! §12 M1). Only function 0x03 frames are ever sent (golden rule #2).

use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod cli;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Config file (`config.example.toml`); without it the `[ble]` defaults apply
    /// and no station names resolve.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    /// Mask MAC addresses and serial numbers in the output (RULES.md golden rule #3).
    #[arg(long, global = true)]
    anonymise: bool,
    #[command(subcommand)]
    command: cli::Command,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();
    let args = Args::parse();
    let (ble, stations) = match &args.config {
        Some(path) => {
            let config = bluetti_controller::config::load(path)?;
            (config.ble, config.stations)
        }
        None => (Default::default(), Vec::new()),
    };
    cli::run(args.command, &ble, &stations, args.anonymise).await
}
