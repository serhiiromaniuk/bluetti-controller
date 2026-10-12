//! BLE transport behind our own [`Gatt`] trait.
//!
//! Core logic reaches a station only through [`Gatt`] and [`Advert`], never
//! through a `bluer` type, so [`replay::ReplayGatt`] stands in for the BlueZ
//! adapter in tests (RULES.md architecture rules, `ROADMAP.md` §11).

use std::future::Future;
use std::time::Duration;

pub mod fixture;
pub mod read;
pub mod replay;

/// One BLE advertisement of a station, in our own types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advert {
    /// MAC address, `AA:BB:CC:DD:EE:FF`.
    pub address: String,
    /// Advertised local name: `<MODEL><serial digits>`.
    pub name: Option<String>,
    pub rssi: Option<i16>,
    /// Manufacturer data stored under
    /// [`COMPANY_ID`](crate::protocol::device::COMPANY_ID), without the id.
    pub manufacturer_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TransportError {
    #[error("no notification within {0:?}")]
    Timeout(Duration),
    #[error("not connected")]
    NotConnected,
    #[error("unexpected write {got}, expected {expected}")]
    UnexpectedWrite { expected: String, got: String },
}

/// The Modbus channel of one connected station (`ROADMAP.md` §4.1).
pub trait Gatt {
    /// Writes one request to characteristic `ff02` (write without response).
    fn write(&mut self, data: &[u8]) -> impl Future<Output = Result<(), TransportError>> + Send;

    /// Waits up to `timeout` for the next notification from characteristic `ff01`.
    fn next_notification(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<Vec<u8>, TransportError>> + Send;

    /// Closes the connection; every later call fails with
    /// [`TransportError::NotConnected`].
    fn disconnect(&mut self) -> impl Future<Output = Result<(), TransportError>> + Send;
}
