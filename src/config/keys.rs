//! Static keys for encrypted stations: the `keys.toml` file `[crypto] keys_file`
//! points at (schema in `ROADMAP.md` §4.4). Errors name the field, never its
//! value, so no key byte reaches a log (golden rule #3).

use anyhow::{Context, anyhow, bail};

use super::{Config, CryptoConfig};
use crate::protocol::crypto::handshake::StaticKeys;

const FIELDS: [&str; 3] = ["local_aes_key", "host_signing_key", "station_verifying_key"];

impl Config {
    /// The static keys an encrypted station needs; a clear error when the
    /// config has no `[crypto]` section.
    pub fn static_keys(&self) -> anyhow::Result<StaticKeys> {
        self.crypto
            .as_ref()
            .ok_or_else(|| {
                anyhow!(
                    "encrypted station needs `[crypto] keys_file` in the config (ROADMAP.md §4.4)"
                )
            })?
            .load_keys()
    }
}

impl CryptoConfig {
    /// Reads and validates the keys file.
    pub fn load_keys(&self) -> anyhow::Result<StaticKeys> {
        let path = self.keys_file.display();
        let text = std::fs::read_to_string(&self.keys_file)
            .with_context(|| format!("cannot read keys file {path}"))?;
        parse_keys(&text).with_context(|| format!("invalid keys file {path}"))
    }
}

/// Parses `keys.toml` text. A table, not a serde struct: serde and toml errors
/// can quote the offending value.
fn parse_keys(text: &str) -> anyhow::Result<StaticKeys> {
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| {
        let line = e
            .span()
            .map_or(0, |s| text[..s.start].matches('\n').count() + 1);
        anyhow!("not valid TOML (line {line})")
    })?;
    if let Some(key) = table.keys().find(|k| !FIELDS.contains(&k.as_str())) {
        bail!(
            "unknown field `{key}`, expected one of {}",
            FIELDS.join(", ")
        );
    }
    let local_aes_key = hex_field::<16>(&table, FIELDS[0])?;
    let host_signing_key = hex_field::<32>(&table, FIELDS[1])?;
    let station_verifying_key = hex_field::<65>(&table, FIELDS[2])?;
    Ok(StaticKeys::new(
        local_aes_key,
        &host_signing_key,
        &station_verifying_key,
    )?)
}

/// `N` bytes from a hex string of exactly `2 * N` digits, either case.
fn hex_field<const N: usize>(table: &toml::Table, name: &str) -> anyhow::Result<[u8; N]> {
    let text = table
        .get(name)
        .ok_or_else(|| anyhow!("missing field `{name}`"))?
        .as_str()
        .ok_or_else(|| anyhow!("field `{name}` must be a hex string"))?;
    let digits = text.as_bytes();
    // The hexdigit check also rules out the `+` that `from_str_radix` accepts.
    if digits.len() != 2 * N || !digits.iter().all(u8::is_ascii_hexdigit) {
        bail!("field `{name}` must be {} hex digits", 2 * N);
    }
    let mut out = [0; N];
    for (byte, pair) in out.iter_mut().zip(digits.chunks(2)) {
        *byte = u8::from_str_radix(std::str::from_utf8(pair)?, 16)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use std::path::PathBuf;

    use super::*;
    use crate::protocol::crypto::handshake::sim::{SimStation, verifying_point};
    use crate::protocol::crypto::handshake::{Handshake, Step};

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().fold(String::new(), |mut s, b| {
            write!(s, "{b:02x}").unwrap();
            s
        })
    }

    /// The simulator's keys in `keys.toml` form; built at runtime, so no key
    /// literal sits in the source.
    fn sim_keys_toml() -> String {
        format!(
            "local_aes_key = \"{}\"\nhost_signing_key = \"{}\"\nstation_verifying_key = \"{}\"\n",
            hex(&[0x33; 16]),
            hex(&[0x11; 32]),
            hex(&verifying_point(&[0x22; 32])).to_uppercase(),
        )
    }

    fn error_of(text: &str) -> String {
        format!("{:#}", parse_keys(text).unwrap_err())
    }

    #[test]
    fn loaded_keys_complete_a_handshake() {
        let keys = parse_keys(&sim_keys_toml()).unwrap();
        let seed = [1, 2, 3, 4];
        let mut sim = SimStation::new(seed);
        let mut hs = Handshake::new(&keys, &[0x44; 32]).unwrap();
        let Step::Reply { frame: msg2, .. } = hs.on_message(&SimStation::challenge(seed)).unwrap()
        else {
            panic!("expected message 2");
        };
        let (msg3, msg4) = sim.on_reply(&msg2);
        hs.on_message(&msg3).unwrap();
        let Step::Reply { frame: msg5, .. } = hs.on_message(&msg4).unwrap() else {
            panic!("expected message 5");
        };
        let msg6 = sim.on_our_key(&msg5);
        assert!(matches!(hs.on_message(&msg6).unwrap(), Step::Done(_)));
    }

    #[test]
    fn hex_keeps_byte_order() {
        let table: toml::Table = "k = \"000102030405060708090A0b0C0d0E0f\"".parse().unwrap();
        let expected: [u8; 16] = std::array::from_fn(|i| i as u8);
        assert_eq!(hex_field::<16>(&table, "k").unwrap(), expected);
    }

    #[test]
    fn rejects_bad_field_without_quoting_it() {
        let good = sim_keys_toml();
        let aes = hex(&[0x33; 16]);
        for (bad, expected) in [
            (format!("{aes}00"), "`local_aes_key` must be 32 hex digits"),
            (
                format!("0x{}", &aes[2..]),
                "`local_aes_key` must be 32 hex digits",
            ),
            (
                format!("+{}", &aes[1..]),
                "`local_aes_key` must be 32 hex digits",
            ),
        ] {
            let err = error_of(&good.replace(&aes, &bad));
            assert!(err.contains(expected), "{err}");
            assert!(!err.contains(&bad), "{err}");
        }
        let point = hex(&verifying_point(&[0x22; 32])).to_uppercase();
        let off_curve = format!("04{}", "AB".repeat(64));
        let err = error_of(&good.replace(&point, &off_curve));
        assert!(err.contains("invalid key: station_verifying_key"), "{err}");
        assert!(!err.contains(&off_curve[..8]), "{err}");
    }

    #[test]
    fn rejects_unknown_missing_and_mistyped_fields() {
        let good = sim_keys_toml();
        let err = error_of(&format!("{good}extra = \"1\"\n"));
        assert!(err.contains("unknown field `extra`"), "{err}");
        let without = good.lines().skip(1).collect::<Vec<_>>().join("\n");
        assert!(error_of(&without).contains("missing field `local_aes_key`"));
        let text = good.replace(&format!("\"{}\"", hex(&[0x11; 32])), "[17, 17]");
        assert!(error_of(&text).contains("`host_signing_key` must be a hex string"));
    }

    #[test]
    fn syntax_error_gives_line_not_text() {
        let text = format!("{}local_aes_key = \"{}", "# c\n", hex(&[0x33; 16]));
        let err = error_of(&text);
        assert!(err.contains("not valid TOML (line 2)"), "{err}");
        assert!(!err.contains("3333"), "{err}");
    }

    #[test]
    fn missing_crypto_section_or_file_is_a_clear_error() {
        let mut config = super::super::parse(include_str!("../../config.example.toml")).unwrap();
        config.crypto = None;
        let err = format!("{:#}", config.static_keys().unwrap_err());
        assert!(
            err.contains("encrypted station needs `[crypto] keys_file`"),
            "{err}"
        );
        let crypto = CryptoConfig {
            keys_file: PathBuf::from("/nonexistent/keys.toml"),
        };
        let err = format!("{:#}", crypto.load_keys().unwrap_err());
        assert!(
            err.contains("cannot read keys file /nonexistent/keys.toml"),
            "{err}"
        );
    }
}
