//! Grid presence per station (ROADMAP.md §7.1): a debounced state machine over
//! one station's successful samples. Pure; the writer task feeds it.

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::protocol::device::Metric;

/// The accepted grid state of one station.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridState {
    /// No accepted reading yet, or no successful sample for > 3 intervals.
    Unknown,
    Present,
    Absent,
}

/// An accepted change of state, backdated to `since` (unix seconds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Change {
    pub state: GridState,
    pub since: i64,
}

/// The §7.1 thresholds (`[metrics]`) plus the poll interval.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    pub voltage_min: f64,
    pub debounce: NonZeroU32,
    pub interval_s: i64,
}

/// Whether one sample shows grid: `ac_in_v ≥ voltage_min`, else (voltage
/// missing) `ac_in_w > 0`; `None` when the sample has neither field.
pub fn reading(values: &BTreeMap<Metric, f64>, voltage_min: f64) -> Option<bool> {
    match values.get(&Metric::AcInputVoltage) {
        Some(volts) => Some(*volts >= voltage_min),
        None => values.get(&Metric::AcInputPower).map(|watts| *watts > 0.0),
    }
}

/// One station's grid presence. Starts `Unknown`.
#[derive(Debug, Clone)]
pub struct Grid {
    params: Params,
    state: GridState,
    since: Option<i64>,
    last_ts: Option<i64>,
    /// The candidate state: (present, first sample ts, consecutive samples).
    pending: Option<(bool, i64, u32)>,
}

impl Grid {
    pub fn new(params: Params) -> Grid {
        Grid {
            params,
            state: GridState::Unknown,
            since: None,
            last_ts: None,
            pending: None,
        }
    }

    /// The accepted state and when it began (`None` before the first change).
    pub fn state(&self) -> (GridState, Option<i64>) {
        (self.state, self.since)
    }

    /// No successful sample for > 3 intervals before `now` → `Unknown`, known
    /// until the last sample. Never an outage.
    pub fn tick(&mut self, now: i64) -> Option<Change> {
        let last = self.last_ts?;
        if now - last <= 3 * self.params.interval_s {
            return None;
        }
        self.pending = None;
        self.last_ts = None;
        self.accept(GridState::Unknown, last)
    }

    /// Feed one successful sample. A sample with no grid field counts as no
    /// sample. Returns the last accepted change, if this sample made one.
    pub fn observe(&mut self, ts: i64, values: &BTreeMap<Metric, f64>) -> Option<Change> {
        let present = reading(values, self.params.voltage_min)?;
        let gap = self.tick(ts);
        self.last_ts = Some(ts);
        let state = if present {
            GridState::Present
        } else {
            GridState::Absent
        };
        if state == self.state {
            self.pending = None;
            return gap;
        }
        let (first, count) = match self.pending {
            Some((p, first, count)) if p == present => (first, count + 1),
            _ => (ts, 1),
        };
        if count < self.params.debounce.get() {
            self.pending = Some((present, first, count));
            return gap;
        }
        self.pending = None;
        self.accept(state, first).or(gap)
    }

    fn accept(&mut self, state: GridState, since: i64) -> Option<Change> {
        if state == self.state {
            return None;
        }
        self.state = state;
        self.since = Some(since);
        Some(Change { state, since })
    }
}

#[cfg(test)]
mod tests;
