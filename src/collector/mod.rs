//! The collector (ROADMAP.md §5.3–§5.5): one task that polls every station in
//! turn, one link at a time, on ticks at `start + k × interval`, and sends each
//! result to the store writer as a [`Record`]. Every poll runs under the §5.4
//! deadline and ends with a disconnect: its own, or one by address on error,
//! deadline or shutdown (golden rule #5). A `watch` channel pauses or stops it. The first
//! successful poll of each UTC hour also sends every profile block as a
//! register snapshot (§6).

use std::future::Future;
use std::time::Duration;

use tokio::sync::{mpsc, watch};
use tokio::time::Instant;

use crate::config::StationConfig;
use crate::protocol::device;
use crate::store::{Failure, Record, Sample, Stage};
use crate::transport::read::{ReadError, ReadPolicy, read_profile, read_slow};
use crate::transport::station::Poll;
use crate::transport::{self, Gatt};

pub mod bluez;

pub use bluez::BluezConnect;

/// Consecutive failures that make a station unreachable (§5.5).
const UNREACHABLE_AFTER: u32 = 3;
/// The longest an unreachable station waits between polls: each miss doubles
/// the wait up to this, so an absent station costs one 15 s scan per 10 min,
/// not per minute (tasks/2026-10-27_ble-reliability/INVESTIGATION.md R3).
const BACKOFF_CAP: Duration = Duration::from_secs(600);

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

/// How far one open got, for the failure record and its log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    /// Where the poll stopped; starts at [`Stage::Discover`].
    pub stage: Stage,
    /// Whether discovery found the station in BlueZ's cache (warm) or had to
    /// scan (cold); `None` until discovery ends.
    pub cached: Option<bool>,
    /// The first key-exchange error while its retry runs: the poll's failure
    /// stays a `handshake` one, also when the deadline cuts the retry.
    retrying: Option<String>,
    /// The poll's own disconnect succeeded, so no release by address is owed.
    disconnected: bool,
}

impl Progress {
    fn new() -> Progress {
        Progress {
            stage: Stage::Discover,
            cached: None,
            retrying: None,
            disconnected: false,
        }
    }
}

/// Opens one station as a [`Gatt`]. No BlueZ type appears here, so tests
/// drive the collector with a fake.
pub trait Connect {
    type Gatt: Gatt + Send;

    /// Finds and connects `station` (key exchange included), returning the
    /// channel and whether it is encrypted. The call moves `progress` as it
    /// gets further, so a failure or a fired deadline names where the poll
    /// stopped. `retry: false` is a station already unreachable: one connect
    /// attempt (§5.4).
    fn open(
        &self,
        station: &StationConfig,
        retry: bool,
        progress: &mut Progress,
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
        // The first tick each station may be polled on again.
        let mut due = vec![0_u32; self.stations.len()];
        let mut hours = vec![None; self.stations.len()];
        let mut last_ok: Vec<Option<Instant>> = vec![None; self.stations.len()];
        // When each station's slow blocks were last read (ROADMAP.md §2.2 R1a).
        let mut slow_at: Vec<Option<Instant>> = vec![None; self.stations.len()];
        let slow_every = Duration::from_secs(device::SLOW_EVERY_S);
        let mut clock = Clock::new(self.stations.len());
        loop {
            let at = start + self.interval.saturating_mul(tick);
            tokio::select! {
                () = tokio::time::sleep_until(at) => {}
                () = shutdown(&mut control) => return,
            }
            let stations = self.stations.iter().zip(&mut failures).zip(&mut hours);
            for (i, ((target, failed), hour)) in stations.enumerate() {
                match *control.borrow() {
                    Control::Shutdown => return,
                    Control::Pause { until } if Instant::now() < until => continue,
                    _ => {}
                }
                if tick < due[i] {
                    continue;
                }
                let ts = clock.now(i);
                let retry = *failed < UNREACHABLE_AFTER;
                let mut progress = Progress::new();
                // Slow blocks get half the deadline from the poll start, so
                // the core sample always has the other half.
                // Measured from poll start to poll start, with half an
                // interval of slack, so a slow read lands every 10th poll.
                let started = Instant::now();
                let slow = !target.config.model.profile().slow_blocks.is_empty()
                    && slow_at[i]
                        .is_none_or(|at: Instant| at.elapsed() + self.interval / 2 >= slow_every);
                let slow_until = slow.then(|| started + deadline / 2);
                let outcome = tokio::select! {
                    result = tokio::time::timeout(deadline, self.poll(target, ts, retry, slow_until, &mut progress)) => Some(result),
                    () = shutdown(&mut control) => None,
                };
                // A second Disconnect after a clean one only made bluetoothd
                // log "No matching connection" once per poll (INVESTIGATION.md R2).
                if !progress.disconnected {
                    self.connect.release(&target.config).await;
                }
                let Some(outcome) = outcome else { return };
                let result = outcome.unwrap_or_else(|_| {
                    let error = format!("no result within the {deadline:?} poll deadline");
                    Err(match &progress.retrying {
                        Some(first) => Failure {
                            ts,
                            stage: Stage::Handshake,
                            error: format!("{first}; retry on a fresh connection: {error}"),
                        },
                        None => Failure {
                            ts,
                            stage: progress.stage,
                            error,
                        },
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
                        last_ok[i] = Some(Instant::now());
                        if slow {
                            slow_at[i] = Some(started);
                        }
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
                        let (cache, since_success) = failure_fields(progress.cached, last_ok[i]);
                        tracing::warn!(
                            station = name,
                            stage = failure.stage.as_str(),
                            cache,
                            since_success,
                            error = failure.error,
                            "poll failed"
                        );
                        if *failed == UNREACHABLE_AFTER {
                            tracing::warn!(station = name, "unreachable");
                        }
                        if *failed >= UNREACHABLE_AFTER {
                            due[i] = tick.saturating_add(backoff(*failed, self.interval));
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
    /// A failed key exchange is retried once on a fresh connection, unless the
    /// station is unreachable (phase-4 PLAN A23 part 2); the deadline bounds both.
    /// A sample comes with every profile block as read, `(start, values)`.
    /// With `slow_until`, the slow blocks follow the core ones, cut at that
    /// instant; whatever they miss only drops extras.
    async fn poll(
        &self,
        target: &Target,
        ts: i64,
        retry: bool,
        slow_until: Option<Instant>,
        progress: &mut Progress,
    ) -> Result<(Sample, Vec<(u16, Vec<u16>)>), Failure> {
        let fail = |stage, error| Failure { ts, stage, error };
        let (mut gatt, encrypted) = match self.connect.open(&target.config, retry, progress).await {
            Err(first) if retry && progress.stage == Stage::Handshake => {
                tracing::info!(
                    station = target.config.name,
                    error = first,
                    "key exchange failed, retrying once on a fresh connection"
                );
                // Set before the release, so a deadline during it keeps `first` (A30).
                progress.retrying = Some(first);
                self.connect.release(&target.config).await;
                progress.cached = None;
                let opened = self.connect.open(&target.config, retry, progress).await;
                let first = progress.retrying.take().unwrap_or_default();
                opened.map_err(|second| {
                    let error = format!("{first}; retry on a fresh connection: {second}");
                    fail(Stage::Handshake, error)
                })?
            }
            opened => opened.map_err(|error| fail(progress.stage, error))?,
        };
        progress.stage = Stage::Read;
        let profile = target.config.model.profile();
        let (read, closed) = transport::with_disconnect(&mut gatt, async |gatt| {
            let mut registers = read_profile(gatt, profile, &self.policy).await?;
            if let Some(until) = slow_until {
                let slow = read_slow(gatt, profile, &self.policy, &mut registers);
                if tokio::time::timeout_at(until, slow).await.is_err() {
                    tracing::warn!(
                        station = target.config.name,
                        "slow blocks cut at half the poll deadline"
                    );
                }
            }
            Ok::<_, ReadError>(registers)
        })
        .await;
        match closed {
            Ok(()) => progress.disconnected = true,
            Err(err) => {
                tracing::warn!(station = target.config.name, error = %err, "disconnect failed");
            }
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

/// Ticks until the next poll of a station after its `failed`-th miss in a
/// row (≥ [`UNREACHABLE_AFTER`]): 2, 4, 8 … up to [`BACKOFF_CAP`].
fn backoff(failed: u32, interval: Duration) -> u32 {
    let cap = BACKOFF_CAP.as_nanos() / interval.as_nanos().max(1);
    let cap = u32::try_from(cap).unwrap_or(u32::MAX).max(1);
    1_u32
        .checked_shl(failed - UNREACHABLE_AFTER + 1)
        .unwrap_or(u32::MAX)
        .min(cap)
}

/// True when `ts` falls in a later UTC hour than the last snapshot, which
/// `last` then records: one snapshot set per station-hour (§6).
fn new_hour(last: &mut Option<i64>, ts: i64) -> bool {
    let hour = ts.div_euclid(3600);
    let due = *last != Some(hour);
    *last = Some(hour);
    due
}

/// The log fields that tell a station-side miss from a host one (phase-4
/// PLAN A23 part 3): the BlueZ cache state (warm / cold / `-` when discovery
/// did not end) and the time since the station's last good poll.
fn failure_fields(cached: Option<bool>, last_ok: Option<Instant>) -> (&'static str, String) {
    let cache = match cached {
        Some(true) => "warm",
        Some(false) => "cold",
        None => "-",
    };
    let since = last_ok.map_or_else(
        || "never".to_owned(),
        |at| format!("{}s", at.elapsed().as_secs()),
    );
    (cache, since)
}

/// Resolves on [`Control::Shutdown`] or when the sender is gone.
async fn shutdown(control: &mut watch::Receiver<Control>) {
    let _ = control.wait_for(|c| *c == Control::Shutdown).await;
}

/// Record timestamps, nondecreasing so the writer's jobs see them in order
/// (amendment A8): the wall clock, never below the previous value, nor below
/// the start plus monotonic time elapsed (which also moves in paused tests).
/// Each station's ts also rises by at least 1 s per record, so a wall clock
/// stepped back never repeats a `(station_id, ts)` key (A9).
/// Trade-off (A9): a wall clock that was ahead at startup and is then set
/// back keeps the records ahead by that offset until a restart; the floor is
/// what moves ts under paused tests, and a few seconds' skew is harmless.
struct Clock {
    anchor: i64,
    start: Instant,
    last: i64,
    /// The last ts per station.
    stations: Vec<i64>,
}

impl Clock {
    fn new(stations: usize) -> Clock {
        let anchor = unix_now();
        Clock {
            anchor,
            start: Instant::now(),
            last: anchor,
            stations: vec![i64::MIN; stations],
        }
    }

    /// The ts of a record for `station` (its index in the collector).
    fn now(&mut self, station: usize) -> i64 {
        let elapsed = i64::try_from(self.start.elapsed().as_secs()).unwrap_or(i64::MAX);
        let previous = &mut self.stations[station];
        self.last = self
            .last
            .max(unix_now())
            .max(self.anchor.saturating_add(elapsed))
            .max(previous.saturating_add(1));
        *previous = self.last;
        self.last
    }
}

#[cfg(not(test))]
fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

/// Tests run on paused tokio time, so the wall clock is frozen: a record's ts
/// then follows virtual time alone. The real second once ticked between two
/// stations of one round (phase-5 A11).
#[cfg(test)]
fn unix_now() -> i64 {
    1_792_022_400
}

#[cfg(test)]
mod tests;
