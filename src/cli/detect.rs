//! `detect`: find one station and print its model, protocol and encryption
//! flag, then connect, print the header of its first notification and
//! disconnect (`ROADMAP.md` §4.1, §4.4: an encrypted station speaks first).

use std::time::Duration;

use bluetti_controller::config::{BleConfig, StationConfig};
use bluetti_controller::protocol::device::Model;
use bluetti_controller::transport::fixture::encode_hex;
use bluetti_controller::transport::{self, Advert, Gatt};

use super::scan;

pub async fn run(
    ble: &BleConfig,
    stations: &[StationConfig],
    target: &str,
    anonymise: bool,
) -> anyhow::Result<()> {
    let advert = transport::find(ble, resolve(stations, target)).await?;
    println!("{}", line(&advert, anonymise));
    let mut gatt = transport::connect(ble, &advert).await?;
    println!("connected");
    let timeout = Duration::from_secs(ble.request_timeout_secs.get());
    // Disconnect before acting on the result, so no path leaves the link up.
    let first = gatt.next_notification(timeout).await;
    let closed = gatt.disconnect().await;
    match first {
        Ok(bytes) => println!("{}", header(&bytes)),
        Err(err) => println!("first notification: none ({err})"),
    }
    closed?;
    println!("disconnected");
    Ok(())
}

/// A configured station name maps to its address; anything else is taken as
/// a MAC address and validated by the transport.
fn resolve<'a>(stations: &'a [StationConfig], target: &'a str) -> &'a str {
    stations
        .iter()
        .find(|station| station.name == target)
        .map_or(target, |station| station.address.as_str())
}

/// The scan line plus the register layout and the raw encryption flag.
fn line(advert: &Advert, anonymise: bool) -> String {
    let protocol = advert
        .name
        .as_deref()
        .and_then(Model::from_ble_name)
        .map_or("unknown".into(), |model| {
            format!("{:?}", model.profile().protocol)
        });
    let flag = advert
        .manufacturer_data
        .get(..7)
        .map_or("?".into(), String::from_utf8_lossy);
    format!(
        "{}  protocol={protocol}  flag={flag}",
        scan::line(advert, anonymise)
    )
}

/// Length and first 4 bytes only: a kex frame's type and length, no payload.
fn header(bytes: &[u8]) -> String {
    let head = encode_hex(&bytes[..bytes.len().min(4)]);
    format!("first notification: {} bytes, starts {head}", bytes.len())
}

#[cfg(test)]
mod tests;
