//! Masks personal identifiers before output or commit (RULES.md golden rule
//! #3): MAC addresses in text, serial digits in advertised names, and the
//! serial-number registers in recorded read responses (`ROADMAP.md` §4.3).
//! Pure: no I/O.

use super::fixture::{Dir, Fixture, FixtureError, PLACEHOLDER_ADDRESS, decode_hex, encode_hex};
use crate::protocol::device::{Protocol, split_ble_name};
use crate::protocol::identity::is_serial;
use crate::protocol::modbus::{FrameError, crc16, parse_read_response};

/// `AA:BB:CC:DD:EE:FF`.
const MAC_LEN: usize = 17;
/// The encryption flag `BLUETTI`/`BLUETTE`/`BLUETTF`; what follows is unknown,
/// so a fixture keeps only the flag.
const FLAG_LEN: usize = 7;

/// Every MAC becomes [`PLACEHOLDER_ADDRESS`], written with the separator it
/// had (`:`, `-`, or `_` in a BlueZ object path).
pub fn mask_macs(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let (mut copied, mut i) = (0, 0);
    while i + MAC_LEN <= bytes.len() {
        match mac_separator(&bytes[i..i + MAC_LEN]) {
            Some(sep) => {
                // A match starts with an ASCII byte, so `i` is a char boundary.
                out.push_str(&text[copied..i]);
                out.push_str(&PLACEHOLDER_ADDRESS.replace(':', sep));
                i += MAC_LEN;
                copied = i;
            }
            None => i += 1,
        }
    }
    out.push_str(&text[copied..]);
    out
}

fn mac_separator(window: &[u8]) -> Option<&'static str> {
    let sep = match window[2] {
        b':' => ":",
        b'-' => "-",
        b'_' => "_",
        _ => return None,
    };
    let is_mac = window.iter().enumerate().all(|(i, &b)| {
        if i % 3 == 2 {
            b == window[2]
        } else {
            b.is_ascii_hexdigit()
        }
    });
    is_mac.then_some(sep)
}

/// `<MODEL><serial digits>` keeps its model prefix; every serial digit becomes
/// `0`. A name of unknown model has every digit masked.
pub fn mask_name(name: &str) -> String {
    match split_ble_name(name) {
        Some((_, serial)) => {
            let prefix = &name[..name.len() - serial.len()];
            format!("{prefix}{}", "0".repeat(serial.len()))
        }
        None => name
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect(),
    }
}

fn covers_serial(start: u16, count: u16, protocol: Option<Protocol>) -> bool {
    (0..count).any(|i| start.checked_add(i).is_some_and(|r| is_serial(r, protocol)))
}

/// Zeroes the serial-number registers in a valid read response to the request
/// `(start, count)` and recomputes the CRC, so the frame still parses. Any
/// other frame (exception, bad CRC, wrong length) comes back unchanged.
pub fn mask_response(frame: &[u8], start: u16, count: u16, protocol: Option<Protocol>) -> Vec<u8> {
    let mut out = frame.to_vec();
    if parse_read_response(frame, count).is_err() {
        return out;
    }
    for i in 0..count {
        if start.checked_add(i).is_some_and(|r| is_serial(r, protocol)) {
            let at = 3 + 2 * usize::from(i);
            out[at..at + 2].fill(0);
        }
    }
    let body = out.len() - 2;
    let crc = crc16(&out[..body]);
    out[body..].copy_from_slice(&crc.to_le_bytes());
    out
}

/// `(start, count)` of a "read holding registers" request.
fn read_range(write: &[u8]) -> Option<(u16, u16)> {
    (write.len() == 8 && write[1] == 0x03).then(|| {
        (
            u16::from_be_bytes([write[2], write[3]]),
            u16::from_be_bytes([write[4], write[5]]),
        )
    })
}

/// Masks a recorded session for commit under `tests/frames/`: the advert name,
/// the manufacturer data past the encryption flag, and the serial registers in
/// every reply. Each reply is the run of notifications after a write; it is
/// masked whole, then cut back into the recorded fragment sizes. Both serial
/// layouts are masked for every model, since the encrypted EB3A's layout is
/// unverified. A reply that may hold serial registers but is not one valid
/// frame is refused, and so is every reply after any request that got none:
/// a late reply would be masked against the wrong request.
pub fn anonymise_fixture(fixture: &Fixture) -> Result<Fixture, FixtureError> {
    let mut out = fixture.clone();
    out.advert.name = mask_name(&fixture.advert.name);
    out.advert.manufacturer_data.truncate(2 * FLAG_LEN);

    let mut request = None;
    let mut answered = true;
    let mut stale = false;
    let mut i = 0;
    while i < out.frames.len() {
        if out.frames[i].dir == Dir::Write {
            // Any unanswered request taints: its late reply would pair with a
            // later request, shifting every reply after it onto the wrong range.
            stale |= !answered;
            request = read_range(&decode_hex(&out.frames[i].hex)?);
            answered = false;
            i += 1;
            continue;
        }
        if stale {
            return Err(FixtureError::Unmaskable { index: i });
        }
        answered = true;
        let end = i + out.frames[i..]
            .iter()
            .take_while(|f| f.dir == Dir::Notify)
            .count();
        let group = &mut out.frames[i..end];
        let fragments = group
            .iter()
            .map(|f| decode_hex(&f.hex))
            .collect::<Result<Vec<_>, _>>()?;
        let reply = fragments.concat();
        let unmaskable = FixtureError::Unmaskable { index: i };
        let (start, count) = request.ok_or(unmaskable)?;
        let complete = matches!(
            parse_read_response(&reply, count),
            Ok(_) | Err(FrameError::Exception(_))
        );
        if !complete && covers_serial(start, count, None) {
            return Err(FixtureError::Unmaskable { index: i });
        }
        let masked = mask_response(&reply, start, count, None);
        let mut rest = masked.as_slice();
        for (frame, fragment) in group.iter_mut().zip(&fragments) {
            let (head, tail) = rest.split_at(fragment.len());
            frame.hex = encode_hex(head);
            rest = tail;
        }
        i = end;
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
