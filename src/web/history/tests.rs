use rusqlite::params;

use super::*;
use crate::store::migrate;

// Hand-computed: 2026-07-01T00:00Z = 1_782_864_000. Kyiv is UTC+3 (EEST) in
// July, so local midnight 2026-07-01 = 06-30T21:00Z = 1_782_864_000 − 10_800.
const D0: i64 = 1_782_853_200;
const INTERVAL: i64 = 60;

fn kyiv() -> TimeZone {
    TimeZone::get("Europe/Kyiv").unwrap()
}

/// An in-memory database with one station of 500 Wh.
fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute_batch("INSERT INTO station VALUES (1, 'office', 'Elite200V2', 500.0, 1);")
        .unwrap();
    conn
}

fn sample(conn: &Connection, ts: i64, soc: f64, in_w: f64, out_w: f64) {
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, ac_in_w, ac_out_w, encrypted)
         VALUES (1, ?1, ?2, ?3, ?4, 1)",
        params![ts, soc, in_w, out_w],
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

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn one_hour_of_100_w_in_50_w_out_is_100_and_50_wh() {
    // 61 samples, 01:00..=02:00 local, 60 s apart: 60 segments of 1/60 h.
    // in = 100 W × 1 h = 100 Wh; out = 50 W × 1 h = 50 Wh.
    // SOC 50 % → 55 % (the last sample): Δstored = 500 × 5 / 100 = 25 Wh.
    // losses = 100 − 50 − 25 = 25 Wh.
    let conn = db();
    for i in 0..=60 {
        let soc = if i == 60 { 55.0 } else { 50.0 };
        sample(&conn, D0 + 3600 + i * 60, soc, 100.0, 50.0);
    }
    // A 30-min house outage counts; a station-scope one does not.
    event(&conn, "house", D0 + 3600, Some(D0 + 5400));
    event(&conn, "office", D0 + 3600, Some(D0 + 9000));
    let now = D0 + 12 * 3600;
    let days = energy(&conn, 1, now, INTERVAL, &kyiv()).unwrap();
    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(
        (day.day.as_str(), day.from, day.to),
        ("2026-07-01", D0, now)
    );
    let s = &day.stations[0];
    assert_eq!(s.station, "office");
    assert!(close(s.energy_in_wh, 100.0), "{s:?}");
    assert!(close(s.energy_out_wh, 50.0), "{s:?}");
    assert!(close(s.stored_delta_wh, 25.0), "{s:?}");
    assert!(close(s.losses_estimate_wh, 25.0), "{s:?}");
    assert_eq!(s.missing_s, 0);
    assert_eq!(
        day.house,
        HouseDay {
            count: 1,
            seconds_without_power: 1800,
            longest_s: 1800
        }
    );
}

#[test]
fn day_boundary_is_local_midnight_not_utc() {
    // 60 W for 60 s = 1 Wh per segment. Samples at D0−120, D0−60, D0, D0+60
    // (local 06-30 23:58, 23:59, 07-01 00:00, 00:01; all still 06-30 in UTC).
    // A segment counts toward its later sample's day: 06-30 gets the one
    // ending at D0−60 (1 Wh); 07-01 gets the ones ending at D0 and D0+60
    // (2 Wh), the first needing the sample before the day. UTC days would put
    // all 3 Wh on 06-30.
    let conn = db();
    for ts in [D0 - 120, D0 - 60, D0, D0 + 60] {
        sample(&conn, ts, 50.0, 60.0, 0.0);
    }
    let now = D0 + 3600;
    let days = energy(&conn, 2, now, INTERVAL, &kyiv()).unwrap();
    let got: Vec<_> = days
        .iter()
        .map(|d| (d.day.as_str(), d.from, d.to, d.stations[0].energy_in_wh))
        .collect();
    assert_eq!(
        got,
        [
            ("2026-07-01", D0, now, 2.0),
            ("2026-06-30", D0 - 86_400, D0, 1.0)
        ]
    );
}

#[test]
fn a_gap_shows_as_missing_coverage() {
    // Samples at D0, D0+60, D0+600, D0+660; the 540 s segment exceeds
    // 3 × 60 s, so it adds no energy and 540 s of missing coverage.
    // in = 2 segments × 60 W × 60 s = 2 Wh.
    let conn = db();
    for ts in [D0, D0 + 60, D0 + 600, D0 + 660] {
        sample(&conn, ts, 50.0, 60.0, 0.0);
    }
    let days = energy(&conn, 1, D0 + 3600, INTERVAL, &kyiv()).unwrap();
    let s = &days[0].stations[0];
    assert!(close(s.energy_in_wh, 2.0), "{s:?}");
    assert_eq!(s.missing_s, 540);
}

#[test]
fn an_open_outage_is_clipped_to_now() {
    // Open house outage since 01:00 local; now = 02:00 local. Today's totals
    // end at now: 3600 s, not the 82 800 s to the day's end. The log shows it
    // ongoing with 3600 s so far.
    let conn = db();
    event(&conn, "house", D0 + 3600, None);
    let now = D0 + 7200;
    let days = energy(&conn, 1, now, INTERVAL, &kyiv()).unwrap();
    assert_eq!(
        days[0].house,
        HouseDay {
            count: 1,
            seconds_without_power: 3600,
            longest_s: 3600
        }
    );
    let log = outages(&conn, 20, now).unwrap();
    assert_eq!(
        log,
        [Outage {
            id: 1,
            scope: "house".into(),
            started_at: D0 + 3600,
            ended_at: None,
            ongoing: true,
            duration_s: 3600,
            precision_s: 60,
        }]
    );
}

#[test]
fn twenty_one_events_keep_the_newest_twenty() {
    // Event i starts at D0 + i h and lasts 600 s; the newest 20 are i = 20..=1.
    let conn = db();
    for i in 0..21 {
        event(&conn, "house", D0 + i * 3600, Some(D0 + i * 3600 + 600));
    }
    let log = outages(&conn, 20, D0 + 86_400).unwrap();
    let starts: Vec<i64> = log.iter().map(|o| o.started_at).collect();
    let want: Vec<i64> = (1..21).rev().map(|i| D0 + i * 3600).collect();
    assert_eq!(starts, want);
    assert!(log.iter().all(|o| !o.ongoing && o.duration_s == 600));
}

#[test]
fn outages_between_keeps_the_overlapping_ones_newest_first() {
    // Window [D0+1000, D0+2000): (0, 500) ends before it; (900, 1100) and
    // (1500, 1600) overlap; the open one from 1900 overlaps (900 s at now =
    // D0+2800); (2000, 2100) starts at its end.
    let conn = db();
    event(&conn, "house", D0, Some(D0 + 500));
    event(&conn, "office", D0 + 900, Some(D0 + 1100));
    event(&conn, "house", D0 + 1500, Some(D0 + 1600));
    event(&conn, "office", D0 + 1900, None);
    event(&conn, "house", D0 + 2000, Some(D0 + 2100));
    let got: Vec<_> = outages_between(&conn, D0 + 1000, D0 + 2000, D0 + 2800)
        .unwrap()
        .into_iter()
        .map(|o| (o.started_at - D0, o.duration_s, o.ongoing))
        .collect();
    assert_eq!(
        got,
        [(1900, 900, true), (1500, 100, false), (900, 200, false)]
    );
}
