use std::num::NonZeroU32;
use std::time::Duration;

use bluetti_controller::control::Refused;
use bluetti_controller::protocol::device::Model;
use bluetti_controller::protocol::modbus::{crc16, read_request, write_request};
use bluetti_controller::transport::fixture::{
    Dir, Fixture, FixtureAdvert, FrameRecord, encode_hex,
};
use bluetti_controller::transport::replay::ReplayGatt;
use clap::Parser;

use super::*;
use crate::cli::Command as Cli;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    command: Cli,
}

fn stations() -> Vec<StationConfig> {
    vec![StationConfig {
        name: "router".into(),
        model: Model::Eb3a,
        address: "AA:BB:CC:00:00:00".into(),
        grid_sensor: false,
    }]
}

const ENABLED: ControlConfig = ControlConfig { enabled: true };

#[test]
fn parses_station_output_and_state() {
    let args = Args::try_parse_from(["x", "set", "router", "ac", "off"]).unwrap();
    assert!(matches!(
        args.command,
        Cli::Set { station, output: Output::Ac, state: false } if station == "router"
    ));
    let args = Args::try_parse_from(["x", "set", "router", "dc", "on"]).unwrap();
    assert!(matches!(
        args.command,
        Cli::Set {
            output: Output::Dc,
            state: true,
            ..
        }
    ));
    for bad in [
        ["x", "set", "router", "usb", "on"],
        ["x", "set", "router", "ac", "1"],
    ] {
        assert!(Args::try_parse_from(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn a_mac_or_unknown_name_is_refused_before_ble() {
    for station in ["AA:BB:CC:00:00:00", "mobile"] {
        let err = command(&stations(), &ENABLED, station, Output::Ac, true).unwrap_err();
        assert!(err.to_string().contains("station name"), "{err}");
    }
}

#[test]
fn a_disabled_config_is_refused_before_ble() {
    let err = command(
        &stations(),
        &ControlConfig::default(),
        "router",
        Output::Ac,
        true,
    )
    .unwrap_err();
    assert_eq!(err.downcast_ref::<Refused>(), Some(&Refused::Disabled));
}

/// EB3A AC on (register 3007 = 1), the echo, then the read-back of register
/// 48 answering 1. Mirrors `transport::write::tests::eb3a_ac_on_fixture`,
/// which a binary test cannot reach (it is `cfg(test)` in the library).
fn eb3a_ac_on_fixture() -> Fixture {
    let write = write_request(3007, 1);
    let mut read_back = vec![0x01, 0x03, 0x02, 0x00, 0x01];
    read_back.extend_from_slice(&crc16(&read_back).to_le_bytes());
    let frames: [(Dir, &[u8]); 4] = [
        (Dir::Write, &write),
        (Dir::Notify, &write),
        (Dir::Write, &read_request(48, 1)),
        (Dir::Notify, &read_back),
    ];
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

#[tokio::test]
async fn apply_writes_the_switch_and_returns_the_read_back() {
    let command = command(&stations(), &ENABLED, "router", Output::Ac, true).unwrap();
    let mut gatt = ReplayGatt::new(&eb3a_ac_on_fixture()).unwrap();
    let policy = ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(3).unwrap(),
        backoff: Duration::ZERO,
    };
    assert!(apply(&mut gatt, &command, &policy).await.unwrap());
}
