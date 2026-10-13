use std::io::{Read, Write};
use std::net::TcpStream;

use super::*;
use crate::config::StationConfig;
use crate::protocol::device::Model;
use crate::store::temp_db::TempDb;
use crate::store::{Sample, insert_sample, open_reader, open_writer, sync_stations};

/// A raw HTTP/1.1 GET; returns the status line and the body.
fn get(addr: std::net::SocketAddr, path: &str) -> (String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
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

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(serve(
        listener,
        router(open_reader(&db.path()).unwrap()),
        async {
            let _ = stopped.await;
        },
    ));

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
