use super::*;
use crate::store::migrate;

/// An in-memory database (phase-5 Q4) with two stations.
fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO station VALUES (1, 'router', 'EB3A', 268.0, 1);
         INSERT INTO station VALUES (2, 'office', 'Elite200V2', 2073.6, 0);",
    )
    .unwrap();
    conn
}

/// A sample with soc and ac_in_w; `ac_in_v` NULL.
fn sample(conn: &Connection, station: i64, ts: i64, soc: f64, ac_in_w: f64) {
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, ac_in_w, encrypted) VALUES (?1, ?2, ?3, ?4, 1)",
        params![station, ts, soc, ac_in_w],
    )
    .unwrap();
}

fn extra(conn: &Connection, station: i64, ts: i64, metric: &str, value: f64) {
    conn.execute(
        "INSERT INTO sample_extra VALUES (?1, ?2, ?3, ?4)",
        params![station, ts, metric, value],
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

fn tss(rows: &[(i64, Values)]) -> Vec<i64> {
    rows.iter().map(|(ts, _)| *ts).collect()
}

#[test]
fn stations_lists_every_row_by_id() {
    let conn = db();
    let rows = stations(&conn).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0],
        StationRow {
            id: 1,
            name: "router".into(),
            model: "EB3A".into(),
            capacity_wh: 268.0,
            grid_sensor: true,
        }
    );
    assert!(!rows[1].grid_sensor);
}

#[test]
fn latest_sample_is_the_newest_and_omits_null_columns() {
    let conn = db();
    assert_eq!(latest_sample(&conn, 1).unwrap(), None);
    sample(&conn, 1, 100, 50.0, 10.0);
    sample(&conn, 1, 160, 51.0, 12.0);
    sample(&conn, 2, 200, 90.0, 0.0);
    let (ts, values) = latest_sample(&conn, 1).unwrap().unwrap();
    assert_eq!(ts, 160);
    assert_eq!(
        values,
        BTreeMap::from([(Metric::BatterySoc, 51.0), (Metric::AcInputPower, 12.0)])
    );
}

#[test]
fn samples_cover_the_window_plus_the_one_before() {
    let conn = db();
    for ts in [100, 160, 220, 280] {
        sample(&conn, 1, ts, 50.0, 0.0);
    }
    sample(&conn, 2, 190, 50.0, 0.0);
    // 160 is the one before `from`; `to` is exclusive.
    assert_eq!(tss(&samples(&conn, 1, 200, 280).unwrap()), [160, 220]);
    // `from` on a sample: the window starts there, 100 is the one before.
    assert_eq!(
        tss(&samples(&conn, 1, 160, 281).unwrap()),
        [100, 160, 220, 280]
    );
    // Nothing before `from`: the window alone.
    assert_eq!(tss(&samples(&conn, 1, 50, 161).unwrap()), [100, 160]);
    assert!(samples(&conn, 1, 0, 100).unwrap().is_empty());
}

#[test]
fn last_samples_keep_grid_rows_oldest_first() {
    let conn = db();
    sample(&conn, 1, 100, 50.0, 10.0);
    sample(&conn, 1, 160, 50.0, 11.0);
    conn.execute(
        "INSERT INTO sample (station_id, ts, soc, encrypted) VALUES (1, 220, 50, 1)",
        [],
    )
    .unwrap();
    sample(&conn, 1, 280, 50.0, 12.0);
    let rows = last_samples(&conn, 1, 2).unwrap();
    assert_eq!(tss(&rows), [160, 280]);
    assert_eq!(rows[1].1[&Metric::AcInputPower], 12.0);
}

#[test]
fn latest_extras_take_each_key_at_its_own_newest_ts() {
    let conn = db();
    extra(&conn, 1, 100, "pack_voltage", 26.0);
    extra(&conn, 1, 160, "pack_voltage", 26.5);
    extra(&conn, 1, 100, "ac_output_on", 1.0);
    extra(&conn, 2, 200, "pack_voltage", 52.0);
    assert_eq!(
        latest_extras(&conn, 1).unwrap(),
        BTreeMap::from([
            ("ac_output_on".to_string(), (100, 1.0)),
            ("pack_voltage".to_string(), (160, 26.5)),
        ])
    );
}

#[test]
fn events_by_window_last_n_and_open_scope() {
    let conn = db();
    event(&conn, "house", 100, Some(200)); // id 1
    event(&conn, "router", 300, Some(400)); // id 2
    event(&conn, "house", 500, None); // id 3
    event(&conn, "router", 600, None); // id 4
    let ids = |rows: Vec<GridEventRow>| rows.iter().map(|e| e.id).collect::<Vec<_>>();
    // Ending exactly at `from` does not overlap; an open one runs on.
    assert_eq!(ids(events_between(&conn, 200, 550).unwrap()), [3, 2]);
    assert_eq!(ids(events_between(&conn, 700, 800).unwrap()), [4, 3]);
    assert_eq!(
        ids(events_between(&conn, 0, 100).unwrap()),
        Vec::<i64>::new()
    );
    assert_eq!(ids(last_events(&conn, 3).unwrap()), [4, 3, 2]);
    let open = open_event(&conn, "house").unwrap().unwrap();
    assert_eq!(
        open,
        GridEventRow {
            id: 3,
            scope: "house".into(),
            started_at: 500,
            ended_at: None,
            precision_s: 60,
        }
    );
    assert_eq!(open_event(&conn, "office").unwrap(), None);
}

#[test]
fn series_raw_averages_into_step_buckets() {
    let conn = db();
    for (ts, soc) in [(0, 40.0), (60, 50.0), (120, 70.0), (180, 80.0), (240, 90.0)] {
        sample(&conn, 1, ts, soc, 0.0);
    }
    let s = series(&conn, 1, Metric::BatterySoc, 60, 240, 1).unwrap();
    assert_eq!(
        (s.source, s.t, s.v),
        (Source::Raw, vec![60, 120, 180], vec![50.0, 70.0, 80.0])
    );
    // Buckets [0,120) = (40+50)/2 = 45 and [120,240) = (70+80)/2 = 75; 240 is past `to`.
    let s = series(&conn, 1, Metric::BatterySoc, 0, 240, 120).unwrap();
    assert_eq!((s.t, s.v), (vec![0, 120], vec![45.0, 75.0]));
    // A NULL column is skipped, not averaged as 0.
    let s = series(&conn, 1, Metric::AcInputVoltage, 0, 300, 1).unwrap();
    assert!(s.t.is_empty());
    assert!(series(&conn, 1, Metric::BatterySoc, 0, 240, 0).is_err());
}

#[test]
fn series_reads_extras_by_key() {
    let conn = db();
    extra(&conn, 1, 100, "pack_voltage", 26.0);
    extra(&conn, 1, 160, "pack_voltage", 27.0);
    extra(&conn, 1, 160, "ac_output_on", 1.0);
    let s = series(&conn, 1, Metric::PackVoltage, 0, 1000, 1000).unwrap();
    assert_eq!((s.source, s.t, s.v), (Source::Raw, vec![0], vec![26.5]));
}

#[test]
fn series_switches_to_the_rollup_past_48_hours() {
    let conn = db();
    sample(&conn, 1, 600, 55.0, 0.0);
    extra(&conn, 1, 600, "pack_voltage", 26.0);
    conn.execute(
        "INSERT INTO rollup_5m (station_id, ts, soc_avg, n) VALUES (1, 300, 42.0, 5)",
        [],
    )
    .unwrap();
    let raw = series(&conn, 1, Metric::BatterySoc, 0, RAW_MAX_S, 1).unwrap();
    assert_eq!((raw.source, raw.v), (Source::Raw, vec![55.0]));
    let rolled = series(&conn, 1, Metric::BatterySoc, 0, RAW_MAX_S + 1, 1).unwrap();
    assert_eq!(
        (rolled.source, rolled.t, rolled.v),
        (Source::Rollup5m, vec![300], vec![42.0])
    );
    // No rollup column: raw, whatever the range.
    let s = series(&conn, 1, Metric::PackVoltage, 0, RAW_MAX_S + 1, 1).unwrap();
    assert_eq!((s.source, s.v), (Source::Raw, vec![26.0]));
    let s = series(&conn, 1, Metric::AcInputVoltage, 0, RAW_MAX_S + 1, 1).unwrap();
    assert_eq!(s.source, Source::Raw);
}
