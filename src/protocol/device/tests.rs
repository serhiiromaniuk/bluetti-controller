use super::*;

#[test]
fn detects_model_from_ble_name() {
    assert_eq!(Model::from_ble_name("EB3A2235000012345"), Some(Model::Eb3a));
    assert_eq!(
        Model::from_ble_name("AC1802335000123456"),
        Some(Model::Ac180)
    );
    assert_eq!(
        Model::from_ble_name("E200V22501000012345"),
        Some(Model::Elite200V2)
    );
    assert_eq!(Model::from_ble_name("AC180P2335000123456"), None);
    assert_eq!(Model::from_ble_name("EB3A"), None);
    assert_eq!(Model::from_ble_name("Pixel 9"), None);
}

#[test]
fn reads_encryption_flag() {
    assert_eq!(is_encrypted(b"BLUETTI"), Some(false));
    assert_eq!(is_encrypted(b"BLUETTE"), Some(true));
    assert_eq!(is_encrypted(b"BLUETTF\x01"), Some(true));
    assert_eq!(is_encrypted(b"BLUE"), None);
    assert_eq!(is_encrypted(b"SOMETHING"), None);
}

#[test]
fn every_field_is_covered_by_a_block() {
    for model in [Model::Eb3a, Model::Ac180, Model::Elite200V2] {
        let profile = model.profile();
        for f in profile.fields {
            let covered = profile
                .blocks
                .iter()
                .any(|&(start, count)| (start..start + count).contains(&f.register));
            assert!(covered, "{model:?}: register {} not polled", f.register);
        }
    }
}

#[test]
fn decodes_eb3a_block() {
    let mut block = [0u16; 40];
    block[36 - 10] = 0; // DC input
    block[37 - 10] = 12; // AC input
    block[38 - 10] = 9; // AC output
    block[43 - 10] = 87; // SOC
    block[48 - 10] = 1; // AC output on
    let mut registers = Registers::default();
    registers.insert_block(10, &block);
    registers.insert_block(
        70,
        &[0; 7].iter().copied().chain([2301]).collect::<Vec<_>>(),
    );

    let metrics = decode(Model::Eb3a.profile(), &registers);
    assert_eq!(metrics[&BatterySoc], 87.0);
    assert_eq!(metrics[&AcInputPower], 12.0);
    assert_eq!(metrics[&AcOutputOn], 1.0);
    assert!((metrics[&AcInputVoltage] - 230.1).abs() < 1e-9);
    assert!(
        !metrics.contains_key(&DcInputVoltage),
        "register 86 was not polled"
    );
}

#[test]
fn drops_implausible_soc() {
    let mut registers = Registers::default();
    registers.insert_block(102, &[250]);
    assert!(!decode(Model::Ac180.profile(), &registers).contains_key(&BatterySoc));
}

#[test]
fn metric_keys_and_units() {
    let table = [
        (BatterySoc, "battery_soc", "%"),
        (AcInputPower, "ac_input_power", "W"),
        (DcInputPower, "dc_input_power", "W"),
        (AcOutputPower, "ac_output_power", "W"),
        (DcOutputPower, "dc_output_power", "W"),
        (AcInputVoltage, "ac_input_voltage", "V"),
        (AcInputFrequency, "ac_input_frequency", "Hz"),
        (DcInputVoltage, "dc_input_voltage", "V"),
        (AcOutputOn, "ac_output_on", ""),
        (DcOutputOn, "dc_output_on", ""),
        (PackVoltage, "pack_voltage", "V"),
        (PackCurrent, "pack_current", "A"),
    ];
    for (metric, key, unit) in table {
        assert_eq!((metric.key(), metric.unit()), (key, unit), "{metric:?}");
    }
}

#[test]
fn field_maps_register_to_metric() {
    let f = field(BatterySoc, 43, 0.1, true);
    assert_eq!(
        (f.metric, f.register, f.scale, f.verified),
        (BatterySoc, 43, 0.1, true)
    );
}

#[test]
fn drops_implausible_output_flag() {
    assert!(AcOutputOn.is_plausible(1.0));
    assert!(!DcOutputOn.is_plausible(2.0));
    assert!(PackCurrent.is_plausible(-5.0));
}
