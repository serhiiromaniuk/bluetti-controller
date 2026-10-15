use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::*;
use crate::config::StationConfig;
use crate::metrics::grid::Params;
use crate::protocol::device::{Metric, Model};
use crate::store::temp_db::TempDb;
use crate::store::{
    Failure, Health, Record, Sample, Stage, Station, Writer, close_outage, insert_failure,
    insert_sample, insert_snapshot, open_outage, open_writer, station_health, sync_stations,
};

const P: i64 = 60;
const D: i64 = 86_400;

/// A fresh file with one EB3A station; returns the connection and its id.
fn setup(db: &TempDb) -> (Connection, i64) {
    let mut conn = open_writer(&db.path()).unwrap();
    let config = StationConfig {
        name: "router".into(),
        model: Model::Eb3a,
        address: "AA:BB:CC:DD:EE:FF".into(),
        grid_sensor: true,
    };
    let id = sync_stations(&mut conn, &[config]).unwrap()["router"];
    (conn, id)
}

/// 100 W in, 40 W AC out, SOC 50, one extra field.
fn sample(ts: i64) -> Sample {
    Sample {
        ts,
        encrypted: true,
        values: BTreeMap::from([
            (Metric::AcInputPower, 100.0),
            (Metric::AcOutputPower, 40.0),
            (Metric::BatterySoc, 50.0),
            (Metric::PackVoltage, 26.5),
        ]),
    }
}

fn store(conn: &mut Connection, id: i64, times: impl IntoIterator<Item = i64>) {
    for ts in times {
        insert_sample(conn, id, &sample(ts)).unwrap();
    }
}

/// `(ts, energy_in_wh, energy_out_wh, n)` per row, rounded to mWh.
fn rows(conn: &Connection) -> Vec<(i64, f64, f64, u32)> {
    let mut stmt = conn
        .prepare("SELECT ts, energy_in_wh, energy_out_wh, n FROM rollup_5m ORDER BY ts")
        .unwrap();
    stmt.query_map([], |r| {
        let wh = |v: f64| (v * 1000.0).round() / 1000.0;
        Ok((r.get(0)?, wh(r.get(1)?), wh(r.get(2)?), r.get(3)?))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

// Samples every 60 s from 0 to 600. Bucket 0 holds 0..240: 4 segments of
// 100 W × 60 s = 4 × 1.667 Wh in, 4 × 0.667 Wh out. Bucket 300 holds 300..540
// plus the crossing segment 240→300: 5 segments = 8.333 Wh in, 3.333 Wh out.
const EXPECTED: [(i64, f64, f64, u32); 2] = [(0, 6.667, 2.667, 5), (300, 8.333, 3.333, 5)];

#[test]
fn running_the_rollup_twice_gives_identical_rows() {
    let db = TempDb::new("rollup-twice");
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, (0..=600).step_by(60));
    assert_eq!(rollup(&mut conn, id, P, 660).unwrap(), 2);
    assert_eq!(rows(&conn), EXPECTED);
    assert_eq!(rollup(&mut conn, id, P, 660).unwrap(), 0);
    assert_eq!(rows(&conn), EXPECTED);
    let soc: (f64, f64) = conn
        .query_row(
            "SELECT soc_avg, soc_min FROM rollup_5m WHERE ts = 0",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(soc, (50.0, 50.0));
}

#[test]
fn a_restart_continues_from_the_newest_rollup() {
    let db = TempDb::new("rollup-restart");
    {
        let (mut conn, id) = setup(&db);
        store(&mut conn, id, (0..=300).step_by(60));
        assert_eq!(rollup(&mut conn, id, P, 360).unwrap(), 1);
    }
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, (360..=600).step_by(60));
    // Only bucket 300 is new, and its crossing segment 240→300 still counts.
    assert_eq!(rollup(&mut conn, id, P, 660).unwrap(), 1);
    assert_eq!(rows(&conn), EXPECTED);
}

#[test]
fn the_open_current_bucket_is_never_rolled_up() {
    let db = TempDb::new("rollup-open");
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, (0..=600).step_by(60));
    assert_eq!(rollup(&mut conn, id, P, 599).unwrap(), 1);
    assert_eq!(rows(&conn), EXPECTED[..1]);
    assert_eq!(rollup(&mut conn, id, P, 600).unwrap(), 1);
    assert_eq!(rows(&conn), EXPECTED);
}

#[test]
fn empty_buckets_get_no_row_and_a_gap_adds_no_energy() {
    let db = TempDb::new("rollup-gap");
    let (mut conn, id) = setup(&db);
    // 0, 60, then nothing until 1200: buckets 300..900 are empty, and the
    // 1140 s segment into bucket 1200 is a gap (> 3 intervals).
    store(&mut conn, id, [0, 60, 1200]);
    assert_eq!(rollup(&mut conn, id, P, 1500).unwrap(), 2);
    assert_eq!(rows(&conn), [(0, 1.667, 0.667, 2), (1200, 0.0, 0.0, 1)]);
}

#[test]
fn retention_deletes_only_rolled_up_old_raw_rows_and_old_audit_rows() {
    let db = TempDb::new("retention");
    let (mut conn, id) = setup(&db);
    let now = 100 * D;
    // 0 and 60 are rolled up; 600 is old but not rolled up; 95 d is recent.
    store(&mut conn, id, [0, 60, 600, 95 * D]);
    assert_eq!(rollup(&mut conn, id, P, 300).unwrap(), 1);
    for ts in [5 * D, 50 * D] {
        let failure = Failure {
            ts,
            stage: Stage::Connect,
            error: "timed out".into(),
        };
        insert_failure(&conn, id, &failure).unwrap();
        insert_snapshot(&conn, id, ts, 0, &[1, 2]).unwrap();
    }
    let event = open_outage(&conn, "house", 0, P).unwrap();
    close_outage(&conn, event, 60).unwrap();

    retention(&mut conn, 30, now).unwrap();

    let ts_of = |table: &str| -> Vec<i64> {
        let sql = format!("SELECT DISTINCT ts FROM {table} ORDER BY ts");
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    };
    assert_eq!(ts_of("sample"), [600, 95 * D]);
    assert_eq!(ts_of("sample_extra"), [600, 95 * D]);
    assert_eq!(ts_of("poll_failure"), [50 * D]);
    assert_eq!(ts_of("register_snapshot"), [50 * D]);
    assert_eq!(count(&conn, "SELECT count(*) FROM grid_event"), 1);
    assert_eq!(count(&conn, "SELECT count(*) FROM rollup_5m"), 1);
}

#[test]
fn retention_keeps_rolled_up_raw_rows_younger_than_the_cutoff() {
    let db = TempDb::new("retention-recent");
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, [95 * D, 95 * D + 60, 95 * D + 300]);
    assert_eq!(rollup(&mut conn, id, P, 95 * D + 300).unwrap(), 1);
    retention(&mut conn, 30, 100 * D).unwrap();
    assert_eq!(count(&conn, "SELECT count(*) FROM sample"), 3);
    assert_eq!(count(&conn, "SELECT count(*) FROM sample_extra"), 3);
}

#[test]
fn retention_keeps_each_station_newest_sample() {
    // Amendment A2: `station_health` counts the failures after the newest sample.
    let db = TempDb::new("retention-newest");
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, [0, 60]);
    assert_eq!(rollup(&mut conn, id, P, 300).unwrap(), 1);
    for ts in [40 * D, 40 * D + 60] {
        let failure = Failure {
            ts,
            stage: Stage::Discover,
            error: "not found".into(),
        };
        insert_failure(&conn, id, &failure).unwrap();
    }
    retention(&mut conn, 30, 41 * D).unwrap();
    assert_eq!(count(&conn, "SELECT count(*) FROM sample"), 1);
    assert_eq!(count(&conn, "SELECT max(ts) FROM sample"), 60);
    // Two failures after the kept sample: still reachable, not 3 since day 0.
    assert_eq!(station_health(&conn, id).unwrap(), Health::Ok);
}

#[test]
fn compact_truncates_the_wal() {
    let db = TempDb::new("compact");
    let (mut conn, id) = setup(&db);
    store(&mut conn, id, [0]);
    compact(&conn).unwrap();
    let wal = db.path().with_extension("db-wal");
    assert_eq!(std::fs::metadata(wal).unwrap().len(), 0);
}

#[test]
fn the_writer_rolls_up_when_records_enter_a_new_bucket() {
    let db = TempDb::new("rollup-writer");
    let (conn, id) = setup(&db);
    let station = Station {
        id,
        name: "router".into(),
        grid_sensor: true,
    };
    let params = Params {
        voltage_min: 180.0,
        debounce: NonZeroU32::new(2).unwrap(),
        interval_s: P,
    };
    let days = NonZeroU32::new(30).unwrap();
    let mut writer = Writer::restore(conn, vec![station], params, days).unwrap();
    for ts in (0..=600).step_by(60) {
        let record = Record::Sample {
            station: id,
            sample: sample(ts),
        };
        writer.handle(&record).unwrap();
    }
    let reader = open_writer(&db.path()).unwrap();
    assert_eq!(rows(&reader), EXPECTED);
}
