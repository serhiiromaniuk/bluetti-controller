use std::num::NonZeroU32;
use std::time::Duration;

use super::*;
use crate::protocol::modbus::{crc16, read_request};
use crate::transport::fixture::{Dir, Fixture, FixtureAdvert, FrameRecord, encode_hex};
use crate::transport::replay::ReplayGatt;

fn policy() -> ReadPolicy {
    ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(1).unwrap(),
        backoff: Duration::ZERO,
    }
}

fn reply(words: &[u16]) -> String {
    let mut frame = vec![0x01, 0x03, (2 * words.len()) as u8];
    for word in words {
        frame.extend_from_slice(&word.to_be_bytes());
    }
    frame.extend_from_slice(&crc16(&frame).to_le_bytes());
    encode_hex(&frame)
}

/// A Modbus exception reply to function 0x03: illegal data address.
fn exception() -> String {
    let frame = [0x01, 0x83, 0x02];
    let mut bytes = frame.to_vec();
    bytes.extend_from_slice(&crc16(&frame).to_le_bytes());
    encode_hex(&bytes)
}

/// A session: each `(start, count, reply hex)` is a request and its reply.
fn replay(name: &str, exchanges: &[(u16, u16, String)]) -> (ReplayGatt, Advert) {
    let frames = exchanges
        .iter()
        .flat_map(|(start, count, hex)| {
            [
                (Dir::Write, encode_hex(&read_request(*start, *count))),
                (Dir::Notify, hex.clone()),
            ]
        })
        .enumerate()
        .map(|(i, (dir, hex))| FrameRecord {
            t_ms: i as u64,
            dir,
            uuid: dir.uuid().into(),
            hex,
        })
        .collect();
    let fixture = Fixture {
        advert: FixtureAdvert {
            name: name.into(),
            manufacturer_data: encode_hex(b"BLUETTF"),
        },
        ephemeral: None,
        frames,
    };
    (
        ReplayGatt::new(&fixture).unwrap(),
        fixture.advert().unwrap(),
    )
}

/// "EB3A" in V1 order, NUL-padded to 6 registers.
const EB3A_V1: [u16; 6] = [0x4542, 0x3341, 0, 0, 0, 0];

#[tokio::test]
async fn v1_station_reads_type_and_both_firmware_versions() {
    let (mut gatt, advert) = replay(
        "EB3A0000000000000",
        &[
            (10, 6, reply(&EB3A_V1)),
            // 0x0001_0BB8 = 68536 → 685.36; 3000 → 30.00
            (23, 2, reply(&[0x0BB8, 0x0001])),
            (25, 2, reply(&[3000, 0])),
        ],
    );
    let identity = detect(&mut gatt, &advert, &policy()).await.unwrap();
    assert_eq!(
        identity,
        Identity {
            device_type: "EB3A".into(),
            layout: Protocol::V1,
            firmware: vec![("arm", "685.36".into()), ("dsp", "30.00".into())],
        }
    );
}

#[tokio::test]
async fn v1_station_without_a_v1_type_falls_back_to_v2() {
    // "EB3A" byte-swapped, as V2 stores it.
    let swapped = [0x4245, 0x4133, 0x2020, 0, 0, 0];
    let (mut gatt, advert) = replay(
        "EB3A0000000000000",
        &[
            (10, 6, reply(&[0, 0, 0, 0, 0, 0])),
            (110, 6, reply(&swapped)),
            (6175, 2, exception()),
        ],
    );
    let identity = detect(&mut gatt, &advert, &policy()).await.unwrap();
    assert_eq!(identity.device_type, "EB3A");
    assert_eq!(identity.layout, Protocol::V2);
    // An unanswered firmware register is left out, not an error.
    assert!(identity.firmware.is_empty());
}

#[tokio::test]
async fn v2_station_tries_v2_first() {
    let (mut gatt, advert) = replay(
        "E200V20000000000",
        &[
            (110, 6, reply(&[0x4c45, 0x3032, 0x5630, 0x0032, 0, 0])),
            (6175, 2, reply(&[1234, 0])),
        ],
    );
    let identity = detect(&mut gatt, &advert, &policy()).await.unwrap();
    assert_eq!(identity.device_type, "EL200V2");
    assert_eq!(identity.firmware, [("bms", "12.34".to_owned())]);
}

#[tokio::test]
async fn neither_layout_answering_is_an_error() {
    let (mut gatt, advert) = replay(
        "E200V20000000000",
        &[(110, 6, exception()), (10, 6, reply(&[0xFFFF; 6]))],
    );
    let err = detect(&mut gatt, &advert, &policy()).await.unwrap_err();
    assert!(
        matches!(err, DetectError::Read(ReadError::Frame(_))),
        "{err:?}"
    );

    let (mut gatt, advert) = replay(
        "E200V20000000000",
        &[(110, 6, reply(&[0; 6])), (10, 6, reply(&[0; 6]))],
    );
    let err = detect(&mut gatt, &advert, &policy()).await.unwrap_err();
    assert_eq!(err, DetectError::NoDeviceType);
}
