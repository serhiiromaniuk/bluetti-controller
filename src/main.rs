//! `bluetti-controller` command line: read-only station tools (`ROADMAP.md`
//! §12 M1) and the `serve` monitor (M3). Only function 0x03 frames are ever sent (golden rule #2).

use std::path::PathBuf;

use anyhow::Context as _;
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
    /// Save the plaintext session (above the encryption layer) as fixture
    /// JSON; only under `captures/`, which is gitignored (raw: holds the serial).
    #[arg(long, global = true, value_name = "PATH")]
    record: Option<PathBuf>,
    /// Save the wire layer (below the encryption) as fixture JSON, with the
    /// session's ephemeral secret; only under `captures/`. `anonymise --wire`
    /// masks it for commit.
    #[arg(long, global = true, value_name = "PATH")]
    record_wire: Option<PathBuf>,
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
    let config = args
        .config
        .as_deref()
        .map(bluetti_controller::config::load)
        .transpose()?;
    // Fail fast on a bad keys file, before any station is touched.
    let keys = config
        .as_ref()
        .and_then(|c| c.crypto.as_ref())
        .map(|c| c.load_keys())
        .transpose()?;
    if let cli::Command::Serve = args.command {
        let config = config.context("serve needs --config")?;
        return cli::serve::run(&config, keys).await;
    }
    let default_ble = Default::default();
    let ctx = cli::Context {
        ble: config.as_ref().map_or(&default_ble, |c| &c.ble),
        stations: config.as_ref().map_or(&[], |c| &c.stations),
        keys: keys.as_ref(),
        anonymise: args.anonymise,
        record: args.record.as_deref(),
        record_wire: args.record_wire.as_deref(),
    };
    cli::run(args.command, &ctx).await
}
