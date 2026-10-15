use rusqlite::{Connection, params};

use super::*;
use crate::protocol::device::Metric;
use crate::store::migrate;
use crate::store::read::series;
use crate::web::tests::{get, start};

/// router: EB3A 268.8 Wh, sensor.
fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO station VALUES (1, 'router', 'EB3A', 268.8, 1)",
        [],
    )
    .unwrap();
    conn
}

#[test]
fn sparkline_points_are_hand_computed() {
    // x = t · 120 / 86400 → 0, 60, 120; y = (20 − v) · 30 / (20 − 10) → 30, 0, 15.
    let points = sparkline(&[0, 43_200, 86_400], &[10.0, 20.0, 15.0], 0, 86_400);
    assert_eq!(points, "0.0,30.0 60.0,0.0 120.0,15.0");
}

#[test]
fn a_flat_series_and_a_single_sample_sit_mid_height() {
    assert_eq!(
        sparkline(&[0, 43_200], &[5.0, 5.0], 0, 86_400),
        "0.0,15.0 60.0,15.0"
    );
    assert_eq!(sparkline(&[43_200], &[7.0], 0, 86_400), "60.0,15.0");
    assert_eq!(sparkline(&[], &[], 0, 86_400), "");
}

#[test]
fn the_window_averages_a_day_of_polls_into_at_most_48_points() {
    let conn = db();
    let now = 1_000_000_007; // 7 s past a 30-min boundary: the worst alignment
    for ts in (now - 30 * 3600..=now).step_by(60) {
        conn.execute(
            "INSERT INTO sample (station_id, ts, soc, encrypted) VALUES (1, ?1, 50, 1)",
            [ts],
        )
        .unwrap();
    }
    let (from, to) = window(now);
    assert!(to - from <= 86_400 && to > now, "{from}..{to}");
    let s = series(&conn, 1, Metric::BatterySoc, from, to, BUCKET_S).unwrap();
    assert_eq!(s.t.len(), 48);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_lists_core_and_extra_metrics_with_their_flag() {
    let now = crate::web::unix_now();
    let conn = db();
    for (ts, soc) in [(now - 3600, 60.0), (now - 60, 50.0)] {
        conn.execute(
            "INSERT INTO sample (station_id, ts, soc, ac_in_v, encrypted) VALUES (1, ?1, ?2, 230, 1)",
            params![ts, soc],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO sample_extra VALUES (1, ?1, 'dc_input_voltage', 12.5)",
        [now - 60],
    )
    .unwrap();
    let (addr, stop, server) = start(AppState::for_tests(conn)).await;
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/telemetry"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    for text in [
        "router",
        "battery_soc",
        "50.0 %",
        "ac_input_voltage",
        "230.0 V",
        "dc_input_voltage",
        "12.5 V",
        r#"<span class="badge warn">unverified</span>"#,
        "<polyline",
    ] {
        assert!(body.contains(text), "{text} missing in {body}");
    }
    assert!(!body.contains("ac_input_power"), "no NULL column: {body}");
    assert!(!body.contains("<script"), "no JS: {body}");
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
