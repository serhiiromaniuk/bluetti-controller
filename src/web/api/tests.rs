use std::net::SocketAddr;

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tokio::sync::oneshot;

use crate::store::migrate;
use crate::web::AppState;
use crate::web::tests::{get, start};
use crate::web::unix_now;

// 2026-10-15T00:00Z = 1_792_022_400; a multiple of 120 and of 300.
const T0: i64 = 1_792_022_400;

/// An in-memory database with `router` (id 1) and `office` (id 2).
fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO station VALUES (1, 'router', 'EB3A', 268.8, 1);
         INSERT INTO station VALUES (2, 'office', 'Elite200V2', 2073.6, 1);",
    )
    .unwrap();
    conn
}

fn soc(conn: &Connection, station: i64, ts: i64, soc: f64) {
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, encrypted) VALUES (?1, ?2, ?3, 1)",
        params![station, ts, soc],
    )
    .unwrap();
}

fn event(conn: &Connection, scope: &str, started_at: i64, ended_at: Option<i64>) {
    conn.execute(
        "INSERT INTO grid_event (scope, kind, started_at, ended_at, precision_s)
         VALUES (?1, 'outage', ?2, ?3, 60)",
        params![scope, started_at, ended_at],
    )
    .unwrap();
}

type Server = (
    SocketAddr,
    oneshot::Sender<()>,
    tokio::task::JoinHandle<anyhow::Result<()>>,
);

async fn stop((_, stop, server): Server) {
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

/// GET `path`; the status line and the body parsed as JSON.
async fn call(addr: SocketAddr, path: &str) -> (String, Value) {
    let path = path.to_owned();
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, &path))
        .await
        .unwrap();
    let json = serde_json::from_str(&body).unwrap_or_else(|_| panic!("not JSON: {body}"));
    (status, json)
}

const OK: &str = "HTTP/1.1 200 OK";
const BAD: &str = "HTTP/1.1 400 Bad Request";

/// `path` answers 400 with a non-empty `{"error": "…"}`.
async fn assert_bad(addr: SocketAddr, path: &str) {
    let (status, body) = call(addr, path).await;
    assert_eq!(status, BAD, "{path}: {body}");
    let message = body["error"].as_str().unwrap_or_default();
    assert!(!message.is_empty(), "{path}: {body}");
    assert_eq!(body.as_object().unwrap().len(), 1, "{path}: {body}");
}

#[tokio::test(flavor = "multi_thread")]
async fn status_has_house_stations_and_paused_until() {
    let server = start(AppState::for_tests(db())).await;
    let (status, body) = call(server.0, "/api/status").await;
    assert_eq!(status, OK);
    // No sample yet: every grid state is unknown, so the vote is too.
    assert_eq!(body["house"], json!({"state": "unknown", "since": null}));
    let names: Vec<_> = body["stations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["router", "office"]);
    assert_eq!(body["stations"][0]["model"], "EB3A");
    assert_eq!(body["stations"][0]["soc"], Value::Null);
    assert_eq!(body["paused_until"], Value::Null);
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn series_within_48_h_reads_raw_rows_by_metric_key() {
    let conn = db();
    for (i, v) in [50.0, 51.0, 52.0].into_iter().enumerate() {
        soc(&conn, 1, T0 + 60 * i as i64, v);
    }
    soc(&conn, 2, T0, 99.0); // another station: never in router's series
    let server = start(AppState::for_tests(conn)).await;
    let base = format!("/api/series?station=router&metric=battery_soc&from={T0}");

    let (status, body) = call(server.0, &format!("{base}&to={}", T0 + 3600)).await;
    assert_eq!(status, OK);
    let t = [T0, T0 + 60, T0 + 120];
    assert_eq!(
        body,
        json!({"source": "raw", "t": t, "v": [50.0, 51.0, 52.0]})
    );

    // step=120: buckets T0 (50, 51 → 50.5) and T0+120 (52).
    let (status, body) = call(server.0, &format!("{base}&to={}&step=120", T0 + 3600)).await;
    assert_eq!(status, OK);
    assert_eq!(
        body,
        json!({"source": "raw", "t": [T0, T0 + 120], "v": [50.5, 52.0]})
    );

    // Exactly 48 h is still raw.
    let (_, body) = call(server.0, &format!("{base}&to={}", T0 + 48 * 3600)).await;
    assert_eq!(body["source"], "raw");
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn series_over_48_h_reads_rollup_5m() {
    let conn = db();
    soc(&conn, 1, T0, 50.0);
    conn.execute(
        "INSERT INTO rollup_5m (station_id, ts, soc_avg, n) VALUES (1, ?1, 42.0, 5)",
        params![T0 + 300],
    )
    .unwrap();
    let server = start(AppState::for_tests(conn)).await;
    let path = format!(
        "/api/series?station=router&metric=battery_soc&from={T0}&to={}",
        T0 + 48 * 3600 + 1
    );
    let (status, body) = call(server.0, &path).await;
    assert_eq!(status, OK);
    assert_eq!(
        body,
        json!({"source": "rollup_5m", "t": [T0 + 300], "v": [42.0]})
    );
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn series_defaults_to_the_last_24_h() {
    let conn = db();
    let now = unix_now();
    soc(&conn, 1, now - 25 * 3600, 10.0);
    soc(&conn, 1, now - 3600, 60.0);
    let server = start(AppState::for_tests(conn)).await;
    let (status, body) = call(server.0, "/api/series?station=router&metric=battery_soc").await;
    assert_eq!(status, OK);
    assert_eq!(
        body,
        json!({"source": "raw", "t": [now - 3600], "v": [60.0]})
    );
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn series_rejects_bad_input_with_a_json_400() {
    let server = start(AppState::for_tests(db())).await;
    let ok = format!("station=router&metric=battery_soc&from={T0}");
    let cases = [
        format!("metric=battery_soc&from={T0}"),
        format!("station=nope&metric=battery_soc&from={T0}"),
        format!("station=router&from={T0}"),
        // A `sample` column name or the enum's name is not a metric key (A6).
        format!("station=router&metric=soc&from={T0}"),
        format!("station=router&metric=BatterySoc&from={T0}"),
        format!("{ok}&to={T0}"),
        format!("{ok}&to={}", T0 - 1),
        format!("{ok}&step=0"),
        format!("{ok}&step=-5"),
        format!("{ok}&step=1.5"),
        "station=router&metric=battery_soc&from=abc".to_owned(),
        "station=router&metric=battery_soc&to=".to_owned(),
    ];
    for query in cases {
        assert_bad(server.0, &format!("/api/series?{query}")).await;
    }
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn outages_overlapping_the_window_newest_first() {
    let conn = db();
    let (from, to) = (T0, T0 + 1000);
    event(&conn, "office", T0 - 500, Some(T0 - 100)); // ended before: out
    event(&conn, "house", T0 - 100, Some(T0 + 100)); // id 2: overlaps
    event(&conn, "router", T0 + 200, None); // id 3: ongoing
    event(&conn, "house", T0 + 1000, None); // starts at `to`: out
    let server = start(AppState::for_tests(conn)).await;
    let (status, body) = call(server.0, &format!("/api/outages?from={from}&to={to}")).await;
    assert_eq!(status, OK);
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 2, "{body}");
    assert_eq!(
        rows[1],
        json!({"id": 2, "scope": "house", "started_at": T0 - 100, "ended_at": T0 + 100,
               "ongoing": false, "duration_s": 200, "precision_s": 60})
    );
    assert_eq!(
        (rows[0]["id"].as_i64(), rows[0]["scope"].as_str()),
        (Some(3), Some("router"))
    );
    assert_eq!(rows[0]["ongoing"], true);
    assert_eq!(rows[0]["ended_at"], Value::Null);
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn outages_default_to_the_last_7_days() {
    let conn = db();
    let now = unix_now();
    event(
        &conn,
        "house",
        now - 8 * 86_400,
        Some(now - 8 * 86_400 + 60),
    );
    event(
        &conn,
        "house",
        now - 6 * 86_400,
        Some(now - 6 * 86_400 + 60),
    );
    let server = start(AppState::for_tests(conn)).await;
    let (status, body) = call(server.0, "/api/outages").await;
    assert_eq!(status, OK);
    let ids: Vec<_> = body
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64())
        .collect();
    assert_eq!(ids, [Some(2)]);
    for query in ["from=x", "to=1.5", &format!("from={T0}&to={T0}")] {
        assert_bad(server.0, &format!("/api/outages?{query}")).await;
    }
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn energy_defaults_to_7_days_capped_by_retention() {
    let mut state = AppState::for_tests(db());
    state.raw_retention_days = 10; // cap = min(30, 10) = 10
    let server = start(state).await;

    let (status, body) = call(server.0, "/api/energy").await;
    assert_eq!(status, OK);
    let days = body.as_array().unwrap();
    assert_eq!(days.len(), 7);
    // Newest first: each day ends where the newer one starts.
    assert_eq!(days[1]["to"], days[0]["from"]);
    assert_eq!(days[0]["stations"][0]["station"], "router");
    assert_eq!(days[0]["stations"][0]["energy_in_wh"], 0.0);
    assert_eq!(days[0]["house"]["count"], 0);

    let (status, body) = call(server.0, "/api/energy?days=10").await;
    assert_eq!(status, OK);
    assert_eq!(body.as_array().unwrap().len(), 10);
    for query in [
        "days=0", "days=11", "days=31", "days=-1", "days=abc", "days=",
    ] {
        assert_bad(server.0, &format!("/api/energy?{query}")).await;
    }
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn energy_cap_is_30_days_when_retention_is_longer() {
    let mut state = AppState::for_tests(db());
    state.raw_retention_days = 90;
    let server = start(state).await;
    let (status, body) = call(server.0, "/api/energy?days=30").await;
    assert_eq!(status, OK);
    assert_eq!(body.as_array().unwrap().len(), 30);
    assert_bad(server.0, "/api/energy?days=31").await;
    stop(server).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn chart_returns_every_station_and_rejects_bad_input() {
    let conn = db();
    let now = unix_now();
    soc(&conn, 1, now - 120, 80.0);
    event(&conn, "house", now - 600, None);
    let server = start(AppState::for_tests(conn)).await;
    let (status, body) = call(server.0, "/api/chart").await;
    assert_eq!(status, OK, "{body}");
    // Default window 24 h; the 60 s poll interval raised to 120 s (≤ 1000 points).
    assert_eq!(
        body["to"].as_i64().unwrap() - body["from"].as_i64().unwrap(),
        86_400
    );
    assert_eq!(body["step"], 120);
    assert_eq!(body["stations"][0]["name"], "router");
    assert_eq!(body["stations"][0]["soc"], json!([80.0]));
    assert_eq!(body["stations"][0]["in_w"], json!([null]));
    assert_eq!(body["stations"][0]["temp_c"], json!([null]));
    assert_eq!(body["stations"][1]["t"], json!([]));
    assert_eq!(body["outages"][0]["scope"], "house");
    for path in [
        "/api/chart?from=10&to=5",
        "/api/chart?from=x",
        "/api/chart?step=0",
        "/api/chart?from=-9223372036854775808&to=0",
        "/api/chart?from=0&to=100000000",
        "/api/chart?step=9223372036854775807",
    ] {
        assert_bad(server.0, path).await;
    }
    stop(server).await;
}
