//! Read queries for the views and the API (ROADMAP.md §6, §8.2, §8.4). Each takes
//! a connection and writes nothing: the web runs them on the read-only
//! connection, and the writer reuses the ones it needs to restore its state.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, Row, params};

use crate::protocol::device::Metric;

/// The six `sample` columns and the metric each holds (§6).
pub const CORE: [(Metric, &str); 6] = [
    (Metric::BatterySoc, "soc"),
    (Metric::AcInputPower, "ac_in_w"),
    (Metric::DcInputPower, "dc_in_w"),
    (Metric::AcOutputPower, "ac_out_w"),
    (Metric::DcOutputPower, "dc_out_w"),
    (Metric::AcInputVoltage, "ac_in_v"),
];

/// The `rollup_5m` column holding a core metric's 5-minute average.
const ROLLUP: [(Metric, &str); 5] = [
    (Metric::BatterySoc, "soc_avg"),
    (Metric::AcInputPower, "ac_in_w_avg"),
    (Metric::DcInputPower, "dc_in_w_avg"),
    (Metric::AcOutputPower, "ac_out_w_avg"),
    (Metric::DcOutputPower, "dc_out_w_avg"),
];

/// Ranges up to this long read raw rows; longer ones read `rollup_5m` (§8.4 f).
pub const RAW_MAX_S: i64 = 48 * 3600;

const SAMPLE_COLUMNS: &str = "ts, soc, ac_in_w, dc_in_w, ac_out_w, dc_out_w, ac_in_v";

/// One sample's core values; a NULL column is left out.
pub type Values = BTreeMap<Metric, f64>;

/// A `station` row.
#[derive(Debug, Clone, PartialEq)]
pub struct StationRow {
    pub id: i64,
    pub name: String,
    pub model: String,
    pub capacity_wh: f64,
    pub grid_sensor: bool,
}

/// A `grid_event` row; `ended_at` is `None` while the outage is ongoing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridEventRow {
    pub id: i64,
    pub scope: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub precision_s: i64,
}

/// Which table a series came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Raw,
    Rollup5m,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Raw => "raw",
            Source::Rollup5m => "rollup_5m",
        }
    }
}

/// A columnar time series: `t[i]` is the bucket start of `v[i]`, oldest first.
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub source: Source,
    pub t: Vec<i64>,
    pub v: Vec<f64>,
}

/// Every station row, by id.
pub fn stations(conn: &Connection) -> anyhow::Result<Vec<StationRow>> {
    let mut stmt =
        conn.prepare("SELECT id, name, model, capacity_wh, grid_sensor FROM station ORDER BY id")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StationRow {
                id: row.get(0)?,
                name: row.get(1)?,
                model: row.get(2)?,
                capacity_wh: row.get(3)?,
                grid_sensor: row.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn values(row: &Row) -> rusqlite::Result<(i64, Values)> {
    let mut values = BTreeMap::new();
    for (i, (metric, _)) in CORE.iter().enumerate() {
        if let Some(value) = row.get::<_, Option<f64>>(i + 1)? {
            values.insert(*metric, value);
        }
    }
    Ok((row.get(0)?, values))
}

/// The station's newest sample.
pub fn latest_sample(conn: &Connection, station_id: i64) -> anyhow::Result<Option<(i64, Values)>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {SAMPLE_COLUMNS} FROM sample WHERE station_id = ?1
                 ORDER BY ts DESC LIMIT 1"
            ),
            [station_id],
            values,
        )
        .optional()?)
}

/// The station's samples with `from ≤ ts < to`, plus the last one before `from`,
/// oldest first: the input `metrics::energy` integrates over a window.
pub fn samples(
    conn: &Connection,
    station_id: i64,
    from: i64,
    to: i64,
) -> anyhow::Result<Vec<(i64, Values)>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SAMPLE_COLUMNS} FROM sample
         WHERE station_id = ?1 AND ts < ?3 AND ts >= coalesce(
           (SELECT max(ts) FROM sample WHERE station_id = ?1 AND ts < ?2), ?2)
         ORDER BY ts"
    ))?;
    let rows = stmt
        .query_map(params![station_id, from, to], values)?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The station's newest `n` samples that carry a grid field, oldest first.
pub fn last_samples(
    conn: &Connection,
    station_id: i64,
    n: u32,
) -> anyhow::Result<Vec<(i64, Values)>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SAMPLE_COLUMNS} FROM sample
         WHERE station_id = ?1 AND (ac_in_v IS NOT NULL OR ac_in_w IS NOT NULL)
         ORDER BY ts DESC LIMIT ?2"
    ))?;
    let mut rows = stmt
        .query_map(params![station_id, n], values)?
        .collect::<Result<Vec<_>, _>>()?;
    rows.reverse();
    Ok(rows)
}

/// The newest value of every `sample_extra` key of the station: key → (ts, value).
/// Only keys stored within a day of the station's newest extra count, so the
/// scan stays one PK range of ~1 day, not the whole retention (phase-5 A4).
pub fn latest_extras(
    conn: &Connection,
    station_id: i64,
) -> anyhow::Result<BTreeMap<String, (i64, f64)>> {
    // SQLite takes the bare `value` from the row that holds `max(ts)`.
    let mut stmt = conn.prepare(
        "SELECT metric, max(ts), value FROM sample_extra
         WHERE station_id = ?1
           AND ts >= (SELECT max(ts) FROM sample_extra WHERE station_id = ?1) - 86400
         GROUP BY metric",
    )?;
    let rows = stmt
        .query_map([station_id], |row| {
            Ok((row.get(0)?, (row.get(1)?, row.get(2)?)))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn event(row: &Row) -> rusqlite::Result<GridEventRow> {
    Ok(GridEventRow {
        id: row.get(0)?,
        scope: row.get(1)?,
        started_at: row.get(2)?,
        ended_at: row.get(3)?,
        precision_s: row.get(4)?,
    })
}

const EVENT_COLUMNS: &str = "id, scope, started_at, ended_at, precision_s";

/// Events of every scope that overlap `[from, to)` (an open one runs to now),
/// newest `started_at` first.
pub fn events_between(conn: &Connection, from: i64, to: i64) -> anyhow::Result<Vec<GridEventRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {EVENT_COLUMNS} FROM grid_event
         WHERE started_at < ?2 AND (ended_at IS NULL OR ended_at > ?1)
         ORDER BY started_at DESC, id DESC"
    ))?;
    let rows = stmt
        .query_map(params![from, to], event)?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The newest `n` events of every scope, newest `started_at` first.
pub fn last_events(conn: &Connection, n: u32) -> anyhow::Result<Vec<GridEventRow>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {EVENT_COLUMNS} FROM grid_event ORDER BY started_at DESC, id DESC LIMIT ?1"
    ))?;
    let rows = stmt.query_map([n], event)?.collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The newest open outage of `scope` (`house` or a station name).
pub fn open_event(conn: &Connection, scope: &str) -> anyhow::Result<Option<GridEventRow>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {EVENT_COLUMNS} FROM grid_event
                 WHERE scope = ?1 AND ended_at IS NULL ORDER BY id DESC LIMIT 1"
            ),
            [scope],
            event,
        )
        .optional()?)
}

/// One metric of one station over `[from, to)` (§8.4 f). Ranges up to
/// [`RAW_MAX_S`] read raw rows: the `sample` column of a core metric, else the
/// `sample_extra` key. Longer ranges read the `rollup_5m` average where the
/// metric has one, raw rows otherwise. Values are averaged into
/// `ts.div_euclid(step) * step` buckets; `step = 1` keeps every row.
pub fn series(
    conn: &Connection,
    station_id: i64,
    metric: Metric,
    from: i64,
    to: i64,
    step: i64,
) -> anyhow::Result<Series> {
    anyhow::ensure!(step >= 1, "step must be ≥ 1 s, got {step}");
    let rollup = ROLLUP.iter().find(|(m, _)| *m == metric);
    let core = CORE.iter().find(|(m, _)| *m == metric);
    let (source, table, column, filter) = match (rollup, core) {
        (Some((_, column)), _) if to - from > RAW_MAX_S => {
            (Source::Rollup5m, "rollup_5m", *column, "")
        }
        (_, Some((_, column))) => (Source::Raw, "sample", *column, ""),
        _ => (Source::Raw, "sample_extra", "value", "AND metric = ?5"),
    };
    // `div_euclid` in SQL: ts is never negative, so `/` truncation is the same.
    let mut stmt = conn.prepare(&format!(
        "SELECT (ts / ?4) * ?4 AS b, avg({column}) FROM {table}
         WHERE station_id = ?1 AND ts >= ?2 AND ts < ?3 AND {column} IS NOT NULL
           {filter}
         GROUP BY b ORDER BY b"
    ))?;
    let (mut t, mut v) = (Vec::new(), Vec::new());
    let mut rows = if filter.is_empty() {
        stmt.query(params![station_id, from, to, step])?
    } else {
        stmt.query(params![station_id, from, to, step, metric.key()])?
    };
    while let Some(row) = rows.next()? {
        t.push(row.get(0)?);
        v.push(row.get(1)?);
    }
    Ok(Series { source, t, v })
}

#[cfg(test)]
mod tests;
