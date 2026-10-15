//! The store writer task (ROADMAP.md §5.3): it owns the only write connection on
//! its own blocking thread, stores every record the collector sends, and drives
//! the §7.1 grid presence and the §7.2 house outage log into `grid_event` rows.
//! It also runs the §6 jobs: rollup every 5 minutes, retention nightly, compaction
//! weekly, timed by the records' own timestamps.

use std::num::NonZeroU32;

use anyhow::{Context, bail};
use rusqlite::Connection;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::read::{last_samples, open_event};
use super::rollup::{compact, retention, rollup};
use super::write::{
    Failure, Sample, close_outage, insert_failure, insert_sample, insert_snapshot, open_outage,
};
use crate::metrics::energy::BUCKET_S;
use crate::metrics::grid::{Grid, GridState, Params};
#[cfg(test)]
use crate::metrics::house::HouseState;
use crate::metrics::house::{Event, House};

/// The `grid_event.scope` of the house outage log.
const HOUSE: &str = "house";

/// One message from the collector; `station` is the id `sync_stations` returned.
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Sample {
        station: i64,
        sample: Sample,
    },
    Failure {
        station: i64,
        failure: Failure,
    },
    /// One register block as read, stored as big-endian u16 values.
    Snapshot {
        station: i64,
        ts: i64,
        start_reg: u16,
        values: Vec<u16>,
    },
}

/// A configured station as the writer sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Station {
    pub id: i64,
    pub name: String,
    /// Votes in the house outage log (§7.2).
    pub grid_sensor: bool,
}

struct Tracked {
    station: Station,
    grid: Grid,
    /// This station's open `grid_event` row: (id, started_at).
    open: Option<(i64, i64)>,
}

/// The single writer. Build it with [`Writer::restore`], run it with [`Writer::spawn`].
pub struct Writer {
    conn: Connection,
    interval_s: i64,
    stations: Vec<Tracked>,
    house: House,
    /// The open house `grid_event` row: (id, started_at).
    house_open: Option<(i64, i64)>,
    raw_retention_days: NonZeroU32,
    /// The 5-minute bucket, UTC day and week the §6 jobs last ran in.
    ran: [Option<i64>; 3],
}

const DAY_S: i64 = 86_400;
/// The §6 job periods: rollup, retention, compaction.
const PERIODS: [i64; 3] = [BUCKET_S, DAY_S, 7 * DAY_S];

impl Writer {
    /// Rebuilds the grid state after a restart (ROADMAP.md §6 "Restart"): each
    /// station's grid from its last `debounce` samples, and every open
    /// `grid_event` row stays open. No extra state table: the rows are the state.
    pub fn restore(
        conn: Connection,
        stations: Vec<Station>,
        params: Params,
        raw_retention_days: NonZeroU32,
    ) -> anyhow::Result<Writer> {
        let mut tracked = Vec::with_capacity(stations.len());
        for station in stations {
            if station.name == HOUSE {
                bail!("station name `{HOUSE}` is reserved for the house outage log");
            }
            let mut grid = Grid::new(params);
            for (ts, values) in last_samples(&conn, station.id, params.debounce.get())? {
                grid.observe(ts, &values);
            }
            let open = open_event(&conn, &station.name)?.map(|e| (e.id, e.started_at));
            tracked.push(Tracked {
                station,
                grid,
                open,
            });
        }
        let house_open = open_event(&conn, HOUSE)?.map(|e| (e.id, e.started_at));
        Ok(Writer {
            conn,
            interval_s: params.interval_s,
            stations: tracked,
            house: House::new(params.interval_s, house_open.map(|(_, start)| start)),
            house_open,
            raw_retention_days,
            ran: [None; 3],
        })
    }

    /// Runs the writer on a blocking thread until every sender is dropped. Each
    /// record is committed before the next is taken, so the return needs no
    /// further flush; a record that fails is logged and skipped.
    pub fn spawn(mut self, mut rx: mpsc::Receiver<Record>) -> JoinHandle<()> {
        tokio::task::spawn_blocking(move || {
            while let Some(record) = rx.blocking_recv() {
                if let Err(err) = self.handle(&record) {
                    tracing::error!(error = format!("{err:#}"), "store writer: record failed");
                }
            }
        })
    }

    /// Stores one record, updates the grid and house events at its time, then
    /// runs any §6 job whose period that time has entered.
    pub fn handle(&mut self, record: &Record) -> anyhow::Result<()> {
        let ts = match record {
            Record::Sample { station, sample } => {
                let index = self.index(*station)?;
                if let Some(cursor) = rolled_past(&self.conn, *station, sample.ts)? {
                    tracing::warn!(
                        station,
                        ts = sample.ts,
                        cursor,
                        "store writer: sample in a rolled-up bucket; it is never rolled up"
                    );
                }
                insert_sample(&mut self.conn, *station, sample)?;
                self.stations[index].grid.observe(sample.ts, &sample.values);
                sample.ts
            }
            Record::Failure { station, failure } => {
                self.index(*station)?;
                insert_failure(&self.conn, *station, failure)?;
                failure.ts
            }
            Record::Snapshot {
                station,
                ts,
                start_reg,
                values,
            } => {
                self.index(*station)?;
                return insert_snapshot(&self.conn, *station, *ts, *start_reg, values);
            }
        };
        self.update_events(ts)?;
        self.maintain(ts).context("store maintenance")
    }

    /// Runs each §6 job once per period, on the first record that enters it (and
    /// once at start). Records arrive in time order from the sequential collector
    /// (§5.3), so a bucket before `ts` takes no more samples once `ts` is seen. A
    /// failed job retries in its next period; the rollup catches up from its cursor.
    fn maintain(&mut self, ts: i64) -> anyhow::Result<()> {
        let due = |job: usize, ran: &mut [Option<i64>; 3]| {
            let period = Some(ts.div_euclid(PERIODS[job]));
            std::mem::replace(&mut ran[job], period) != period
        };
        if due(0, &mut self.ran) {
            for t in &self.stations {
                rollup(&mut self.conn, t.station.id, self.interval_s, ts)?;
            }
        }
        if due(1, &mut self.ran) {
            retention(&mut self.conn, self.raw_retention_days.get(), ts)?;
        }
        if due(2, &mut self.ran) {
            compact(&self.conn)?;
        }
        Ok(())
    }

    fn index(&self, id: i64) -> anyhow::Result<usize> {
        self.stations
            .iter()
            .position(|t| t.station.id == id)
            .with_context(|| format!("station id {id} is not configured"))
    }

    /// Ticks every station at `ts` (amendment A3: the house reads the state after
    /// the tick), then matches each station's open event to its accepted state:
    /// `Absent` opens one, `Present` closes it backdated; `Unknown` changes nothing.
    fn update_events(&mut self, ts: i64) -> anyhow::Result<()> {
        for t in &mut self.stations {
            t.grid.tick(ts);
            let (state, since) = t.grid.state();
            let since = since.unwrap_or(ts);
            match (state, t.open) {
                (GridState::Absent, None) => {
                    let id = open_outage(&self.conn, &t.station.name, since, self.interval_s)?;
                    t.open = Some((id, since));
                }
                (GridState::Present, Some((id, start))) => {
                    close_outage(&self.conn, id, since.max(start))?;
                    t.open = None;
                }
                _ => {}
            }
        }
        let sensors: Vec<_> = self
            .stations
            .iter()
            .filter(|t| t.station.grid_sensor)
            .map(|t| t.grid.state())
            .collect();
        match self.house.update(&sensors) {
            // The house event is edge-based: on a failed write, put `House` back so
            // the next round sees the edge again (amendment A6).
            Some(Event {
                start,
                end: None,
                precision_s,
            }) => match open_outage(&self.conn, HOUSE, start, precision_s) {
                Ok(id) => self.house_open = Some((id, start)),
                Err(err) => {
                    self.house = House::new(self.interval_s, None);
                    return Err(err);
                }
            },
            Some(Event { end: Some(end), .. }) => {
                let (id, start) = self
                    .house_open
                    .take()
                    .context("house outage ended but none is open")?;
                if let Err(err) = close_outage(&self.conn, id, end) {
                    self.house = House::new(self.interval_s, Some(start));
                    self.house_open = Some((id, start));
                    return Err(err);
                }
            }
            None => {}
        }
        Ok(())
    }
}

/// The station's rollup cursor when `ts` falls in a bucket the rollup has
/// passed (amendment A8): such a sample is stored but never rolled up, and
/// retention later deletes it. The collector keeps record times nondecreasing.
fn rolled_past(conn: &Connection, station_id: i64, ts: i64) -> anyhow::Result<Option<i64>> {
    let cursor: Option<i64> = conn.query_row(
        "SELECT max(ts) FROM rollup_5m WHERE station_id = ?1",
        [station_id],
        |row| row.get(0),
    )?;
    Ok(cursor.filter(|cursor| ts < cursor + BUCKET_S))
}

#[cfg(test)]
mod tests;
