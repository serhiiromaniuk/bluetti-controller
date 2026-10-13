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
        // 77 = 222 raw with the grid on: whole volts (CF-4 dump, 2026-10-13).
        &[0; 7].iter().copied().chain([222]).collect::<Vec<_>>(),
    );

    let metrics = decode(Model::Eb3a.profile(), &registers);
    assert_eq!(metrics[&BatterySoc], 87.0);
    assert_eq!(metrics[&AcInputPower], 12.0);
    assert_eq!(metrics[&AcOutputOn], 1.0);
    assert_eq!(metrics[&AcInputVoltage], 222.0);
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

/// AC180 registers with every field set from `(register, raw)` pairs, the
/// rest of each polled block zero.
fn ac180_registers(values: &[(u16, u16)]) -> Registers {
    let mut registers = Registers::default();
    for &(start, count) in Model::Ac180.profile().blocks {
        registers.insert_block(start, &vec![0; usize::from(count)]);
    }
    for &(register, raw) in values {
        registers.insert_block(register, &[raw]);
    }
    registers
}

#[test]
fn plausible_poll_has_no_problems() {
    // 53.20 V, 5.2 A, 87 %, 120 W AC out, 50.0 Hz, 230.1 V, AC on.
    let registers = ac180_registers(&[
        (100, 5320),
        (101, 52),
        (102, 87),
        (142, 120),
        (1300, 500),
        (1314, 2301),
        (2011, 1),
    ]);
    assert_eq!(problems(Model::Ac180.profile(), &registers), vec![]);
}

#[test]
fn names_each_out_of_range_field_with_its_value() {
    let registers = ac180_registers(&[(102, 101), (142, 5001)]);
    assert_eq!(
        problems(Model::Ac180.profile(), &registers),
        vec![
            Problem::OutOfRange {
                metric: BatterySoc,
                value: 101.0
            },
            Problem::OutOfRange {
                metric: AcOutputPower,
                value: 5001.0
            },
        ]
    );
}

#[test]
fn names_each_field_whose_register_is_absent() {
    let mut registers = Registers::default();
    registers.insert_block(10, &[0; 40]); // block (70, 21) not read
    assert_eq!(
        problems(Model::Eb3a.profile(), &registers),
        vec![
            Problem::Missing {
                metric: AcInputVoltage
            },
            Problem::Missing {
                metric: DcInputVoltage
            },
        ]
    );
}

/// Raw values that decode to exactly each AC180 field's upper bound.
const AC180_AT_BOUND: [(u16, u16); 12] = [
    (100, 6000), // 60.00 V
    (101, 1000), // 100.0 A
    (102, 100),  // 100 %
    (140, 5000), // W
    (142, 5000),
    (144, 5000),
    (146, 5000),
    (1213, 1500), // 150.0 V
    (1300, 700),  // 70.0 Hz
    (1314, 3000), // 300.0 V
    (2011, 1),
    (2012, 1),
];

#[test]
fn accepts_values_exactly_at_the_bound() {
    let registers = ac180_registers(&AC180_AT_BOUND);
    assert_eq!(problems(Model::Ac180.profile(), &registers), vec![]);
}

#[test]
fn rejects_one_raw_step_past_the_bound() {
    let over: Vec<_> = AC180_AT_BOUND
        .iter()
        .map(|&(r, raw)| (r, raw + 1))
        .collect();
    let found = problems(Model::Ac180.profile(), &ac180_registers(&over));
    let named: Vec<_> = found
        .iter()
        .map(|p| match p {
            Problem::OutOfRange { metric, .. } => *metric,
            Problem::Missing { metric } => panic!("{metric:?} reported missing"),
        })
        .collect();
    let expected: Vec<_> = Model::Ac180
        .profile()
        .fields
        .iter()
        .map(|f| f.metric)
        .collect();
    assert_eq!(named, expected);
}

#[test]
fn every_range_accepts_the_idle_zero() {
    // An unplugged input or an idle output reads 0 and must stay plausible.
    for metric in [
        BatterySoc,
        AcInputPower,
        DcInputPower,
        AcOutputPower,
        DcOutputPower,
        AcInputVoltage,
        AcInputFrequency,
        DcInputVoltage,
        AcOutputOn,
        DcOutputOn,
        PackVoltage,
        PackCurrent,
    ] {
        assert!(metric.range().contains(&0.0), "{metric:?}");
    }
    // Register 101 is a magnitude; 0xFFFF-style garbage stays out.
    assert!(!PackCurrent.range().contains(&-0.1), "unsigned");
    assert!(!PackCurrent.range().contains(&6553.5), "unsigned");
}

/// Hand-read from the step-14 dump of our Elite 200 V2 (2026-10-13, on
/// battery, grid off): 100 = 3949, 101 = 98, 102 = 77, 142 = 389, the rest 0.
/// 389 W / 39.49 V = 9.85 A matches 101; 6003–6005 echoed 3950/98/77.
#[test]
fn decodes_elite_200_v2_dump() {
    let mut registers = Registers::default();
    let mut first = [0u16; 20];
    first[..3].copy_from_slice(&[3949, 98, 77]);
    registers.insert_block(100, &first);
    registers.insert_block(140, &[0, 0, 389, 0, 0, 0, 0, 0]);
    registers.insert_block(1300, &[0; 15]);

    let profile = Model::Elite200V2.profile();
    let metrics = decode(profile, &registers);
    assert!((metrics[&PackVoltage] - 39.49).abs() < 1e-9);
    assert!((metrics[&PackCurrent] - 9.8).abs() < 1e-9);
    assert_eq!(metrics[&BatterySoc], 77.0);
    assert_eq!(metrics[&AcOutputPower], 389.0);
    assert_eq!(metrics[&AcInputVoltage], 0.0);
    assert_eq!(problems(profile, &registers), []);
}
