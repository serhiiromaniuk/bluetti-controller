use bluetti_controller::config;

use super::*;

fn stations() -> Vec<StationConfig> {
    let text = std::fs::read_to_string("config.example.toml").unwrap();
    config::parse(&text).unwrap().stations
}

#[test]
fn station_name_resolves_to_its_address() {
    let stations = stations();
    assert_eq!(resolve(&stations, "living-room"), "AA:BB:CC:DD:EE:03");
}

#[test]
fn unknown_target_passes_through_as_an_address() {
    assert_eq!(
        resolve(&stations(), "AA:BB:CC:00:00:09"),
        "AA:BB:CC:00:00:09"
    );
    assert_eq!(resolve(&[], "router"), "router");
}

#[test]
fn line_adds_protocol_and_flag() {
    let advert = Advert {
        address: "AA:BB:CC:00:00:01".into(),
        name: Some("E200V22345000123".into()),
        rssi: Some(-70),
        manufacturer_data: b"BLUETTF\x01".to_vec(),
    };
    assert_eq!(
        line(&advert, true),
        "AA:BB:CC:00:00:00  E200V20000000000  -70 dBm  Elite200V2  encrypted=yes  \
         protocol=V2  flag=BLUETTF"
    );
}

#[test]
fn line_of_an_unknown_station() {
    let advert = Advert {
        address: String::new(),
        name: None,
        rssi: None,
        manufacturer_data: b"BL".to_vec(),
    };
    assert!(line(&advert, false).ends_with("protocol=unknown  flag=?"));
}

#[test]
fn header_shows_length_and_four_bytes() {
    assert_eq!(
        header(&[0x2A, 0x2A, 0x01, 0x04, 0xDE, 0xAD]),
        "first notification: 6 bytes, starts 2a2a0104"
    );
    assert_eq!(header(&[0x01]), "first notification: 1 bytes, starts 01");
}
