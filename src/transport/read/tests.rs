use std::num::NonZeroU32;
use std::time::Duration;

use super::*;
use crate::protocol::device::Model;
use crate::protocol::modbus::{crc16, read_request};
use crate::transport::fixture::{Dir, Fixture, FixtureAdvert, FrameRecord, encode_hex};
use crate::transport::replay::ReplayGatt;

fn policy(attempts: u32) -> ReadPolicy {
    ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(attempts).unwrap(),
        backoff: Duration::ZERO,
    }
}

fn with_crc(body: &[u8]) -> Vec<u8> {
    let mut frame = body.to_vec();
    frame.extend_from_slice(&crc16(body).to_le_bytes());
    frame
}

/// A successful read response carrying `values`.
fn response(values: &[u16]) -> Vec<u8> {
    let mut body = vec![0x01, 0x03, (2 * values.len()) as u8];
    for value in values {
        body.extend_from_slice(&value.to_be_bytes());
    }
    with_crc(&body)
}

fn replay(frames: &[(Dir, Vec<u8>)]) -> ReplayGatt {
    let fixture = Fixture {
        advert: FixtureAdvert {
            name: "EB3A0000000000000".into(),
            manufacturer_data: "424c5545545449".into(),
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
    };
    ReplayGatt::new(&fixture).unwrap()
}

fn write(start: u16, count: u16) -> (Dir, Vec<u8>) {
    (Dir::Write, read_request(start, count).to_vec())
}

fn notify(bytes: &[u8]) -> (Dir, Vec<u8>) {
    (Dir::Notify, bytes.to_vec())
}

/// The fixture is used up: one more write finds its end.
async fn assert_exhausted(gatt: &mut ReplayGatt) {
    assert!(matches!(
        gatt.write(&read_request(0, 1)).await,
        Err(TransportError::UnexpectedWrite { expected, .. }) if expected == "end of fixture"
    ));
}

#[tokio::test]
async fn reassembles_three_fragments() {
    let frame = response(&[0x0064, 0x1234]);
    assert_eq!(frame.len(), 9);
    let mut gatt = replay(&[
        write(10, 2),
        notify(&frame[..3]),
        notify(&frame[3..6]),
        notify(&frame[6..]),
    ]);
    let values = read_block(&mut gatt, 10, 2, &policy(1)).await.unwrap();
    assert_eq!(values, vec![100, 0x1234]);
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn exception_stops_at_five_bytes_without_retry() {
    // A 40-register read would wait for 85 bytes; the exception is 5.
    let mut gatt = replay(&[write(10, 40), notify(&with_crc(&[0x01, 0x83, 0x02]))]);
    let err = read_block(&mut gatt, 10, 40, &policy(3)).await.unwrap_err();
    assert_eq!(err, ReadError::Frame(FrameError::Exception(0x02)));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn bad_crc_is_retried() {
    let good = response(&[7]);
    let mut bad = good.clone();
    bad[4] ^= 0xFF;
    let mut gatt = replay(&[write(43, 1), notify(&bad), write(43, 1), notify(&good)]);
    assert_eq!(read_block(&mut gatt, 43, 1, &policy(2)).await, Ok(vec![7]));
}

#[tokio::test]
async fn bad_crc_on_every_attempt_fails() {
    let mut bad = response(&[7]);
    bad[4] ^= 0xFF;
    let mut gatt = replay(&[write(43, 1), notify(&bad), write(43, 1), notify(&bad)]);
    let err = read_block(&mut gatt, 43, 1, &policy(2)).await.unwrap_err();
    assert!(matches!(err, ReadError::Frame(FrameError::BadCrc { .. })));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn timeout_after_bounded_retries() {
    let mut gatt = replay(&[write(43, 1), write(43, 1), write(43, 1)]);
    let err = read_block(&mut gatt, 43, 1, &policy(3)).await.unwrap_err();
    assert_eq!(
        err,
        ReadError::Transport(TransportError::Timeout(Duration::from_secs(5)))
    );
    // Exactly three attempts: no fourth write was sent.
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn retry_drains_a_late_fragment() {
    let good = response(&[7]);
    let mut bad = good.clone();
    bad[4] ^= 0xFF;
    // A stray byte after the bad reply must not prefix the retried reply.
    let mut gatt = replay(&[
        write(43, 1),
        notify(&bad),
        notify(&[0xAA]),
        write(43, 1),
        notify(&good),
    ]);
    assert_eq!(read_block(&mut gatt, 43, 1, &policy(2)).await, Ok(vec![7]));
}

#[tokio::test]
async fn not_connected_is_not_retried() {
    let mut gatt = replay(&[write(43, 1)]);
    gatt.disconnect().await.unwrap();
    assert_eq!(
        read_block(&mut gatt, 43, 1, &policy(3)).await,
        Err(ReadError::Transport(TransportError::NotConnected))
    );
}

#[tokio::test]
async fn reads_every_block_of_a_profile() {
    let profile = Model::Eb3a.profile();
    let mut frames = Vec::new();
    for &(start, count) in profile.blocks {
        let values: Vec<u16> = (start..start + count).collect();
        frames.push(write(start, count));
        frames.push(notify(&response(&values)));
    }
    let mut gatt = replay(&frames);
    let registers = read_profile(&mut gatt, profile, &policy(1)).await.unwrap();
    // Each register holds its own address; 43 is battery % (spec §4.3).
    for register in [10, 43, 49, 70, 90] {
        assert_eq!(registers.get(register), Some(register));
    }
    assert_eq!(registers.get(50), None);
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn profile_read_fails_on_a_failed_block() {
    let profile = Model::Eb3a.profile();
    let (start, count) = profile.blocks[0];
    let values: Vec<u16> = (start..start + count).collect();
    let mut gatt = replay(&[write(start, count), notify(&response(&values))]);
    let err = read_profile(&mut gatt, profile, &policy(1))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        ReadError::Transport(TransportError::UnexpectedWrite { .. })
    ));
}

#[tokio::test(start_paused = true)]
async fn backoff_doubles_between_retries() {
    let mut gatt = replay(&[write(43, 1), write(43, 1), write(43, 1)]);
    let policy = ReadPolicy {
        backoff: Duration::from_secs(1),
        ..policy(3)
    };
    let started = Instant::now();
    let err = read_block(&mut gatt, 43, 1, &policy).await.unwrap_err();
    assert!(matches!(
        err,
        ReadError::Transport(TransportError::Timeout(_))
    ));
    // The replay times out at once, so all elapsed time is backoff: 1 s + 2 s.
    assert_eq!(started.elapsed(), Duration::from_secs(3));
}
