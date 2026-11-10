//! The subcommands. Each one reaches a station only through
//! [`bluetti_controller::transport`]; only `set` writes, through the control
//! gate (golden rule #2).

use std::path::{Path, PathBuf};

use anyhow::bail;
use bluetti_controller::config::{BleConfig, StationConfig};
use bluetti_controller::protocol::control::Output;
use bluetti_controller::protocol::crypto::handshake::StaticKeys;
use bluetti_controller::transport::anonymise::{anonymise_fixture, anonymise_wire};
use bluetti_controller::transport::fixture::Fixture;
use clap::Subcommand;

pub mod check;
mod detect;
mod dump;
mod hash_password;
mod read;
mod scan;
pub mod serve;
mod session;
pub mod set;

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
    /// Validate the config and keys file and compare the database schema with
    /// this binary's, read-only and without BLE (`ROADMAP.md` §10.6). Exit 0
    /// ok, 1 keys or database unusable, 2 the database is newer than this
    /// binary, 4 the config is rejected.
    /// Needs `--config`.
    Check,
    /// Poll every configured station, store the readings and serve
    /// `/healthz` until SIGINT/SIGTERM (`ROADMAP.md` §5.3). Needs `--config`.
    Serve,
    /// Switch one output on or off and print the read-back state, for the
    /// hardware checks (`ROADMAP.md` §R3). Refused before any BLE unless
    /// `control.enabled = true` and the model allow-lists the output (golden
    /// rule #2). Needs `--config`; with `serve` running, pause polling first.
    Set {
        /// A station name from the config (never a MAC address).
        station: String,
        /// `ac` or `dc`.
        #[arg(value_parser = set::parse_output)]
        output: Output,
        /// `on` or `off`.
        #[arg(value_parser = set::parse_state, action = clap::ArgAction::Set)]
        state: bool,
    },
    /// Read one password line from stdin and print its argon2id PHC string
    /// for `web.password_hash` (`ROADMAP.md` ADR-011). No config, no station.
    HashPassword,
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
        Command::HashPassword => hash_password::run(),
        // main runs them with the owned config and keys.
        Command::Serve | Command::Check | Command::Set { .. } => {
            bail!("serve, check and set are dispatched by main")
        }
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
