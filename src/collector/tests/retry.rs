//! The key-exchange retry on a fresh connection (phase-4 PLAN A23 part 2).

use super::*;

#[tokio::test(start_paused = true)]
async fn a_handshake_failure_is_retried_once_on_a_fresh_connection() {
    let (collector, state, mut rx) = collector(&[("a", Behave::FailFirst(Stage::Handshake))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    let stages: Vec<_> = summary(&drain(&mut rx)).iter().map(|r| r.2).collect();
    assert_eq!(stages, ["ok"]);
    let state = state.lock().unwrap();
    assert_eq!((state.opens.len(), state.releases), (2, 1));
}

#[tokio::test(start_paused = true)]
async fn an_unreachable_station_gets_no_handshake_retry() {
    let (collector, state, mut rx) = collector(&[("a", Behave::Fail(Stage::Handshake))]);
    let (_tx, control) = watch::channel(Control::Run);
    // Ticks 0, 1, 2, then the backed-off tick 4.
    run_for(collector, control, 250).await;
    let stages: Vec<_> = summary(&drain(&mut rx)).iter().map(|r| r.2).collect();
    assert_eq!(stages, ["handshake"; 4]);
    let retries: Vec<_> = state.lock().unwrap().opens.iter().map(|o| o.1).collect();
    assert_eq!(retries, [true, true, true, true, true, true, false]);
}

#[tokio::test(start_paused = true)]
async fn other_open_failures_are_not_retried_in_the_poll() {
    let (collector, state, _rx) = collector(&[
        ("a", Behave::FailFirst(Stage::Discover)),
        ("b", Behave::FailFirst(Stage::Connect)),
    ]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    assert_eq!(state.lock().unwrap().opens.len(), 2);
}

#[tokio::test(start_paused = true)]
async fn a_deadline_during_the_retry_still_names_the_handshake() {
    let (collector, _state, mut rx) = collector(&[("a", Behave::FailThenHang(Stage::Handshake))]);
    let (_tx, control) = watch::channel(Control::Run);
    // One station: the deadline is the whole 60 s interval.
    run_for(collector, control, 70).await;
    let records = drain(&mut rx);
    let Record::Failure { failure, .. } = &records[0] else {
        panic!("{records:?}")
    };
    assert_eq!(failure.stage, Stage::Handshake);
    assert!(failure.error.contains("deadline"), "{}", failure.error);
    assert!(
        failure.error.contains("handshake failed"),
        "{}",
        failure.error
    );
}

#[tokio::test(start_paused = true)]
async fn a_failed_retry_keeps_both_errors_under_the_handshake() {
    let (collector, _state, mut rx) = collector(&[("a", Behave::Fail(Stage::Handshake))]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 30).await;
    let records = drain(&mut rx);
    let Record::Failure { failure, .. } = &records[0] else {
        panic!("{records:?}")
    };
    assert_eq!(failure.stage, Stage::Handshake);
    assert_eq!(
        failure.error.matches("handshake failed").count(),
        2,
        "{}",
        failure.error
    );
}

#[tokio::test(start_paused = true)]
async fn failure_fields_name_the_cache_and_the_time_since_success() {
    let at = Instant::now();
    tokio::time::advance(Duration::from_secs(75)).await;
    assert_eq!(
        failure_fields(Some(true), Some(at)),
        ("warm", "75s".to_owned())
    );
    assert_eq!(
        failure_fields(Some(false), None),
        ("cold", "never".to_owned())
    );
    assert_eq!(failure_fields(None, None), ("-", "never".to_owned()));
}

#[tokio::test(start_paused = true)]
async fn a_deadline_during_the_retry_release_keeps_the_first_error() {
    let (mut collector, _state, mut rx) = collector(&[("a", Behave::Fail(Stage::Handshake))]);
    // The release before the retry outlasts the 60 s deadline (A30).
    collector.connect.release_delay = Duration::from_secs(70);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 140).await;
    let records = drain(&mut rx);
    let Record::Failure { failure, .. } = &records[0] else {
        panic!("{records:?}")
    };
    assert_eq!(failure.stage, Stage::Handshake);
    assert!(failure.error.contains("deadline"), "{}", failure.error);
    assert!(
        failure.error.contains("handshake failed"),
        "{}",
        failure.error
    );
}
