//! House outage log (ROADMAP.md §7.2): the sensor stations' grid states vote.

pub use super::grid::GridState;

/// The house's power state, voted by the sensor stations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseState {
    /// Every sensor is unknown or unreachable (and no outage is open).
    Unknown,
    Power,
    Outage,
}

/// One outage, unix seconds. `end` is `None` while it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub start: i64,
    pub end: Option<i64>,
    /// The poll interval: how far `start` and `end` may be off.
    pub precision_s: i64,
}

/// Outage totals over a window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    pub count: usize,
    pub seconds_without_power: i64,
    pub longest_s: i64,
}

/// The §7.2 vote over the sensor stations only. An unreachable station is
/// passed as `Unknown` and does not vote: any `Present` → `Power`; else any
/// `Absent` → `Outage`; else `Unknown`.
pub fn vote(sensors: &[GridState]) -> HouseState {
    if sensors.contains(&GridState::Present) {
        HouseState::Power
    } else if sensors.contains(&GridState::Absent) {
        HouseState::Outage
    } else {
        HouseState::Unknown
    }
}

/// Turns votes into outage events. An open outage stays open through
/// `Unknown` and ends only when a sensor shows grid again (ROADMAP.md §6
/// "Restart").
#[derive(Debug, Clone)]
pub struct House {
    precision_s: i64,
    /// Start of the open outage.
    open: Option<i64>,
    last: HouseState,
}

impl House {
    /// `precision_s` is the poll interval. `open` is the start of the open
    /// `grid_event` row restored after a restart, if any.
    pub fn new(precision_s: i64, open: Option<i64>) -> House {
        House {
            precision_s,
            open,
            last: HouseState::Unknown,
        }
    }

    /// `Outage` while an event is open, else the last vote.
    pub fn state(&self) -> HouseState {
        if self.open.is_some() {
            HouseState::Outage
        } else {
            self.last
        }
    }

    /// Feed every sensor's `Grid::state()` once per poll round.
    ///
    /// Contract (amendment A3): read the state *after* this round's
    /// `Grid::tick(now)` or `Grid::observe(now, ..)` on every sensor — never
    /// rebuild it from `observe`'s return, which can drop the `Unknown` change
    /// of a gap. Returns the opened event (`end: None`) or the closed one.
    /// An outage starts at the latest `since` among the non-`Present` sensors
    /// (when the last one stopped showing grid) and ends at the earliest
    /// `since` among the `Present` ones, never before its start.
    pub fn update(&mut self, sensors: &[(GridState, Option<i64>)]) -> Option<Event> {
        let states: Vec<GridState> = sensors.iter().map(|s| s.0).collect();
        self.last = vote(&states);
        let since = |present: bool| {
            sensors
                .iter()
                .filter(move |s| (s.0 == GridState::Present) == present)
                .filter_map(|s| s.1)
        };
        match (self.last, self.open) {
            (HouseState::Outage, None) => {
                let start = since(false).max()?;
                self.open = Some(start);
                Some(self.event(start, None))
            }
            (HouseState::Power, Some(start)) => {
                self.open = None;
                let end = since(true).min().unwrap_or(start).max(start);
                Some(self.event(start, Some(end)))
            }
            _ => None,
        }
    }

    fn event(&self, start: i64, end: Option<i64>) -> Event {
        Event {
            start,
            end,
            precision_s: self.precision_s,
        }
    }
}

/// Totals over the events that overlap `[from, to)`. Each event is clipped to
/// the window (an open one ends at `to`), so `seconds_without_power` and
/// `longest_s` count only the part inside it.
pub fn totals(events: &[Event], from: i64, to: i64) -> Totals {
    let mut t = Totals::default();
    for ev in events {
        let (start, end) = (ev.start.max(from), ev.end.unwrap_or(to).min(to));
        if ev.start >= to || ev.end.unwrap_or(to) <= from {
            continue;
        }
        let secs = end - start;
        t.count += 1;
        t.seconds_without_power += secs;
        t.longest_s = t.longest_s.max(secs);
    }
    t
}

#[cfg(test)]
mod tests;
