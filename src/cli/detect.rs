//! `detect`: find one station and print its model, protocol and encryption
//! flag, then connect (running the key exchange when the flag says
//! encrypted), read its identity registers and disconnect (`ROADMAP.md`
//! §4.3, §4.4).

use anyhow::anyhow;
use bluetti_controller::config::{BleConfig, StationConfig};
use bluetti_controller::protocol::crypto::handshake::StaticKeys;
use bluetti_controller::protocol::device::Model;
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::station::{self, Identity};
use bluetti_controller::transport::{self, Advert, Gatt, Link};

use super::scan;

pub async fn run(
    ble: &BleConfig,
    stations: &[StationConfig],
    keys: Option<&StaticKeys>,
    target: &str,
    anonymise: bool,
) -> anyhow::Result<()> {
    let advert = transport::find(ble, resolve(stations, target)).await?;
    println!("{}", line(&advert, anonymise));
    let mut link = transport::open(ble, &advert, keys).await?;
    match link {
        Link::Plain(_) => println!("connected"),
        Link::Encrypted(_) => println!("connected, session established"),
    }
    let policy = ReadPolicy::from_ble(ble);
    let identity = tokio::select! {
        identity = station::detect(&mut link, &advert, &policy) => identity.map_err(Into::into),
        _ = tokio::signal::ctrl_c() => Err(anyhow!("interrupted")),
    };
    // Disconnect before acting on the result, so no path leaves the link up.
    let closed = link.disconnect().await;
    match &identity {
        Ok(identity) => println!("{}", identity_line(identity)),
        Err(err) => println!("identity: none ({err:#})"),
    }
    closed?;
    println!("disconnected");
    identity.map(drop)
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

/// Device type, the layout that answered and each firmware version read.
fn identity_line(identity: &Identity) -> String {
    let firmware: Vec<String> = identity
        .firmware
        .iter()
        .map(|(name, version)| format!("{name}={version}"))
        .collect();
    let firmware = if firmware.is_empty() {
        "not found over BLE".to_owned()
    } else {
        firmware.join(" ")
    };
    format!(
        "device type={}  layout={:?}  firmware: {firmware}",
        identity.device_type, identity.layout
    )
}

#[cfg(test)]
mod tests;
