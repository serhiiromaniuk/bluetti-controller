//! The subcommands. Each one reaches a station only through
//! [`bluetti_controller::transport`].

use std::path::{Path, PathBuf};

use anyhow::bail;
use bluetti_controller::config::{BleConfig, StationConfig};
use bluetti_controller::protocol::crypto::handshake::StaticKeys;
use bluetti_controller::transport::anonymise::{anonymise_fixture, anonymise_wire};
use bluetti_controller::transport::fixture::Fixture;
use clap::Subcommand;

mod detect;
mod dump;
mod read;
mod scan;
pub mod serve;
mod session;

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
    /// Detect, then read and decode the station's profile once. Exits non-zero
    /// when a field is missing or implausible.
    Read {
        /// A station name from the config, or a MAC address.
        station: String,
    },
    /// Read-only sweep of a register range, in chunks of at most 40 registers.
    Dump {
        /// A station name from the config, or a MAC address.
        station: String,
        /// First register.
        #[arg(long)]
        from: u16,
        /// Last register, inclusive.
        #[arg(long)]
        to: u16,
    },
    /// Poll every configured station, store the readings and serve
    /// `/healthz` until SIGINT/SIGTERM (`ROADMAP.md` §5.3). Needs `--config`.
    Serve,
    /// Mask a capture for commit under `tests/frames/` (RULES.md golden rule
    /// #3). No station is touched.
    Anonymise {
        /// A `--record` capture (or `--record-wire` with `--wire`).
        input: PathBuf,
        /// The masked fixture.
        output: PathBuf,
        /// The input is a wire capture: keep only the committable handshake
        /// artefacts of `ROADMAP.md` §4.4.
        #[arg(long)]
        wire: bool,
    },
}

/// What every command shares: config, keys and the global flags.
pub struct Context<'a> {
    pub ble: &'a BleConfig,
    pub stations: &'a [StationConfig],
    pub keys: Option<&'a StaticKeys>,
    pub anonymise: bool,
    /// `--record`: where the plaintext session is saved.
    pub record: Option<&'a Path>,
    /// `--record-wire`: where the wire layer (below the encryption) is saved.
    pub record_wire: Option<&'a Path>,
}

pub async fn run(command: Command, ctx: &Context<'_>) -> anyhow::Result<()> {
    match command {
        Command::Scan { timeout } => {
            if ctx.record.is_some() || ctx.record_wire.is_some() {
                bail!(
                    "--record and --record-wire need a command that connects (detect, read, dump)"
                );
            }
            scan::run(ctx.ble, timeout, ctx.anonymise).await
        }
        Command::Detect { station } => detect::run(ctx, &station).await,
        Command::Read { station } => read::run(ctx, &station).await,
        Command::Dump { station, from, to } => dump::run(ctx, &station, from, to).await,
        // main runs it with the owned config and keys.
        Command::Serve => bail!("serve is dispatched by main"),
        Command::Anonymise {
            input,
            output,
            wire,
        } => {
            let capture = Fixture::load(&input)?;
            let masked = if wire {
                anonymise_wire(&capture)?
            } else {
                anonymise_fixture(&capture)?
            };
            masked.save(&output)?;
            println!("{} frames to {}", masked.frames.len(), output.display());
            Ok(())
        }
    }
}
