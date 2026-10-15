use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};

use crate::protocol::device::Metric;
use crate::store::temp_db::TempDb;
use crate::store::{Stage, Update, open_reader, open_writer};
use crate::web::{AppState, router, serve};

/// Serves `state` on a free loopback port until the sender fires; keeps the
/// `Arc` so a test can publish and fire `shutdown`.
async fn start(
    state: AppState,
) -> (
    SocketAddr,
    Arc<AppState>,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let state = Arc::new(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, router(Arc::clone(&state)), async {
        let _ = stopped.await;
    }));
    (addr, state, stop, server)
}

/// A state over a fresh migrated DB; the `TempDb` must outlive the test.
fn state(name: &str) -> (TempDb, AppState) {
    let db = TempDb::new(name);
    drop(open_writer(&db.path()).unwrap());
    let state = AppState::for_tests(open_reader(&db.path()).unwrap());
    (db, state)
}

/// Reads from `stream` until `needle` has arrived (5 s max); returns all read.
fn read_until(stream: &mut TcpStream, needle: &str) -> String {
    let mut got = Vec::new();
    let mut buf = [0u8; 1024];
    while !String::from_utf8_lossy(&got).contains(needle) {
        let text = String::from_utf8_lossy(&got).into_owned();
        let n = stream
            .read(&mut buf)
            .unwrap_or_else(|e| panic!("waiting for {needle:?} ({e}), got {text:?}"));
        assert!(n > 0, "EOF before {needle:?}, got {text:?}");
        got.extend_from_slice(&buf[..n]);
    }
    String::from_utf8_lossy(&got).into_owned()
}

/// Opens `GET /events` and returns once the response head arrived, i.e. once
/// the handler has subscribed.
async fn connect(addr: SocketAddr) -> TcpStream {
    tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(stream, "GET /events HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
        let head = read_until(&mut stream, "\r\n\r\n");
        assert!(head.starts_with("HTTP/1.1 200 OK"), "{head}");
        assert!(head.contains("content-type: text/event-stream"), "{head}");
        stream
    })
    .await
    .unwrap()
}

/// Reads on a blocking thread until `needle`; hands the stream back.
async fn receive(mut stream: TcpStream, needle: &'static str) -> (TcpStream, String) {
    tokio::task::spawn_blocking(move || {
        let text = read_until(&mut stream, needle);
        (stream, text)
    })
    .await
    .unwrap()
}

fn failure(ts: i64) -> Update {
    Update::PollFailure {
        station: 1,
        ts,
        stage: Stage::Connect,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn each_update_reaches_a_client_as_its_named_event() {
    let (_db, state) = state("sse-events");
    let (addr, state, stop, server) = start(state).await;
    let client = connect(addr).await;

    let core = [(Metric::BatterySoc, 55.0), (Metric::AcInputPower, 120.0)];
    state
        .updates
        .send(Update::Sample {
            station: 3,
            ts: 1000,
            core: core.into_iter().collect(),
        })
        .unwrap();
    state
        .updates
        .send(Update::GridEvent {
            scope: "house".into(),
            started_at: 900,
            ended_at: None,
        })
        .unwrap();
    state.updates.send(failure(1100)).unwrap();

    let (_client, text) = receive(client, "\"ts\":1100}\n\n").await;
    for block in [
        "event: sample\ndata: {\"ac_in_w\":120.0,\"soc\":55.0,\"station_id\":3,\"ts\":1000}\n\n",
        "event: grid_event\ndata: {\"ended_at\":null,\"scope\":\"house\",\"started_at\":900}\n\n",
        "event: poll_failure\ndata: {\"stage\":\"connect\",\"station_id\":1,\"ts\":1100}\n\n",
    ] {
        assert!(text.contains(block), "missing {block:?} in {text:?}");
    }

    state.shutdown.send_replace(true);
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

/// Current-thread runtime: no server task runs between the sends, so the
/// subscriber deterministically lags behind the capacity-1 channel.
#[tokio::test]
async fn a_lagged_client_gets_the_next_update() {
    let (_db, mut state) = state("sse-lagged");
    state.updates = broadcast::channel(1).0;
    let (addr, state, stop, server) = start(state).await;
    let client = connect(addr).await;

    for ts in 1..=3 {
        state.updates.send(failure(ts)).unwrap();
    }
    state.updates.send(failure(4)).unwrap();

    let (client, text) = receive(client, "\"ts\":4}").await;
    assert!(!text.contains("\"ts\":1}"), "{text:?}");
    assert!(!text.contains("\"ts\":3}"), "{text:?}");

    // The stream stays open after the lag.
    state.updates.send(failure(5)).unwrap();
    let (_client, text) = receive(client, "\"ts\":5}").await;
    assert!(text.contains("event: poll_failure"), "{text:?}");

    state.shutdown.send_replace(true);
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_with_an_open_events_client_stops_within_2_s_of_shutdown() {
    let (_db, state) = state("sse-shutdown");
    let (addr, state, stop, server) = start(state).await;
    let _client = connect(addr).await;

    // `serve`'s order: end the streams, then stop the server.
    state.shutdown.send_replace(true);
    stop.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .expect("server still running 2 s after shutdown")
        .unwrap()
        .unwrap();
}
