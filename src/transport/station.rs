//! Station-level reads over any [`Gatt`]: no `bluer` type appears here, so
//! `tests/` can drive them through [`ReplayGatt`](super::replay::ReplayGatt).

use std::collections::BTreeMap;

use super::Advert;
use super::Gatt;
use super::read::{ReadError, ReadPolicy, read_block, read_profile};
use crate::protocol::device::{self, Metric, Model, Problem, Profile, Protocol};
use crate::protocol::identity::{self, Layout};

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
    let mut last = None;
    for layout in identity::layouts(expected) {
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
    Ok(identity::ascii(&words, layout.swapped))
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
            Ok(words) => versions.push((name, identity::version(&[words[0], words[1]]))),
            Err(err) => tracing::warn!(name, register, error = %err, "firmware unread"),
        }
    }
    versions
}

/// One poll of a profile: what decoded and every field that failed the
/// plausibility check (`ROADMAP.md` §4.3, RULES.md v0.3 gate).
#[derive(Debug, Clone, PartialEq)]
pub struct Poll {
    pub values: BTreeMap<Metric, f64>,
    /// Empty when every field is present and in range.
    pub problems: Vec<Problem>,
}

/// Reads every block of `profile` and decodes it. A failed block fails the
/// poll; a missing or implausible field is named in [`Poll::problems`].
pub async fn poll(
    gatt: &mut impl Gatt,
    profile: &Profile,
    policy: &ReadPolicy,
) -> Result<Poll, ReadError> {
    let registers = read_profile(gatt, profile, policy).await?;
    Ok(Poll {
        values: device::decode(profile, &registers),
        problems: device::problems(profile, &registers),
    })
}

#[cfg(test)]
mod tests;
