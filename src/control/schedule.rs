//! Scheduler arithmetic (`ROADMAP.md` §R3 "Scheduler", "Missed runs", Q3).
//! Pure: no I/O, no clock; the caller passes `now` in `general.timezone`.

use jiff::civil::Time;
use jiff::{SignedDuration, Zoned};

use crate::protocol::control::Output;
use crate::store::read::ScheduleEntry;

/// One schedule slot to apply: switch `output` of `station_id` to `on`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due {
    pub station_id: i64,
    pub output: Output,
    pub on: bool,
    /// Unix time of the slot; recorded as `last_due` once applied.
    pub due_ts: i64,
}

/// Per entry, the latest `on`/`off` occurrence at or before `now`, returned
/// when it is newer than the entry's `last_due` (`None` = never run, so the
/// latest due state applies). Only the latest slot counts: a missed `on` and a
/// later missed `off` yield the `off` alone. Errs on a time past 23:59, which
/// the web boundary never stores.
pub fn due(entries: &[ScheduleEntry], now: &Zoned) -> Result<Vec<Due>, jiff::Error> {
    let mut slots = Vec::new();
    for entry in entries {
        let mut latest: Option<(i64, bool)> = None;
        for (minute, on) in [(entry.on_min, true), (entry.off_min, false)] {
            let Some(minute) = minute else { continue };
            let ts = occurrence(minute, now)?;
            if latest.is_none_or(|(best, _)| ts > best) {
                latest = Some((ts, on));
            }
        }
        if let Some((due_ts, on)) = latest
            && entry.last_due.is_none_or(|last| due_ts > last)
        {
            slots.push(Due {
                station_id: entry.station_id,
                output: entry.output,
                on,
                due_ts,
            });
        }
    }
    Ok(slots)
}

/// Unix time of the latest local `minute` after midnight at or before `now`:
/// today's, else yesterday's. A time in a DST gap resolves by `jiff`'s
/// default (compatible) disambiguation.
fn occurrence(minute: u16, now: &Zoned) -> Result<i64, jiff::Error> {
    let time = Time::midnight().checked_add(SignedDuration::from_mins(i64::from(minute)))?;
    let tz = now.time_zone();
    let today = now.date().to_datetime(time).to_zoned(tz.clone())?;
    let slot = if today.timestamp() <= now.timestamp() {
        today
    } else {
        now.date()
            .yesterday()?
            .to_datetime(time)
            .to_zoned(tz.clone())?
    };
    Ok(slot.timestamp().as_second())
}

#[cfg(test)]
mod tests;
