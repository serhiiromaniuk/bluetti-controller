//! Discovery of Bluetti stations (`ROADMAP.md` §4.1).

use std::time::Duration;

use anyhow::{Context, bail};

use crate::protocol::device::COMPANY_ID;
use crate::transport::Advert;
use crate::transport::anonymise::mask_macs;

/// Extra time for the D-Bus calls around discovery, so a hung BlueZ call still
/// ends (golden rule #5).
const DBUS_MARGIN: Duration = Duration::from_secs(5);

/// Runs an LE discovery on `adapter` (e.g. `hci0`) for `timeout` and returns
/// every device currently in range that advertises manufacturer data under
/// [`COMPANY_ID`], strongest signal first.
pub async fn scan(adapter: &str, timeout: Duration) -> anyhow::Result<Vec<Advert>> {
    tokio::time::timeout(timeout + DBUS_MARGIN, discover(adapter, timeout))
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

async fn discover(adapter_name: &str, timeout: Duration) -> anyhow::Result<Vec<Advert>> {
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
    // the company id is checked below.
    adapter
        .set_discovery_filter(::bluer::DiscoveryFilter {
            transport: ::bluer::DiscoveryTransport::Le,
            ..Default::default()
        })
        .await?;
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
