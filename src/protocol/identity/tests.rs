use super::*;

#[test]
fn ascii_trims_padding_and_rejects_garbage() {
    assert_eq!(
        ascii(&[0x4542, 0x3341, 0, 0, 0, 0], false).as_deref(),
        Some("EB3A")
    );
    assert_eq!(ascii(&[0x4245, 0x4133], true).as_deref(), Some("EB3A"));
    assert_eq!(ascii(&[0x2045, 0x4200], false).as_deref(), Some("EB"));
    assert_eq!(ascii(&[0x0145], false), None);
    assert_eq!(ascii(&[0xFFFF], false), None);
    assert_eq!(ascii(&[0x2020], false), None);
}

#[test]
fn version_is_low_word_first_in_hundredths() {
    assert_eq!(version(&[0x0BB8, 0x0001]), "685.36");
    assert_eq!(version(&[3000, 0]), "30.00");
    assert_eq!(version(&[7, 0]), "0.07");
}

#[test]
fn layouts_put_the_expected_one_first() {
    assert_eq!(
        layouts(Protocol::V1).map(|l| l.protocol),
        [Protocol::V1, Protocol::V2]
    );
    assert_eq!(
        layouts(Protocol::V2).map(|l| l.protocol),
        [Protocol::V2, Protocol::V1]
    );
}

#[test]
fn serial_registers_per_layout() {
    assert!(is_serial(17, Some(Protocol::V1)) && is_serial(20, None));
    assert!(!is_serial(17, Some(Protocol::V2)) && !is_serial(21, None));
    assert!(is_serial(116, Some(Protocol::V2)) && is_serial(119, None));
    assert!(!is_serial(119, Some(Protocol::V1)) && !is_serial(115, None));
}
