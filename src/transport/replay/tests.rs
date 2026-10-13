use std::time::Duration;

use super::*;
use crate::protocol::modbus::read_request;
use crate::transport::fixture::{FixtureAdvert, FrameRecord};

const WAIT: Duration = Duration::from_secs(5);

fn fixture(frames: &[(Dir, &str)]) -> Fixture {
    Fixture {
        advert: FixtureAdvert {
            name: "EB3A0000000000000".into(),
            manufacturer_data: "424c5545545449".into(),
        },
        ephemeral: None,
        frames: frames
            .iter()
            .enumerate()
            .map(|(i, (dir, hex))| FrameRecord {
                t_ms: 10 * i as u64,
                dir: *dir,
                uuid: dir.uuid().into(),
                hex: (*hex).into(),
            })
            .collect(),
    }
}

fn request() -> String {
    encode_hex(&read_request(10, 1))
}

#[tokio::test]
async fn serves_fragments_in_order() {
    let req = request();
    let fx = fixture(&[
        (Dir::Write, &req),
        (Dir::Notify, "0103"),
        (Dir::Notify, "0200"),
        (Dir::Notify, "647820"),
    ]);
    let mut gatt = ReplayGatt::new(&fx).unwrap();
    gatt.write(&read_request(10, 1)).await.unwrap();
    for want in [vec![0x01, 0x03], vec![0x02, 0x00], vec![0x64, 0x78, 0x20]] {
        assert_eq!(gatt.next_notification(WAIT).await, Ok(want));
    }
}

#[tokio::test]
async fn unexpected_write_is_error() {
    let req = request();
    let mut gatt = ReplayGatt::new(&fixture(&[(Dir::Write, &req), (Dir::Notify, "01")])).unwrap();
    let err = gatt.write(&read_request(11, 1)).await.unwrap_err();
    assert_eq!(
        err,
        TransportError::UnexpectedWrite {
            expected: req.clone(),
            got: encode_hex(&read_request(11, 1)),
        }
    );
    // The rejected write consumed nothing: the right one still matches.
    gatt.write(&read_request(10, 1)).await.unwrap();
    // A write while a notification is unread is out of order.
    assert!(matches!(
        gatt.write(&read_request(10, 1)).await,
        Err(TransportError::UnexpectedWrite { .. })
    ));
}

#[tokio::test]
async fn exhausted_fixture_times_out() {
    let req = request();
    let mut gatt = ReplayGatt::new(&fixture(&[(Dir::Write, &req)])).unwrap();
    // A write is pending, so no notification is due yet.
    assert_eq!(
        gatt.next_notification(WAIT).await,
        Err(TransportError::Timeout(WAIT))
    );
    gatt.write(&read_request(10, 1)).await.unwrap();
    assert_eq!(
        gatt.next_notification(WAIT).await,
        Err(TransportError::Timeout(WAIT))
    );
    assert!(matches!(
        gatt.write(&read_request(10, 1)).await,
        Err(TransportError::UnexpectedWrite { expected, .. }) if expected == "end of fixture"
    ));
}

#[tokio::test]
async fn disconnect_ends_the_session() {
    let mut gatt = ReplayGatt::new(&fixture(&[(Dir::Notify, "01")])).unwrap();
    gatt.disconnect().await.unwrap();
    assert_eq!(
        gatt.next_notification(WAIT).await,
        Err(TransportError::NotConnected)
    );
    assert_eq!(gatt.write(&[1]).await, Err(TransportError::NotConnected));
}

#[test]
fn new_rejects_bad_frames() {
    let mut fx = fixture(&[(Dir::Write, "01")]);
    fx.frames[0].uuid = "ff01".into();
    assert!(matches!(
        ReplayGatt::new(&fx),
        Err(FixtureError::WrongUuid { index: 0, .. })
    ));
    assert!(matches!(
        ReplayGatt::new(&fixture(&[(Dir::Notify, "0g")])),
        Err(FixtureError::Hex(_))
    ));
}
