//! [`open`]: one ready Modbus channel to a station over BlueZ, encrypted or
//! not as its advert says (`ROADMAP.md` §4.4, §12 M2). The wire layer below
//! the encryption is always teed into memory; `--record-wire` saves it.

use std::time::Duration;

use anyhow::{anyhow, bail};

use super::fixture::Fixture;
use super::record::RecordingGatt;
use super::{Advert, BluerGatt, CryptoGatt, Gatt, TransportError, connect, crypto};
use crate::config::BleConfig;
use crate::protocol::crypto::handshake::StaticKeys;
use crate::protocol::device::is_encrypted;

/// The BlueZ channel with its frames teed, as they crossed the air.
pub type Wire = RecordingGatt<BluerGatt>;

/// The key exchange failed after the link came up: a typed source, so a
/// caller tells a `handshake` failure from a `connect` one (§5.4 stages).
#[derive(Debug, thiserror::Error)]
#[error("key exchange: {0}")]
pub struct HandshakeFailed(#[source] pub TransportError);

/// A connected station: plaintext, or behind an established session.
pub enum Link {
    Plain(Wire),
    Encrypted(CryptoGatt<Wire>),
}

impl Link {
    /// Every frame that crossed the air so far; on an encrypted link it also
    /// holds the session's ephemeral secret (gitignored `captures/` only).
    pub fn wire(&self) -> &Fixture {
        match self {
            Link::Plain(wire) => wire.fixture(),
            Link::Encrypted(gatt) => gatt.inner().fixture(),
        }
    }
}

/// Connects to `advert`'s station and, when its flag says encrypted, runs the
/// key exchange. An encrypted station without `keys`, or an unknown flag, is
/// refused before any connection is made.
pub async fn open(
    ble: &BleConfig,
    advert: &Advert,
    keys: Option<&StaticKeys>,
) -> anyhow::Result<Link> {
    let keys = match is_encrypted(&advert.manufacturer_data) {
        None => bail!("unknown encryption flag in the advert, refusing to connect"),
        Some(false) => None,
        Some(true) => Some(keys.ok_or_else(|| {
            anyhow!(
                "the station is encrypted and no keys are loaded: set `[crypto] keys_file` \
                 in the config (ROADMAP.md §4.4)"
            )
        })?),
    };
    // Drawn before connecting, so an RNG failure leaves no link up.
    let ephemeral = keys.map(|_| crypto::ephemeral()).transpose()?;
    let mut wire = RecordingGatt::new(connect(ble, advert).await?, advert);
    let (Some(keys), Some(ephemeral)) = (keys, ephemeral) else {
        return Ok(Link::Plain(wire));
    };
    wire.set_ephemeral(&ephemeral);
    let timeout = Duration::from_secs(ble.request_timeout_secs.get());
    // `establish_with` disconnects on failure.
    Ok(Link::Encrypted(
        CryptoGatt::establish_with(wire, keys, timeout, ephemeral)
            .await
            .map_err(HandshakeFailed)?,
    ))
}

impl Gatt for Link {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        match self {
            Link::Plain(gatt) => gatt.write(data).await,
            Link::Encrypted(gatt) => gatt.write(data).await,
        }
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        match self {
            Link::Plain(gatt) => gatt.write_with_response(data).await,
            Link::Encrypted(gatt) => gatt.write_with_response(data).await,
        }
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        match self {
            Link::Plain(gatt) => gatt.next_notification(timeout).await,
            Link::Encrypted(gatt) => gatt.next_notification(timeout).await,
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        match self {
            Link::Plain(gatt) => gatt.disconnect().await,
            Link::Encrypted(gatt) => gatt.disconnect().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::crypto::handshake::sim;

    fn advert(flag: &[u8]) -> Advert {
        Advert {
            // Invalid on purpose: a refusal must come before parsing it.
            address: "not a mac".into(),
            name: Some("EB3A0000000000000".into()),
            rssi: None,
            manufacturer_data: flag.to_vec(),
        }
    }

    async fn refusal(flag: &[u8], keys: Option<&StaticKeys>) -> String {
        let err = open(&BleConfig::default(), &advert(flag), keys)
            .await
            .err()
            .expect("refused");
        format!("{err:#}")
    }

    #[tokio::test]
    async fn encrypted_station_without_keys_is_refused_before_connecting() {
        let err = refusal(b"BLUETTF", None).await;
        assert!(err.contains("no keys are loaded"), "{err}");
    }

    #[tokio::test]
    async fn unknown_flag_is_refused_before_connecting() {
        let keys = sim::keys();
        let err = refusal(b"XLUETTI", Some(&keys)).await;
        assert!(err.contains("unknown encryption flag"), "{err}");
    }

    #[tokio::test]
    async fn known_flag_with_keys_goes_on_to_connect() {
        // Reaching the address parse proves the flag checks passed.
        let keys = sim::keys();
        for flag in [&b"BLUETTI"[..], b"BLUETTF"] {
            let err = refusal(flag, Some(&keys)).await;
            assert!(err.contains("is not a MAC address"), "{err}");
        }
    }
}
