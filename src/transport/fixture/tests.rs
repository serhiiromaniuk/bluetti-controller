use super::*;
use crate::protocol::device::{Model, is_encrypted};

const EB3A_JSON: &str = r#"{
  "advert": { "name": "EB3A0000000000000", "manufacturer_data": "424c5545545449" },
  "frames": [
    { "t_ms": 0, "dir": "write", "uuid": "ff02", "hex": "0103000a000165c8" },
    { "t_ms": 42, "dir": "notify", "uuid": "ff01", "hex": "01030200647820" }
  ]
}"#;

#[test]
fn parses_spec_shape() {
    let fixture = Fixture::from_json(EB3A_JSON).unwrap();
    assert_eq!(fixture.frames.len(), 2);
    assert_eq!(fixture.frames[1].t_ms, 42);
    assert_eq!(fixture.frames[1].dir, Dir::Notify);
}

#[test]
fn advert_replays_model_and_encryption() {
    let advert = Fixture::from_json(EB3A_JSON).unwrap().advert().unwrap();
    assert_eq!(advert.address, PLACEHOLDER_ADDRESS);
    assert_eq!(
        Model::from_ble_name(advert.name.as_deref().unwrap()),
        Some(Model::Eb3a)
    );
    assert_eq!(is_encrypted(&advert.manufacturer_data), Some(false));
}

#[test]
fn rejects_unknown_key_and_dir() {
    let extra = EB3A_JSON.replace("\"t_ms\": 0,", "\"t_ms\": 0, \"mac\": \"x\",");
    assert!(matches!(
        Fixture::from_json(&extra),
        Err(FixtureError::Json(_))
    ));
    let dir = EB3A_JSON.replace("\"write\"", "\"read\"");
    assert!(matches!(
        Fixture::from_json(&dir),
        Err(FixtureError::Json(_))
    ));
}

#[test]
fn load_reports_missing_file() {
    let err = Fixture::load(Path::new("tests/frames/missing.json")).unwrap_err();
    assert!(err.to_string().contains("missing.json"), "{err}");
}

#[test]
fn hex_round_trips_and_rejects_garbage() {
    assert_eq!(encode_hex(&[0x01, 0xab, 0xff]), "01abff");
    assert_eq!(decode_hex("01ABff").unwrap(), vec![0x01, 0xab, 0xff]);
    for bad in ["0", "zz", "0é", "+1"] {
        assert!(
            matches!(decode_hex(bad), Err(FixtureError::Hex(_))),
            "{bad}"
        );
    }
}
