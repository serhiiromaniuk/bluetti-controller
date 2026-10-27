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

/// An LE discovery that runs until this is dropped or stopped.
struct Discovery {
    stop: tokio::sync::oneshot::Sender<()>,
    task: tokio::task::JoinHandle<()>,
}

impl Discovery {
    /// Stops the discovery and waits until its token is gone, so the next
    /// start opens a new one instead of reusing this one.
    async fn stop(self) {
        drop(self.stop);
        let _ = self.task.await;
    }
}

/// Starts a discovery on a spawned task. bluer arms its StopDiscovery only
/// once StartDiscovery returns, so a caller cancelled in between (a poll
/// deadline) left the shared session's BlueZ client discovering, and every
/// later start failed "Operation already in progress". The task always runs
/// to the end and stops the discovery when this handle is dropped.
async fn start_discovery(adapter: &::bluer::Adapter) -> anyhow::Result<Discovery> {
    let adapter = adapter.clone();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let (started_tx, started) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        match adapter.discover_devices().await {
            Ok(events) => {
                let _ = started_tx.send(Ok(()));
                let _ = stopped.await;
                drop(events);
            }
            Err(err) => {
                let _ = started_tx.send(Err(err));
            }
        }
    });
    started.await.context("LE discovery task ended")??;
    Ok(Discovery { stop, task })
}

/// `adapter_name`, powered, with an LE-only discovery filter.
async fn le_adapter(adapter_name: &str) -> anyhow::Result<::bluer::Adapter> {
    let session = super::session()
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
    let filter = ::bluer::DiscoveryFilter {
        transport: ::bluer::DiscoveryTransport::Le,
        ..Default::default()
    };
    match adapter.set_discovery_filter(filter).await {
        // A cancelled scan's discovery still stopping: the filter, the same
        // every time, was set on the shared session by an earlier call.
        Err(err) if err.kind == ::bluer::ErrorKind::DiscoveryActive => {}
        result => result?,
    }
    Ok(adapter)
}

async fn listen_for(
    adapter_name: &str,
    address: Address,
    timeout: Duration,
) -> anyhow::Result<Advert> {
    let adapter = le_adapter(adapter_name).await?;
    // Discovery runs while `_session` lives.
    let mut _session = Some(start_discovery(&adapter).await?);
    let device = adapter.device(address)?;
    let until = tokio::time::Instant::now() + timeout;
    let mut last_error = None;
    // Kept apart, so a later "unknown device" error cannot hide it (A29).
    let mut restart_error = None;
    let mut restarts = 0_u32;
    loop {
        // Under `serve` BlueZ ends the LE discovery 0.3–1 s after it starts and
        // does not restart it (phase-4 LOG step 18a), so it is restarted here:
        // dropping the session stops it, a new one starts it again.
        match adapter.is_discovering().await {
            Ok(true) => {}
            Ok(false) => {
                if let Some(discovery) = _session.take() {
                    discovery.stop().await;
                }
                restarts += 1;
                match start_discovery(&adapter).await {
                    Ok(discovery) => _session = Some(discovery),
                    Err(err) => {
                        let text = mask_macs(&err.to_string());
                        tracing::warn!(error = %text, "LE discovery restart failed");
                        restart_error = Some(text);
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
            return Err(not_found(timeout, last_error, restart_error));
        }
        tokio::time::sleep(HEARD_POLL).await;
    }
}

/// The error of a station not heard within `timeout`, naming the last BlueZ
/// error and the last failed discovery restart when there were any.
fn not_found(
    timeout: Duration,
    last_error: Option<String>,
    restart_error: Option<String>,
) -> anyhow::Error {
    let mut text = format!(
        "station not found within {timeout:?}: out of range, or the official app is \
         connected (ROADMAP.md §4.6)"
    );
    if let Some(err) = last_error {
        text.push_str(&format!("; last BlueZ error: {err}"));
    }
    if let Some(err) = restart_error {
        text.push_str(&format!("; LE discovery restart failed: {err}"));
    }
    anyhow!(text)
}

async fn discover(adapter_name: &str, timeout: Duration) -> anyhow::Result<Vec<Advert>> {
    let adapter = le_adapter(adapter_name).await?;
    // Discovery runs while `_discovery` lives; properties are read before it
    // stops, because BlueZ drops the RSSI of devices no longer seen.
    let _discovery = start_discovery(&adapter).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_names_both_the_last_error_and_a_failed_restart() {
        let timeout = Duration::from_secs(15);
        let both = not_found(timeout, Some("unknown".into()), Some("busy".into())).to_string();
        assert!(both.starts_with("station not found within 15s"), "{both}");
        assert!(
            both.ends_with("; last BlueZ error: unknown; LE discovery restart failed: busy"),
            "{both}"
        );
        let none = not_found(timeout, None, None).to_string();
        assert!(none.ends_with("(ROADMAP.md §4.6)"), "{none}");
    }
}
