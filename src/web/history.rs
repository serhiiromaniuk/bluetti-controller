//! The history views (ROADMAP.md §8.4 d–e): the energy table and the outage
//! log. Sync DB reads only; handlers call them in `spawn_blocking`. Every type
//! is `Serialize`: `/api/energy` and `/api/outages` return it as JSON, the
//! pages render it, so the field names are the API's and stay stable.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::Serialize;

use crate::metrics::localtime::local_day_bounds;
use crate::metrics::{energy, house};
use crate::store::read::{self, GridEventRow};

/// One local day of the energy table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyDay {
    /// The local date, `YYYY-MM-DD`.
    pub day: String,
    /// The day's local midnight, unix seconds.
    pub from: i64,
    /// The next local midnight, or `now` for today (a partial day).
    pub to: i64,
    /// One row per station, by station id.
    pub stations: Vec<StationEnergy>,
    /// The house outages that overlap the day, clipped to it.
    pub house: HouseDay,
}

/// One station's energy over one day, from `metrics::energy::Totals` (§7.4).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StationEnergy {
    /// `station.name`.
    pub station: String,
    pub energy_in_wh: f64,
    pub energy_out_wh: f64,
    pub stored_delta_wh: f64,
    /// `in − out − Δstored`; shown labelled "estimate" (§7.4).
    pub losses_estimate_wh: f64,
    /// Seconds of gaps longer than 3 poll intervals, counted as missing coverage.
    pub missing_s: i64,
}

/// `metrics::house::totals` over one day (§7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HouseDay {
    pub count: usize,
    /// Hours without power, in seconds.
    pub seconds_without_power: i64,
    pub longest_s: i64,
}

/// One row of the outage log (§8.4 e).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Outage {
    pub id: i64,
    /// `house` or a station name.
    pub scope: String,
    pub started_at: i64,
    /// `None` while the outage is ongoing.
    pub ended_at: Option<i64>,
    pub ongoing: bool,
    /// `ended_at − started_at`, or the duration so far (`now − started_at`).
    pub duration_s: i64,
    /// The poll interval at detection: how far the bounds may be off.
    pub precision_s: i64,
}

/// The energy table: the last `days` local days in `tz`, **newest first**,
/// today included as a partial day ending at `now`. `interval_s` is the poll
/// interval, which sets the gap rule of `energy::totals`.
///
/// shortcut: a day without samples, or a run that stops long before the day
/// ends, reports `missing_s = 0` (only gaps between samples count, §7.4);
/// count those too if the table misleads in practice.
pub fn energy(
    conn: &Connection,
    days: u32,
    now: i64,
    interval_s: i64,
    tz: &TimeZone,
) -> anyhow::Result<Vec<EnergyDay>> {
    let mut bounds = Vec::new();
    let mut ts = now;
    for _ in 0..days {
        let (from, to) = local_day_bounds(ts, tz)?;
        bounds.push((from, to.min(now)));
        ts = from - 1;
    }
    let Some(&(oldest, _)) = bounds.last() else {
        return Ok(Vec::new());
    };
    let stations = read::stations(conn)?;
    let house_events: Vec<house::Event> = read::events_between(conn, oldest, now)?
        .into_iter()
        .filter(|e| e.scope == "house")
        .map(|e| house::Event {
            start: e.started_at,
            end: e.ended_at,
            precision_s: e.precision_s,
        })
        .collect();
    let mut out = Vec::new();
    for (from, to) in bounds {
        let mut rows = Vec::new();
        for station in &stations {
            let samples = read::samples(conn, station.id, from, to)?;
            let refs: Vec<_> = samples.iter().map(|(ts, v)| (*ts, v)).collect();
            let t = energy::totals(&refs, interval_s, station.capacity_wh, from, to);
            rows.push(StationEnergy {
                station: station.name.clone(),
                energy_in_wh: t.energy_in_wh,
                energy_out_wh: t.energy_out_wh,
                stored_delta_wh: t.stored_delta_wh,
                losses_estimate_wh: t.losses_estimate_wh,
                missing_s: t.missing_s,
            });
        }
        let h = house::totals(&house_events, from, to);
        out.push(EnergyDay {
            day: Timestamp::from_second(from)?
                .to_zoned(tz.clone())
                .date()
                .to_string(),
            from,
            to,
            stations: rows,
            house: HouseDay {
                count: h.count,
                seconds_without_power: h.seconds_without_power,
                longest_s: h.longest_s,
            },
        });
    }
    Ok(out)
}

fn outage(e: GridEventRow, now: i64) -> Outage {
    Outage {
        ongoing: e.ended_at.is_none(),
        duration_s: e.ended_at.unwrap_or(now) - e.started_at,
        id: e.id,
        scope: e.scope,
        started_at: e.started_at,
        ended_at: e.ended_at,
        precision_s: e.precision_s,
    }
}

/// The newest `n` `grid_event` rows of every scope, newest `started_at` first
/// (§8.4 e); an open one is ongoing, its duration counted to `now`.
pub fn outages(conn: &Connection, n: u32, now: i64) -> anyhow::Result<Vec<Outage>> {
    Ok(read::last_events(conn, n)?
        .into_iter()
        .map(|e| outage(e, now))
        .collect())
}

/// The events of every scope that overlap `[from, to)`, newest `started_at`
/// first (`/api/outages`, §8.4 f); an open one's duration is counted to `now`.
pub fn outages_between(
    conn: &Connection,
    from: i64,
    to: i64,
    now: i64,
) -> anyhow::Result<Vec<Outage>> {
    Ok(read::events_between(conn, from, to)?
        .into_iter()
        .map(|e| outage(e, now))
        .collect())
}

#[cfg(test)]
mod tests;
