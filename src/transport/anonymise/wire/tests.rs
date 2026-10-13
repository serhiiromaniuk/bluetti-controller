use super::*;
use crate::protocol::crypto::handshake::sim::{self, SimStation};
use crate::protocol::crypto::handshake::{Handshake, Step};
use crate::protocol::modbus::{crc16, read_request};
use crate::transport::fixture::{FixtureAdvert, FrameRecord};

const SEED: [u8; 4] = [0xff, 0xdf, 0x22, 0xb7];
const EPHEMERAL: [u8; 32] = [0x44; 32];
const REQUEST_SEED: [u8; 4] = [9, 8, 7, 6];
const REPLY_SEED: [u8; 4] = [1, 2, 3, 4];

/// The frames of one simulated encrypted session, as a wire capture.
struct Wire {
    msg1: Vec<u8>,
    msg2: Vec<u8>,
    msg3: Vec<u8>,
    msg4: Vec<u8>,
    msg5: Vec<u8>,
    msg6: Vec<u8>,
    request: Vec<u8>,
    reply: Vec<u8>,
}

fn wire() -> Wire {
    let keys = sim::keys();
    let mut station = SimStation::new(SEED);
    let mut us = Handshake::new(&keys, &EPHEMERAL).unwrap();
    let reply = |step| match step {
        Ok(Step::Reply { frame, .. }) => frame,
        _ => panic!("expected a reply"),
    };
    let msg1 = SimStation::challenge(SEED);
    let msg2 = reply(us.on_message(&msg1));
    let (msg3, msg4) = station.on_reply(&msg2);
    assert!(matches!(us.on_message(&msg3), Ok(Step::Wait)));
    let msg5 = reply(us.on_message(&msg4));
    let msg6 = station.on_our_key(&msg5);
    let Ok(Step::Done(session)) = us.on_message(&msg6) else {
        panic!("handshake not done");
    };
    let mut modbus = vec![0x01, 0x03, 0x04, 0x12, 0x34, 0x56, 0x78];
    modbus.extend_from_slice(&crc16(&modbus).to_le_bytes());
    Wire {
        request: session.encrypt(REQUEST_SEED, &read_request(36, 2)).unwrap(),
        reply: station.session().encrypt(REPLY_SEED, &modbus).unwrap(),
        msg1,
        msg2,
        msg3,
        msg4,
        msg5,
        msg6,
    }
}

fn capture(frames: &[(Dir, &[u8])]) -> Fixture {
    Fixture {
        advert: FixtureAdvert {
            name: "EB3A2345678901234".into(),
            manufacturer_data: encode_hex(b"BLUETTF\x01\x02"),
        },
        ephemeral: Some(encode_hex(&EPHEMERAL)),
        frames: frames
            .iter()
            .enumerate()
            .map(|(i, (dir, bytes))| FrameRecord {
                t_ms: 10 * i as u64,
                dir: *dir,
                uuid: dir.uuid().into(),
                hex: encode_hex(bytes),
            })
            .collect(),
    }
}

/// `frame` with every byte from `header` on zeroed.
fn masked(frame: &[u8], header: usize) -> Vec<u8> {
    let mut out = frame.to_vec();
    out[header..].fill(0);
    out
}

fn bytes(fixture: &Fixture) -> Vec<Vec<u8>> {
    fixture
        .frames
        .iter()
        .map(|f| decode_hex(&f.hex).unwrap())
        .collect()
}

#[test]
fn keeps_messages_1_to_3_and_envelope_headers_only() {
    let w = wire();
    let (notify, write) = (Dir::Notify, Dir::Write);
    // Messages 3 + 4 share a notification and 4 straddles into the next; the
    // reply is split, and a partial frame trails.
    let msg34 = [&w.msg3[..], &w.msg4].concat();
    let (head34, tail34) = msg34.split_at(20);
    let (reply_a, reply_b) = w.reply.split_at(10);
    let partial = &w.reply[..9];
    let raw = capture(&[
        (notify, &w.msg1),
        (write, &w.msg2),
        (notify, head34),
        (notify, tail34),
        (write, &w.msg5),
        (write, &w.msg5),
        (notify, &w.msg6),
        (write, &w.request),
        (notify, reply_a),
        (notify, reply_b),
        (notify, partial),
    ]);
    let out = anonymise_wire(&raw).unwrap();

    assert_eq!(out.ephemeral, None);
    assert_eq!(out.advert.name, "EB3A0000000000000");
    assert_eq!(out.advert.manufacturer_data, encode_hex(b"BLUETTF"));
    let masked34 = [&w.msg3[..], &masked(&w.msg4, 2)].concat();
    let masked_reply = masked(&w.reply, 6);
    let expected: Vec<Vec<u8>> = vec![
        w.msg1.clone(),
        w.msg2.clone(),
        masked34[..20].to_vec(),
        masked34[20..].to_vec(),
        masked(&w.msg5, 2),
        masked(&w.msg5, 2),
        masked(&w.msg6, 2),
        masked(&w.request, 6),
        masked_reply[..10].to_vec(),
        masked_reply[10..].to_vec(),
        vec![0; 9],
    ];
    assert_eq!(bytes(&out), expected);
    // The seeds survive: they are envelope headers, not key material.
    assert_eq!(bytes(&out)[7][2..6], REQUEST_SEED);
    let times: Vec<u64> = out.frames.iter().map(|f| f.t_ms).collect();
    assert_eq!(times, (0..11).map(|i| 10 * i).collect::<Vec<_>>());
}

#[test]
fn refuses_a_capture_without_the_key_exchange() {
    let plain = capture(&[
        (Dir::Write, &read_request(36, 2)),
        (Dir::Notify, &[0x01, 0x03, 0x04, 0, 0, 0, 0, 0, 0]),
    ]);
    assert!(matches!(
        anonymise_wire(&plain),
        Err(FixtureError::Unmaskable { index: 0 })
    ));
}

#[test]
fn refuses_a_kex_frame_past_message_3() {
    let w = wire();
    let msg5_in_clear = crate::protocol::crypto::encode_kex(0x05, &[0; 128]).unwrap();
    let raw = capture(&[(Dir::Notify, &msg5_in_clear), (Dir::Write, &w.msg2)]);
    assert!(matches!(
        anonymise_wire(&raw),
        Err(FixtureError::Unmaskable { index: 0 })
    ));
}

#[test]
fn refuses_a_write_that_is_not_one_whole_frame() {
    let w = wire();
    let raw = capture(&[(Dir::Notify, &w.msg1), (Dir::Write, &w.msg2[..5])]);
    assert!(matches!(
        anonymise_wire(&raw),
        Err(FixtureError::Unmaskable { index: 1 })
    ));
}
