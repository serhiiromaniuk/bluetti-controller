//! Wire shapes of the encrypted layer (ROADMAP.md §4.4 "Framing"):
//!
//! - kex frame (plaintext): `2A 2A` · type · len · payload · checksum u16 BE,
//!   the wrapping sum of type, len and payload;
//! - temp envelope: plain_len u16 BE · AES-128-CBC ciphertext (IV = temp IV);
//! - session envelope: plain_len u16 BE · seed[4] · AES-256-CBC ciphertext
//!   (IV = `md5(seed)`).
//!
//! Plaintext is zero-padded to whole blocks and cut back to `plain_len`, so
//! [`envelope_len`] gives the total length for reassembly.

use super::{AesKey, BLOCK, CryptoError, TempKey, cbc_decrypt, cbc_encrypt, session_iv};

/// Magic of a key-exchange frame.
pub const KEX_MAGIC: [u8; 2] = [0x2A, 0x2A];
/// Magic, type, len and checksum around a kex payload.
const KEX_OVERHEAD: usize = 6;
const TEMP_HEADER: usize = 2;
const SESSION_HEADER: usize = 6;

/// One decoded key-exchange frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KexFrame {
    pub kind: u8,
    pub payload: Vec<u8>,
}

/// Which wire shape a reassembly buffer is collecting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Kex,
    Temp,
    Session,
}

/// Total wire length of the frame whose first bytes are `prefix`, or `None`
/// until its length field has arrived.
pub fn envelope_len(layer: Layer, prefix: &[u8]) -> Option<usize> {
    match layer {
        Layer::Kex => prefix.get(3).map(|&len| KEX_OVERHEAD + usize::from(len)),
        Layer::Temp | Layer::Session => {
            let header = if layer == Layer::Temp {
                TEMP_HEADER
            } else {
                SESSION_HEADER
            };
            let plain_len = u16::from_be_bytes([*prefix.first()?, *prefix.get(1)?]);
            Some(header + padded_len(plain_len.into()))
        }
    }
}

fn padded_len(plain_len: usize) -> usize {
    plain_len.div_ceil(BLOCK) * BLOCK
}

fn kex_checksum(body: &[u8]) -> u16 {
    body.iter().fold(0u16, |sum, &b| sum.wrapping_add(b.into()))
}

/// Encodes a key-exchange frame; the payload is at most 255 bytes.
pub fn encode_kex(kind: u8, payload: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let len =
        u8::try_from(payload.len()).map_err(|_| CryptoError::PayloadTooLong(payload.len()))?;
    let mut frame = Vec::with_capacity(KEX_OVERHEAD + payload.len());
    frame.extend_from_slice(&KEX_MAGIC);
    frame.push(kind);
    frame.push(len);
    frame.extend_from_slice(payload);
    let sum = kex_checksum(&frame[2..]);
    frame.extend_from_slice(&sum.to_be_bytes());
    Ok(frame)
}

/// Parses one complete key-exchange frame: magic, exact length, checksum.
pub fn parse_kex(frame: &[u8]) -> Result<KexFrame, CryptoError> {
    if frame.len() < 4 {
        return Err(CryptoError::TooShort(frame.len()));
    }
    if frame[..2] != KEX_MAGIC {
        return Err(CryptoError::BadMagic);
    }
    let expected = KEX_OVERHEAD + usize::from(frame[3]);
    if frame.len() != expected {
        return Err(CryptoError::LengthMismatch {
            expected,
            actual: frame.len(),
        });
    }
    let (body, sum) = frame[2..].split_at(expected - 4);
    let actual = u16::from_be_bytes([sum[0], sum[1]]);
    let want = kex_checksum(body);
    if actual != want {
        return Err(CryptoError::BadChecksum {
            expected: want,
            actual,
        });
    }
    Ok(KexFrame {
        kind: frame[2],
        payload: body[2..].to_vec(),
    })
}

/// Wraps `plain` in a temp envelope.
pub fn seal_temp(key: &TempKey, plain: &[u8]) -> Result<Vec<u8>, CryptoError> {
    seal(Vec::new(), AesKey::Aes128(&key.key), &key.iv, plain)
}

/// Unwraps a complete temp envelope.
pub fn open_temp(key: &TempKey, envelope: &[u8]) -> Result<Vec<u8>, CryptoError> {
    open(TEMP_HEADER, AesKey::Aes128(&key.key), &key.iv, envelope)
}

/// Wraps `plain` in a session envelope under a fresh per-frame `seed`.
pub fn seal_session(
    session_key: &[u8; 32],
    seed: [u8; 4],
    plain: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let iv = session_iv(seed);
    seal(seed.to_vec(), AesKey::Aes256(session_key), &iv, plain)
}

/// Unwraps a complete session envelope, taking its IV from the carried seed.
pub fn open_session(session_key: &[u8; 32], envelope: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if envelope.len() < SESSION_HEADER {
        return Err(CryptoError::TooShort(envelope.len()));
    }
    let iv = session_iv([envelope[2], envelope[3], envelope[4], envelope[5]]);
    open(SESSION_HEADER, AesKey::Aes256(session_key), &iv, envelope)
}

/// plain_len · `extra` header bytes · ciphertext of the zero-padded `plain`.
fn seal(
    extra: Vec<u8>,
    key: AesKey<'_>,
    iv: &[u8; BLOCK],
    plain: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let plain_len =
        u16::try_from(plain.len()).map_err(|_| CryptoError::PayloadTooLong(plain.len()))?;
    let mut body = plain.to_vec();
    body.resize(padded_len(plain.len()), 0);
    cbc_encrypt(key, iv, &mut body);
    let mut envelope = plain_len.to_be_bytes().to_vec();
    envelope.extend(extra);
    envelope.extend(body);
    Ok(envelope)
}

/// Checks the length, decrypts, checks the zero padding and cuts it off.
fn open(
    header: usize,
    key: AesKey<'_>,
    iv: &[u8; BLOCK],
    envelope: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if envelope.len() < header {
        return Err(CryptoError::TooShort(envelope.len()));
    }
    let plain_len = usize::from(u16::from_be_bytes([envelope[0], envelope[1]]));
    let expected = header + padded_len(plain_len);
    if envelope.len() != expected {
        return Err(CryptoError::LengthMismatch {
            expected,
            actual: envelope.len(),
        });
    }
    let mut body = envelope[header..].to_vec();
    cbc_decrypt(key, iv, &mut body);
    if body[plain_len..].iter().any(|&b| b != 0) {
        return Err(CryptoError::BadPadding);
    }
    body.truncate(plain_len);
    Ok(body)
}
