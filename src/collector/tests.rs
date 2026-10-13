use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use super::*;
use crate::protocol::device::Model;
use crate::protocol::modbus::crc16;
use crate::transport::TransportError;

const INTERVAL: Duration = Duration::from_secs(60);

/// How a fake station answers.
#[derive(Debug, Clone, Copy)]
enum Behave {
    /// Every register reads this value (0 decodes plausibly, 0xFFFF does not).
    Fill(u16),
    /// Connects, then never answers a read.
    Silent,
    /// Connects, then a read never returns.
    Stall,
    /// Opening fails at this stage.
    Fail(Stage),
    /// Opening never returns.
    Hang,
}

#[derive(Default)]
struct State {
    /// The station whose link is up.
    live: Option<String>,
    /// (station, retry, seconds since the test start) per open.
    opens: Vec<(String, bool, u64)>,
    disconnects: usize,
    releases: usize,
}

#[derive(Clone)]
struct Fake {
    behave: Vec<(String, Behave)>,
    state: Arc<Mutex<State>>,
    start: Instant,
    /// How long each disconnect by address takes.
    release_delay: Duration,
}

struct FakeGatt {
    stall: bool,
    fill: Option<u16>,
    replies: VecDeque<Vec<u8>>,
    state: Arc<Mutex<State>>,
}

impl Connect for Fake {
    type Gatt = FakeGatt;

    async fn open(
        &self,
        station: &StationConfig,
        retry: bool,
        stage: &mut Stage,
    ) -> Result<(FakeGatt, bool), String> {
        let behave = self
            .behave
            .iter()
            .find(|(name, _)| *name == station.name)
            .expect("configured")
            .1;
        {
            let mut state = self.state.lock().unwrap();
            assert_eq!(state.live, None, "two links open at once");
            let at = self.start.elapsed().as_secs();
            state.opens.push((station.name.clone(), retry, at));
        }
        *stage = Stage::Connect;
        let fill = match behave {
            Behave::Fill(value) => Some(value),
            Behave::Silent | Behave::Stall => None,
            Behave::Fail(failed) => {
                *stage = failed;
                return Err(format!("{} failed", failed.as_str()));
            }
            Behave::Hang => std::future::pending().await,
        };
        self.state.lock().unwrap().live = Some(station.name.clone());
        let gatt = FakeGatt {
            stall: matches!(behave, Behave::Stall),
            fill,
            replies: VecDeque::new(),
            state: Arc::clone(&self.state),
        };
        Ok((gatt, false))
    }

    async fn release(&self, _station: &StationConfig) {
        tokio::time::sleep(self.release_delay).await;
        let mut state = self.state.lock().unwrap();
        state.live = None;
        state.releases += 1;
    }
}

impl Gatt for FakeGatt {
    /// Queues a Modbus reply to a read request: every register `fill`.
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let Some(fill) = self.fill else {
            return Ok(());
        };
        let count = u16::from_be_bytes([data[4], data[5]]);
        let mut frame = vec![0x01, 0x03, u8::try_from(2 * count).unwrap()];
        for _ in 0..count {
            frame.extend(fill.to_be_bytes());
        }
        frame.extend(crc16(&frame).to_le_bytes());
        self.replies.push_back(frame);
        Ok(())
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.write(data).await
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        if self.stall {
            std::future::pending::<()>().await;
        }
        self.replies
            .pop_front()
            .ok_or(TransportError::Timeout(timeout))
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.state.lock().unwrap().disconnects += 1;
        Ok(())
    }
}

fn collector(
    stations: &[(&str, Behave)],
) -> (Collector<Fake>, Arc<Mutex<State>>, mpsc::Receiver<Record>) {
    let state = Arc::new(Mutex::new(State::default()));
    let fake = Fake {
        behave: stations
            .iter()
            .map(|(name, behave)| ((*name).to_owned(), *behave))
            .collect(),
        state: Arc::clone(&state),
        start: Instant::now(),
        release_delay: Duration::ZERO,
    };
    let targets = stations
        .iter()
        .zip(1..)
        .map(|((name, _), id)| Target {
            id,
            config: StationConfig {
                name: (*name).to_owned(),
                model: Model::Eb3a,
                address: "AA:BB:CC:DD:EE:FF".into(),
                grid_sensor: true,
            },
        })
        .collect();
    let (records, rx) = mpsc::channel(256);
    let collector = Collector {
        connect: fake,
        stations: targets,
        interval: INTERVAL,
        policy: ReadPolicy {
            timeout: Duration::from_secs(5),
            attempts: NonZeroU32::MIN,
            backoff: Duration::ZERO,
        },
        records,
    };
    (collector, state, rx)
}

/// Runs the collector for `secs` of virtual time, then drops it.
async fn run_for(collector: Collector<Fake>, control: watch::Receiver<Control>, secs: u64) {
    let finished = tokio::time::timeout(Duration::from_secs(secs), collector.run(control)).await;
    assert!(finished.is_err(), "collector returned early");
}

fn drain(rx: &mut mpsc::Receiver<Record>) -> Vec<Record> {
    let mut records = Vec::new();
    while let Ok(record) = rx.try_recv() {
        records.push(record);
    }
    records
}

/// (station id, ts, `Ok` or the failure stage) per poll; snapshots left out.
fn summary(records: &[Record]) -> Vec<(i64, i64, &'static str)> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::Sample { station, sample } => Some((*station, sample.ts, "ok")),
            Record::Failure { station, failure } => {
                Some((*station, failure.ts, failure.stage.as_str()))
            }
            Record::Snapshot { .. } => None,
        })
        .collect()
}

/// (station id, ts, start register, values) per snapshot record.
fn snapshots(records: &[Record]) -> Vec<(i64, i64, u16, Vec<u16>)> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::Snapshot {
                station,
                ts,
                start_reg,
                values,
            } => Some((*station, *ts, *start_reg, values.clone())),
            _ => None,
        })
        .collect()
}

#[tokio::test(start_paused = true)]
async fn stations_are_polled_in_turn_and_every_poll_disconnects() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Fill(0)), ("b", Behave::Fill(0))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 150).await;
    let records = summary(&drain(&mut rx));
    let ids: Vec<_> = records.iter().map(|r| (r.0, r.2)).collect();
    assert_eq!(ids, [(1, "ok"), (2, "ok")].repeat(3));
    // One round per tick: 60 s between rounds, nondecreasing within one.
    let t0 = records[0].1;
    let ts: Vec<_> = records.iter().map(|r| r.1 - t0).collect();
    assert_eq!(ts, [0, 0, 60, 60, 120, 120]);
    let state = state.lock().unwrap();
    assert_eq!(
        (state.opens.len(), state.disconnects, state.releases),
        (6, 6, 6)
    );
    assert_eq!(state.live, None);
}

#[tokio::test(start_paused = true)]
async fn each_failure_names_its_stage() {
    let (collector, state, mut rx) = collector(&[
        ("a", Behave::Fail(Stage::Discover)),
        ("b", Behave::Fail(Stage::Handshake)),
        ("c", Behave::Silent),
        ("d", Behave::Fill(0xFFFF)),
    ]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    let stages: Vec<_> = summary(&drain(&mut rx)).iter().map(|r| r.2).collect();
    assert_eq!(stages, ["discover", "handshake", "read", "decode"]);
    // The failed opens left no link; the two that connected disconnected.
    let state = state.lock().unwrap();
    assert_eq!((state.disconnects, state.releases), (2, 4));
}

#[tokio::test(start_paused = true)]
async fn third_failure_makes_a_station_unreachable_with_one_attempt() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Fail(Stage::Connect))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 190).await;
    assert_eq!(drain(&mut rx).len(), 4);
    let retries: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.1).collect();
    assert_eq!(retries, [true, true, true, false]);
}

#[tokio::test(start_paused = true)]
async fn a_hanging_station_never_delays_the_others_ticks() {
    let (collector, state, mut rx) = collector(&[
        ("a", Behave::Hang),
        ("b", Behave::Fill(0)),
        ("c", Behave::Fill(0)),
    ]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 150).await;
    // Deadline 60 s / 3 = 20 s: `a` opens on each tick, `b` and `c` 20 s later.
    let opens: Vec<_> = state
        .lock()
        .unwrap()
        .opens
        .iter()
        .map(|(name, _, at)| (name.clone(), *at))
        .collect();
    let expected: Vec<_> = [0, 60, 120]
        .into_iter()
        .flat_map(|tick| [("a", tick), ("b", tick + 20), ("c", tick + 20)])
        .map(|(name, at)| (name.to_owned(), at))
        .collect();
    assert_eq!(opens, expected);
    let records = drain(&mut rx);
    let Record::Failure { failure, .. } = &records[0] else {
        panic!("{records:?}")
    };
    assert_eq!(failure.stage, Stage::Connect);
    assert!(failure.error.contains("deadline"), "{}", failure.error);
    assert_eq!(state.lock().unwrap().releases, 9);
}

#[tokio::test(start_paused = true)]
async fn a_deadline_during_the_read_names_the_read_stage() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Stall)]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 70).await;
    let records = drain(&mut rx);
    let Record::Failure { failure, .. } = &records[0] else {
        panic!("{records:?}")
    };
    assert_eq!(failure.stage, Stage::Read);
    assert!(failure.error.contains("deadline"), "{}", failure.error);
    assert_eq!(state.lock().unwrap().releases, 1);
}

#[tokio::test(start_paused = true)]
async fn a_late_round_skips_the_ticks_it_missed() {
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    collector.connect.release_delay = Duration::from_secs(130);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 200).await;
    // The round ending at 130 s missed the 60 s and 120 s ticks: next is 180 s.
    let opens: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.2).collect();
    assert_eq!(opens, [0, 180]);
}

#[tokio::test(start_paused = true)]
async fn shutdown_during_the_sleep_returns_at_once() {
    let (collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let (tx, control) = watch::channel(Control::Run);
    let start = Instant::now();
    let task = tokio::spawn(collector.run(control));
    tokio::time::sleep(Duration::from_secs(10)).await;
    tx.send(Control::Shutdown).unwrap();
    task.await.unwrap();
    assert_eq!(start.elapsed(), Duration::from_secs(10));
    assert_eq!(state.lock().unwrap().opens.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn shutdown_during_a_poll_disconnects_and_stops() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Hang)]);
    let (tx, control) = watch::channel(Control::Run);
    let task = tokio::spawn(collector.run(control));
    tokio::time::sleep(Duration::from_secs(5)).await;
    tx.send(Control::Shutdown).unwrap();
    task.await.unwrap();
    assert!(drain(&mut rx).is_empty());
    assert_eq!(state.lock().unwrap().releases, 1);
}

#[tokio::test(start_paused = true)]
async fn a_dropped_control_sender_stops_the_collector() {
    let (collector, _state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let (tx, control) = watch::channel(Control::Run);
    drop(tx);
    collector.run(control).await;
}

#[tokio::test(start_paused = true)]
async fn pause_skips_polls_until_it_ends() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Fill(0))]);
    let until = Instant::now() + Duration::from_secs(90);
    let (_tx, control) = watch::channel(Control::Pause { until });
    run_for(collector, control, 150).await;
    assert_eq!(summary(&drain(&mut rx)).len(), 1);
    let opens: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.2).collect();
    assert_eq!(opens, [120]);
}

#[tokio::test(start_paused = true)]
async fn the_first_successful_poll_of_each_utc_hour_sends_a_snapshot_set() {
    let (collector, _state, mut rx) = collector(&[("a", Behave::Fill(0))]);
    let (_tx, control) = watch::channel(Control::Run);
    // 62 polls over 3660 s: at least two UTC hours, at least one with two polls.
    run_for(collector, control, 3700).await;
    let records = drain(&mut rx);
    let polls = summary(&records);
    assert_eq!(polls.len(), 62);
    let mut firsts: Vec<i64> = Vec::new();
    for &(_, ts, _) in &polls {
        if firsts
            .last()
            .is_none_or(|first| first.div_euclid(3600) != ts.div_euclid(3600))
        {
            firsts.push(ts);
        }
    }
    assert!(firsts.len() >= 2, "{firsts:?}");
    // One set per hour, at its first poll: every EB3A block, as read.
    let expected: Vec<_> = firsts
        .iter()
        .flat_map(|&ts| [(1, ts, 10, vec![0; 40]), (1, ts, 70, vec![0; 21])])
        .collect();
    assert_eq!(snapshots(&records), expected);
}

#[tokio::test(start_paused = true)]
async fn a_failed_poll_sends_no_snapshot() {
    let (collector, _state, mut rx) = collector(&[
        ("a", Behave::Fail(Stage::Connect)),
        ("b", Behave::Silent),
        ("c", Behave::Fill(0xFFFF)),
    ]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 130).await;
    let records = drain(&mut rx);
    assert_eq!(summary(&records).len(), 9);
    assert!(snapshots(&records).is_empty());
}

#[test]
fn a_new_hour_is_due_once() {
    let mut last = None;
    let due: Vec<_> = [3599, 3600, 3660, 7199, 7200, 7200]
        .into_iter()
        .map(|ts| new_hour(&mut last, ts))
        .collect();
    assert_eq!(due, [true, true, false, false, true, false]);
}

#[test]
fn next_tick_is_the_first_one_not_yet_past() {
    let at = |secs| Duration::from_secs(secs);
    assert_eq!(next_tick(0, at(0), INTERVAL), 1);
    assert_eq!(next_tick(0, at(59), INTERVAL), 1);
    // Due exactly now: not missed, it runs at once.
    assert_eq!(next_tick(0, at(60), INTERVAL), 1);
    assert_eq!(next_tick(0, at(130), INTERVAL), 3);
}

#[test]
fn record_timestamps_never_go_back() {
    let mut clock = Clock::new();
    clock.last = i64::MAX - 1;
    assert_eq!(clock.now(), i64::MAX - 1);
}
