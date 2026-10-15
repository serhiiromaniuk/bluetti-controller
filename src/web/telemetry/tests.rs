use rusqlite::{Connection, params};

use super::*;
use crate::protocol::device::{Field, Metric};
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
fn the_flag_follows_the_matching_field() {
    let field = |metric, verified| Field {
        metric,
        register: 0,
        scale: 1.0,
        verified,
        wide: false,
    };
    let fields = [
        field(Metric::BatterySoc, true),
        field(Metric::AcInputVoltage, false),
    ];
    assert!(is_verified(&fields, Some(Metric::BatterySoc)));
    assert!(!is_verified(&fields, Some(Metric::AcInputVoltage)));
    assert!(!is_verified(&fields, Some(Metric::DcInputVoltage)));
    assert!(!is_verified(&fields, None));
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
        "<polyline",
    ] {
        assert!(body.contains(text), "{text} missing in {body}");
    }
    // The extra's own row carries the flag, not just some row on the page.
    let row = body
        .split("<tr>")
        .find(|r| r.contains("<td>dc_input_voltage</td>"))
        .expect("dc_input_voltage row");
    assert!(
        row.contains(r#"<span class="badge warn">unverified</span>"#),
        "{row}"
    );
    assert!(!body.contains("ac_input_power"), "no NULL column: {body}");
    // Only the base template's inline theme toggle: no script file, no chart.
    assert!(!body.contains("<script src"), "no JS file: {body}");
    assert!(body.contains("No qualifying discharge yet"), "{body}");
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn the_page_shows_the_capacity_estimate_from_a_discharge() {
    let conn = db();
    // Three buckets, SOC 90 → 80, 10 Wh out each, no input: 30 − 10 = 20 Wh
    // over 10 points = 200 Wh; 200 / 268.8 = 0.74. The charging bucket after
    // it and a station without rollups add nothing.
    for (ts, soc, in_wh) in [
        (0, 90.0, 0.0),
        (300, 85.0, 0.0),
        (600, 80.0, 0.0),
        (900, 81.0, 5.0),
    ] {
        conn.execute(
            "INSERT INTO rollup_5m (station_id, ts, soc_avg, energy_in_wh, energy_out_wh, n)
             VALUES (1, ?1, ?2, ?3, 10, 5)",
            params![ts, soc, in_wh],
        )
        .unwrap();
    }
    conn.execute(
        "INSERT INTO station VALUES (2, 'mobile', 'AC180', 1152, 0)",
        [],
    )
    .unwrap();
    let (addr, stop, server) = start(AppState::for_tests(conn)).await;
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/telemetry"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    let (router, mobile) = body.split_once("mobile").expect("both stations");
    for text in [
        "Battery health",
        "≈ 200 Wh usable of 269 Wh nominal, factor 0.74 (median of the newest 1)",
        "unverified output meter",
        "<td>0.2</td><td>90 → 80 %</td><td>20</td><td>200</td>",
    ] {
        assert!(router.contains(text), "{text} missing in {router}");
    }
    assert!(mobile.contains("No qualifying discharge yet"), "{mobile}");
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}
