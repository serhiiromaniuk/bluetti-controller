//! Discovery of Bluetti stations (`ROADMAP.md` §4.1).

use std::time::Duration;

use ::bluer::Address;
use anyhow::{Context, anyhow, bail};

use crate::protocol::device::COMPANY_ID;
use crate::transport::Advert;
use crate::transport::anonymise::mask_macs;

/// Extra time for the D-Bus calls around discovery, so a hung BlueZ call still
/// ends (golden rule #5).
const DBUS_MARGIN: Duration = Duration::from_secs(5);

/// How often [`scan_for`] checks whether its station has been heard.
const HEARD_POLL: Duration = Duration::from_millis(250);

/// Runs an LE discovery on `adapter` (e.g. `hci0`) for `timeout` and returns
/// every device currently in range that advertises manufacturer data under
/// [`COMPANY_ID`], strongest signal first.
pub async fn scan(adapter: &str, timeout: Duration) -> anyhow::Result<Vec<Advert>> {
    tokio::time::timeout(timeout + DBUS_MARGIN, discover(adapter, timeout))
        .await
        .with_context(|| format!("scan on {adapter} did not finish within {timeout:?}"))?
}

/// Runs an LE discovery on `adapter` until the station at `address` is heard
/// with manufacturer data under [`COMPANY_ID`], at most `timeout`; a "not
/// found" error naming the last BlueZ error when it is not. BlueZ drops a station it no longer hears from its cache ~30 s
/// after a disconnect, so a 60 s poll usually finds it here, not in the cache.
pub(super) async fn scan_for(
    adapter: &str,
    address: Address,
    timeout: Duration,
) -> anyhow::Result<Advert> {
    tokio::time::timeout(timeout + DBUS_MARGIN, listen_for(adapter, address, timeout))
        .await
        .with_context(|| format!("scan on {adapter} did not finish within {timeout:?}"))?
}

/// The advert BlueZ holds for `device`, or `None` when it carries no
/// manufacturer data under [`COMPANY_ID`].
pub(super) async fn advert_of(device: &::bluer::Device) -> ::bluer::Result<Option<Advert>> {
    let Some(data) = device
        .manufacturer_data()
        .await?
        .and_then(|mut m| m.remove(&COMPANY_ID))
    else {
        return Ok(None);
    };
    Ok(Some(Advert {
        address: device.address().to_string(),
        name: device.name().await?,
        rssi: device.rssi().await?,
        manufacturer_data: data,
    }))
}

/// `adapter_name`, powered, with an LE-only discovery filter.
async fn le_adapter(adapter_name: &str) -> anyhow::Result<::bluer::Adapter> {
    let session = ::bluer::Session::new()
        .await
        .context("cannot reach bluetoothd over D-Bus")?;
    let adapter = session
        .adapter(adapter_name)
        .with_context(|| format!("no Bluetooth adapter {adapter_name}"))?;
    if !adapter.is_powered().await? {
        bail!("Bluetooth adapter {adapter_name} is powered off");
    }
    // BlueZ cannot filter on manufacturer data, so the filter is LE-only and
    // the company id is checked by `advert_of`.
    adapter
        .set_discovery_filter(::bluer::DiscoveryFilter {
            transport: ::bluer::DiscoveryTransport::Le,
            ..Default::default()
        })
        .await?;
    Ok(adapter)
}

async fn listen_for(
    adapter_name: &str,
    address: Address,
    timeout: Duration,
) -> anyhow::Result<Advert> {
    let adapter = le_adapter(adapter_name).await?;
    // Discovery runs while `_session` lives.
    let mut _session = Some(adapter.discover_devices().await?);
    let device = adapter.device(address)?;
    let until = tokio::time::Instant::now() + timeout;
    let mut last_error = None;
    let mut restarts = 0_u32;
    loop {
        // Under `serve` BlueZ ends the LE discovery 0.3–1 s after it starts and
        // does not restart it (phase-4 LOG step 18a), so it is restarted here:
        // dropping the session stops it, a new one starts it again.
        match adapter.is_discovering().await {
            Ok(true) => {}
            Ok(false) => {
                _session = None;
                restarts += 1;
                match adapter.discover_devices().await {
                    Ok(stream) => _session = Some(stream),
                    Err(err) => {
                        let text = mask_macs(&err.to_string());
                        tracing::warn!(error = %text, "LE discovery restart failed");
                        last_error = Some(text);
                    }
                }
            }
            Err(err) => {
                let text = mask_macs(&err.to_string());
                tracing::debug!(error = %text, "adapter Discovering not readable");
                last_error = Some(text);
            }
        }
        match advert_of(&device).await {
            // An RSSI means heard in this discovery.
            Ok(Some(advert)) if advert.rssi.is_some() => {
                tracing::debug!(restarts, "station heard");
                return Ok(advert);
            }
            Ok(_) => {}
            // Unknown to BlueZ reads as an error too, so it is kept, not
            // failed on: a real D-Bus failure then shows in the not-found error.
            Err(err) => {
                let text = mask_macs(&err.to_string());
                tracing::debug!(error = %text, "station properties not readable");
                last_error = Some(text);
            }
        }
        if tokio::time::Instant::now() >= until {
            tracing::debug!(restarts, "station not heard");
            let last = last_error.map_or_else(String::new, |e| format!("; last BlueZ error: {e}"));
            return Err(anyhow!(
                "station not found within {timeout:?}: out of range, or the official app \
                 is connected (ROADMAP.md §4.6){last}"
            ));
        }
        tokio::time::sleep(HEARD_POLL).await;
    }
}

async fn discover(adapter_name: &str, timeout: Duration) -> anyhow::Result<Vec<Advert>> {
    let adapter = le_adapter(adapter_name).await?;
    // Discovery runs while `_events` lives; properties are read before it
    // stops, because BlueZ drops the RSSI of devices no longer seen.
    let _events = adapter.discover_devices().await?;
    tokio::time::sleep(timeout).await;

    let mut adverts = Vec::new();
    for address in adapter.device_addresses().await? {
        let device = adapter.device(address)?;
        match advert_of(&device).await {
            // No RSSI: cached by BlueZ but not heard in this discovery.
            Ok(Some(advert)) if advert.rssi.is_some() => adverts.push(advert),
            Ok(_) => {}
            // One device's D-Bus error must not fail the whole scan.
            Err(err) => tracing::debug!(
                error = %mask_macs(&err.to_string()),
                "skipping a device whose properties cannot be read"
            ),
        }
    }
    adverts.sort_by_key(|a| std::cmp::Reverse(a.rssi));
    Ok(adverts)
}
