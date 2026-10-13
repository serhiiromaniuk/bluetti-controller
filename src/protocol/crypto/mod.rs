//! Encryption layer of `BLUETTF`/`BLUETTE` stations (ROADMAP.md §4.4): AES-CBC
//! with zero padding, the temporary key derived from the station's seed, and
//! the per-frame session IV.
//!
//! Pure: no I/O, no async and no RNG. Seeds and IVs are inputs, so the
//! transport owns all randomness. Ported from `rustytti` (Rust, MIT) and
//! cross-checked against nhurman `bluetti_mqtt` (MIT); see README.md.

use std::fmt;

use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyIvInit};
use aes::{Aes128, Aes256};
use md5::{Digest, Md5};

mod frame;

pub use frame::*;

/// AES block size; envelopes are zero-padded to a multiple of it.
pub const BLOCK: usize = 16;

/// Why an encrypted-layer frame was rejected. Never carries payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CryptoError {
    TooShort(usize),
    BadMagic,
    BadChecksum {
        expected: u16,
        actual: u16,
    },
    LengthMismatch {
        expected: usize,
        actual: usize,
    },
    PayloadTooLong(usize),
    /// Non-zero bytes after `plain_len`: a wrong key or a corrupt frame.
    BadPadding,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(len) => write!(f, "encrypted frame too short: {len} bytes"),
            Self::BadMagic => write!(f, "key-exchange frame without the 2A2A magic"),
            Self::BadChecksum { expected, actual } => {
                write!(
                    f,
                    "bad key-exchange checksum: expected {expected:#06x}, got {actual:#06x}"
                )
            }
            Self::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "encrypted frame length mismatch: expected {expected} bytes, got {actual}"
                )
            }
            Self::PayloadTooLong(len) => write!(f, "payload too long to frame: {len} bytes"),
            Self::BadPadding => write!(f, "non-zero padding (wrong key or corrupt frame)"),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Temporary-layer key material of one session (§4.4 message 2). No `Debug`:
/// it is key material.
pub struct TempKey {
    /// `md5(reverse(seed))`, the CBC IV of every temp envelope.
    pub iv: [u8; BLOCK],
    /// `iv XOR local_aes_key`, the AES-128 key.
    pub key: [u8; BLOCK],
}

impl TempKey {
    /// Derives the temp IV and key from the station's 4-byte seed (message 1).
    pub fn derive(seed: [u8; 4], local_aes_key: &[u8; BLOCK]) -> Self {
        let mut reversed = seed;
        reversed.reverse();
        let iv: [u8; BLOCK] = Md5::digest(reversed).into();
        let key = std::array::from_fn(|i| iv[i] ^ local_aes_key[i]);
        Self { iv, key }
    }

    /// The payload of message 2: IV bytes 8..12.
    pub fn reply(&self) -> [u8; 4] {
        [self.iv[8], self.iv[9], self.iv[10], self.iv[11]]
    }
}

/// CBC IV of a session envelope: `md5(seed)`, seed not reversed.
pub fn session_iv(seed: [u8; 4]) -> [u8; BLOCK] {
    Md5::digest(seed).into()
}

/// AES key of either layer: AES-128 temp, AES-256 session.
pub enum AesKey<'a> {
    Aes128(&'a [u8; 16]),
    Aes256(&'a [u8; 32]),
}

/// CBC-encrypts `buf` in place. Callers pad it to whole blocks first.
fn cbc_encrypt(key: AesKey<'_>, iv: &[u8; BLOCK], buf: &mut [u8]) {
    let (blocks, rest) = buf.as_chunks_mut::<BLOCK>();
    debug_assert!(rest.is_empty(), "CBC input is not block-aligned");
    match key {
        AesKey::Aes128(k) => {
            let mut mode = cbc::Encryptor::<Aes128>::new(k.into(), iv.into());
            blocks.iter_mut().for_each(|b| mode.encrypt_block(b.into()));
        }
        AesKey::Aes256(k) => {
            let mut mode = cbc::Encryptor::<Aes256>::new(k.into(), iv.into());
            blocks.iter_mut().for_each(|b| mode.encrypt_block(b.into()));
        }
    }
}

/// CBC-decrypts `buf` in place; `buf` is whole blocks (callers check the length).
fn cbc_decrypt(key: AesKey<'_>, iv: &[u8; BLOCK], buf: &mut [u8]) {
    let (blocks, rest) = buf.as_chunks_mut::<BLOCK>();
    debug_assert!(rest.is_empty(), "CBC input is not block-aligned");
    match key {
        AesKey::Aes128(k) => {
            let mut mode = cbc::Decryptor::<Aes128>::new(k.into(), iv.into());
            blocks.iter_mut().for_each(|b| mode.decrypt_block(b.into()));
        }
        AesKey::Aes256(k) => {
            let mut mode = cbc::Decryptor::<Aes256>::new(k.into(), iv.into());
            blocks.iter_mut().for_each(|b| mode.decrypt_block(b.into()));
        }
    }
}

#[cfg(test)]
mod tests;
