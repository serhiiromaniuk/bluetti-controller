use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

use tokio::sync::oneshot;

use super::*;
use crate::config::StationConfig;
use crate::protocol::device::Model;
use crate::store::temp_db::TempDb;
use crate::store::{Sample, insert_sample, open_reader, open_writer, sync_stations};

/// A raw HTTP/1.1 GET; returns the status line and the body.
fn get(addr: SocketAddr, path: &str) -> (String, String) {
    request(addr, "GET", path)
}

/// A raw HTTP/1.1 request with no body; returns the status line and the body.
fn request(addr: SocketAddr, method: &str, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response.lines().next().unwrap_or_default().to_owned();
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_owned())
        .unwrap_or_default();
    (status, body)
}

/// Serves `router(state)` on a free loopback port until the sender fires.
async fn start(
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
async fn every_unbuilt_route_answers_501() {
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let (addr, stop, server) = start(AppState::for_tests(reader)).await;

    let routes = [
        ("GET", "/"),
        ("GET", "/telemetry"),
        ("GET", "/assets/app.js"),
        ("GET", "/api/status"),
        ("GET", "/api/series?station=router&metric=soc"),
        ("GET", "/api/outages"),
        ("GET", "/api/energy?days=7"),
        ("GET", "/events"),
        ("POST", "/api/polling/pause?minutes=5"),
    ];
    for (method, path) in routes {
        let (status, _) = tokio::task::spawn_blocking(move || request(addr, method, path))
            .await
            .unwrap();
        assert_eq!(status, "HTTP/1.1 501 Not Implemented", "{method} {path}");
    }
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/healthz"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(body, r#"{"status":"ok","last_poll_age_s":null}"#);

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
