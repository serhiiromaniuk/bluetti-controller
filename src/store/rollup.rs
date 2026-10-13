//! Rollup and retention (ROADMAP.md §6 "M3 decisions"): each completed 5-minute
//! bucket becomes one `rollup_5m` row through `metrics::energy::rollup`; the
//! nightly job deletes rolled-up raw rows past `raw_retention_days`, snapshots and
//! failures past 90 days, and never touches `grid_event`.
//!
//! Progress is `max(rollup_5m.ts)` per station, so a restart continues where the
//! last run stopped and a repeat run inserts nothing.

use std::collections::BTreeMap;

use anyhow::Context;
use rusqlite::{Connection, params};

use crate::metrics::energy::{self, BUCKET_S};
use crate::protocol::device::Metric;

/// `register_snapshot` and `poll_failure` retention (§6).
pub const AUDIT_RETENTION_DAYS: i64 = 90;

const DAY_S: i64 = 86_400;

/// Rolls up every completed bucket of `station_id` after its newest `rollup_5m`
/// row and before the bucket holding `now`, which is still open. Buckets without
/// samples get no row. Returns the rows written.
pub fn rollup(
    conn: &mut Connection,
    station_id: i64,
    interval_s: i64,
    now: i64,
) -> anyhow::Result<usize> {
    let open_bucket = now.div_euclid(BUCKET_S) * BUCKET_S;
    let cursor: Option<i64> = conn.query_row(
        "SELECT max(ts) FROM rollup_5m WHERE station_id = ?1",
        [station_id],
        |row| row.get(0),
    )?;
    let first = cursor.map_or(i64::MIN, |ts| ts + BUCKET_S);
    let samples = load(conn, station_id, first, open_bucket)?;
    let refs: Vec<_> = samples.iter().map(|(ts, v)| (*ts, v)).collect();

    let tx = conn.transaction()?;
    let mut written = 0;
    // `refs[0]` may be the sample before `first`: it only feeds the crossing segment.
    let mut start = refs.iter().position(|(ts, _)| *ts >= first);
    while let Some(i) = start {
        let bucket = refs[i].0.div_euclid(BUCKET_S) * BUCKET_S;
        let end = refs[i..]
            .iter()
            .position(|(ts, _)| *ts >= bucket + BUCKET_S)
            .map_or(refs.len(), |n| i + n);
        let row = energy::rollup(&refs[i.saturating_sub(1)..end], interval_s, bucket)
            .context("a bucket with samples has a rollup")?;
        tx.execute(
            "INSERT INTO rollup_5m (station_id, ts, soc_avg, soc_min, ac_in_w_avg, dc_in_w_avg,
                                    ac_out_w_avg, dc_out_w_avg, energy_in_wh, energy_out_wh, n)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                station_id,
                row.ts,
                row.soc_avg,
                row.soc_min,
                row.ac_in_w_avg,
                row.dc_in_w_avg,
                row.ac_out_w_avg,
                row.dc_out_w_avg,
                row.energy_in_wh,
                row.energy_out_wh,
                row.n,
            ],
        )
        .with_context(|| format!("insert rollup_5m {station_id}@{bucket}"))?;
        written += 1;
        start = (end < refs.len()).then_some(end);
    }
    tx.commit()?;
    Ok(written)
}

/// The station's samples with `from ≤ ts < to`, plus the last one before `from`,
/// oldest first, as the metric maps `energy::rollup` reads.
fn load(
    conn: &Connection,
    station_id: i64,
    from: i64,
    to: i64,
) -> anyhow::Result<Vec<(i64, BTreeMap<Metric, f64>)>> {
    let mut stmt = conn.prepare(
        "SELECT ts, soc, ac_in_w, dc_in_w, ac_out_w, dc_out_w FROM sample
         WHERE station_id = ?1 AND ts < ?3 AND ts >= coalesce(
           (SELECT max(ts) FROM sample WHERE station_id = ?1 AND ts < ?2), ?2)
         ORDER BY ts",
    )?;
    let fields = [
        (Metric::BatterySoc, 1),
        (Metric::AcInputPower, 2),
        (Metric::DcInputPower, 3),
        (Metric::AcOutputPower, 4),
        (Metric::DcOutputPower, 5),
    ];
    let rows = stmt
        .query_map(params![station_id, from, to], |row| {
            let mut values = BTreeMap::new();
            for (metric, column) in fields {
                if let Some(value) = row.get::<_, Option<f64>>(column)? {
                    values.insert(metric, value);
                }
            }
            Ok((row.get(0)?, values))
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// The nightly job: deletes raw `sample`/`sample_extra` rows older than
/// `raw_retention_days` whose bucket is rolled up, keeping each station's newest
/// sample (amendment A2: `station_health` cuts at it), and `register_snapshot`
/// and `poll_failure` rows older than 90 days. `grid_event` is never touched.
pub fn retention(conn: &mut Connection, raw_retention_days: u32, now: i64) -> anyhow::Result<()> {
    let raw_cutoff = now - i64::from(raw_retention_days) * DAY_S;
    let audit_cutoff = now - AUDIT_RETENTION_DAYS * DAY_S;
    let tx = conn.transaction()?;
    for table in ["sample", "sample_extra"] {
        tx.execute(
            &format!(
                "DELETE FROM {table} AS t WHERE ts < ?1
                   AND ts < (SELECT max(ts) + {BUCKET_S} FROM rollup_5m r
                             WHERE r.station_id = t.station_id)
                   AND ts < (SELECT max(ts) FROM sample s WHERE s.station_id = t.station_id)"
            ),
            [raw_cutoff],
        )
        .with_context(|| format!("retention on {table}"))?;
    }
    for table in ["register_snapshot", "poll_failure"] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE ts < ?1"),
            [audit_cutoff],
        )
        .with_context(|| format!("retention on {table}"))?;
    }
    tx.commit()?;
    Ok(())
}

/// The weekly job: `optimize` + `wal_checkpoint(TRUNCATE)` (§6), in that order
/// so the statistics `optimize` writes are checkpointed too.
pub fn compact(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("PRAGMA optimize")?;
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
    Ok(())
}

#[cfg(test)]
mod tests;
