//! Device models, register maps and decoding of polled register blocks.
//!
//! Register addresses come from the community projects listed in
//! `ROADMAP.md` §4.3 (bluetti-bt-lib, bluetti_mqtt). `verified` means confirmed
//! on our own hardware (RULES.md conventions), so every field starts `false`
//! until the M1/M2 acceptance runs.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

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
        split_ble_name(name).map(|(model, _)| model)
    }

    pub fn profile(self) -> &'static Profile {
        match self {
            Model::Eb3a => &EB3A,
            Model::Ac180 => &AC180,
            Model::Elite200V2 => &ELITE_200_V2,
        }
    }
}

/// Splits an advertised BLE name `<MODEL><serial digits>` into the model and
/// its serial digits.
pub fn split_ble_name(name: &str) -> Option<(Model, &str)> {
    NAME_PREFIXES.iter().find_map(|(prefix, model)| {
        let serial = name.strip_prefix(prefix)?;
        let is_serial = !serial.is_empty() && serial.bytes().all(|b| b.is_ascii_digit());
        is_serial.then_some((*model, serial))
    })
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

    /// Plausible decoded values, shared by every model; a value outside fails
    /// the poll (RULES.md v0.3 gate). Each bound admits 0, the idle reading.
    pub const fn range(self) -> RangeInclusive<f64> {
        let (min, max) = match self {
            // A state of charge is a percentage.
            Metric::BatterySoc => (0.0, 100.0),
            // ~2x the largest inverter (Elite 200 V2, 2600 W) and above the
            // AC180's 2700 W power lifting; catches garbage such as 0xFFFF.
            Metric::AcInputPower
            | Metric::DcInputPower
            | Metric::AcOutputPower
            | Metric::DcOutputPower => (0.0, 5000.0),
            // 230 V nominal grid +30%; 0 while unplugged (an outage).
            Metric::AcInputVoltage => (0.0, 300.0),
            // 50/60 Hz grid with headroom; 0 while unplugged.
            Metric::AcInputFrequency => (0.0, 70.0),
            // Low-voltage solar/car input: tens of volts, 150 V is far above.
            Metric::DcInputVoltage => (0.0, 150.0),
            // A switch is 0 (off) or 1 (on).
            Metric::AcOutputOn | Metric::DcOutputOn => (0.0, 1.0),
            // 16S LiFePO4 at 3.65 V/cell = 58.4 V full.
            Metric::PackVoltage => (0.0, 60.0),
            // 2600 W at a 40 V empty pack ~ 65 A; signed for charge/discharge.
            Metric::PackCurrent => (-100.0, 100.0),
        };
        RangeInclusive::new(min, max)
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
    Field {
        metric,
        register,
        scale,
        verified,
    }
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
        field(DcInputPower, 36, 1.0, false),
        field(AcInputPower, 37, 1.0, false),
        field(AcOutputPower, 38, 1.0, false),
        field(DcOutputPower, 39, 1.0, false),
        field(BatterySoc, 43, 1.0, false),
        field(AcOutputOn, 48, 1.0, false),
        field(DcOutputOn, 49, 1.0, false),
        field(AcInputVoltage, 77, 0.1, false),
        field(DcInputVoltage, 86, 0.01, false),
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
        field(BatterySoc, 102, 1.0, false),
        field(DcOutputPower, 140, 1.0, false),
        field(AcOutputPower, 142, 1.0, false),
        field(DcInputPower, 144, 1.0, false),
        field(AcInputPower, 146, 1.0, false),
        field(DcInputVoltage, 1213, 0.1, false),
        field(AcInputFrequency, 1300, 0.1, false),
        field(AcInputVoltage, 1314, 0.1, false),
        field(AcOutputOn, 2011, 1.0, false),
        field(DcOutputOn, 2012, 1.0, false),
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

/// A profile field that fails the plausibility check of one poll.
#[derive(Debug, Clone, PartialEq)]
pub enum Problem {
    /// The field's register was not in the poll.
    Missing { metric: Metric },
    /// The decoded value lies outside [`Metric::range`].
    OutOfRange { metric: Metric, value: f64 },
}

/// Names every profile field, in profile order, that is missing from
/// `registers` or decodes outside its [`Metric::range`]. Empty means the poll
/// is plausible. [`decode`] is unaffected: it still keeps what it can.
pub fn problems(profile: &Profile, registers: &Registers) -> Vec<Problem> {
    profile
        .fields
        .iter()
        .filter_map(|f| {
            let Some(raw) = registers.get(f.register) else {
                return Some(Problem::Missing { metric: f.metric });
            };
            let value = f64::from(raw) * f.scale;
            (!f.metric.range().contains(&value)).then_some(Problem::OutOfRange {
                metric: f.metric,
                value,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
