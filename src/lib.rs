//! Local monitor for Bluetti power stations over Bluetooth LE.
//!
//! The hardware-independent protocol layer (Modbus framing, device detection,
//! register decoding), config loading, the BLE transport, SQLite storage and
//! the derived metrics.
//! The collector polls the stations; the web UI follows milestone by milestone (see `ROADMAP.md` §12).

pub mod collector;
pub mod config;
pub mod metrics;
pub mod protocol;
pub mod store;
pub mod transport;
pub mod web;
