use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

use tokio::sync::oneshot;

use super::*;
use crate::config::StationConfig;
use crate::protocol::device::Model;
use crate::store::temp_db::TempDb;
use crate::store::{Sample, insert_sample, open_reader, open_writer, sync_stations};

/// A raw HTTP/1.1 GET; returns the status line and the body.
pub(super) fn get(addr: SocketAddr, path: &str) -> (String, String) {
    request(addr, "GET", path)
}

/// A raw HTTP/1.1 request with no body; returns the status line and the body.
pub(super) fn request(addr: SocketAddr, method: &str, path: &str) -> (String, String) {
    let (status, _, body) = raw_request(addr, method, path);
    (status, String::from_utf8(body).unwrap())
}

/// A raw HTTP/1.1 request with no body; returns the status line, the header
/// lines and the body bytes.
pub(super) fn raw_request(
    addr: SocketAddr,
    method: &str,
    path: &str,
) -> (String, Vec<String>, Vec<u8>) {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(response.len());
    let head = String::from_utf8_lossy(&response[..split]).into_owned();
    let mut lines = head.lines().map(str::to_owned);
    let status = lines.next().unwrap_or_default();
    let body = response.get(split + 4..).unwrap_or_default().to_vec();
    (status, lines.collect(), body)
}

/// Serves `router(state)` on a free loopback port until the sender fires.
pub(super) async fn start(
    state: AppState,
) -> (
    SocketAddr,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = oneshot::channel::<()>();
    let server = tokio::spawn(serve(listener, router(Arc::new(state)), async {
        let _ = stopped.await;
    }));
    (addr, stop, server)
}

#[tokio::test(flavor = "multi_thread")]
async fn healthz_reports_the_newest_sample_age() {
    let db = TempDb::new("healthz");
    let mut writer = open_writer(&db.path()).unwrap();
    let station = StationConfig {
        name: "router".into(),
        model: Model::Eb3a,
        address: "AA:BB:CC:00:00:00".into(),
        grid_sensor: true,
    };
    let id = sync_stations(&mut writer, &[station]).unwrap()["router"];

    let (addr, stop, server) = start(AppState::for_tests(open_reader(&db.path()).unwrap())).await;

    let request = |path: &'static str| tokio::task::spawn_blocking(move || get(addr, path));
    let (status, body) = request("/healthz").await.unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(body, r#"{"status":"ok","last_poll_age_s":null}"#);

    let sample = Sample {
        ts: unix_now() - 30,
        encrypted: false,
        values: Default::default(),
    };
    insert_sample(&mut writer, id, &sample).unwrap();
    let (status, body) = request("/healthz").await.unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    let health: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(health["status"], "ok");
    let age = health["last_poll_age_s"].as_i64().unwrap();
    assert!((30..=35).contains(&age), "age {age}");

    let (status, _) = request("/nope").await.unwrap();
    assert_eq!(status, "HTTP/1.1 404 Not Found");

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn healthz_is_null_before_the_first_poll() {
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let (addr, stop, server) = start(AppState::for_tests(reader)).await;

    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/healthz"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(body, r#"{"status":"ok","last_poll_age_s":null}"#);

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

/// §8.4 i: each file's share of the 100 KB page weight (rebalanced
/// 2026-10-15). Over budget → trim the file, never the cap.
#[test]
fn assets_stay_within_the_page_weight_budget() {
    for (name, cap) in [
        ("app.js", 12_000),
        ("app.css", 5_000),
        ("uPlot.iife.min.js", 51_081),
        ("uPlot.min.css", 1_857),
    ] {
        let len = std::fs::metadata(format!("{}/assets/{name}", env!("CARGO_MANIFEST_DIR")))
            .unwrap()
            .len();
        assert!(len <= cap, "{name}: {len} B > {cap} B");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn every_asset_is_served_with_its_type_and_bytes() {
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let (addr, stop, server) = start(AppState::for_tests(reader)).await;

    let assets = [
        ("uPlot.iife.min.js", "text/javascript; charset=utf-8"),
        ("uPlot.min.css", "text/css; charset=utf-8"),
        ("app.js", "text/javascript; charset=utf-8"),
        ("app.css", "text/css; charset=utf-8"),
    ];
    for (name, content_type) in assets {
        let file = std::fs::read(format!("{}/assets/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        let path = format!("/assets/{name}");
        let (status, headers, body) =
            tokio::task::spawn_blocking(move || raw_request(addr, "GET", &path))
                .await
                .unwrap();
        assert_eq!(status, "HTTP/1.1 200 OK", "{name}");
        let header = |key: &str| {
            headers
                .iter()
                .find_map(|line| {
                    let (k, v) = line.split_once(": ")?;
                    k.eq_ignore_ascii_case(key).then(|| v.to_owned())
                })
                .unwrap_or_default()
        };
        assert_eq!(header("content-type"), content_type, "{name}");
        assert_eq!(header("cache-control"), "max-age=86400", "{name}");
        assert_eq!(header("content-length"), file.len().to_string(), "{name}");
        assert_eq!(body, file, "{name}");
    }
    let (status, _) = tokio::task::spawn_blocking(move || get(addr, "/assets/nope"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 404 Not Found");

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
