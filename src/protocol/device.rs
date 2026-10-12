//! Device models, register maps and decoding of polled register blocks.
//!
//! Register addresses come from the community projects listed in
//! `docs/RESEARCH.md` (bluetti-bt-lib, bluetti_mqtt). Fields marked
//! `verified: false` are not yet confirmed on that model and need a hardware
//! check (milestone M1).

use std::collections::BTreeMap;

/// Bluetti company id in BLE manufacturer data (ASCII "BL", little-endian).
pub const COMPANY_ID: u16 = 0x4C42;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    Eb3a,
    Ac180,
    Elite200V2,
}

/// Register layout generation. V1 (EB3A) and V2 (AC180, Elite 200 V2) use the
/// same Modbus framing but different register addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    V1,
    V2,
}

const NAME_PREFIXES: [(&str, Model); 4] = [
    ("EB3A", Model::Eb3a),
    ("AC180", Model::Ac180),
    ("E200V2", Model::Elite200V2),
    ("Elite 200 V2", Model::Elite200V2),
];

impl Model {
    /// Detects the model from the advertised BLE name: `<MODEL><serial digits>`.
    pub fn from_ble_name(name: &str) -> Option<Model> {
        NAME_PREFIXES.iter().find_map(|(prefix, model)| {
            let serial = name.strip_prefix(prefix)?;
            let is_serial = !serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit());
            is_serial.then_some(*model)
        })
    }

    pub fn profile(self) -> &'static Profile {
        match self {
            Model::Eb3a => &EB3A,
            Model::Ac180 => &AC180,
            Model::Elite200V2 => &ELITE_200_V2,
        }
    }
}

/// Reads the encryption flag from the manufacturer data stored under
/// [`COMPANY_ID`]: `BLUETTI` means plaintext, `BLUETTE`/`BLUETTF` encrypted.
/// Encryption depends on firmware, not model, so it is checked on every scan.
pub fn is_encrypted(manufacturer_data: &[u8]) -> Option<bool> {
    match manufacturer_data.get(..7)? {
        b"BLUETTI" => Some(false),
        b"BLUETTE" | b"BLUETTF" => Some(true),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Metric {
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
}

impl Metric {
    /// Stable name for storage and the HTTP API.
    pub fn key(self) -> &'static str {
        match self {
            Metric::BatterySoc => "battery_soc",
            Metric::AcInputPower => "ac_input_power",
            Metric::DcInputPower => "dc_input_power",
            Metric::AcOutputPower => "ac_output_power",
            Metric::DcOutputPower => "dc_output_power",
            Metric::AcInputVoltage => "ac_input_voltage",
            Metric::AcInputFrequency => "ac_input_frequency",
            Metric::DcInputVoltage => "dc_input_voltage",
            Metric::AcOutputOn => "ac_output_on",
            Metric::DcOutputOn => "dc_output_on",
            Metric::PackVoltage => "pack_voltage",
            Metric::PackCurrent => "pack_current",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Metric::BatterySoc => "%",
            Metric::AcInputPower
            | Metric::DcInputPower
            | Metric::AcOutputPower
            | Metric::DcOutputPower => "W",
            Metric::AcInputVoltage | Metric::DcInputVoltage | Metric::PackVoltage => "V",
            Metric::AcInputFrequency => "Hz",
            Metric::PackCurrent => "A",
            Metric::AcOutputOn | Metric::DcOutputOn => "",
        }
    }

    fn is_plausible(self, value: f64) -> bool {
        match self {
            Metric::BatterySoc => value <= 100.0,
            Metric::AcOutputOn | Metric::DcOutputOn => value <= 1.0,
            _ => true,
        }
    }
}

/// One register mapped to a metric: `value = raw * scale`.
#[derive(Debug, Clone, Copy)]
pub struct Field {
    pub metric: Metric,
    pub register: u16,
    pub scale: f64,
    pub verified: bool,
}

const fn field(metric: Metric, register: u16, scale: f64, verified: bool) -> Field {
    Field { metric, register, scale, verified }
}

#[derive(Debug)]
pub struct Profile {
    pub model: Model,
    pub protocol: Protocol,
    /// Register blocks read on every poll, as `(start, count)`.
    pub blocks: &'static [(u16, u16)],
    pub fields: &'static [Field],
    /// Nominal battery capacity from the spec sheet, for runtime estimates.
    pub nominal_capacity_wh: f64,
}

use Metric::*;

static EB3A: Profile = Profile {
    model: Model::Eb3a,
    protocol: Protocol::V1,
    blocks: &[(10, 40), (70, 21)],
    fields: &[
        field(DcInputPower, 36, 1.0, true),
        field(AcInputPower, 37, 1.0, true),
        field(AcOutputPower, 38, 1.0, true),
        field(DcOutputPower, 39, 1.0, true),
        field(BatterySoc, 43, 1.0, true),
        field(AcOutputOn, 48, 1.0, true),
        field(DcOutputOn, 49, 1.0, true),
        field(AcInputVoltage, 77, 0.1, true),
        field(DcInputVoltage, 86, 0.01, true),
    ],
    nominal_capacity_wh: 268.8,
};

static AC180: Profile = Profile {
    model: Model::Ac180,
    protocol: Protocol::V2,
    blocks: &[(100, 20), (140, 8), (1213, 1), (1300, 15), (2011, 2)],
    fields: &[
        field(PackVoltage, 100, 0.01, false),
        field(PackCurrent, 101, 0.1, false),
        field(BatterySoc, 102, 1.0, true),
        field(DcOutputPower, 140, 1.0, true),
        field(AcOutputPower, 142, 1.0, true),
        field(DcInputPower, 144, 1.0, true),
        field(AcInputPower, 146, 1.0, true),
        field(DcInputVoltage, 1213, 0.1, true),
        field(AcInputFrequency, 1300, 0.1, true),
        field(AcInputVoltage, 1314, 0.1, true),
        field(AcOutputOn, 2011, 1.0, true),
        field(DcOutputOn, 2012, 1.0, true),
    ],
    nominal_capacity_wh: 1152.0,
};

// Provisional: no merged upstream profile yet (bluetti-bt-lib PR #49).
// Uses the generic V2 layout, so every field needs a hardware check.
static ELITE_200_V2: Profile = Profile {
    model: Model::Elite200V2,
    protocol: Protocol::V2,
    blocks: &[(100, 20), (140, 8), (1300, 15)],
    fields: &[
        field(PackVoltage, 100, 0.01, false),
        field(PackCurrent, 101, 0.1, false),
        field(BatterySoc, 102, 1.0, false),
        field(DcOutputPower, 140, 1.0, false),
        field(AcOutputPower, 142, 1.0, false),
        field(DcInputPower, 144, 1.0, false),
        field(AcInputPower, 146, 1.0, false),
        field(AcInputFrequency, 1300, 0.1, false),
        field(AcInputVoltage, 1314, 0.1, false),
    ],
    nominal_capacity_wh: 2073.6,
};

/// Register values collected during one poll, keyed by address.
#[derive(Debug, Default, Clone)]
pub struct Registers(BTreeMap<u16, u16>);

impl Registers {
    pub fn insert_block(&mut self, start: u16, values: &[u16]) {
        for (offset, &value) in values.iter().enumerate() {
            if let Some(register) = start.checked_add(offset as u16) {
                self.0.insert(register, value);
            }
        }
    }

    pub fn get(&self, register: u16) -> Option<u16> {
        self.0.get(&register).copied()
    }
}

/// Decodes all profile fields present in `registers`. Missing registers and
/// implausible values are skipped, so a partial poll still yields data.
pub fn decode(profile: &Profile, registers: &Registers) -> BTreeMap<Metric, f64> {
    profile
        .fields
        .iter()
        .filter_map(|f| {
            let value = f64::from(registers.get(f.register)?) * f.scale;
            f.metric.is_plausible(value).then_some((f.metric, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_model_from_ble_name() {
        assert_eq!(Model::from_ble_name("EB3A2235000012345"), Some(Model::Eb3a));
        assert_eq!(Model::from_ble_name("AC1802335000123456"), Some(Model::Ac180));
        assert_eq!(Model::from_ble_name("E200V22501000012345"), Some(Model::Elite200V2));
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
        registers.insert_block(70, &[0; 7].iter().copied().chain([2301]).collect::<Vec<_>>());

        let metrics = decode(Model::Eb3a.profile(), &registers);
        assert_eq!(metrics[&BatterySoc], 87.0);
        assert_eq!(metrics[&AcInputPower], 12.0);
        assert_eq!(metrics[&AcOutputOn], 1.0);
        assert!((metrics[&AcInputVoltage] - 230.1).abs() < 1e-9);
        assert!(!metrics.contains_key(&DcInputVoltage), "register 86 was not polled");
    }

    #[test]
    fn drops_implausible_soc() {
        let mut registers = Registers::default();
        registers.insert_block(102, &[250]);
        assert!(!decode(Model::Ac180.profile(), &registers).contains_key(&BatterySoc));
    }
}
