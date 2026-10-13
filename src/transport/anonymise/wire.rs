//! [`anonymise_wire`]: masks a `--record-wire` capture of an encrypted station
//! down to what `ROADMAP.md` §4.4 "Committable artefacts" allows.

use super::mask_header;
use crate::protocol::crypto::{Layer, envelope_len, parse_kex};
use crate::transport::fixture::{Dir, Fixture, FixtureError, decode_hex, encode_hex};

/// Station messages before the session: 1 and 3 are kex frames, 4 and 6 temp
/// envelopes.
const HANDSHAKE: [Layer; 4] = [Layer::Kex, Layer::Kex, Layer::Temp, Layer::Temp];
/// The plaintext kex frames kept verbatim: messages 1, 2 and 3.
const PLAIN_KINDS: [u8; 3] = [0x01, 0x02, 0x03];

/// Keeps messages 1–3 verbatim and the header of every envelope (plain_len,
/// plus the seed of a session envelope); every ciphertext byte becomes zero,
/// at the same length. Station messages are reassembled across
/// notifications, masked whole, then cut back into the recorded fragments; a
/// trailing partial message is zeroed. The ephemeral secret is dropped. A
/// capture that does not open with the key exchange is refused.
pub fn anonymise_wire(fixture: &Fixture) -> Result<Fixture, FixtureError> {
    let mut out = mask_header(fixture);
    let (mut stream, mut masked) = (Vec::new(), Vec::new());
    let (mut station, mut writes) = (0, 0);
    for (index, frame) in out.frames.iter_mut().enumerate() {
        let bytes = decode_hex(&frame.hex)?;
        let unmaskable = || FixtureError::Unmaskable { index };
        match frame.dir {
            Dir::Notify => {
                stream.extend_from_slice(&bytes);
                loop {
                    let layer = station_layer(station);
                    let rest = &stream[masked.len()..];
                    let Some(len) = envelope_len(layer, rest).filter(|&len| len <= rest.len())
                    else {
                        break;
                    };
                    masked.extend(mask(layer, &rest[..len]).ok_or_else(unmaskable)?);
                    station += 1;
                }
            }
            Dir::Write => {
                // Message 2, then message 5 (and its resend) until message 6
                // is in, then session frames.
                let layer = match (writes, station) {
                    (0, _) => Layer::Kex,
                    (_, ..4) => Layer::Temp,
                    _ => Layer::Session,
                };
                writes += 1;
                if envelope_len(layer, &bytes) != Some(bytes.len()) {
                    return Err(unmaskable());
                }
                frame.hex = encode_hex(&mask(layer, &bytes).ok_or_else(unmaskable)?);
            }
        }
    }
    masked.resize(stream.len(), 0);
    let mut rest = masked.as_slice();
    for frame in out.frames.iter_mut().filter(|f| f.dir == Dir::Notify) {
        let (head, tail) = rest.split_at(frame.hex.len() / 2);
        frame.hex = encode_hex(head);
        rest = tail;
    }
    Ok(out)
}

fn station_layer(station: usize) -> Layer {
    HANDSHAKE.get(station).copied().unwrap_or(Layer::Session)
}

/// One complete frame of `layer`, masked; `None` for a kex frame that is not
/// one of messages 1–3.
fn mask(layer: Layer, frame: &[u8]) -> Option<Vec<u8>> {
    let header = match layer {
        Layer::Kex => {
            let kex = parse_kex(frame).ok()?;
            return PLAIN_KINDS.contains(&kex.kind).then(|| frame.to_vec());
        }
        Layer::Temp => 2,
        Layer::Session => 6,
    };
    let mut out = frame.to_vec();
    out[header..].fill(0);
    Some(out)
}

#[cfg(test)]
mod tests;
