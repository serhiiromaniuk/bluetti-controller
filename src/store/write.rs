//! Rows the single writer stores (ROADMAP.md §6): stations from the config,
//! samples with their extras, poll failures, register snapshots and grid events.
//! Every function takes the writer's connection; none opens one.

use std::collections::BTreeMap;

use anyhow::{Context, ensure};
use rusqlite::{Connection, params};

use crate::config::StationConfig;
use crate::protocol::device::{Metric, Problem};
use crate::transport::station::Poll;

/// The six `sample` columns; every other metric goes to `sample_extra` (§6, §7.5).
const CORE: [Metric; 6] = [
    Metric::BatterySoc,
    Metric::AcInputPower,
    Metric::DcInputPower,
    Metric::AcOutputPower,
    Metric::DcOutputPower,
    Metric::AcInputVoltage,
];

/// Where a poll failed (`poll_failure.stage`, §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Discover,
    Connect,
    Handshake,
    Read,
    Decode,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Stage::Discover => "discover",
            Stage::Connect => "connect",
            Stage::Handshake => "handshake",
            Stage::Read => "read",
            Stage::Decode => "decode",
        }
    }
}

/// One successful poll: the core columns plus every other decoded field.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    /// Unix seconds, UTC.
    pub ts: i64,
    pub encrypted: bool,
    pub values: BTreeMap<Metric, f64>,
}

/// One failed poll.
#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub ts: i64,
    pub stage: Stage,
    pub error: String,
}

impl Sample {
    /// A poll with any problem is a `decode` failure listing them, never a
    /// partial sample (§5.4, phase-4 Q4).
    pub fn from_poll(ts: i64, encrypted: bool, poll: &Poll) -> Result<Sample, Failure> {
        if poll.problems.is_empty() {
            return Ok(Sample {
                ts,
                encrypted,
                values: poll.values.clone(),
            });
        }
        let error = poll
            .problems
            .iter()
            .map(|problem| match problem {
                Problem::Missing { metric } => format!("{} missing", metric.key()),
                Problem::OutOfRange { metric, value } => {
                    format!("{} out of range: {value}", metric.key())
                }
            })
            .collect::<Vec<_>>()
            .join("; ");
        Err(Failure {
            ts,
            stage: Stage::Decode,
            error,
        })
    }
}

/// Whether a station answers polls (§5.5: derived, not stored).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Ok,
    /// The newest 3 or more poll results are failures; `since` is the first of them.
    Unreachable {
        since: i64,
    },
}

/// Upserts every configured station by name and returns `name → id`. A station
/// removed from the config keeps its row and its history.
pub fn sync_stations(
    conn: &mut Connection,
    stations: &[StationConfig],
) -> anyhow::Result<BTreeMap<String, i64>> {
    let tx = conn.transaction()?;
    let mut ids = BTreeMap::new();
    for station in stations {
        let id: i64 = tx
            .query_row(
                "INSERT INTO station (name, model, capacity_wh, grid_sensor)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (name) DO UPDATE SET model = excluded.model,
                   capacity_wh = excluded.capacity_wh, grid_sensor = excluded.grid_sensor
                 RETURNING id",
                params![
                    station.name,
                    station.model.name(),
                    station.model.profile().nominal_capacity_wh,
                    station.grid_sensor,
                ],
                |row| row.get(0),
            )
            .with_context(|| format!("sync station {}", station.name))?;
        ids.insert(station.name.clone(), id);
    }
    tx.commit()?;
    Ok(ids)
}

/// Stores one sample: the core columns in `sample`, the rest in `sample_extra`,
/// in one transaction.
pub fn insert_sample(
    conn: &mut Connection,
    station_id: i64,
    sample: &Sample,
) -> anyhow::Result<()> {
    let core = |metric| sample.values.get(&metric).copied();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO sample (station_id, ts, soc, ac_in_w, dc_in_w, ac_out_w, dc_out_w,
                             ac_in_v, encrypted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            station_id,
            sample.ts,
            core(Metric::BatterySoc),
            core(Metric::AcInputPower),
            core(Metric::DcInputPower),
            core(Metric::AcOutputPower),
            core(Metric::DcOutputPower),
            core(Metric::AcInputVoltage),
            sample.encrypted,
        ],
    )
    .with_context(|| format!("insert sample {station_id}@{}", sample.ts))?;
    {
        let mut extra = tx.prepare(
            "INSERT INTO sample_extra (station_id, ts, metric, value) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (metric, value) in sample.values.iter().filter(|(m, _)| !CORE.contains(m)) {
            extra.execute(params![station_id, sample.ts, metric.key(), value])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn insert_failure(conn: &Connection, station_id: i64, failure: &Failure) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO poll_failure (station_id, ts, stage, error) VALUES (?1, ?2, ?3, ?4)",
        params![
            station_id,
            failure.ts,
            failure.stage.as_str(),
            failure.error
        ],
    )
    .with_context(|| format!("insert poll_failure {station_id}@{}", failure.ts))?;
    Ok(())
}

/// Stores one register block as big-endian u16 values.
pub fn insert_snapshot(
    conn: &Connection,
    station_id: i64,
    ts: i64,
    start_reg: u16,
    values: &[u16],
) -> anyhow::Result<()> {
    let data: Vec<u8> = values.iter().flat_map(|v| v.to_be_bytes()).collect();
    conn.execute(
        "INSERT INTO register_snapshot (station_id, ts, start_reg, data) VALUES (?1, ?2, ?3, ?4)",
        params![station_id, ts, start_reg, data],
    )
    .with_context(|| format!("insert register_snapshot {station_id}@{ts} reg {start_reg}"))?;
    Ok(())
}

/// Opens an outage event for `scope` (`house` or a station name); returns its id.
pub fn open_outage(
    conn: &Connection,
    scope: &str,
    started_at: i64,
    precision_s: i64,
) -> anyhow::Result<i64> {
    conn.execute(
        "INSERT INTO grid_event (scope, kind, started_at, precision_s)
         VALUES (?1, 'outage', ?2, ?3)",
        params![scope, started_at, precision_s],
    )
    .with_context(|| format!("open outage {scope}@{started_at}"))?;
    Ok(conn.last_insert_rowid())
}

/// Closes an open event; closing an unknown or already closed one is an error.
pub fn close_outage(conn: &Connection, id: i64, ended_at: i64) -> anyhow::Result<()> {
    let closed = conn.execute(
        "UPDATE grid_event SET ended_at = ?2 WHERE id = ?1 AND ended_at IS NULL",
        params![id, ended_at],
    )?;
    ensure!(closed == 1, "grid_event {id} is not open");
    Ok(())
}

/// `Unreachable` once the failures after the newest sample number 3 or more.
pub fn station_health(conn: &Connection, station_id: i64) -> anyhow::Result<Health> {
    let (failures, since): (i64, Option<i64>) = conn.query_row(
        "SELECT count(*), min(ts) FROM poll_failure
         WHERE station_id = ?1
           AND ts > coalesce((SELECT max(ts) FROM sample WHERE station_id = ?1), -1)",
        [station_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(match since {
        Some(since) if failures >= 3 => Health::Unreachable { since },
        _ => Health::Ok,
    })
}

#[cfg(test)]
mod tests;
