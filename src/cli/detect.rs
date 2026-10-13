//! `detect`: find one station and print its model, protocol and encryption
//! flag, then connect (running the key exchange when the flag says
//! encrypted), read its identity registers and disconnect (`ROADMAP.md`
//! §4.3, §4.4).

use bluetti_controller::config::StationConfig;
use bluetti_controller::protocol::device::Model;
use bluetti_controller::transport::Advert;
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::station::{self, Identity};

use super::{Context, scan, session};

pub async fn run(ctx: &Context<'_>, target: &str) -> anyhow::Result<()> {
    session::run(ctx, target, async |gatt, advert| {
        let identity = station::detect(gatt, advert, &ReadPolicy::from_ble(ctx.ble)).await?;
        println!("{}", identity_line(&identity));
        Ok(())
    })
    .await
}

/// A configured station name maps to its address; anything else is taken as
/// a MAC address and validated by the transport.
pub(super) fn resolve<'a>(stations: &'a [StationConfig], target: &'a str) -> &'a str {
    stations
        .iter()
        .find(|station| station.name == target)
        .map_or(target, |station| station.address.as_str())
}

/// The scan line plus the register layout and the raw encryption flag.
pub(super) fn line(advert: &Advert, anonymise: bool) -> String {
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
pub(super) fn identity_line(identity: &Identity) -> String {
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
