//! Configuration: the one place every config value is read and validated
//! (RULES.md architecture rules). Schema per `ROADMAP.md` §9; see
//! `config.example.toml`. [`load`] fails fast with a clear error on a missing
//! file, an unknown key or an invalid value.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use serde::{Deserialize, Deserializer};

use crate::protocol::device::Model;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub general: GeneralConfig,
    pub storage: StorageConfig,
    pub web: WebConfig,
    #[serde(default)]
    pub ble: BleConfig,
    /// Only encrypted stations need it (phase 3).
    pub crypto: Option<CryptoConfig>,
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub control: ControlConfig,
    #[serde(default, rename = "station")]
    pub stations: Vec<StationConfig>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneralConfig {
    pub poll_interval_secs: NonZeroU64,
    pub timezone: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    pub path: PathBuf,
    pub raw_retention_days: NonZeroU32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebConfig {
    pub listen: SocketAddr,
    /// Bearer token for POST endpoints; `None` disables them.
    pub token: Option<String>,
}

/// BLE adapter and timing (golden rule #5: every GATT operation is bounded).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BleConfig {
    pub adapter: String,
    pub connect_timeout_secs: NonZeroU64,
    pub request_timeout_secs: NonZeroU64,
    pub connect_attempts: NonZeroU32,
}

impl Default for BleConfig {
    /// The `ROADMAP.md` §9 values, for CLI runs without a config file.
    fn default() -> Self {
        Self {
            adapter: "hci0".into(),
            connect_timeout_secs: NonZeroU64::new(15).unwrap(),
            request_timeout_secs: NonZeroU64::new(5).unwrap(),
            connect_attempts: NonZeroU32::new(3).unwrap(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CryptoConfig {
    pub keys_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricsConfig {
    pub grid_voltage_min: f64,
    pub debounce_samples: NonZeroU32,
    pub usable_capacity_factor: f64,
}

/// Device writes (golden rule #2). `enabled` is false unless set explicitly.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StationConfig {
    pub name: String,
    #[serde(deserialize_with = "model_from_str")]
    pub model: Model,
    /// BLE MAC, `XX:XX:XX:XX:XX:XX`. Never logged unmasked.
    pub address: String,
    pub grid_sensor: bool,
}

/// `src/protocol/` stays serde-free, so the model names are mapped here.
fn model_from_str<'de, D: Deserializer<'de>>(de: D) -> Result<Model, D::Error> {
    let name = String::deserialize(de)?;
    match name.as_str() {
        "EB3A" => Ok(Model::Eb3a),
        "AC180" => Ok(Model::Ac180),
        "Elite200V2" => Ok(Model::Elite200V2),
        _ => Err(serde::de::Error::custom(format!(
            "unknown model `{name}`, expected one of EB3A, AC180, Elite200V2"
        ))),
    }
}

/// Reads, parses and validates the config file at `path`.
pub fn load(path: &Path) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read config file {}", path.display()))?;
    parse(&text).with_context(|| format!("invalid config file {}", path.display()))
}

/// Parses and validates config text.
pub fn parse(text: &str) -> anyhow::Result<Config> {
    let config: Config = toml::from_str(text)?;
    config.validate()?;
    Ok(config)
}

impl Config {
    fn validate(&self) -> anyhow::Result<()> {
        let mut names = HashSet::new();
        for station in &self.stations {
            if !names.insert(station.name.as_str()) {
                bail!("duplicate station name `{}`", station.name);
            }
            if !is_mac(&station.address) {
                // The value may be a real address: name the station, not the MAC.
                bail!(
                    "station `{}`: address is not a MAC (XX:XX:XX:XX:XX:XX)",
                    station.name
                );
            }
        }
        Ok(())
    }
}

fn is_mac(text: &str) -> bool {
    let parts: Vec<&str> = text.split(':').collect();
    parts.len() == 6
        && parts
            .iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../config.example.toml");

    fn error_of(text: &str) -> String {
        format!("{:#}", parse(text).unwrap_err())
    }

    #[test]
    fn example_parses() {
        let config = parse(EXAMPLE).unwrap();
        assert_eq!(config.ble, BleConfig::default());
        assert!(!config.control.enabled);
        assert_eq!(config.stations.len(), 3);
        assert_eq!(config.stations[0].model, Model::Eb3a);
        assert_eq!(config.stations[2].model, Model::Elite200V2);
        assert_eq!(config.web.token, None);
    }

    #[test]
    fn control_defaults_to_disabled() {
        let text = EXAMPLE.replace("[control]\nenabled = false", "");
        assert_ne!(text, EXAMPLE);
        assert!(!parse(&text).unwrap().control.enabled);
    }

    #[test]
    fn ble_section_is_optional() {
        let start = EXAMPLE.find("[ble]").unwrap();
        let end = EXAMPLE.find("[crypto]").unwrap();
        let text = format!("{}{}", &EXAMPLE[..start], &EXAMPLE[end..]);
        assert_eq!(parse(&text).unwrap().ble, BleConfig::default());
    }

    #[test]
    fn rejects_bad_mac() {
        let text = EXAMPLE.replace("AA:BB:CC:DD:EE:02", "AA:BB:CC:DD:EE");
        assert!(error_of(&text).contains("station `mobile`: address is not a MAC"));
        let text = EXAMPLE.replace("AA:BB:CC:DD:EE:02", "AA:BB:CC:DD:EE:GG");
        assert!(error_of(&text).contains("not a MAC"));
    }

    #[test]
    fn rejects_unknown_model() {
        let text = EXAMPLE.replace("\"AC180\"", "\"AC200\"");
        assert!(error_of(&text).contains("unknown model `AC200`"));
    }

    #[test]
    fn rejects_zero_timeout_and_attempts() {
        for (from, to) in [
            ("request_timeout_secs = 5", "request_timeout_secs = 0"),
            ("connect_timeout_secs = 15", "connect_timeout_secs = 0"),
            ("connect_attempts = 3", "connect_attempts = 0"),
            ("poll_interval_secs = 60", "poll_interval_secs = 0"),
        ] {
            assert!(
                error_of(&EXAMPLE.replace(from, to)).contains("nonzero"),
                "{to}"
            );
        }
    }

    #[test]
    fn rejects_duplicate_name() {
        let text = EXAMPLE.replace("\"mobile\"", "\"router\"");
        assert!(error_of(&text).contains("duplicate station name `router`"));
    }

    #[test]
    fn rejects_unknown_key() {
        let text = EXAMPLE.replace("grid_sensor = false", "grid_sensor = false\ncolour = 1");
        assert!(error_of(&text).contains("unknown field `colour`"));
    }

    #[test]
    fn rejects_missing_file() {
        let err = load(Path::new("/nonexistent/bluetti.toml")).unwrap_err();
        assert!(format!("{err:#}").contains("cannot read config file /nonexistent/bluetti.toml"));
    }
}
