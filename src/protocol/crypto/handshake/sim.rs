//! Test-only station side of the handshake, with test-generated keys. Step 10's
//! `SimStation` transport double wraps it.

use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{Signature, SigningKey};
use p256::{PublicKey, SecretKey};

use super::super::{BLOCK, TempKey, encode_kex, open_temp, parse_kex, seal_temp};
use super::{POINT, Session, StaticKeys, raw_point};

const LOCAL_AES_KEY: [u8; BLOCK] = [0x33; BLOCK];
const HOST_SCALAR: [u8; 32] = [0x11; 32];
const STATION_SCALAR: [u8; 32] = [0x22; 32];
const STATION_EPHEMERAL: [u8; 32] = [0x55; 32];

/// Test-generated static keys the simulated station agrees with.
pub(crate) fn keys() -> StaticKeys {
    StaticKeys::new(
        LOCAL_AES_KEY,
        &HOST_SCALAR,
        &verifying_point(&STATION_SCALAR),
    )
    .unwrap()
}

/// SEC1 uncompressed public point of a test signing scalar.
pub(crate) fn verifying_point(scalar: &[u8; 32]) -> [u8; 65] {
    let key = SigningKey::from_slice(scalar).unwrap();
    key.verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .try_into()
        .unwrap()
}

/// A station that answers our messages; it never checks the order itself.
pub(crate) struct SimStation {
    pub(crate) temp: TempKey,
    ephemeral: SecretKey,
    pub(crate) session_key: Option<[u8; 32]>,
}

impl SimStation {
    pub(crate) fn new(seed: [u8; 4]) -> Self {
        Self {
            temp: TempKey::derive(seed, &LOCAL_AES_KEY),
            ephemeral: SecretKey::from_slice(&STATION_EPHEMERAL).unwrap(),
            session_key: None,
        }
    }

    /// Message 1: the challenge carrying the seed.
    pub(crate) fn challenge(seed: [u8; 4]) -> Vec<u8> {
        encode_kex(0x01, &seed).unwrap()
    }

    /// Answers message 2 with message 3 and message 4.
    pub(crate) fn on_reply(&self, msg2: &[u8]) -> (Vec<u8>, Vec<u8>) {
        assert_eq!(parse_kex(msg2).unwrap().payload, self.temp.reply());
        (encode_kex(0x03, &[0x00]).unwrap(), self.station_key())
    }

    /// Message 4, signed with the station's static key.
    pub(crate) fn station_key(&self) -> Vec<u8> {
        let point = raw_point(&self.ephemeral.public_key());
        let station = SigningKey::from_slice(&STATION_SCALAR).unwrap();
        let sig: Signature = station.sign(&[&point[..], &self.temp.iv].concat());
        let body = [&point[..], &sig.to_bytes()[..]].concat();
        seal_temp(&self.temp, &encode_kex(0x04, &body).unwrap()).unwrap()
    }

    /// Checks message 5's host signature, derives the session key, returns message 6.
    pub(crate) fn on_our_key(&mut self, msg5: &[u8]) -> Vec<u8> {
        let frame = parse_kex(&open_temp(&self.temp, msg5).unwrap()).unwrap();
        assert_eq!(frame.kind, 0x05);
        let (point, sig) = frame.payload.split_at(POINT);
        let host = SigningKey::from_slice(&HOST_SCALAR).unwrap();
        host.verifying_key()
            .verify(
                &[point, &self.temp.iv].concat(),
                &Signature::from_slice(sig).unwrap(),
            )
            .unwrap();
        let ours = PublicKey::from_sec1_bytes(&[&[0x04], point].concat()).unwrap();
        self.session_key = Some(self.ephemeral.diffie_hellman(&ours).raw_secret_bytes().0);
        self.temp_frame(0x06, &[0x00])
    }

    /// Any kex frame sealed in this session's temp layer.
    pub(crate) fn temp_frame(&self, kind: u8, payload: &[u8]) -> Vec<u8> {
        seal_temp(&self.temp, &encode_kex(kind, payload).unwrap()).unwrap()
    }

    /// The station's view of the session, once message 5 arrived.
    pub(crate) fn session(&self) -> Session {
        Session {
            key: self.session_key.expect("message 5 not seen"),
        }
    }
}
