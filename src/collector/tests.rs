use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use super::*;
use crate::protocol::device::{Metric, Model};
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
    /// The first open fails at this stage, every later one reads 0.
    FailFirst(Stage),
    /// The first open fails at this stage, every later one never returns.
    FailThenHang(Stage),
    /// Opening fails at the connect while the test is younger than this
    /// many seconds, then every open reads 0.
    FailUntil(u64),
    /// Opening never returns.
    Hang,
    /// Reads 0, but its own disconnect fails and leaves the link up.
    DisconnectFails,
    /// Reads 0, but the slow block at 6007 answers a Modbus exception.
    SlowRefused,
    /// Reads 0, but the slow block at 150 never answers.
    SlowStalls,
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
    behave: Behave,
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
        progress: &mut Progress,
    ) -> Result<(FakeGatt, bool), String> {
        let behave = self
            .behave
            .iter()
            .find(|(name, _)| *name == station.name)
            .expect("configured")
            .1;
        let first = {
            let mut state = self.state.lock().unwrap();
            assert_eq!(state.live, None, "two links open at once");
            let at = self.start.elapsed().as_secs();
            state.opens.push((station.name.clone(), retry, at));
            state.opens.iter().filter(|o| o.0 == station.name).count() == 1
        };
        progress.cached = Some(first);
        progress.stage = Stage::Connect;
        let fill = match behave {
            Behave::Fill(value) => Some(value),
            Behave::SlowRefused | Behave::SlowStalls | Behave::DisconnectFails => Some(0),
            Behave::Silent | Behave::Stall => None,
            Behave::Fail(failed) => {
                progress.stage = failed;
                return Err(format!("{} failed", failed.as_str()));
            }
            Behave::FailFirst(failed) | Behave::FailThenHang(failed) if first => {
                progress.stage = failed;
                return Err(format!("{} failed", failed.as_str()));
            }
            Behave::FailFirst(_) => Some(0),
            Behave::FailUntil(secs) if self.start.elapsed().as_secs() < secs => {
                return Err("connect failed".into());
            }
            Behave::FailUntil(_) => Some(0),
            Behave::FailThenHang(_) => std::future::pending().await,
            Behave::Hang => std::future::pending().await,
        };
        self.state.lock().unwrap().live = Some(station.name.clone());
        let gatt = FakeGatt {
            stall: matches!(behave, Behave::Stall),
            behave,
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
        let start = u16::from_be_bytes([data[2], data[3]]);
        let count = u16::from_be_bytes([data[4], data[5]]);
        match (self.behave, start) {
            (Behave::SlowStalls, 150) => self.stall = true,
            (Behave::SlowRefused, 6007) => {
                let frame = [0x01, 0x83, 0x02];
                let crc = crc16(&frame).to_le_bytes();
                self.replies.push_back([&frame[..], &crc].concat());
                return Ok(());
            }
            _ => {}
        }
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
        let mut state = self.state.lock().unwrap();
        state.disconnects += 1;
        if matches!(self.behave, Behave::DisconnectFails) {
            return Err(TransportError::Ble("disconnect: no answer".into()));
        }
        state.live = None;
        Ok(())
    }
}

fn collector(
    stations: &[(&str, Behave)],
) -> (Collector<Fake>, Arc<Mutex<State>>, mpsc::Receiver<Record>) {
    collector_of(Model::Eb3a, stations)
}

fn collector_of(
    model: Model,
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
                model,
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
    // A poll that disconnected sends no second Disconnect by address.
    let state = state.lock().unwrap();
    assert_eq!(
        (state.opens.len(), state.disconnects, state.releases),
        (6, 6, 0)
    );
    assert_eq!(state.live, None);
}

#[tokio::test(start_paused = true)]
async fn a_failed_disconnect_is_followed_by_a_release_by_address() {
    let (collector, state, mut rx) = collector(&[("a", Behave::DisconnectFails)]);
    let (_tx, control) = watch::channel(Control::Run);
    // Each next open would panic on a link still up.
    run_for(collector, control, 150).await;
    assert_eq!(summary(&drain(&mut rx)).len(), 3);
    let state = state.lock().unwrap();
    assert_eq!((state.disconnects, state.releases), (3, 3));
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
    // The two that connected disconnected; only the failed opens release.
    // The handshake failure was retried once, after its own release.
    let state = state.lock().unwrap();
    assert_eq!((state.disconnects, state.releases), (2, 3));
}

#[tokio::test(start_paused = true)]
async fn third_failure_makes_a_station_unreachable_with_one_attempt() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Fail(Stage::Connect))]);
    let (_tx, control) = watch::channel(Control::Run);
    // Ticks 0, 1, 2, then 4: the first backed-off poll.
    run_for(collector, control, 250).await;
    assert_eq!(drain(&mut rx).len(), 4);
    let retries: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.1).collect();
    assert_eq!(retries, [true, true, true, false]);
}

/// Seconds since the start of each open of `station`.
fn opens_of(state: &Mutex<State>, station: &str) -> Vec<u64> {
    let state = state.lock().unwrap();
    state
        .opens
        .iter()
        .filter(|o| o.0 == station)
        .map(|o| o.2)
        .collect()
}

#[tokio::test(start_paused = true)]
async fn an_unreachable_station_is_polled_ever_less_often_up_to_ten_minutes() {
    let (collector, state, _rx) =
        collector(&[("a", Behave::Fail(Stage::Connect)), ("b", Behave::Fill(0))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 2_190).await;
    // Ticks 0, 1, 2, then 2, 4, 8 ticks apart, then capped at 10 (10 min).
    let ticks: Vec<_> = opens_of(&state, "a").iter().map(|s| s / 60).collect();
    assert_eq!(ticks, [0, 1, 2, 4, 8, 16, 26, 36]);
    // The other station keeps every tick.
    assert_eq!(opens_of(&state, "b").len(), 37);
}

#[tokio::test(start_paused = true)]
async fn a_success_returns_a_backed_off_station_to_every_tick() {
    let (collector, state, _rx) = collector(&[("a", Behave::FailUntil(300))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 610).await;
    // Fails at ticks 0, 1, 2, 4; tick 8 reads, then every tick again.
    let ticks: Vec<_> = opens_of(&state, "a").iter().map(|s| s / 60).collect();
    assert_eq!(ticks, [0, 1, 2, 4, 8, 9, 10]);
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
    // Only the deadline cuts of `a` release; `b` and `c` disconnected.
    assert_eq!(state.lock().unwrap().releases, 3);
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
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fail(Stage::Connect))]);
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
async fn a_pause_sent_while_running_skips_polls_until_it_ends() {
    let (collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let (tx, control) = watch::channel(Control::Run);
    let pause = async {
        tokio::time::sleep(Duration::from_secs(70)).await;
        let until = Instant::now() + Duration::from_secs(90);
        tx.send(Control::Pause { until }).unwrap();
    };
    tokio::join!(run_for(collector, control, 250), pause);
    let opens: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.2).collect();
    assert_eq!(opens, [0, 60, 180, 240]);
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
    let mut clock = Clock::new(1);
    clock.last = i64::MAX - 1;
    assert_eq!(clock.now(0), i64::MAX - 1);
    assert_eq!(clock.now(0), i64::MAX);
}

#[test]
fn a_clock_stepped_back_still_gives_each_station_a_new_ts() {
    let mut clock = Clock::new(2);
    // The wall clock stepped back an hour after the last record.
    clock.last += 3600;
    let first = clock.last;
    let ts: Vec<_> = [0, 1, 0, 1, 0].map(|s| clock.now(s) - first).into();
    assert_eq!(ts, [0, 0, 1, 1, 2]);
}

mod retry;
mod snapshot;

/// The extras of each sample, in order: whether it has a lifetime counter (a
/// slow block) and the pack temperature (the other slow block).
fn slow_extras(records: &[Record]) -> Vec<(bool, bool)> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::Sample { sample, .. } => Some((
                sample.values.contains_key(&Metric::DcOutEnergy),
                sample.values.contains_key(&Metric::PackTemperature),
            )),
            _ => None,
        })
        .collect()
}

#[tokio::test(start_paused = true)]
async fn slow_blocks_are_read_on_the_first_poll_then_every_ten_minutes() {
    let (mut collector, _state, mut rx) = collector_of(Model::Ac180, &[("a", Behave::Fill(0))]);
    // A poll that takes time must not push the slow read to the 11th poll.
    collector.connect.release_delay = Duration::from_secs(5);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 690).await;
    let records = drain(&mut rx);
    let mut want = vec![(false, false); 12];
    want[0] = (true, true);
    want[10] = (true, true);
    assert_eq!(slow_extras(&records), want);
    // The core extras (103–105) come with every sample.
    assert!(records.iter().all(|r| match r {
        Record::Sample { sample, .. } => sample.values.contains_key(&Metric::TimeToEmpty),
        _ => true,
    }));
}

#[tokio::test(start_paused = true)]
async fn a_refused_slow_block_only_drops_its_extras() {
    let (collector, _state, mut rx) = collector_of(Model::Ac180, &[("a", Behave::SlowRefused)]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    let records = drain(&mut rx);
    assert_eq!(
        summary(&records).iter().map(|r| r.2).collect::<Vec<_>>(),
        ["ok"]
    );
    assert_eq!(slow_extras(&records), [(true, false)]);
}

#[tokio::test(start_paused = true)]
async fn a_stalled_slow_block_is_cut_at_half_the_deadline_and_the_sample_kept() {
    let (collector, state, mut rx) = collector_of(Model::Ac180, &[("a", Behave::SlowStalls)]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 45).await;
    let records = drain(&mut rx);
    // One station: a 60 s deadline, cut at 30 s, the sample stored.
    assert_eq!(
        summary(&records).iter().map(|r| r.2).collect::<Vec<_>>(),
        ["ok"]
    );
    assert_eq!(slow_extras(&records), [(false, false)]);
    assert_eq!(state.lock().unwrap().live, None);
}
