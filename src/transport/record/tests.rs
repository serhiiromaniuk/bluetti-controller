use std::time::Duration;

use super::*;
use crate::protocol::modbus::read_request;
use crate::transport::fixture::{FixtureAdvert, FrameRecord, encode_hex};
use crate::transport::replay::ReplayGatt;

const WAIT: Duration = Duration::from_secs(5);

fn session() -> Fixture {
    let frames = [
        (Dir::Write, encode_hex(&read_request(10, 1))),
        (Dir::Notify, "010302".into()),
        (Dir::Notify, "0064b9af".into()),
    ];
    Fixture {
        advert: FixtureAdvert {
            name: "EB3A0000000000000".into(),
            manufacturer_data: "424c5545545449".into(),
        },
        ephemeral: None,
        frames: frames
            .into_iter()
            .enumerate()
            .map(|(i, (dir, hex))| FrameRecord {
                t_ms: 10 * i as u64,
                dir,
                uuid: dir.uuid().into(),
                hex,
            })
            .collect(),
    }
}

fn advert() -> Advert {
    Advert {
        address: "AA:BB:CC:00:00:00".into(),
        name: Some("EB3A0000000000000".into()),
        rssi: Some(-60),
        manufacturer_data: b"BLUETTI".to_vec(),
    }
}

fn without_time(frames: &[FrameRecord]) -> Vec<(Dir, String, String)> {
    frames
        .iter()
        .map(|f| (f.dir, f.uuid.clone(), f.hex.clone()))
        .collect()
}

async fn record_session() -> RecordingGatt<ReplayGatt> {
    let original = session();
    let mut gatt = RecordingGatt::new(ReplayGatt::new(&original).unwrap(), &advert());
    gatt.write(&read_request(10, 1)).await.unwrap();
    gatt.next_notification(WAIT).await.unwrap();
    gatt.next_notification(WAIT).await.unwrap();
    // A timeout and a refused write carry no frame, so neither is recorded.
    assert!(gatt.next_notification(WAIT).await.is_err());
    assert!(gatt.write(&read_request(20, 1)).await.is_err());
    gatt.disconnect().await.unwrap();
    gatt
}

#[tokio::test]
async fn tees_every_frame() {
    let gatt = record_session().await;
    let recorded = gatt.fixture();
    assert_eq!(
        without_time(&recorded.frames),
        without_time(&session().frames)
    );
    assert_eq!(recorded.advert, session().advert);
    let times: Vec<u64> = recorded.frames.iter().map(|f| f.t_ms).collect();
    assert!(times.is_sorted());
}

#[tokio::test]
async fn saved_fixture_replays() {
    let gatt = record_session().await;
    let path = std::env::temp_dir().join(format!("record-test-{}.json", std::process::id()));
    gatt.fixture().save(&path).unwrap();
    let loaded = Fixture::load(&path);
    std::fs::remove_file(&path).unwrap();
    let loaded = loaded.unwrap();
    assert_eq!(&loaded, gatt.fixture());
    let mut replay = ReplayGatt::new(&loaded).unwrap();
    replay.write(&read_request(10, 1)).await.unwrap();
    assert_eq!(replay.next_notification(WAIT).await.unwrap(), [1, 3, 2]);
}
