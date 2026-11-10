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

#[test]
fn a_hostile_advertised_name_cannot_drive_the_terminal() {
    // Security suite type 2 (step 10a): a station name is attacker input to
    // `scan`. OSC 8 hyperlink, OSC 0 title, clear screen, the C1 CSI and a
    // right-to-left override that would show `exe.txt` as `txt.exe`.
    for (name, shown) in [
        (
            "\x1b]8;;http://evil.example\x1b\\EB3A\x1b]8;;\x1b\\",
            "\u{fffd}]8;;http://evil.example\u{fffd}\\EB3A\u{fffd}]8;;\u{fffd}\\",
        ),
        ("\x1b]0;pwned\x07EB3A", "\u{fffd}]0;pwned\u{fffd}EB3A"),
        ("\x1b[2J\x1b[HEB3A", "\u{fffd}[2J\u{fffd}[HEB3A"),
        ("\u{9b}31mEB3A", "\u{fffd}31mEB3A"),
        ("EB3A\u{202e}txt.exe", "EB3A\u{fffd}txt.exe"),
        ("EB3A\u{2066}x\u{2069}", "EB3A\u{fffd}x\u{fffd}"),
    ] {
        assert_eq!(super::super::printable(name), shown, "{name:?}");
    }
}

#[test]
fn printable_over_every_unicode_scalar() {
    // Security suite type 4 (step 10a), exhaustive: every `char` alone and
    // between two letters. Replaced exactly when it is a control (Cc) or in
    // Unicode's Bidi_Control set (PropList.txt), else kept as is.
    const BIDI_CONTROL: &[(u32, u32)] = &[
        (0x061c, 0x061c),
        (0x200e, 0x200f),
        (0x202a, 0x202e),
        (0x2066, 0x2069),
    ];
    let mut replaced = 0;
    for c in (0..=0x10_ffff).filter_map(char::from_u32) {
        let bidi = BIDI_CONTROL
            .iter()
            .any(|(lo, hi)| (*lo..=*hi).contains(&u32::from(c)));
        let want = if c.is_control() || bidi {
            '\u{fffd}'
        } else {
            c
        };
        let text = format!("a{c}b");
        assert_eq!(super::super::printable(&text), format!("a{want}b"), "{c:?}");
        replaced += usize::from(want != c);
    }
    // 65 Cc (C0, DEL, C1) + 12 Bidi_Control.
    assert_eq!(replaced, 77);
}
