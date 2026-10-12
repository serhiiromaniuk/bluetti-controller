//! Local monitor for Bluetti power stations over Bluetooth LE.
//!
//! M0 contains only the hardware-independent protocol layer: Modbus framing,
//! device detection and register decoding. Transport, storage and the web UI
//! follow in later milestones (see `ROADMAP.md` §12).

pub mod protocol;
