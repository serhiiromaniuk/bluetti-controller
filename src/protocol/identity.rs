//! Identity registers of each layout and their pure decoders (`ROADMAP.md`
//! §4.3): device type, firmware versions and serial numbers. The reads
//! themselves live in `transport::station`.

use super::device::Protocol;

/// Device type (start, count) and firmware registers of one layout.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub protocol: Protocol,
    pub device_type: (u16, u16),
    /// ASCII bytes are stored low byte first (V2: "byte-swapped").
    pub swapped: bool,
    /// `(name, register)` of each two-register firmware version.
    pub firmware: &'static [(&'static str, u16)],
}

pub const V1: Layout = Layout {
    protocol: Protocol::V1,
    device_type: (10, 6),
    swapped: false,
    firmware: &[("arm", 23), ("dsp", 25)],
};

pub const V2: Layout = Layout {
    protocol: Protocol::V2,
    device_type: (110, 6),
    swapped: true,
    firmware: &[("bms", 6175)],
};

/// Both layouts, the one `expected` names first.
pub fn layouts(expected: Protocol) -> [Layout; 2] {
    match expected {
        Protocol::V1 => [V1, V2],
        Protocol::V2 => [V2, V1],
    }
}

/// Serial-number registers; `None` (layout unknown) means both layouts.
pub fn is_serial(register: u16, protocol: Option<Protocol>) -> bool {
    let v1 = (17..=20).contains(&register);
    let v2 = (116..=119).contains(&register);
    match protocol {
        Some(Protocol::V1) => v1,
        Some(Protocol::V2) => v2,
        None => v1 || v2,
    }
}

/// ASCII text of `words`, NUL/space padding trimmed; `None` unless every
/// remaining byte is printable and at least one is left.
pub fn ascii(words: &[u16], swapped: bool) -> Option<String> {
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
pub fn version(words: &[u16; 2]) -> String {
    let value = u32::from(words[0]) | u32::from(words[1]) << 16;
    format!("{}.{:02}", value / 100, value % 100)
}

#[cfg(test)]
mod tests;
