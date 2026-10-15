use rusqlite::params;

use super::*;
use crate::store::migrate;

// 2026-10-15T00:00Z = 1_792_022_400; a multiple of 60.
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

#[allow(clippy::too_many_arguments)]
fn sample(
    conn: &Connection,
    station: i64,
    ts: i64,
    soc: Option<f64>,
    ac_in: Option<f64>,
    dc_in: Option<f64>,
    ac_out: Option<f64>,
    dc_out: Option<f64>,
) {
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, ac_in_w, dc_in_w, ac_out_w, dc_out_w, encrypted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![station, ts, soc, ac_in, dc_in, ac_out, dc_out],
    )
    .unwrap();
}

#[test]
fn step_grows_with_the_window() {
    assert_eq!(step_for(0, 12 * 3600, 60), 60);
    // 24 h / 1000 = 86.4 → 87 → the next multiple of 60.
    assert_eq!(step_for(0, 86_400, 60), 120);
    // Over 48 h the base is the rollup's 300 s: 7 d / 1000 = 604.8 → 900.
    assert_eq!(step_for(0, 7 * 86_400, 60), 900);
    // 50 h / 1000 = 180 → still one rollup row per bucket.
    assert_eq!(step_for(0, 50 * 3600, 60), 300);
    assert_eq!(step_for(0, 10, 0), 1);
    assert_eq!(step_for(0, 10, 300), 300);
}

#[test]
fn joins_metrics_per_station_and_sums_ac_dc() {
    let conn = db();
    sample(
        &conn,
        1,
        T0,
        Some(80.0),
        Some(10.0),
        Some(5.0),
        Some(9.0),
        None,
    );
    // SOC missing, inputs both missing → in_w null; outputs AC + DC.
    sample(&conn, 1, T0 + 60, None, None, None, Some(7.0), Some(2.0));
    sample(
        &conn,
        2,
        T0 + 30,
        Some(50.0),
        Some(0.0),
        None,
        Some(118.0),
        Some(0.0),
    );
    let c = chart(&conn, T0, T0 + 120, 1, T0 + 120).unwrap();
    assert_eq!(c.step, 1);
    let router = &c.stations[0];
    assert_eq!(router.name, "router");
    assert_eq!(router.t, [T0, T0 + 60]);
    assert_eq!(router.soc, [Some(80.0), None]);
    assert_eq!(router.in_w, [Some(15.0), None]);
    assert_eq!(router.out_w, [Some(9.0), Some(9.0)]);
    let office = &c.stations[1];
    assert_eq!(office.t, [T0 + 30]);
    assert_eq!(office.in_w, [Some(0.0)]);
    assert_eq!(office.out_w, [Some(118.0)]);
}

#[test]
fn buckets_average_and_round_to_one_decimal() {
    let conn = db();
    for (i, w) in [10.0, 10.0, 11.0].into_iter().enumerate() {
        sample(
            &conn,
            1,
            T0 + 60 * i as i64,
            Some(99.0),
            None,
            None,
            Some(w),
            None,
        );
    }
    let c = chart(&conn, T0, T0 + 180, 180, T0 + 180).unwrap();
    assert_eq!(c.stations[0].t, [T0]);
    // (10 + 10 + 11) / 3 = 10.333… → 10.3.
    assert_eq!(c.stations[0].out_w, [Some(10.3)]);
}

#[test]
fn outages_overlapping_the_window_are_included() {
    let conn = db();
    conn.execute_batch(&format!(
        "INSERT INTO grid_event (scope, kind, started_at, ended_at, precision_s)
         VALUES ('house', 'outage', {}, {}, 60),
                ('house', 'outage', {}, NULL, 60),
                ('house', 'outage', {}, {}, 60);",
        T0 - 600,
        T0 + 60,
        T0 + 300,
        T0 - 9000,
        T0 - 8000,
    ))
    .unwrap();
    let c = chart(&conn, T0, T0 + 600, 60, T0 + 600).unwrap();
    let starts: Vec<_> = c.outages.iter().map(|o| o.started_at).collect();
    assert_eq!(starts, [T0 + 300, T0 - 600]);
    assert!(c.outages[0].ongoing);
}
