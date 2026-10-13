//! BLE transport behind our own [`Gatt`] trait.
//!
//! Core logic reaches a station only through [`Gatt`] and [`Advert`], never
//! through a `bluer` type, so [`replay::ReplayGatt`] stands in for the BlueZ
//! adapter in tests (RULES.md architecture rules, `ROADMAP.md` §11).

use std::future::Future;
use std::time::Duration;

use crate::protocol::crypto::CryptoError;
use crate::protocol::crypto::handshake::HandshakeError;

pub mod anonymise;
pub mod bluer;
pub mod crypto;
pub mod fixture;
pub mod link;
pub mod read;
pub mod record;
pub mod replay;
pub mod station;

pub use self::bluer::gatt::{BluerGatt, connect, find, find_cached, release};
pub use self::bluer::scan::scan;
pub use self::crypto::CryptoGatt;
pub use self::link::{HandshakeFailed, Link, open};

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
    /// A BlueZ call failed or hung; the text holds no MAC address.
    #[error("BLE: {0}")]
    Ble(String),
    #[error("handshake: {0}")]
    Handshake(HandshakeError),
    /// A session envelope that does not open: a wrong key or a corrupt frame.
    #[error("session frame: {0}")]
    Crypto(CryptoError),
    #[error("OS RNG: {0}")]
    Rng(String),
}

/// The Modbus channel of one connected station (`ROADMAP.md` §4.1).
pub trait Gatt {
    /// Writes one request to characteristic `ff02` as a Write Command
    /// (without response).
    fn write(&mut self, data: &[u8]) -> impl Future<Output = Result<(), TransportError>> + Send;

    /// Writes to `ff02` as an ATT Write Request, which the station
    /// acknowledges. Handshake message 2 needs it: a station drops that
    /// message as a Write Command (`ROADMAP.md` §4.4).
    fn write_with_response(
        &mut self,
        data: &[u8],
    ) -> impl Future<Output = Result<(), TransportError>> + Send;

    /// Waits up to `timeout` for the next notification from characteristic `ff01`.
    fn next_notification(
        &mut self,
        timeout: Duration,
    ) -> impl Future<Output = Result<Vec<u8>, TransportError>> + Send;

    /// Closes the connection; every later call fails with
    /// [`TransportError::NotConnected`].
    fn disconnect(&mut self) -> impl Future<Output = Result<(), TransportError>> + Send;
}

/// Runs `body` on `gatt`, then disconnects whatever the body returned: the
/// one always-disconnect path of the CLI and the collector (golden rule #5).
pub async fn with_disconnect<G: Gatt, T>(
    gatt: &mut G,
    body: impl AsyncFnOnce(&mut G) -> T,
) -> (T, Result<(), TransportError>) {
    let result = body(gatt).await;
    (result, gatt.disconnect().await)
}
