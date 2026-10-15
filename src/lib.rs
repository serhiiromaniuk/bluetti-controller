//! Local monitor for Bluetti power stations over Bluetooth LE.
//!
//! The hardware-independent protocol layer (Modbus framing, device detection,
//! register decoding), config loading, the BLE transport, SQLite storage and
//! the derived metrics.
//! The collector polls the stations; the web UI follows milestone by milestone (see `ROADMAP.md` §12).

/// The build this binary came from: `BLUETTI_BUILD` at compile time (the
/// Makefile sets `git describe --tags --always --dirty`), else `v` + the crate
/// version (also when it is set but empty, e.g. built outside git). `--version`
/// and `/healthz` report it (ROADMAP.md §10.6).
pub const BUILD: &str = match option_env!("BLUETTI_BUILD") {
    Some(build) if !build.is_empty() => build,
    _ => concat!("v", env!("CARGO_PKG_VERSION")),
};

pub mod collector;
pub mod config;
pub mod metrics;
pub mod protocol;
pub mod store;
pub mod transport;
pub mod web;
