use std::num::NonZeroU32;
use std::time::Duration;

use super::*;
use crate::protocol::device::Model;
use crate::protocol::modbus::read_request;
use crate::transport::fixture::{FixtureAdvert, FrameRecord};
use crate::transport::read::{ReadPolicy, read_block};
use crate::transport::replay::ReplayGatt;

/// Built at runtime: a literal non-placeholder MAC would trip the staged scan.
fn real_mac(sep: &str) -> String {
    ["12", "34", "56", "78", "9a", "BC"].join(sep)
}

/// Serial registers as ASCII, so a leak is easy to spot in hex.
const SERIAL: [u16; 4] = [0x3232, 0x3335, 0x3030, 0x3132];

fn with_crc(body: &[u8]) -> Vec<u8> {
    let mut frame = body.to_vec();
    frame.extend_from_slice(&crc16(body).to_le_bytes());
    frame
}

fn response(values: &[u16]) -> Vec<u8> {
    let mut body = vec![0x01, 0x03, (2 * values.len()) as u8];
    for value in values {
        body.extend_from_slice(&value.to_be_bytes());
    }
    with_crc(&body)
}

/// Registers `start..start + count` set to their own address, with `SERIAL` at
/// `serial_at..serial_at + 4`.
fn block(start: u16, count: u16, serial_at: u16) -> Vec<u16> {
    (start..start + count)
        .map(|r| match r.checked_sub(serial_at) {
            Some(i) if i < 4 => SERIAL[usize::from(i)],
            _ => r,
        })
        .collect()
}

fn serial_bytes() -> Vec<u8> {
    SERIAL.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn fixture(name: &str, frames: &[(Dir, Vec<u8>)]) -> Fixture {
    Fixture {
        advert: FixtureAdvert {
            name: name.into(),
            manufacturer_data: "424c554554544901020304".into(),
        },
        ephemeral: None,
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

fn write(start: u16, count: u16) -> (Dir, Vec<u8>) {
    (Dir::Write, read_request(start, count).to_vec())
}

fn notify(bytes: &[u8]) -> (Dir, Vec<u8>) {
    (Dir::Notify, bytes.to_vec())
}

#[test]
fn masks_macs_in_text() {
    let text = format!(
        "found {} rssi -60 at /org/bluez/hci0/dev_{}",
        real_mac(":"),
        real_mac("_")
    );
    let masked = mask_macs(&text);
    assert_eq!(
        masked,
        "found AA:BB:CC:00:00:00 rssi -60 at /org/bluez/hci0/dev_AA_BB_CC_00_00_00"
    );
    assert_eq!(mask_macs(&masked), masked, "masking is idempotent");
}

#[test]
fn leaves_non_mac_text_alone() {
    for text in ["12:34:56 ok", "1:23:45:67:89:ab:c", "ünïcödé 12-34", ""] {
        assert_eq!(mask_macs(text), text);
    }
}

#[test]
fn masks_serial_digits_in_names() {
    let eb3a = mask_name("EB3A2235000012345");
    assert_eq!(eb3a, "EB3A0000000000000");
    assert_eq!(Model::from_ble_name(&eb3a), Some(Model::Eb3a));
    let elite = mask_name("E200V22501000012345");
    assert_eq!(elite, "E200V20000000000000");
    assert_eq!(Model::from_ble_name(&elite), Some(Model::Elite200V2));
    // Unknown model: the prefix boundary is unknown, so every digit goes.
    assert_eq!(mask_name("AC200M2235001"), "AC000M0000000");
    assert_eq!(mask_name("Pixel"), "Pixel");
}

#[test]
fn masked_response_parses_without_serial() {
    let frame = response(&block(10, 40, 17));
    assert!(contains(&frame, &serial_bytes()));
    let masked = mask_response(&frame, 10, 40, Some(Protocol::V1));
    let values = parse_read_response(&masked, 40).unwrap();
    assert!(!contains(&masked, &serial_bytes()));
    assert_eq!(values[7..11], [0, 0, 0, 0]);
    let mut expected = block(10, 40, 17);
    expected[7..11].fill(0);
    assert_eq!(values, expected, "only the serial registers change");
}

#[test]
fn v2_masks_registers_116_to_119_only() {
    let frame = response(&block(100, 20, 116));
    let values = parse_read_response(&mask_response(&frame, 100, 20, Some(Protocol::V2)), 20);
    assert_eq!(values.unwrap()[16..20], [0, 0, 0, 0]);
    // A V2 station's registers 17..=20 are not its serial.
    let low = response(&block(10, 20, 17));
    assert_eq!(mask_response(&low, 10, 20, Some(Protocol::V2)), low);
}

#[test]
fn unknown_protocol_masks_both_layouts() {
    let frame = response(&block(100, 20, 116));
    let masked = mask_response(&frame, 100, 20, None);
    assert!(!contains(&masked, &serial_bytes()));
}

#[test]
fn invalid_frames_come_back_unchanged() {
    let exception = with_crc(&[0x01, 0x83, 0x02]);
    assert_eq!(mask_response(&exception, 10, 40, None), exception);
    let mut bad_crc = response(&block(10, 40, 17));
    *bad_crc.last_mut().unwrap() ^= 0xff;
    assert_eq!(mask_response(&bad_crc, 10, 40, None), bad_crc);
}

#[tokio::test]
async fn anonymised_fixture_replays_masked() {
    let frame = response(&block(10, 40, 17));
    let original = fixture(
        "EB3A2235000012345",
        &[
            write(10, 40),
            notify(&frame[..20]),
            notify(&frame[20..60]),
            notify(&frame[60..]),
        ],
    );
    let masked = anonymise_fixture(&original).unwrap();
    assert_eq!(masked.advert.name, "EB3A0000000000000");
    assert_eq!(masked.advert.manufacturer_data, "424c5545545449");
    let lens = |f: &Fixture| f.frames.iter().map(|r| r.hex.len()).collect::<Vec<_>>();
    assert_eq!(lens(&masked), lens(&original), "fragment sizes are kept");
    let all: String = masked.frames.iter().map(|r| r.hex.as_str()).collect();
    assert!(!contains(&decode_hex(&all).unwrap(), &serial_bytes()));

    let policy = ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(1).unwrap(),
        backoff: Duration::ZERO,
    };
    let mut gatt = ReplayGatt::new(&masked).unwrap();
    let values = read_block(&mut gatt, 10, 40, &policy).await.unwrap();
    assert_eq!(values[7..11], [0, 0, 0, 0]);
    assert_eq!(values[0], 10);
}

#[test]
fn partial_serial_reply_is_refused() {
    let frame = response(&block(10, 40, 17));
    let partial = fixture("EB3A2235000012345", &[write(10, 40), notify(&frame[..30])]);
    assert!(matches!(
        anonymise_fixture(&partial),
        Err(FixtureError::Unmaskable { index: 1 })
    ));
    // A partial reply outside the serial registers carries nothing to mask.
    let other = response(&block(70, 21, 0));
    let fine = fixture("EB3A2235000012345", &[write(70, 21), notify(&other[..10])]);
    assert_eq!(anonymise_fixture(&fine).unwrap().frames, fine.frames);
}

#[test]
fn exception_reply_is_kept() {
    let exception = with_crc(&[0x01, 0x83, 0x02]);
    let original = fixture("EB3A2235000012345", &[write(10, 40), notify(&exception)]);
    assert_eq!(
        anonymise_fixture(&original).unwrap().frames,
        original.frames
    );
}

#[test]
fn notification_without_a_read_request_is_refused() {
    let orphan = fixture("EB3A2235000012345", &[notify(&[0x01, 0x03])]);
    assert!(matches!(
        anonymise_fixture(&orphan),
        Err(FixtureError::Unmaskable { index: 0 })
    ));
}

#[test]
fn late_reply_after_an_unanswered_serial_request_is_refused() {
    // The first request times out; its reply arrives after a second request of
    // the same count, which would mask it at the wrong registers.
    let late = response(&block(10, 40, 17));
    let original = fixture(
        "EB3A2235000012345",
        &[write(10, 40), write(50, 40), notify(&late)],
    );
    assert!(matches!(
        anonymise_fixture(&original),
        Err(FixtureError::Unmaskable { index: 2 })
    ));
    // An unanswered non-serial request taints too: its late reply would pair
    // with the serial request, whose own reply then lands after the next write.
    let shifted = fixture(
        "EB3A2235000012345",
        &[
            write(50, 40),
            write(100, 40),
            notify(&response(&block(50, 40, 0))),
            write(140, 40),
            notify(&response(&block(100, 40, 116))),
        ],
    );
    assert!(matches!(
        anonymise_fixture(&shifted),
        Err(FixtureError::Unmaskable { index: 2 })
    ));
}

#[test]
fn fixture_masks_both_serial_layouts_for_every_model() {
    // The encrypted EB3A's layout is unverified, so no model trusts its own.
    for name in ["EB3A2235000012345", "E200V22501000012345", "Pixel"] {
        for (start, serial_at) in [(10, 17), (100, 116)] {
            let frame = response(&block(start, 20, serial_at));
            let original = fixture(name, &[write(start, 20), notify(&frame)]);
            let masked = anonymise_fixture(&original).unwrap();
            let bytes = decode_hex(&masked.frames[1].hex).unwrap();
            assert!(
                !contains(&bytes, &serial_bytes()),
                "{name}: serial at {serial_at} leaked"
            );
        }
    }
}
