use super::sim::{SimStation, keys, verifying_point};
use super::*;

const SEED: [u8; 4] = [0xFF, 0xDF, 0x22, 0xB7];
const EPHEMERAL: [u8; 32] = [0x44; 32];

fn reply(step: Step) -> (Vec<u8>, bool) {
    match step {
        Step::Reply { frame, acked } => (frame, acked),
        _ => panic!("expected a reply"),
    }
}

/// Runs messages 1–3 and returns the handshake waiting for message 4.
fn up_to_station_key<'k>(keys: &'k StaticKeys, sim: &SimStation) -> Handshake<'k> {
    let mut hs = Handshake::new(keys, &EPHEMERAL).unwrap();
    let (msg2, _) = reply(hs.on_message(&SimStation::challenge(SEED)).unwrap());
    let (msg3, _) = sim.on_reply(&msg2);
    assert!(matches!(hs.on_message(&msg3).unwrap(), Step::Wait));
    hs
}

#[test]
fn round_trip_gives_equal_session_keys() {
    let keys = keys();
    let mut sim = SimStation::new(SEED);
    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();

    assert_eq!(hs.expecting(), Layer::Kex);
    let (msg2, acked) = reply(hs.on_message(&SimStation::challenge(SEED)).unwrap());
    assert!(acked, "message 2 must be a Write Request");
    let (msg3, msg4) = sim.on_reply(&msg2);
    assert!(matches!(hs.on_message(&msg3).unwrap(), Step::Wait));
    assert_eq!(hs.expecting(), Layer::Temp);
    let (msg5, acked) = reply(hs.on_message(&msg4).unwrap());
    assert!(!acked, "message 5 is a Write Command");
    let msg6 = sim.on_our_key(&msg5);
    let Step::Done(session) = hs.on_message(&msg6).unwrap() else {
        panic!("expected the session");
    };
    assert_eq!(hs.expecting(), Layer::Session);

    let station = sim.session();
    assert_eq!(session.key, station.key);
    let request = [0x01, 0x03, 0x00, 0x0A, 0x00, 0x06, 0xA4, 0x0A];
    let wire = session.encrypt([1, 2, 3, 4], &request).unwrap();
    assert_eq!(station.decrypt(&wire).unwrap(), request);
    let wire = station.encrypt([5, 6, 7, 8], &request).unwrap();
    assert_eq!(session.decrypt(&wire).unwrap(), request);

    assert_eq!(hs.on_message(&msg6).err(), Some(HandshakeError::Finished));
}

#[test]
fn bad_station_signature_fails() {
    let keys = keys();
    let sim = SimStation::new(SEED);
    let mut hs = up_to_station_key(&keys, &sim);
    let mut body = parse_kex(&open_temp(&sim.temp, &sim.station_key()).unwrap())
        .unwrap()
        .payload;
    body[POINT + SIG - 1] ^= 0x01;
    let err = hs.on_message(&sim.temp_frame(0x04, &body)).err();
    assert_eq!(err, Some(HandshakeError::BadStationSignature));
    // Any error ends the handshake.
    assert_eq!(
        hs.on_message(&sim.station_key()).err(),
        Some(HandshakeError::Finished)
    );
}

#[test]
fn signature_from_another_station_key_fails() {
    let sim = SimStation::new(SEED);
    let other = verifying_point(&[0x66; 32]);
    let keys = StaticKeys::new([0x33; BLOCK], &[0x11; 32], &other).unwrap();
    let mut hs = up_to_station_key(&keys, &sim);
    let err = hs.on_message(&sim.station_key()).err();
    assert_eq!(err, Some(HandshakeError::BadStationSignature));
}

#[test]
fn out_of_order_messages_fail() {
    let keys = keys();
    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();
    let err = hs.on_message(&encode_kex(0x03, &[0x00]).unwrap()).err();
    assert_eq!(
        err,
        Some(HandshakeError::OutOfOrder {
            expected: 0x01,
            got: 0x03
        })
    );

    let sim = SimStation::new(SEED);
    let mut hs = up_to_station_key(&keys, &sim);
    let err = hs.on_message(&sim.temp_frame(0x06, &[0x00])).err();
    assert_eq!(
        err,
        Some(HandshakeError::OutOfOrder {
            expected: 0x04,
            got: 0x06
        })
    );
}

#[test]
fn truncated_frames_fail() {
    let keys = keys();
    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();
    let challenge = SimStation::challenge(SEED);
    let err = hs.on_message(&challenge[..challenge.len() - 1]).err();
    assert!(matches!(
        err,
        Some(HandshakeError::Frame(CryptoError::LengthMismatch { .. }))
    ));

    let sim = SimStation::new(SEED);
    let mut hs = up_to_station_key(&keys, &sim);
    let msg4 = sim.station_key();
    let err = hs.on_message(&msg4[..msg4.len() - 1]).err();
    assert!(matches!(
        err,
        Some(HandshakeError::Frame(CryptoError::LengthMismatch { .. }))
    ));
}

#[test]
fn wrong_payload_length_and_rejection_fail() {
    let keys = keys();
    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();
    let err = hs.on_message(&encode_kex(0x01, &[0; 5]).unwrap()).err();
    assert_eq!(
        err,
        Some(HandshakeError::BadPayloadLength { kind: 1, len: 5 })
    );

    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();
    reply(hs.on_message(&SimStation::challenge(SEED)).unwrap());
    let err = hs.on_message(&encode_kex(0x03, &[0x01]).unwrap()).err();
    assert_eq!(err, Some(HandshakeError::Rejected { kind: 3, status: 1 }));
}

#[test]
fn station_point_off_the_curve_fails() {
    let keys = keys();
    let sim = SimStation::new(SEED);
    let mut hs = up_to_station_key(&keys, &sim);
    // A valid signature over a point that is not on P-256 must still be refused.
    let point = [0x01; POINT];
    let station = SigningKey::from_slice(&[0x22; 32]).unwrap();
    let sig: Signature = station.sign(&[&point[..], &sim.temp.iv].concat());
    let body = [&point[..], &sig.to_bytes()[..]].concat();
    let err = hs.on_message(&sim.temp_frame(0x04, &body)).err();
    assert_eq!(err, Some(HandshakeError::BadStationPoint));
}

#[test]
fn message_two_matches_recorded_vector() {
    // rustytti's captured challenge → reply (§4.4): depends on the seed only.
    let keys = keys();
    let mut hs = Handshake::new(&keys, &EPHEMERAL).unwrap();
    let challenge = [0x2a, 0x2a, 0x01, 0x04, 0xff, 0xdf, 0x22, 0xb7, 0x02, 0xbc];
    let (msg2, _) = reply(hs.on_message(&challenge).unwrap());
    assert_eq!(
        msg2,
        [0x2a, 0x2a, 0x02, 0x04, 0x5a, 0x4f, 0xd3, 0x0e, 0x01, 0x90]
    );
}

#[test]
fn invalid_keys_are_named_and_debug_hides_bytes() {
    let point: [u8; 65] = [0x04; 65];
    let err = StaticKeys::new([0; BLOCK], &[0x11; 32], &point).err();
    assert!(matches!(
        err,
        Some(HandshakeError::InvalidKey("station_verifying_key"))
    ));
    let good = verifying_point(&[0x22; 32]);
    let err = StaticKeys::new([0; BLOCK], &[0; 32], &good).err();
    assert!(matches!(
        err,
        Some(HandshakeError::InvalidKey("host_signing_key"))
    ));
    let keys = keys();
    assert!(matches!(
        Handshake::new(&keys, &[0; 32]).err(),
        Some(HandshakeError::InvalidKey("ephemeral"))
    ));
    let shown = format!("{keys:?}");
    assert_eq!(shown, "StaticKeys { <redacted> }");
    assert!(HandshakeError::InvalidKey("x").to_string().contains('x'));
}

#[test]
fn errors_display_without_payload_bytes() {
    let all = [
        HandshakeError::Frame(CryptoError::BadMagic),
        HandshakeError::OutOfOrder {
            expected: 1,
            got: 3,
        },
        HandshakeError::BadPayloadLength { kind: 4, len: 7 },
        HandshakeError::Rejected { kind: 6, status: 2 },
        HandshakeError::BadStationPoint,
        HandshakeError::BadStationSignature,
        HandshakeError::Finished,
    ];
    for e in all {
        assert!(!e.to_string().is_empty());
    }
}
