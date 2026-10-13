//! Local monitor for Bluetti power stations over Bluetooth LE.
//!
//! The hardware-independent protocol layer (Modbus framing, device detection,
//! register decoding), config loading, the BLE transport and SQLite storage.
//! The collector and the web UI follow milestone by milestone (see `ROADMAP.md` §12).

pub mod config;
pub mod protocol;
pub mod store;
pub mod transport;
