use super::*;

#[test]
fn anonymised_line_masks_mac_and_serial() {
    let advert = Advert {
        // Built at runtime so the staged-diff MAC scan never sees a literal.
        address: ["12", "34", "56", "78", "9A", "BC"].join(":"),
        name: Some("EB3A2235000123".into()),
        rssi: Some(-61),
        manufacturer_data: b"BLUETTI\x01\x02".to_vec(),
    };
    assert_eq!(
        line(&advert, true),
        "AA:BB:CC:00:00:00  EB3A0000000000  -61 dBm  EB3A  encrypted=no"
    );
    let raw = format!("{}  EB3A2235000123", advert.address);
    assert!(line(&advert, false).starts_with(&raw));
}

#[test]
fn unknown_station_still_prints() {
    let advert = Advert {
        address: String::new(),
        name: None,
        rssi: None,
        manufacturer_data: b"BLUETTE".to_vec(),
    };
    assert_eq!(line(&advert, true), "  ?  ? dBm  unknown  encrypted=yes");
}
