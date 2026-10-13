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

impl Connect for BluezConnect {
    type Gatt = Link;

    async fn open(
        &self,
        station: &StationConfig,
        retry: bool,
        stage: &mut Stage,
    ) -> Result<(Link, bool), String> {
        let text = |err: anyhow::Error| mask_macs(&format!("{err:#}"));
        let found = if retry {
            transport::find(&self.ble, &station.address).await
        } else {
            transport::find_cached(&self.ble, &station.address).await
        };
        let advert = found.map_err(text)?;
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
                if err.downcast_ref::<HandshakeFailed>().is_some() {
                    *stage = Stage::Handshake;
                }
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
