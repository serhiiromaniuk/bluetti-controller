//! The scheduler (`ROADMAP.md` §R3 "Scheduler", "Missed runs", "Retry cap",
//! Q3): [`due`] is the pure slot arithmetic; [`run`] is the `serve` task that
//! reads the table every tick and sends the due switches to the collector.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context as _;
use jiff::civil::Time;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, Zoned};
use rusqlite::Connection;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::MissedTickBehavior;

use super::{Command, PAUSED, Request};
use crate::collector::Target;
use crate::config::ControlConfig;
use crate::protocol::control::Output;
use crate::store::Record;
use crate::store::read::{ScheduleEntry, schedule_entries};

/// The scheduler's tick: one table read per minute (§R3, no reload signal).
const TICK: Duration = Duration::from_secs(60);
/// Consecutive failures per due slot within one service run (§R3 "Retry cap").
pub const MAX_FAILURES: u32 = 10;

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

/// Runs the scheduler until `shutdown` turns `true` (or its sender or the
/// collector is gone). The first tick is immediate, so enabling control
/// applies each entry's latest due state at once.
pub async fn run(scheduler: Scheduler, tz: TimeZone, mut shutdown: watch::Receiver<bool>) {
    let mut scheduler = scheduler;
    let mut ticks = tokio::time::interval(TICK);
    ticks.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = ticks.tick() => {}
            _ = shutdown.wait_for(|stop| *stop) => return,
        }
        let now = Timestamp::now().to_zoned(tz.clone());
        tokio::select! {
            going = scheduler.tick(&now) => if !going {
                return;
            },
            _ = shutdown.wait_for(|stop| *stop) => return,
        }
    }
}

/// The scheduler's state: its read-only connection, the gate's inputs, the
/// channels, and per station output the slot it last applied and the
/// failures of the current slot (both in memory, reset by a restart).
pub struct Scheduler {
    reader: Arc<Mutex<Connection>>,
    stations: Vec<Target>,
    control: ControlConfig,
    commands: mpsc::Sender<Request>,
    records: mpsc::WeakSender<Record>,
    applied: HashMap<(i64, Output), i64>,
    failed: HashMap<(i64, Output), (i64, u32)>,
}

impl Scheduler {
    /// `reader` comes from [`crate::store::open_reader`]; `records` is weak,
    /// so the writer drains and stops once the collector drops its sender.
    pub fn new(
        reader: Connection,
        stations: Vec<Target>,
        control: ControlConfig,
        commands: mpsc::Sender<Request>,
        records: mpsc::WeakSender<Record>,
    ) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            stations,
            control,
            commands,
            records,
            applied: HashMap::new(),
            failed: HashMap::new(),
        }
    }

    /// One pass at `now`: every due slot, one request at a time. `false` once
    /// the collector or the writer is gone (shutdown).
    pub async fn tick(&mut self, now: &Zoned) -> bool {
        let entries = match self.read().await {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!(error = format!("{err:#}"), "schedule read failed");
                return true;
            }
        };
        let entries: Vec<ScheduleEntry> = entries
            .into_iter()
            .map(|mut entry| {
                // A success the writer has not yet stored must not repeat.
                if let Some(&ts) = self.applied.get(&(entry.station_id, entry.output)) {
                    entry.last_due = Some(entry.last_due.map_or(ts, |last| last.max(ts)));
                }
                entry
            })
            .collect();
        let slots = match due(&entries, now) {
            Ok(slots) => slots,
            Err(err) => {
                tracing::warn!(error = %err, "schedule holds an invalid time");
                return true;
            }
        };
        for slot in slots {
            if !self.attempt(slot).await {
                return false;
            }
        }
        true
    }

    async fn read(&self) -> anyhow::Result<Vec<ScheduleEntry>> {
        let reader = Arc::clone(&self.reader);
        tokio::task::spawn_blocking(move || {
            let conn = reader
                .lock()
                .map_err(|_| anyhow::anyhow!("schedule reader poisoned"))?;
            schedule_entries(&conn)
        })
        .await
        .context("schedule read task")?
    }

    /// Sends one due slot through the gate and records its outcome.
    async fn attempt(&mut self, slot: Due) -> bool {
        let key = (slot.station_id, slot.output);
        let failures = match self.failed.get(&key) {
            Some(&(due_ts, n)) if due_ts == slot.due_ts => n,
            _ => 0,
        };
        if failures >= MAX_FAILURES {
            return true;
        }
        let Some(target) = self.stations.iter().find(|t| t.id == slot.station_id) else {
            tracing::debug!(
                station_id = slot.station_id,
                "schedule entry for an unconfigured station"
            );
            return true;
        };
        let station = target.config.name.as_str();
        let output = slot.output.key();
        let command = match Command::new(&self.control, &target.config, slot.output, slot.on) {
            Ok(command) => command,
            Err(refused) => {
                tracing::warn!(station, output, %refused, "scheduled switch refused");
                return true;
            }
        };
        let (reply, answer) = oneshot::channel();
        if self
            .commands
            .send(Request { command, reply })
            .await
            .is_err()
        {
            return false;
        }
        let Ok(answer) = answer.await else {
            return false;
        };
        match answer {
            Ok(_) => {
                self.applied.insert(key, slot.due_ts);
                self.failed.remove(&key);
                tracing::info!(
                    station,
                    output,
                    state = slot.on,
                    due = slot.due_ts,
                    "scheduled switch applied"
                );
                let run = Record::ScheduleRun {
                    station: slot.station_id,
                    output: slot.output,
                    due: slot.due_ts,
                };
                let Some(records) = self.records.upgrade() else {
                    return false;
                };
                return records.send(run).await.is_ok();
            }
            Err(error) if error == PAUSED => {
                tracing::info!(
                    station,
                    output,
                    "scheduled switch waits for the pause to end"
                );
            }
            Err(error) => {
                let n = failures + 1;
                self.failed.insert(key, (slot.due_ts, n));
                if n == MAX_FAILURES {
                    tracing::warn!(
                        station,
                        output,
                        error,
                        "scheduled switch failed {MAX_FAILURES} times; skipped until the next slot"
                    );
                } else {
                    tracing::warn!(
                        station,
                        output,
                        error,
                        failures = n,
                        "scheduled switch failed; retried next tick"
                    );
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests;
