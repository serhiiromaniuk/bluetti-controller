use std::num::NonZeroU32;
use std::time::Duration;

use super::*;
use crate::protocol::control::{Output, switches};
use crate::protocol::device::Model;
use crate::protocol::modbus::{FrameError, crc16, read_request};
use crate::transport::fixture::{Dir, Fixture, FixtureAdvert, FrameRecord, encode_hex};
use crate::transport::replay::ReplayGatt;

/// EB3A AC on: register 3007 (0x0BBF) = 1, CRC 0xCA7B (hand-computed in
/// `protocol/modbus/tests.rs`).
const EB3A_AC_ON: [u8; 8] = [0x01, 0x06, 0x0B, 0xBF, 0x00, 0x01, 0x7B, 0xCA];

fn with_crc(body: &[u8]) -> Vec<u8> {
    let mut frame = body.to_vec();
    frame.extend_from_slice(&crc16(body).to_le_bytes());
    frame
}

/// A one-register read response carrying `value`.
fn read_reply(value: u16) -> Vec<u8> {
    let [hi, lo] = value.to_be_bytes();
    with_crc(&[0x01, 0x03, 0x02, hi, lo])
}

fn fixture(frames: &[(Dir, Vec<u8>)]) -> Fixture {
    Fixture {
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
    }
}

fn write(bytes: &[u8]) -> (Dir, Vec<u8>) {
    (Dir::Write, bytes.to_vec())
}

fn notify(bytes: &[u8]) -> (Dir, Vec<u8>) {
    (Dir::Notify, bytes.to_vec())
}

/// EB3A AC on, accepted: the write, its echo split over two notifications,
/// then the read-back of register 48 answering 1. Reused by the CLI `set`
/// tests (PLAN step 9).
pub(crate) fn eb3a_ac_on_fixture() -> Fixture {
    fixture(&[
        write(&EB3A_AC_ON),
        notify(&EB3A_AC_ON[..3]),
        notify(&EB3A_AC_ON[3..]),
        write(&read_request(48, 1)),
        notify(&read_reply(1)),
    ])
}

fn policy() -> ReadPolicy {
    ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(3).unwrap(),
        backoff: Duration::ZERO,
    }
}

fn switch(model: Model, output: Output) -> &'static OutputSwitch {
    switches(model)
        .iter()
        .find(|s| s.output() == output)
        .unwrap()
}

/// The fixture is used up: one more write finds its end.
async fn assert_exhausted(gatt: &mut ReplayGatt) {
    assert!(matches!(
        gatt.write(&read_request(0, 1)).await,
        Err(TransportError::UnexpectedWrite { expected, .. }) if expected == "end of fixture"
    ));
}

#[tokio::test]
async fn echo_and_read_back_return_the_state() {
    let mut gatt = ReplayGatt::new(&eb3a_ac_on_fixture()).unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(state, Ok(true));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn off_writes_zero_and_reads_back_zero() {
    // AC180 DC off: register 2012 (0x07DC) = 0, read back from 2012.
    let request = with_crc(&[0x01, 0x06, 0x07, 0xDC, 0x00, 0x00]);
    let mut gatt = ReplayGatt::new(&fixture(&[
        write(&request),
        notify(&request),
        write(&read_request(2012, 1)),
        notify(&read_reply(0)),
    ]))
    .unwrap();
    let state = set_output(
        &mut gatt,
        switch(Model::Ac180, Output::Dc),
        false,
        &policy(),
    )
    .await;
    assert_eq!(state, Ok(false));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn exception_reply_fails_without_read_back() {
    let mut gatt = ReplayGatt::new(&fixture(&[
        write(&EB3A_AC_ON),
        notify(&with_crc(&[0x01, 0x86, 0x02])),
    ]))
    .unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(state, Err(WriteError::Frame(FrameError::Exception(0x02))));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn echo_of_another_value_fails_without_read_back() {
    let mut gatt = ReplayGatt::new(&fixture(&[
        write(&EB3A_AC_ON),
        notify(&with_crc(&[0x01, 0x06, 0x0B, 0xBF, 0x00, 0x00])),
    ]))
    .unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(state, Err(WriteError::Frame(FrameError::EchoMismatch)));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn read_back_of_the_old_state_is_a_mismatch() {
    let mut gatt = ReplayGatt::new(&fixture(&[
        write(&EB3A_AC_ON),
        notify(&EB3A_AC_ON),
        write(&read_request(48, 1)),
        notify(&read_reply(0)),
    ]))
    .unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(
        state,
        Err(WriteError::ReadBackMismatch {
            wanted: true,
            got: 0
        })
    );
}

#[tokio::test]
async fn read_back_retries_per_policy() {
    // The first read-back gets no reply; the retry reads 1.
    let mut gatt = ReplayGatt::new(&fixture(&[
        write(&EB3A_AC_ON),
        notify(&EB3A_AC_ON),
        write(&read_request(48, 1)),
        write(&read_request(48, 1)),
        notify(&read_reply(1)),
    ]))
    .unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(state, Ok(true));
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn timed_out_write_is_not_retried() {
    // No echo: the write frame must not be sent a second time (it may have landed).
    let mut gatt = ReplayGatt::new(&fixture(&[write(&EB3A_AC_ON)])).unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(
        state,
        Err(WriteError::Transport(TransportError::Timeout(
            Duration::from_secs(5)
        )))
    );
    assert_exhausted(&mut gatt).await;
}

#[tokio::test]
async fn stale_notification_is_drained_before_the_write() {
    let mut gatt = ReplayGatt::new(&fixture(&[
        notify(&read_reply(7)),
        write(&EB3A_AC_ON),
        notify(&EB3A_AC_ON),
        write(&read_request(48, 1)),
        notify(&read_reply(1)),
    ]))
    .unwrap();
    let state = set_output(&mut gatt, switch(Model::Eb3a, Output::Ac), true, &policy()).await;
    assert_eq!(state, Ok(true));
}
