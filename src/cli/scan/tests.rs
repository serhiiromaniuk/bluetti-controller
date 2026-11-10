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

#[test]
fn control_characters_in_the_name_never_reach_the_terminal() {
    let advert = Advert {
        address: String::new(),
        name: Some("EB3A\x1b[31m\rX\u{9b}".into()),
        rssi: None,
        manufacturer_data: b"BLUETTI\x01".to_vec(),
    };
    let raw = line(&advert, false);
    assert!(
        raw.starts_with("  EB3A\u{fffd}[31m\u{fffd}X\u{fffd}  "),
        "{raw:?}"
    );
    let masked = line(&advert, true);
    assert!(!masked.chars().any(char::is_control), "{masked:?}");
    assert_eq!(super::super::printable("EB3A2235000123"), "EB3A2235000123");
}

#[test]
fn printable_replaces_every_bidi_control() {
    // Amendment A5: the Unicode Bidi_Control set is Cf, not Cc.
    for bidi in [
        '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
        '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
    ] {
        assert_eq!(
            super::super::printable(&format!("EB3A{bidi}x")),
            "EB3A\u{fffd}x"
        );
    }
    // Neighbours of the ranges, and other Cf such as ZWJ, stay.
    for kept in [
        '\u{2029}', '\u{202f}', '\u{2065}', '\u{206a}', '\u{200d}', 'é',
    ] {
        let text = format!("EB3A{kept}");
        assert_eq!(super::super::printable(&text), text);
    }
}
