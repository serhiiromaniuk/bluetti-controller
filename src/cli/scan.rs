//! `scan`: one LE discovery, one line per station.

use std::time::Duration;

use bluetti_controller::config::BleConfig;
use bluetti_controller::protocol::device::{Model, is_encrypted};
use bluetti_controller::transport::anonymise::{mask_macs, mask_name};
use bluetti_controller::transport::{self, Advert};

pub async fn run(ble: &BleConfig, timeout_secs: u64, anonymise: bool) -> anyhow::Result<()> {
    let adverts = transport::scan(&ble.adapter, Duration::from_secs(timeout_secs)).await?;
    if adverts.is_empty() {
        tracing::warn!("no Bluetti station found in {timeout_secs} s");
    }
    for advert in &adverts {
        println!("{}", line(advert, anonymise));
    }
    Ok(())
}

/// `<address>  <name>  <rssi> dBm  <model>  encrypted=<yes|no|unknown>`.
pub(super) fn line(advert: &Advert, anonymise: bool) -> String {
    let name = advert.name.as_deref().unwrap_or("?");
    let model = Model::from_ble_name(name).map_or("unknown", Model::name);
    let encrypted = match is_encrypted(&advert.manufacturer_data) {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    };
    let rssi = advert.rssi.map_or("?".into(), |r| r.to_string());
    let (address, name) = if anonymise {
        (mask_macs(&advert.address), mask_name(name))
    } else {
        (advert.address.clone(), name.to_string())
    };
    format!("{address}  {name}  {rssi} dBm  {model}  encrypted={encrypted}")
}

#[cfg(test)]
mod tests;
