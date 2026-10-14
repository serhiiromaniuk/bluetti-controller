//! Hardware tests, run only by a human next to a station (RULES.md testing):
//! `BLUETTI_TEST_MAC=<mac> cargo test --test hardware -- --ignored`.
//! Read-only: nothing is written to the station.

use std::time::Duration;

use bluetti_controller::config::BleConfig;
use bluetti_controller::protocol::device::is_encrypted;
use bluetti_controller::transport::{self, Gatt, TransportError};

#[tokio::test]
#[ignore = "needs a station in BLE range and BLUETTI_TEST_MAC"]
async fn connects_subscribes_and_disconnects() {
    let mac = std::env::var("BLUETTI_TEST_MAC").expect("set BLUETTI_TEST_MAC");
    let ble = BleConfig::default();
    let (advert, _) = transport::find(&ble, &mac).await.expect("station found");
    let mut gatt = transport::connect(&ble, &advert).await.expect("connected");
    let first = gatt.next_notification(Duration::from_secs(5)).await;
    gatt.disconnect().await.expect("disconnected");
    assert_eq!(gatt.write(&[]).await, Err(TransportError::NotConnected));
    if is_encrypted(&advert.manufacturer_data) == Some(true) {
        // ROADMAP.md §4.4 message 1: kex frame, type 01, len 4.
        assert!(
            first
                .expect("challenge")
                .starts_with(&[0x2A, 0x2A, 0x01, 0x04])
        );
    }
}
