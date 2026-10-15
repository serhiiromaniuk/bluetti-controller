use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::time::Instant;

use rusqlite::params;

use super::*;
use crate::collector::Control;
use crate::store::migrate;

// 2026-10-15T00:00Z = 1767225600 (2026-01-01) + 287 d × 86400 = 1792022400;
// NOW = 09:00Z = 12:00 in Kyiv (EEST, +03 until 2026-10-25).
const NOW: i64 = 1_792_022_400 + 9 * 3600;

/// router: EB3A 268.8 Wh, sensor; office: Elite200V2 2073.6 Wh, sensor.
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

/// A sample with soc, ac_out_w and ac_in_v; encrypted.
fn sample(conn: &Connection, station: i64, ts: i64, soc: f64, out_w: f64, in_v: f64) {
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, ac_out_w, ac_in_v, encrypted)
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        params![station, ts, soc, out_w, in_v],
    )
    .unwrap();
}

fn failure(conn: &Connection, station: i64, ts: i64, stage: &str) {
    conn.execute(
        "INSERT INTO poll_failure VALUES (?1, ?2, ?3, 'boom')",
        params![station, ts, stage],
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

/// The status over `conn` with the test state (Kyiv, 60 s, factor 0.9).
fn view(conn: Connection, now: i64) -> Status {
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    status(&conn, now, &state).unwrap()
}

#[test]
fn on_battery_lasts_until_local_hh_mm() {
    let conn = db();
    // 100 W both samples → EWMA load 100 W.
    sample(&conn, 1, NOW - 60, 51.0, 100.0, 0.0);
    sample(&conn, 1, NOW, 50.0, 100.0, 0.0);
    event(&conn, "router", NOW - 120, None);
    let card = &view(conn, NOW).stations[0];
    // 268.8 × 50 / 100 × 0.9 = 120.96 Wh; / 100 W = 1.2096 h = 4354.56 s → 4355 s.
    // NOW + 4355 s = 10:12:35Z = 13:12 Kyiv.
    assert_eq!(card.grid, "absent");
    assert_eq!(card.forecast, "lasts until 13:12");
    assert_eq!(card.name, "router");
    assert_eq!(card.model, "EB3A");
    assert_eq!(card.soc, Some(50.0));
    assert_eq!(card.out_w, Some(100.0));
    assert_eq!(card.in_w, None);
    assert_eq!(card.last_seen, Some(NOW));
    assert_eq!(card.encrypted, Some(true));
    assert!(card.unverified, "every EB3A field is verified: false");
    assert_eq!(card.health, "ok");
    assert_eq!(card.samples_24h, 2);
}

#[test]
fn grid_present_shows_if_the_power_goes_now() {
    let conn = db();
    sample(&conn, 1, NOW - 60, 50.0, 100.0, 221.0);
    sample(&conn, 1, NOW, 50.0, 100.0, 221.0);
    let card = &view(conn, NOW).stations[0];
    // 1.2096 h, one decimal → 1.2.
    assert_eq!(card.grid, "present");
    assert_eq!(card.forecast, "if the power goes now: 1.2 h");
}

#[test]
fn stale_sample_is_unknown() {
    let conn = db();
    // 181 s > 3 × 60 s.
    sample(&conn, 1, NOW - 181, 50.0, 100.0, 221.0);
    let card = &view(conn, NOW).stations[0];
    assert_eq!(card.grid, "unknown");
    assert_eq!(card.forecast, "—");
    assert_eq!(card.last_seen, Some(NOW - 181));
}

#[test]
fn sample_at_three_intervals_is_still_fresh() {
    let conn = db();
    sample(&conn, 1, NOW - 180, 50.0, 100.0, 221.0);
    assert_eq!(view(conn, NOW).stations[0].grid, "present");
}

#[test]
fn stale_station_with_open_event_is_unknown_not_absent() {
    // Amendment A1: staleness wins over the open station event.
    let conn = db();
    sample(&conn, 1, NOW - 600, 50.0, 100.0, 0.0);
    event(&conn, "router", NOW - 900, None);
    let card = &view(conn, NOW).stations[0];
    assert_eq!(card.grid, "unknown");
    assert_eq!(card.forecast, "—");
}

#[test]
fn no_sample_is_unknown() {
    let card = &view(db(), NOW).stations[1];
    assert_eq!(card.grid, "unknown");
    assert_eq!(card.forecast, "—");
    assert_eq!(card.soc, None);
    assert_eq!(card.last_seen, None);
    assert_eq!(card.encrypted, None);
}

#[test]
fn three_failures_are_unreachable() {
    let conn = db();
    sample(&conn, 1, NOW - 300, 50.0, 100.0, 221.0);
    failure(&conn, 1, NOW - 180, "connect");
    failure(&conn, 1, NOW - 120, "connect");
    failure(&conn, 1, NOW - 60, "handshake");
    // Older than 24 h: not counted.
    failure(&conn, 1, NOW - 90_000, "read");
    let card = &view(conn, NOW).stations[0];
    assert_eq!(card.health, "unreachable");
    assert_eq!(card.unreachable_since, Some(NOW - 180));
    assert_eq!(card.failures_24h, 3);
    assert_eq!(card.samples_24h, 1);
    assert_eq!(card.last_failure_stage.as_deref(), Some("handshake"));
}

#[test]
fn both_sensors_absent_is_a_house_outage_since_the_open_event() {
    let conn = db();
    for id in [1, 2] {
        sample(&conn, id, NOW, 50.0, 100.0, 0.0);
    }
    event(&conn, "router", NOW - 400, None);
    event(&conn, "office", NOW - 300, None);
    event(&conn, "house", NOW - 300, None);
    let house = view(conn, NOW).house;
    assert_eq!(
        house,
        Banner {
            state: "outage",
            since: Some(NOW - 300)
        }
    );
}

#[test]
fn open_house_event_wins_over_the_vote() {
    let conn = db();
    sample(&conn, 1, NOW, 50.0, 100.0, 221.0);
    event(&conn, "house", NOW - 300, None);
    let house = view(conn, NOW).house;
    assert_eq!(house.state, "outage");
    assert_eq!(house.since, Some(NOW - 300));
}

#[test]
fn power_since_the_newest_closed_house_event() {
    let conn = db();
    sample(&conn, 1, NOW, 50.0, 100.0, 221.0);
    event(&conn, "house", NOW - 9000, Some(NOW - 8000));
    event(&conn, "house", NOW - 4000, Some(NOW - 3000));
    let house = view(conn, NOW).house;
    assert_eq!(
        house,
        Banner {
            state: "power",
            since: Some(NOW - 3000)
        }
    );
}

#[test]
fn no_sensor_reachable_is_unknown() {
    let house = view(db(), NOW).house;
    assert_eq!(
        house,
        Banner {
            state: "unknown",
            since: None
        }
    );
}

#[test]
fn paused_until_follows_control() {
    let conn = db();
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    assert_eq!(status(&conn, NOW, &state).unwrap().paused_until, None);

    let unix_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    state.control.send_replace(Control::Pause {
        until: Instant::now() + Duration::from_secs(600),
    });
    let until = status(&conn, NOW, &state).unwrap().paused_until.unwrap();
    assert!((until - (unix_now + 600)).abs() <= 2, "{until}");

    let past = Instant::now().checked_sub(Duration::from_secs(1)).unwrap();
    state.control.send_replace(Control::Pause { until: past });
    assert_eq!(status(&conn, NOW, &state).unwrap().paused_until, None);
}
