//! [`BluezConnect`]: the collector's [`Connect`] over the BlueZ transport.
//! It calls only `transport` functions; the BlueZ crate stays in its adapter.

use std::num::NonZeroU32;

use super::Connect;
use crate::config::{BleConfig, StationConfig};
use crate::protocol::crypto::handshake::StaticKeys;
use crate::protocol::device::Model;
use crate::store::Stage;
use crate::transport::anonymise::mask_macs;
use crate::transport::{self, HandshakeFailed, Link};

pub struct BluezConnect {
    pub ble: BleConfig,
    /// Needed for encrypted stations (§4.4).
    pub keys: Option<StaticKeys>,
}

/// The stage a failed `transport::open` stopped at: the handshake when the
/// error has the typed [`HandshakeFailed`] source, else the connect (§5.4).
fn open_stage(err: &anyhow::Error) -> Stage {
    if err.downcast_ref::<HandshakeFailed>().is_some() {
        Stage::Handshake
    } else {
        Stage::Connect
    }
}

impl Connect for BluezConnect {
    type Gatt = Link;

    async fn open(
        &self,
        station: &StationConfig,
        retry: bool,
        stage: &mut Stage,
    ) -> Result<(Link, bool), String> {
        let text = |err: anyhow::Error| mask_macs(&format!("{err:#}"));
        let advert = transport::find(&self.ble, &station.address)
            .await
            .map_err(text)?;
        let advertised = advert.name.as_deref().and_then(Model::from_ble_name);
        if let Some(model) = advertised
            && model != station.model
        {
            return Err(format!(
                "advertises {model:?}, configured as {:?}",
                station.model
            ));
        }
        *stage = Stage::Connect;
        let ble = if retry {
            self.ble.clone()
        } else {
            BleConfig {
                connect_attempts: NonZeroU32::MIN,
                ..self.ble.clone()
            }
        };
        match transport::open(&ble, &advert, self.keys.as_ref()).await {
            Ok(link) => {
                let encrypted = matches!(link, Link::Encrypted(_));
                Ok((link, encrypted))
            }
            Err(err) => {
                *stage = open_stage(&err);
                Err(text(err))
            }
        }
    }

    async fn release(&self, station: &StationConfig) {
        if let Err(err) = transport::release(&self.ble, &station.address).await {
            tracing::warn!(
                station = station.name,
                error = mask_macs(&format!("{err:#}")),
                "disconnect by address failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{Context, anyhow};

    use super::*;
    use crate::transport::TransportError;

    #[test]
    fn a_typed_handshake_error_names_the_handshake_stage() {
        let handshake = anyhow::Error::from(HandshakeFailed(TransportError::NotConnected));
        assert_eq!(open_stage(&handshake), Stage::Handshake);
        let wrapped = Err::<(), _>(HandshakeFailed(TransportError::NotConnected))
            .context("open")
            .unwrap_err();
        assert_eq!(open_stage(&wrapped), Stage::Handshake);
        assert_eq!(open_stage(&anyhow!("connect timed out")), Stage::Connect);
    }
}
