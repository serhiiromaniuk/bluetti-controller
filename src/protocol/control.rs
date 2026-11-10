//! The write allow-list (ROADMAP.md §4.3 "Write allow-list", ADR-010).
//!
//! The only registers this project ever writes are the AC/DC output switches
//! listed here. [`OutputSwitch`] has private fields and no public constructor,
//! so [`switches`] is the only way to hold one and no code path can name
//! another register (golden rule #2 enforced by the type). Writing still needs
//! `control.enabled = true`, checked by the caller's gate.

use super::device::Model;

/// A station output that can be switched on or off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Output {
    Ac,
    Dc,
}

impl Output {
    /// The lowercase key used in the CLI, the web API and the database.
    pub fn key(self) -> &'static str {
        match self {
            Output::Ac => "ac",
            Output::Dc => "dc",
        }
    }

    /// Parses a [`key`](Self::key); anything else is `None`.
    pub fn from_key(key: &str) -> Option<Output> {
        match key {
            "ac" => Some(Output::Ac),
            "dc" => Some(Output::Dc),
            _ => None,
        }
    }
}

/// One allowed output switch: the register written (value 1 = on, 0 = off) and
/// the register read back afterwards to confirm the state.
#[derive(Debug, PartialEq, Eq)]
pub struct OutputSwitch {
    output: Output,
    register: u16,
    read_back: u16,
    verified: bool,
}

impl OutputSwitch {
    /// The output this switch controls.
    pub fn output(&self) -> Output {
        self.output
    }

    /// The holding register written with function 0x06.
    pub fn register(&self) -> u16 {
        self.register
    }

    /// The register read after the write to confirm the state.
    pub fn read_back(&self) -> u16 {
        self.read_back
    }

    /// Whether the pair is confirmed on real hardware (HUMAN-CHECKLIST H14–H16).
    pub fn verified(&self) -> bool {
        self.verified
    }
}

const EB3A: [OutputSwitch; 2] = [
    OutputSwitch {
        output: Output::Ac,
        register: 3007,
        read_back: 48,
        verified: false,
    },
    OutputSwitch {
        output: Output::Dc,
        register: 3008,
        read_back: 49,
        verified: false,
    },
];

const AC180: [OutputSwitch; 2] = [
    OutputSwitch {
        output: Output::Ac,
        register: 2011,
        read_back: 2011,
        verified: false,
    },
    OutputSwitch {
        output: Output::Dc,
        register: 2012,
        read_back: 2012,
        verified: false,
    },
];

/// The output switches allowed for `model`. The Elite 200 V2 has none until
/// H16 confirms its 2011/2012 pair (ROADMAP.md §4.3, PLAN A2).
pub fn switches(model: Model) -> &'static [OutputSwitch] {
    match model {
        Model::Eb3a => &EB3A,
        Model::Ac180 => &AC180,
        Model::Elite200V2 => &[],
    }
}

#[cfg(test)]
mod tests;
