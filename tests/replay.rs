//! Replay of one recorded `read` session per station (`ROADMAP.md` §11):
//! `detect` then `poll` through [`ReplayGatt`], exactly as `read` runs them.
//! The fixtures hold the plaintext layer (`--record`, above `CryptoGatt`),
//! masked by `anonymise`, so no key is needed. Expected values are hand-read
//! from the fixture frames.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;
use std::time::Duration;

use bluetti_controller::protocol::device::{Metric, Model, Protocol};
use bluetti_controller::transport::fixture::Fixture;
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::replay::ReplayGatt;
use bluetti_controller::transport::station::{self, Identity};
use bluetti_controller::transport::{Gatt, TransportError};

/// One try, no backoff: a replayed reply is either there or not.
fn policy() -> ReadPolicy {
    ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(1).unwrap(),
        backoff: Duration::ZERO,
    }
}

/// Runs the `read` session over `tests/frames/<name>.json` and checks that
/// it consumed every recorded frame.
async fn replay_read(name: &str, model: Model) -> (Identity, station::Poll) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/frames/{name}.json"));
    let fixture = Fixture::load(&path).unwrap();
    let advert = fixture.advert().unwrap();
    assert_eq!(
        advert.name.as_deref().and_then(Model::from_ble_name),
        Some(model)
    );
    let mut gatt = ReplayGatt::new(&fixture).unwrap();
    let identity = station::detect(&mut gatt, &advert, &policy())
        .await
        .unwrap();
    let poll = station::poll(&mut gatt, model.profile(), &policy())
        .await
        .unwrap();
    match gatt.write(&[0]).await {
        Err(TransportError::UnexpectedWrite { expected, .. }) => {
            assert_eq!(expected, "end of fixture");
        }
        other => panic!("fixture not consumed: {other:?}"),
    }
    (identity, poll)
}

#[tokio::test]
async fn eb3a_read_session_replays() {
    let (identity, poll) = replay_read("eb3a", Model::Eb3a).await;
    assert_eq!(
        identity,
        Identity {
            device_type: "EB3A".into(),
            layout: Protocol::V1,
            // 23-24 = 0x5a36, 3 → 0x3_5a36 = 219702; 25-26 = 0x59d1, 3 → 219601
            firmware: vec![("arm", "2197.02".into()), ("dsp", "2196.01".into())],
        }
    );
    // Block 10-49: 43 = 90, 48 = 1, the rest of the fields 0; block 70-90:
    // 77 and 86 are 0 (no input during the recording).
    let expected = BTreeMap::from([
        (Metric::DcInputPower, 0.0),
        (Metric::AcInputPower, 0.0),
        (Metric::AcOutputPower, 0.0),
        (Metric::DcOutputPower, 0.0),
        (Metric::BatterySoc, 90.0),
        (Metric::AcOutputOn, 1.0),
        (Metric::DcOutputOn, 0.0),
        (Metric::AcInputVoltage, 0.0),
        (Metric::DcInputVoltage, 0.0),
    ]);
    assert_eq!(poll.values, expected);
    assert_eq!(poll.problems, []);
}

#[tokio::test]
async fn elite_200_v2_read_session_replays() {
    let (identity, poll) = replay_read("elite200v2", Model::Elite200V2).await;
    assert_eq!(
        identity,
        Identity {
            // 110-115 byte-swapped: "lE" "ti" " e" "02" " 0" "2V"
            device_type: "Elite 200 V2".into(),
            layout: Protocol::V2,
            // 6175-6176 = 0xa13b, 1 → 0x1_a13b = 106811
            firmware: vec![("bms", "1068.11".into())],
        }
    );
    // 100 = 3931 × 0.01, 101 = 99 × 0.1, 102 = 69, 142 = 408; 140, 144,
    // 146, 1300 and 1314 are 0 (on battery, no input).
    let expected = BTreeMap::from([
        (Metric::PackVoltage, 39.31),
        (Metric::PackCurrent, 9.9),
        (Metric::BatterySoc, 69.0),
        (Metric::DcOutputPower, 0.0),
        (Metric::AcOutputPower, 408.0),
        (Metric::DcInputPower, 0.0),
        (Metric::AcInputPower, 0.0),
        (Metric::AcInputFrequency, 0.0),
        (Metric::AcInputVoltage, 0.0),
        // 103–105: charging status, time to full / to empty (R1a extras).
        (Metric::ChargingStatus, 2.0),
        (Metric::TimeToFull, 228.0),
        (Metric::TimeToEmpty, 228.0),
    ]);
    assert_eq!(
        poll.values.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (metric, want) in &expected {
        let got = poll.values[metric];
        assert!((got - want).abs() < 1e-9, "{metric:?}: {got} != {want}");
    }
    assert_eq!(poll.problems, []);
}

#[tokio::test]
async fn ac180_read_session_replays() {
    let (identity, poll) = replay_read("ac180", Model::Ac180).await;
    assert_eq!(
        identity,
        Identity {
            // 110-112 byte-swapped: "AC" "18" "0"
            device_type: "AC180".into(),
            layout: Protocol::V2,
            // 6175-6176 = 0x9af3, 1 → 0x1_9af3 = 105203
            firmware: vec![("bms", "1052.03".into())],
        }
    );
    // 100 = 3309 × 0.01, 101 = 10 × 0.1, 102 = 93, 1213 = 1 × 0.1,
    // 1314 = 21 × 0.1, 2011 = 1; 140-147, 1300 and 2012 are 0 (on battery,
    // AC output on, no load).
    let expected = BTreeMap::from([
        (Metric::PackVoltage, 33.09),
        (Metric::PackCurrent, 1.0),
        (Metric::BatterySoc, 93.0),
        (Metric::DcOutputPower, 0.0),
        (Metric::AcOutputPower, 0.0),
        (Metric::DcInputPower, 0.0),
        (Metric::AcInputPower, 0.0),
        (Metric::DcInputVoltage, 0.1),
        (Metric::AcInputFrequency, 0.0),
        (Metric::AcInputVoltage, 2.1),
        (Metric::AcOutputOn, 1.0),
        (Metric::DcOutputOn, 0.0),
        (Metric::ChargingStatus, 0.0),
        (Metric::TimeToFull, 6376.0),
        (Metric::TimeToEmpty, 6376.0),
    ]);
    assert_eq!(
        poll.values.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (metric, want) in &expected {
        let got = poll.values[metric];
        assert!((got - want).abs() < 1e-9, "{metric:?}: {got} != {want}");
    }
    assert_eq!(poll.problems, []);
}
