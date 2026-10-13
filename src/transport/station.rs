//! Station-level reads over any [`Gatt`]: no `bluer` type appears here, so
//! `tests/` can drive them through [`ReplayGatt`](super::replay::ReplayGatt).

use super::Advert;
use super::Gatt;
use super::read::{ReadError, ReadPolicy, read_block};
use crate::protocol::device::{Model, Protocol};

/// What a station says about itself (`ROADMAP.md` §4.3 identity registers).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Device type string, e.g. `EB3A`.
    pub device_type: String,
    /// The register layout that answered with a device type.
    pub layout: Protocol,
    /// `(name, version)` per firmware register; empty when none reads.
    pub firmware: Vec<(&'static str, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DetectError {
    #[error(transparent)]
    Read(#[from] ReadError),
    #[error("no printable device type in the V1 (10-15) or V2 (110-115) registers")]
    NoDeviceType,
}

/// Device type (start, count) and firmware registers of each layout.
struct Layout {
    protocol: Protocol,
    device_type: (u16, u16),
    /// ASCII bytes are stored low byte first (V2: "byte-swapped").
    swapped: bool,
    firmware: &'static [(&'static str, u16)],
}

const V1: Layout = Layout {
    protocol: Protocol::V1,
    device_type: (10, 6),
    swapped: false,
    firmware: &[("arm", 23), ("dsp", 25)],
};

const V2: Layout = Layout {
    protocol: Protocol::V2,
    device_type: (110, 6),
    swapped: true,
    firmware: &[("bms", 6175)],
};

/// Reads the identity registers, trying the layout the advertised model
/// implies first and the other one when that yields no device type: the
/// encrypted EB3A may use the V2 layout (`ROADMAP.md` §4.3). Fails only
/// when neither layout answers with a device type.
pub async fn detect(
    gatt: &mut impl Gatt,
    advert: &Advert,
    policy: &ReadPolicy,
) -> Result<Identity, DetectError> {
    let expected = advert
        .name
        .as_deref()
        .and_then(Model::from_ble_name)
        .map_or(Protocol::V1, |model| model.profile().protocol);
    let order = match expected {
        Protocol::V1 => [V1, V2],
        Protocol::V2 => [V2, V1],
    };
    let mut last = None;
    for layout in order {
        match device_type(gatt, &layout, policy).await {
            Ok(Some(device_type)) => {
                return Ok(Identity {
                    device_type,
                    layout: layout.protocol,
                    firmware: firmware(gatt, &layout, policy).await,
                });
            }
            Ok(None) => tracing::warn!(layout = ?layout.protocol, "no device type"),
            Err(err) => {
                tracing::warn!(layout = ?layout.protocol, error = %err, "device type unread");
                last = Some(err);
            }
        }
    }
    Err(last.map_or(DetectError::NoDeviceType, DetectError::Read))
}

/// The device type, or `None` when the registers hold no printable ASCII.
async fn device_type(
    gatt: &mut impl Gatt,
    layout: &Layout,
    policy: &ReadPolicy,
) -> Result<Option<String>, ReadError> {
    let (start, count) = layout.device_type;
    let words = read_block(gatt, start, count, policy).await?;
    Ok(ascii(&words, layout.swapped))
}

/// Firmware versions the station answers for; one that fails is logged and
/// left out, since no register identifies every model's firmware (§14 Q5).
async fn firmware(
    gatt: &mut impl Gatt,
    layout: &Layout,
    policy: &ReadPolicy,
) -> Vec<(&'static str, String)> {
    let mut versions = Vec::new();
    for &(name, register) in layout.firmware {
        match read_block(gatt, register, 2, policy).await {
            Ok(words) => versions.push((name, version(&words))),
            Err(err) => tracing::warn!(name, register, error = %err, "firmware unread"),
        }
    }
    versions
}

/// ASCII text of `words`, NUL/space padding trimmed; `None` unless every
/// remaining byte is printable and at least one is left.
fn ascii(words: &[u16], swapped: bool) -> Option<String> {
    let bytes: Vec<u8> = words
        .iter()
        .flat_map(|w| {
            let [hi, lo] = w.to_be_bytes();
            if swapped { [lo, hi] } else { [hi, lo] }
        })
        .collect();
    let text = std::str::from_utf8(&bytes).ok()?;
    let text = text.trim_matches(|c| c == '\0' || c == ' ');
    let printable = !text.is_empty() && text.bytes().all(|b| b.is_ascii_graphic() || b == b' ');
    printable.then(|| text.to_owned())
}

/// A version pair, low word first, in hundredths (bt `VersionField`):
/// `[0x0BB8, 0]` → `30.00`. Unverified on our stations.
fn version(words: &[u16]) -> String {
    let value = u32::from(words[0]) | u32::from(words[1]) << 16;
    format!("{}.{:02}", value / 100, value % 100)
}

#[cfg(test)]
mod tests;
