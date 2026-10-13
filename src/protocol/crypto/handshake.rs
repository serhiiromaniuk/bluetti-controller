//! Key-exchange state machine of encrypted stations (ROADMAP.md §4.4
//! "Sequence"): station messages in, our replies out, ending in a [`Session`].
//!
//! Pure: the ephemeral P-256 secret and every session seed are inputs, so the
//! transport owns all randomness and all timing (timeouts, the message-5
//! resend). Ported from `rustytti` `crypto.rs` (MIT), cross-checked against
//! nhurman `encryption.py` (MIT); see README.md.

use std::fmt;

use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::ToSec1Point;
use p256::{PublicKey, SecretKey};

use super::{BLOCK, CryptoError, Layer, TempKey, encode_kex, open_session, open_temp, parse_kex};
use super::{seal_session, seal_temp};

/// Raw X‖Y of a P-256 point (SEC1 uncompressed without the `04` tag).
const POINT: usize = 64;
/// Raw r‖s of an ECDSA-SHA256 signature.
const SIG: usize = 64;
/// Status byte of messages 3 and 6 that means "accepted".
const ACCEPTED: u8 = 0x00;

/// The three static values shared by every station (§4.4 "Static values").
/// Its `Debug` prints no key bytes.
pub struct StaticKeys {
    local_aes_key: [u8; BLOCK],
    host_signing_key: SigningKey,
    station_verifying_key: VerifyingKey,
}

impl StaticKeys {
    /// Builds the keys from the `keys.toml` byte forms: a P-256 private scalar
    /// (big-endian) and a 65-byte SEC1 uncompressed point.
    pub fn new(
        local_aes_key: [u8; BLOCK],
        host_signing_key: &[u8; 32],
        station_verifying_key: &[u8; 65],
    ) -> Result<Self, HandshakeError> {
        Ok(Self {
            local_aes_key,
            host_signing_key: SigningKey::from_slice(host_signing_key)
                .map_err(|_| HandshakeError::InvalidKey("host_signing_key"))?,
            station_verifying_key: VerifyingKey::from_sec1_bytes(station_verifying_key)
                .map_err(|_| HandshakeError::InvalidKey("station_verifying_key"))?,
        })
    }
}

impl fmt::Debug for StaticKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticKeys { <redacted> }")
    }
}

/// Why a handshake failed. Never carries key or payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandshakeError {
    Frame(CryptoError),
    /// A message of the wrong type for this point of the sequence.
    OutOfOrder {
        expected: u8,
        got: u8,
    },
    BadPayloadLength {
        kind: u8,
        len: usize,
    },
    /// Message 3 or 6 carried a non-zero status.
    Rejected {
        kind: u8,
        status: u8,
    },
    BadStationPoint,
    BadStationSignature,
    InvalidKey(&'static str),
    /// A message after the handshake ended or failed.
    Finished,
}

impl From<CryptoError> for HandshakeError {
    fn from(e: CryptoError) -> Self {
        Self::Frame(e)
    }
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(e) => write!(f, "handshake frame: {e}"),
            Self::OutOfOrder { expected, got } => {
                write!(
                    f,
                    "handshake out of order: expected message {expected}, got {got}"
                )
            }
            Self::BadPayloadLength { kind, len } => {
                write!(f, "handshake message {kind} has a {len}-byte payload")
            }
            Self::Rejected { kind, status } => {
                write!(
                    f,
                    "station rejected the handshake: message {kind} status {status:#04x}"
                )
            }
            Self::BadStationPoint => write!(f, "station public key is not a P-256 point"),
            Self::BadStationSignature => write!(f, "station signature does not verify"),
            Self::InvalidKey(field) => write!(f, "invalid key: {field}"),
            Self::Finished => write!(f, "handshake already finished"),
        }
    }
}

impl std::error::Error for HandshakeError {}

/// What to do after one station message.
pub enum Step {
    /// Send `frame`. `acked` = as an ATT Write Request (message 2; the station
    /// drops it as a Write Command), else as a Write Command (§4.4).
    Reply { frame: Vec<u8>, acked: bool },
    /// Nothing to send; wait for the next station message.
    Wait,
    /// The handshake is done; Modbus frames go through the session.
    Done(Session),
}

enum State {
    Challenge,
    Accept(TempKey),
    StationKey(TempKey),
    SessionAccept {
        temp: TempKey,
        session_key: [u8; 32],
    },
    Finished,
}

/// One connection's handshake. Feed it every complete station message.
pub struct Handshake<'k> {
    keys: &'k StaticKeys,
    ephemeral: SecretKey,
    state: State,
}

impl<'k> Handshake<'k> {
    /// Starts a handshake with our ephemeral P-256 secret (big-endian scalar,
    /// fresh from the transport's RNG for every connection).
    pub fn new(keys: &'k StaticKeys, ephemeral: &[u8; 32]) -> Result<Self, HandshakeError> {
        Ok(Self {
            keys,
            ephemeral: SecretKey::from_slice(ephemeral)
                .map_err(|_| HandshakeError::InvalidKey("ephemeral"))?,
            state: State::Challenge,
        })
    }

    /// The wire shape the next station message has, for reassembly
    /// ([`super::envelope_len`]).
    pub fn expecting(&self) -> Layer {
        match self.state {
            State::Challenge | State::Accept(_) => Layer::Kex,
            State::StationKey(_) | State::SessionAccept { .. } => Layer::Temp,
            State::Finished => Layer::Session,
        }
    }

    /// Consumes one complete station message. Any error ends the handshake.
    pub fn on_message(&mut self, msg: &[u8]) -> Result<Step, HandshakeError> {
        let state = std::mem::replace(&mut self.state, State::Finished);
        let (next, step) = self.advance(state, msg)?;
        self.state = next;
        Ok(step)
    }

    fn advance(&self, state: State, msg: &[u8]) -> Result<(State, Step), HandshakeError> {
        match state {
            State::Challenge => {
                let frame = parse_kex(msg)?;
                let seed = kex(&frame, 0x01, 4)?.try_into().expect("len checked");
                let temp = TempKey::derive(seed, &self.keys.local_aes_key);
                let frame = encode_kex(0x02, &temp.reply())?;
                Ok((State::Accept(temp), Step::Reply { frame, acked: true }))
            }
            State::Accept(temp) => {
                status(&parse_kex(msg)?, 0x03)?;
                Ok((State::StationKey(temp), Step::Wait))
            }
            State::StationKey(temp) => {
                let frame = parse_kex(&open_temp(&temp, msg)?)?;
                let session_key = self.station_key(&temp, kex(&frame, 0x04, POINT + SIG)?)?;
                let frame = seal_temp(&temp, &encode_kex(0x05, &self.our_key(&temp))?)?;
                let next = State::SessionAccept { temp, session_key };
                Ok((
                    next,
                    Step::Reply {
                        frame,
                        acked: false,
                    },
                ))
            }
            State::SessionAccept { temp, session_key } => {
                status(&parse_kex(&open_temp(&temp, msg)?)?, 0x06)?;
                Ok((State::Finished, Step::Done(Session { key: session_key })))
            }
            State::Finished => Err(HandshakeError::Finished),
        }
    }

    /// Verifies message 4's signature over point ‖ temp IV, then runs ECDH.
    fn station_key(&self, temp: &TempKey, body: &[u8]) -> Result<[u8; 32], HandshakeError> {
        let (point, sig) = body.split_at(POINT);
        let sig = Signature::from_slice(sig).map_err(|_| HandshakeError::BadStationSignature)?;
        self.keys
            .station_verifying_key
            .verify(&[point, &temp.iv].concat(), &sig)
            .map_err(|_| HandshakeError::BadStationSignature)?;
        let station = PublicKey::from_sec1_bytes(&[&[0x04], point].concat())
            .map_err(|_| HandshakeError::BadStationPoint)?;
        Ok(self.ephemeral.diffie_hellman(&station).raw_secret_bytes().0)
    }

    /// Message 5's payload: our point ‖ host signature over point ‖ temp IV.
    fn our_key(&self, temp: &TempKey) -> Vec<u8> {
        let point = raw_point(&self.ephemeral.public_key());
        let sig: Signature = self
            .keys
            .host_signing_key
            .sign(&[&point[..], &temp.iv].concat());
        [&point[..], &sig.to_bytes()[..]].concat()
    }
}

/// Raw X‖Y of a public key.
fn raw_point(key: &PublicKey) -> [u8; POINT] {
    key.to_sec1_point(false).as_bytes()[1..]
        .try_into()
        .expect("uncompressed P-256 point is 65 bytes")
}

/// Checks a kex frame's type and payload length; returns the payload.
fn kex(frame: &super::KexFrame, kind: u8, len: usize) -> Result<&[u8], HandshakeError> {
    if frame.kind != kind {
        return Err(HandshakeError::OutOfOrder {
            expected: kind,
            got: frame.kind,
        });
    }
    if frame.payload.len() != len {
        return Err(HandshakeError::BadPayloadLength {
            kind,
            len: frame.payload.len(),
        });
    }
    Ok(&frame.payload)
}

/// Checks an accept message (3 or 6): type, one byte, status `00`.
fn status(frame: &super::KexFrame, kind: u8) -> Result<(), HandshakeError> {
    match kex(frame, kind, 1)?[0] {
        ACCEPTED => Ok(()),
        status => Err(HandshakeError::Rejected { kind, status }),
    }
}

/// An established session: AES-256 over the raw ECDH x-coordinate. No `Debug`:
/// it is key material.
pub struct Session {
    key: [u8; 32],
}

impl Session {
    /// Wraps a Modbus frame under a fresh per-frame `seed` from the transport.
    pub fn encrypt(&self, seed: [u8; 4], modbus: &[u8]) -> Result<Vec<u8>, CryptoError> {
        seal_session(&self.key, seed, modbus)
    }

    /// Unwraps one complete session envelope into a Modbus frame.
    pub fn decrypt(&self, envelope: &[u8]) -> Result<Vec<u8>, CryptoError> {
        open_session(&self.key, envelope)
    }
}

#[cfg(test)]
pub(crate) mod sim;
#[cfg(test)]
mod tests;
