//! Recorded BLE sessions for replay tests (`ROADMAP.md` §11): an anonymised
//! advert header plus the frames `[{t_ms, dir, uuid, hex}]`, stored as
//! `tests/frames/<model>.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Advert;

/// Fixtures carry no real address; a replayed advert gets this placeholder.
pub const PLACEHOLDER_ADDRESS: &str = "AA:BB:CC:00:00:00";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub advert: FixtureAdvert,
    /// Wire captures only (`--record-wire`): our ephemeral P-256 secret (hex),
    /// so the real-key test can replay the session. Never in `tests/frames/`:
    /// the anonymisers drop it (`ROADMAP.md` §4.4 "Committable artefacts").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<String>,
    pub frames: Vec<FrameRecord>,
}

/// The advert fields `detect` needs: model (from the name) and encryption.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureAdvert {
    pub name: String,
    /// Hex, as stored under the Bluetti company id.
    pub manufacturer_data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRecord {
    /// Milliseconds since the connection opened.
    pub t_ms: u64,
    pub dir: Dir,
    /// Short characteristic id: `ff02` for writes, `ff01` for notifications.
    pub uuid: String,
    pub hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    Write,
    Notify,
}

impl Dir {
    /// The characteristic this direction uses.
    pub fn uuid(self) -> &'static str {
        match self {
            Dir::Write => "ff02",
            Dir::Notify => "ff01",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    #[error("read fixture {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("parse fixture: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid hex {0:?}")]
    Hex(String),
    #[error("frame {index}: {dir:?} on {uuid}, expected {}", dir.uuid())]
    WrongUuid {
        index: usize,
        dir: Dir,
        uuid: String,
    },
    #[error("frame {index}: reply to a serial-number read is not one valid frame, cannot mask it")]
    Unmaskable { index: usize },
}

impl Fixture {
    pub fn load(path: &Path) -> Result<Fixture, FixtureError> {
        let text = std::fs::read_to_string(path).map_err(|source| FixtureError::Io {
            path: path.to_owned(),
            source,
        })?;
        Fixture::from_json(&text)
    }

    pub fn from_json(text: &str) -> Result<Fixture, FixtureError> {
        Ok(serde_json::from_str(text)?)
    }

    /// Writes the fixture as pretty JSON.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json + "\n")
    }

    /// The recorded advert, under [`PLACEHOLDER_ADDRESS`].
    pub fn advert(&self) -> Result<Advert, FixtureError> {
        Ok(Advert {
            address: PLACEHOLDER_ADDRESS.to_owned(),
            name: Some(self.advert.name.clone()),
            rssi: None,
            manufacturer_data: decode_hex(&self.advert.manufacturer_data)?,
        })
    }
}

/// Lowercase hex, no separators.
pub fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn decode_hex(text: &str) -> Result<Vec<u8>, FixtureError> {
    let bad = || FixtureError::Hex(text.to_owned());
    // from_str_radix alone would accept a sign ("+1").
    if !text.len().is_multiple_of(2) || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| bad()))
        .collect()
}

#[cfg(test)]
mod tests;
