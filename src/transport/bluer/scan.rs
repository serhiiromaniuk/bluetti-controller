//! Discovery of Bluetti stations (`ROADMAP.md` §4.1).

use std::time::Duration;

use anyhow::{Context, bail};

use crate::protocol::device::COMPANY_ID;
use crate::transport::Advert;

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
        // No RSSI: cached by BlueZ but not heard in this discovery.
        let Some(rssi) = device.rssi().await? else {
            continue;
        };
        let Some(data) = device
            .manufacturer_data()
            .await?
            .and_then(|mut m| m.remove(&COMPANY_ID))
        else {
            continue;
        };
        adverts.push(Advert {
            address: address.to_string(),
            name: device.name().await?,
            rssi: Some(rssi),
            manufacturer_data: data,
        });
    }
    adverts.sort_by_key(|a| std::cmp::Reverse(a.rssi));
    Ok(adverts)
}
