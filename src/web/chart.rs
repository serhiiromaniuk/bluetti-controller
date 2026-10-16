//! The dashboard chart data (`GET /api/chart`): every station's SOC, input
//! and output power and pack temperature on one time axis per station, plus
//! the outages in the window — one request per refresh instead of one per
//! metric and station.
//! Sync DB reads only; the handler calls it in `spawn_blocking`.

use std::collections::BTreeMap;

use rusqlite::Connection;
use serde::Serialize;

use crate::protocol::device::Metric;
use crate::store::read;
use crate::web::history::{self, Outage};

/// No response carries more than this many buckets per station: the step
/// grows with the window, so a 30-day request stays as small as a 12 h one.
pub const MAX_POINTS: i64 = 1000;

/// The `/api/chart` body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Chart {
    pub from: i64,
    pub to: i64,
    /// The bucket width actually used, seconds.
    pub step: i64,
    /// One entry per station row, by id.
    pub stations: Vec<StationSeries>,
    /// Every scope's events overlapping the window, newest first.
    pub outages: Vec<Outage>,
}

/// One station's series; `soc`, `in_w`, `out_w` and `temp_c` are parallel to
/// `t`, `null` where the bucket had no value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StationSeries {
    pub name: String,
    pub t: Vec<i64>,
    pub soc: Vec<Option<f64>>,
    /// `ac_in_w + dc_in_w`; `null` when both are missing.
    pub in_w: Vec<Option<f64>>,
    /// `ac_out_w + dc_out_w`; `null` when both are missing.
    pub out_w: Vec<Option<f64>>,
    /// `pack_temperature` in °C, unverified (R1b); read every 10 min, so most
    /// small buckets are `null`.
    pub temp_c: Vec<Option<f64>>,
}

const METRICS: [Metric; 6] = [
    Metric::BatterySoc,
    Metric::AcInputPower,
    Metric::DcInputPower,
    Metric::AcOutputPower,
    Metric::DcOutputPower,
    Metric::PackTemperature,
];

/// The 5-min rollup's row spacing, read for windows over `read::RAW_MAX_S`.
const ROLLUP_S: i64 = 300;

/// The step for `[from, to)`: `step` (at least 1 s; a multiple of 300 s when
/// the window reads `rollup_5m`) raised to its smallest multiple that holds
/// the window to at most [`MAX_POINTS`] buckets, so rows never fall unevenly
/// into buckets. The handler bounds the inputs, so nothing overflows.
pub fn step_for(from: i64, to: i64, step: i64) -> i64 {
    let span = to - from;
    let mut step = step.max(1);
    if span > read::RAW_MAX_S {
        step = (step + ROLLUP_S - 1) / ROLLUP_S * ROLLUP_S;
    }
    let min = (span + MAX_POINTS - 1) / MAX_POINTS;
    (min + step - 1) / step * step
}

/// Builds the chart body for `[from, to)` at `now`; `step` as [`step_for`].
pub fn chart(conn: &Connection, from: i64, to: i64, step: i64, now: i64) -> anyhow::Result<Chart> {
    let step = step_for(from, to, step);
    let mut stations = Vec::new();
    for row in read::stations(conn)? {
        // Bucket → the six metrics in `METRICS` order.
        let mut joined: BTreeMap<i64, [Option<f64>; 6]> = BTreeMap::new();
        for (i, metric) in METRICS.into_iter().enumerate() {
            let s = read::series(conn, row.id, metric, from, to, step)?;
            for (t, v) in s.t.into_iter().zip(s.v) {
                joined.entry(t).or_default()[i] = Some(v);
            }
        }
        let sum = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (None, None) => None,
            _ => Some(round1(a.unwrap_or(0.0) + b.unwrap_or(0.0))),
        };
        let mut s = StationSeries {
            name: row.name,
            t: Vec::with_capacity(joined.len()),
            soc: Vec::with_capacity(joined.len()),
            in_w: Vec::with_capacity(joined.len()),
            out_w: Vec::with_capacity(joined.len()),
            temp_c: Vec::with_capacity(joined.len()),
        };
        for (t, v) in joined {
            s.t.push(t);
            s.soc.push(v[0].map(round1));
            s.in_w.push(sum(v[1], v[2]));
            s.out_w.push(sum(v[3], v[4]));
            // Stored as read in °F (R1a); the average is linear, so convert after.
            s.temp_c.push(v[5].map(|f| round1((f - 32.0) * 5.0 / 9.0)));
        }
        stations.push(s);
    }
    Ok(Chart {
        from,
        to,
        step,
        stations,
        outages: history::outages_between(conn, from, to, now)?,
    })
}

/// One decimal: bucket averages otherwise serialise as 17 digits.
fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests;
