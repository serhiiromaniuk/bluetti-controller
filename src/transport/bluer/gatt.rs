//! [`BluerGatt`]: the Modbus channel of one station over BlueZ (`ROADMAP.md`
//! §4.1, §4.6). Every D-Bus call runs under a timeout (bluer #182), the
//! connect is retried with backoff, and a failed setup always disconnects
//! (golden rule #5). The caller ends every session with
//! [`Gatt::disconnect`]; nothing holds a link open on its own.

use std::future::Future;
use std::time::Duration;

use ::bluer::gatt::remote::{Characteristic, CharacteristicWriteRequest};
use ::bluer::gatt::{CharacteristicReader, WriteOp};
use ::bluer::{Address, Device, Session, Uuid};
use anyhow::{Context, anyhow};

use super::scan::{advert_of, scan};
use crate::config::BleConfig;
use crate::transport::anonymise::mask_macs;
use crate::transport::{Advert, Gatt, TransportError};

const SERVICE: Uuid = Uuid::from_u128(0x0000ff00_0000_1000_8000_00805f9b34fb);
/// Responses arrive as notifications here.
const NOTIFY: Uuid = Uuid::from_u128(0x0000ff01_0000_1000_8000_00805f9b34fb);
/// Requests are written here.
const WRITE: Uuid = Uuid::from_u128(0x0000ff02_0000_1000_8000_00805f9b34fb);
/// Pause before the first connect retry; doubles on each further retry.
const CONNECT_BACKOFF: Duration = Duration::from_secs(2);

/// A connected station: `ff02` for writes, a subscription to `ff01`.
#[derive(Debug)]
pub struct BluerGatt {
    device: Device,
    write: Characteristic,
    /// `None` once disconnected.
    notify: Option<CharacteristicReader>,
    /// The BlueZ link is still up: set until a D-Bus `Disconnect` succeeds,
    /// so a failed one is retried by the next [`Gatt::disconnect`].
    disconnect_owed: bool,
    request_timeout: Duration,
}

/// Finds the station at `address` (`XX:XX:XX:XX:XX:XX`): BlueZ's device cache
/// first, then a scan of `connect_timeout_secs`.
pub async fn find(ble: &BleConfig, address: &str) -> anyhow::Result<Advert> {
    let parsed = parse_address(address)?;
    let timeout = Duration::from_secs(ble.connect_timeout_secs.get());
    if let Some(advert) = cached(&ble.adapter, parsed, timeout).await? {
        return Ok(advert);
    }
    tracing::debug!("station not in the BlueZ cache, scanning");
    scan(&ble.adapter, timeout)
        .await?
        .into_iter()
        .find(|advert| advert.address == parsed.to_string())
        .ok_or_else(|| {
            anyhow!(
                "station not found within {timeout:?}: out of range, or the official app \
                 is connected (ROADMAP.md §4.6)"
            )
        })
}

/// Connects to `advert`'s station with `connect_attempts` tries, resolves
/// `ff01`/`ff02` and subscribes to `ff01`.
pub async fn connect(ble: &BleConfig, advert: &Advert) -> anyhow::Result<BluerGatt> {
    let address = parse_address(&advert.address)?;
    let connect_timeout = Duration::from_secs(ble.connect_timeout_secs.get());
    let request_timeout = Duration::from_secs(ble.request_timeout_secs.get());
    let session = bounded(request_timeout, "open a D-Bus session", Session::new()).await?;
    let adapter = session
        .adapter(&ble.adapter)
        .with_context(|| format!("no Bluetooth adapter {}", ble.adapter))?;
    let device = adapter.device(address).map_err(ble_error("device"))?;

    let attempts = ble.connect_attempts.get();
    let mut backoff = CONNECT_BACKOFF;
    for attempt in 1..=attempts {
        match bounded(connect_timeout, "connect", device.connect()).await {
            Ok(()) => break,
            Err(err) => {
                // Cancels a Connect BlueZ may still be running.
                disconnect_quietly(&device, request_timeout).await;
                if attempt == attempts {
                    return Err(err).context(format!("connect failed after {attempts} attempts"));
                }
                tracing::warn!(attempt, error = %err, "connect failed, retrying");
                tokio::time::sleep(backoff).await;
                backoff = backoff.saturating_mul(2);
            }
        }
    }
    match open_channel(device.clone(), connect_timeout, request_timeout).await {
        Ok(gatt) => Ok(gatt),
        Err(err) => {
            disconnect_quietly(&device, request_timeout).await;
            Err(err.into())
        }
    }
}

impl Gatt for BluerGatt {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.write_op(data, WriteOp::Command).await
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.write_op(data, WriteOp::Request).await
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let reader = self.notify.as_ref().ok_or(TransportError::NotConnected)?;
        match tokio::time::timeout(timeout, reader.recv()).await {
            Err(_) => Err(TransportError::Timeout(timeout)),
            Ok(Ok(value)) if !value.is_empty() => Ok(value),
            Ok(Ok(_)) => Err(TransportError::Ble(
                "ff01: notification stream closed".into(),
            )),
            Ok(Err(err)) => Err(TransportError::Ble(format!("ff01: {err}"))),
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        // Dropping the reader ends the subscription.
        self.notify = None;
        if self.disconnect_owed {
            bounded(self.request_timeout, "disconnect", self.device.disconnect()).await?;
            self.disconnect_owed = false;
        }
        Ok(())
    }
}

impl BluerGatt {
    async fn write_op(&mut self, data: &[u8], op_type: WriteOp) -> Result<(), TransportError> {
        if self.notify.is_none() {
            return Err(TransportError::NotConnected);
        }
        let request = CharacteristicWriteRequest {
            op_type,
            ..Default::default()
        };
        let call = self.write.write_ext(data, &request);
        bounded(self.request_timeout, "write ff02", call).await
    }
}

/// The advert BlueZ holds for `address`, or `None` when the device is unknown
/// or carries no Bluetti manufacturer data.
async fn cached(
    adapter_name: &str,
    address: Address,
    timeout: Duration,
) -> anyhow::Result<Option<Advert>> {
    let session = bounded(timeout, "open a D-Bus session", Session::new()).await?;
    let adapter = session
        .adapter(adapter_name)
        .with_context(|| format!("no Bluetooth adapter {adapter_name}"))?;
    let known = bounded(timeout, "list devices", adapter.device_addresses()).await?;
    if !known.contains(&address) {
        return Ok(None);
    }
    let device = adapter.device(address).map_err(ble_error("device"))?;
    Ok(bounded(timeout, "read device properties", advert_of(&device)).await?)
}

async fn open_channel(
    device: Device,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> Result<BluerGatt, TransportError> {
    let services = bounded(connect_timeout, "resolve services", device.services()).await?;
    let mut service = None;
    for candidate in services {
        if bounded(request_timeout, "service uuid", candidate.uuid()).await? == SERVICE {
            service = Some(candidate);
            break;
        }
    }
    let service = service.ok_or_else(|| TransportError::Ble("no service ff00".into()))?;
    let (mut write, mut notify) = (None, None);
    let characteristics = bounded(
        request_timeout,
        "characteristics",
        service.characteristics(),
    )
    .await?;
    for characteristic in characteristics {
        match bounded(
            request_timeout,
            "characteristic uuid",
            characteristic.uuid(),
        )
        .await?
        {
            uuid if uuid == NOTIFY => notify = Some(characteristic),
            uuid if uuid == WRITE => write = Some(characteristic),
            _ => {}
        }
    }
    let (Some(write), Some(notify)) = (write, notify) else {
        return Err(TransportError::Ble(
            "service ff00 lacks ff01 or ff02".into(),
        ));
    };
    let reader = bounded(request_timeout, "subscribe to ff01", notify.notify_io()).await?;
    Ok(BluerGatt {
        device,
        write,
        notify: Some(reader),
        disconnect_owed: true,
        request_timeout,
    })
}

async fn disconnect_quietly(device: &Device, timeout: Duration) {
    if let Err(err) = bounded(timeout, "disconnect", device.disconnect()).await {
        tracing::warn!(error = %err, "disconnect failed");
    }
}

fn parse_address(address: &str) -> anyhow::Result<Address> {
    address
        .parse()
        .map_err(|_| anyhow!("`{address}` is not a MAC address (XX:XX:XX:XX:XX:XX)"))
}

/// Runs one BlueZ call under `timeout`.
async fn bounded<T>(
    timeout: Duration,
    what: &str,
    call: impl Future<Output = ::bluer::Result<T>>,
) -> Result<T, TransportError> {
    match tokio::time::timeout(timeout, call).await {
        Ok(result) => result.map_err(ble_error(what)),
        Err(_) => Err(TransportError::Ble(format!(
            "{what}: no answer within {timeout:?}"
        ))),
    }
}

/// A D-Bus object path holds the device address, so the text is MAC-masked
/// (golden rule #3).
fn ble_error(what: &str) -> impl FnOnce(::bluer::Error) -> TransportError + '_ {
    move |err| TransportError::Ble(format!("{what}: {}", mask_macs(&err.to_string())))
}
