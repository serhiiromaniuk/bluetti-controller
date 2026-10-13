//! The collector (ROADMAP.md §5.3–§5.5): one task that polls every station in
//! turn, one link at a time, on ticks at `start + k × interval`, and sends each
//! result to the store writer as a [`Record`]. Every poll runs under the §5.4
//! deadline and ends with a disconnect by address, also on error, deadline or
//! shutdown (golden rule #5). A `watch` channel pauses or stops it. The first
//! successful poll of each UTC hour also sends every profile block as a
//! register snapshot (§6).

use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::config::StationConfig;
use crate::protocol::device;
use crate::store::{Failure, Record, Sample, Stage};
use crate::transport::read::{ReadPolicy, read_profile};
use crate::transport::station::Poll;
use crate::transport::{self, Gatt};

pub mod bluez;

pub use bluez::BluezConnect;

/// Consecutive failures that make a station unreachable (§5.5).
const UNREACHABLE_AFTER: u32 = 3;

/// What the collector should do; the sender is the `serve` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Run,
    /// Skip every poll that would start before `until`.
    Pause {
        until: Instant,
    },
    /// End the current poll (with its disconnect) and return.
    Shutdown,
}

/// Opens one station as a [`Gatt`]. No BlueZ type appears here, so tests
/// drive the collector with a fake.
pub trait Connect {
    type Gatt: Gatt + Send;

    /// Finds and connects `station` (key exchange included), returning the
    /// channel and whether it is encrypted. `stage` starts at
    /// [`Stage::Discover`]; the call moves it as it gets further, so a failure
    /// or a fired deadline names where the poll stopped. `retry: false` is a
    /// station already unreachable: no fresh scan, one connect attempt (§5.4).
    fn open(
        &self,
        station: &StationConfig,
        retry: bool,
        stage: &mut Stage,
    ) -> impl Future<Output = Result<(Self::Gatt, bool), String>> + Send;

    /// Disconnects `station` by address, whatever a cancelled poll left up.
    fn release(&self, station: &StationConfig) -> impl Future<Output = ()> + Send;
}

/// A configured station with its `station.id` from `sync_stations`.
#[derive(Debug, Clone)]
pub struct Target {
    pub id: i64,
    pub config: StationConfig,
}

pub struct Collector<C> {
    pub connect: C,
    pub stations: Vec<Target>,
    pub interval: Duration,
    pub policy: ReadPolicy,
    pub records: mpsc::Sender<Record>,
}

impl<C: Connect + Sync> Collector<C> {
    /// Polls until [`Control::Shutdown`], the control sender is dropped, or
    /// the writer is gone. A round that overruns skips the ticks it missed.
    pub async fn run(self, mut control: watch::Receiver<Control>) {
        // §5.4 per-round budget: a round of n stations fits one interval.
        let deadline =
            self.interval / u32::try_from(self.stations.len().max(1)).unwrap_or(u32::MAX);
        let start = Instant::now();
        let mut tick = 0_u32;
        let mut failures = vec![0_u32; self.stations.len()];
        let mut hours = vec![None; self.stations.len()];
        let mut clock = Clock::new();
        loop {
            let at = start + self.interval.saturating_mul(tick);
            tokio::select! {
                () = tokio::time::sleep_until(at) => {}
                () = shutdown(&mut control) => return,
            }
            for ((target, failed), hour) in self.stations.iter().zip(&mut failures).zip(&mut hours)
            {
                match *control.borrow() {
                    Control::Shutdown => return,
                    Control::Pause { until } if Instant::now() < until => continue,
                    _ => {}
                }
                let ts = clock.now();
                let retry = *failed < UNREACHABLE_AFTER;
                let mut stage = Stage::Discover;
                let outcome = tokio::select! {
                    result = tokio::time::timeout(deadline, self.poll(target, ts, retry, &mut stage)) => Some(result),
                    () = shutdown(&mut control) => None,
                };
                self.connect.release(&target.config).await;
                let Some(outcome) = outcome else { return };
                let result = outcome.unwrap_or_else(|_| {
                    Err(Failure {
                        ts,
                        stage,
                        error: format!("no result within the {deadline:?} poll deadline"),
                    })
                });
                let name = &target.config.name;
                let mut snapshot = Vec::new();
                let record = match result {
                    Ok((sample, blocks)) => {
                        if *failed >= UNREACHABLE_AFTER {
                            tracing::info!(station = name, "reachable again");
                        }
                        *failed = 0;
                        if new_hour(hour, ts) {
                            snapshot = blocks
                                .into_iter()
                                .map(|(start_reg, values)| Record::Snapshot {
                                    station: target.id,
                                    ts,
                                    start_reg,
                                    values,
                                })
                                .collect();
                        }
                        Record::Sample {
                            station: target.id,
                            sample,
                        }
                    }
                    Err(failure) => {
                        *failed += 1;
                        tracing::warn!(
                            station = name,
                            stage = failure.stage.as_str(),
                            error = failure.error,
                            "poll failed"
                        );
                        if *failed == UNREACHABLE_AFTER {
                            tracing::warn!(station = name, "unreachable");
                        }
                        Record::Failure {
                            station: target.id,
                            failure,
                        }
                    }
                };
                for record in std::iter::once(record).chain(snapshot) {
                    if self.records.send(record).await.is_err() {
                        tracing::error!("store writer gone, collector stops");
                        return;
                    }
                }
            }
            tick = next_tick(tick, start.elapsed(), self.interval);
        }
    }

    /// One poll: open, read the profile, always disconnect, decode (§5.4).
    /// A sample comes with every profile block as read, `(start, values)`.
    async fn poll(
        &self,
        target: &Target,
        ts: i64,
        retry: bool,
        stage: &mut Stage,
    ) -> Result<(Sample, Vec<(u16, Vec<u16>)>), Failure> {
        let fail = |stage, error| Failure { ts, stage, error };
        let (mut gatt, encrypted) = self
            .connect
            .open(&target.config, retry, stage)
            .await
            .map_err(|error| fail(*stage, error))?;
        *stage = Stage::Read;
        let profile = target.config.model.profile();
        let (read, closed) = transport::with_disconnect(&mut gatt, async |gatt| {
            read_profile(gatt, profile, &self.policy).await
        })
        .await;
        if let Err(err) = closed {
            tracing::warn!(station = target.config.name, error = %err, "disconnect failed");
        }
        let registers = read.map_err(|err| fail(Stage::Read, err.to_string()))?;
        let poll = Poll {
            values: device::decode(profile, &registers),
            problems: device::problems(profile, &registers),
        };
        let sample = Sample::from_poll(ts, encrypted, &poll)?;
        // read_profile succeeded, so every register of every block is present.
        let blocks = profile
            .blocks
            .iter()
            .map(|&(start, count)| {
                let values = (start..start + count)
                    .filter_map(|register| registers.get(register))
                    .collect();
                (start, values)
            })
            .collect();
        Ok((sample, blocks))
    }
}

/// The first tick after `tick` not already past at `elapsed`: a late round
/// skips the ticks it missed instead of bursting them (§5.4).
fn next_tick(tick: u32, elapsed: Duration, interval: Duration) -> u32 {
    let due = elapsed.as_nanos().div_ceil(interval.as_nanos());
    u32::try_from(due)
        .unwrap_or(u32::MAX)
        .max(tick.saturating_add(1))
}

/// True when `ts` falls in a later UTC hour than the last snapshot, which
/// `last` then records: one snapshot set per station-hour (§6).
fn new_hour(last: &mut Option<i64>, ts: i64) -> bool {
    let hour = ts.div_euclid(3600);
    let due = *last != Some(hour);
    *last = Some(hour);
    due
}

/// Resolves on [`Control::Shutdown`] or when the sender is gone.
async fn shutdown(control: &mut watch::Receiver<Control>) {
    let _ = control.wait_for(|c| *c == Control::Shutdown).await;
}

/// Record timestamps, nondecreasing so the writer's jobs see them in order
/// (amendment A8): the wall clock, never below the previous value, nor below
/// the start plus monotonic time elapsed (which also moves in paused tests).
struct Clock {
    anchor: i64,
    start: Instant,
    last: i64,
}

impl Clock {
    fn new() -> Clock {
        let anchor = unix_now();
        Clock {
            anchor,
            start: Instant::now(),
            last: anchor,
        }
    }

    fn now(&mut self) -> i64 {
        let elapsed = i64::try_from(self.start.elapsed().as_secs()).unwrap_or(i64::MAX);
        self.last = self
            .last
            .max(unix_now())
            .max(self.anchor.saturating_add(elapsed));
        self.last
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests;
