//! Output switch requests between poll rounds (§R3, golden rule #5).

use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use super::*;
use crate::config::ControlConfig;
use crate::control::{Command, Request};
use crate::protocol::control::Output;

/// `01 06 0B BF 00 01` + CRC: EB3A AC (3007) on.
const EB3A_AC_ON: [u8; 8] = [0x01, 0x06, 0x0B, 0xBF, 0x00, 0x01, 0x7B, 0xCA];

/// A command to switch `station`'s AC on, sent after `secs` of virtual time;
/// the handle yields its answer.
fn send_after(
    collector: &mut Collector<Fake>,
    station: usize,
    secs: u64,
) -> JoinHandle<Result<bool, String>> {
    let (tx, rx) = mpsc::channel(4);
    collector.commands = rx;
    let control = ControlConfig { enabled: true };
    let command = Command::new(
        &control,
        &collector.stations[station].config,
        Output::Ac,
        true,
    )
    .expect("allowed");
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(secs)).await;
        let (reply, answer) = oneshot::channel();
        tx.send(Request { command, reply })
            .await
            .expect("collector runs");
        let answer = answer.await.expect("answered");
        drop(tx);
        answer
    })
}

#[tokio::test(start_paused = true)]
async fn a_request_between_rounds_writes_once_and_reads_back() {
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let answer = send_after(&mut collector, 0, 10);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    assert_eq!(answer.await.unwrap(), Ok(true));
    let state = state.lock().unwrap();
    assert_eq!(state.writes, [EB3A_AC_ON.to_vec()]);
    let opens: Vec<_> = state.opens.iter().map(|o| (o.0.as_str(), o.2)).collect();
    assert_eq!(opens, [("a", 0), ("a", 10)]);
    assert_eq!((state.disconnects, state.releases), (2, 0));
    assert_eq!(state.live, None);
}

#[tokio::test(start_paused = true)]
async fn a_request_during_a_round_waits_for_its_last_disconnect() {
    // `b` stalls its read until the 30 s deadline; the fake panics on two
    // links at once.
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fill(0)), ("b", Behave::Stall)]);
    let answer = send_after(&mut collector, 0, 5);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 50).await;
    assert_eq!(answer.await.unwrap(), Ok(true));
    let state = state.lock().unwrap();
    let opens: Vec<_> = state.opens.iter().map(|o| (o.0.as_str(), o.2)).collect();
    assert_eq!(opens, [("a", 0), ("b", 0), ("a", 30)]);
    assert_eq!(state.writes.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn an_unreachable_station_answers_an_error_without_a_frame() {
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fail(Stage::Connect))]);
    let answer = send_after(&mut collector, 0, 10);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    assert_eq!(answer.await.unwrap(), Err("connect failed".to_owned()));
    let state = state.lock().unwrap();
    assert!(state.writes.is_empty());
    assert_eq!(state.opens.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn a_request_while_paused_opens_no_link() {
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let answer = send_after(&mut collector, 0, 10);
    let until = Instant::now() + Duration::from_secs(3600);
    let (_tx, control) = watch::channel(Control::Pause { until });
    run_for(collector, control, 30).await;
    assert_eq!(answer.await.unwrap(), Err("polling paused".to_owned()));
    let state = state.lock().unwrap();
    assert!(state.opens.is_empty());
    assert!(state.writes.is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_request_whose_caller_gave_up_opens_no_link() {
    let (mut collector, state, _rx) = collector(&[("a", Behave::Fill(0))]);
    let (tx, rx) = mpsc::channel(4);
    collector.commands = rx;
    let command = Command::new(
        &ControlConfig { enabled: true },
        &collector.stations[0].config,
        Output::Ac,
        true,
    )
    .unwrap();
    let (reply, answer) = oneshot::channel();
    drop(answer);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        tx.send(Request { command, reply }).await.unwrap();
        // Keeps the channel open past the request.
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    let state = state.lock().unwrap();
    assert_eq!(state.opens.len(), 1);
    assert!(state.writes.is_empty());
}
