use super::*;
use crate::protocol::device::Model;
use crate::store::open_writer;
use crate::store::temp_db::TempDb;

fn station(name: &str, model: Model, grid_sensor: bool) -> StationConfig {
    StationConfig {
        name: name.into(),
        model,
        address: "AA:BB:CC:DD:EE:FF".into(),
        grid_sensor,
    }
}

/// An EB3A-shaped poll: every profile field present and in range.
fn eb3a_poll() -> Poll {
    Poll {
        values: BTreeMap::from([
            (Metric::DcInputPower, 0.0),
            (Metric::AcInputPower, 12.0),
            (Metric::AcOutputPower, 9.0),
            (Metric::DcOutputPower, 3.0),
            (Metric::BatterySoc, 87.0),
            (Metric::AcOutputOn, 1.0),
            (Metric::DcOutputOn, 0.0),
            (Metric::AcInputVoltage, 221.0),
            (Metric::DcInputVoltage, 0.42),
        ]),
        problems: Vec::new(),
    }
}

fn count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .unwrap()
}

fn failure(ts: i64) -> Failure {
    Failure {
        ts,
        stage: Stage::Connect,
        error: "timed out".into(),
    }
}

#[test]
fn sync_takes_capacity_from_the_profile_and_keeps_removed_stations() {
    let db = TempDb::new("sync");
    let mut conn = open_writer(&db.path()).unwrap();
    let ids = sync_stations(
        &mut conn,
        &[
            station("router", Model::Eb3a, true),
            station("living-room", Model::Elite200V2, true),
        ],
    )
    .unwrap();
    let row: (String, f64, bool) = conn
        .query_row(
            "SELECT model, capacity_wh, grid_sensor FROM station WHERE id = ?1",
            [ids["living-room"]],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(row, ("Elite200V2".into(), 2073.6, true));

    // `router` leaves the config and `living-room` stops voting: same ids, no row lost.
    let again = sync_stations(
        &mut conn,
        &[station("living-room", Model::Elite200V2, false)],
    )
    .unwrap();
    assert_eq!(again["living-room"], ids["living-room"]);
    assert_eq!(count(&conn, "station"), 2);
    let sensor: bool = conn
        .query_row(
            "SELECT grid_sensor FROM station WHERE id = ?1",
            [ids["living-room"]],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!sensor);
}

#[test]
fn eb3a_poll_lands_six_core_columns_and_three_extras() {
    let db = TempDb::new("sample");
    let mut conn = open_writer(&db.path()).unwrap();
    let id = sync_stations(&mut conn, &[station("router", Model::Eb3a, true)]).unwrap()["router"];
    let sample = Sample::from_poll(1_760_000_000, true, &eb3a_poll()).unwrap();
    insert_sample(&mut conn, id, &sample).unwrap();

    type Core = (f64, f64, f64, f64, f64, f64, bool);
    let core: Core = conn
        .query_row(
            "SELECT soc, ac_in_w, dc_in_w, ac_out_w, dc_out_w, ac_in_v, encrypted
             FROM sample WHERE station_id = ?1 AND ts = 1760000000",
            [id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(core, (87.0, 12.0, 0.0, 9.0, 3.0, 221.0, true));

    let mut stmt = conn
        .prepare("SELECT metric, value FROM sample_extra WHERE station_id = ?1 ORDER BY metric")
        .unwrap();
    let extras: Vec<(String, f64)> = stmt
        .query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        extras,
        [
            ("ac_output_on".into(), 1.0),
            ("dc_input_voltage".into(), 0.42),
            ("dc_output_on".into(), 0.0),
        ]
    );
}

#[test]
fn a_poll_with_a_problem_lands_only_a_decode_failure() {
    let db = TempDb::new("problem");
    let mut conn = open_writer(&db.path()).unwrap();
    let id = sync_stations(&mut conn, &[station("router", Model::Eb3a, true)]).unwrap()["router"];
    let mut poll = eb3a_poll();
    poll.values.remove(&Metric::BatterySoc);
    poll.problems = vec![
        Problem::Missing {
            metric: Metric::BatterySoc,
        },
        Problem::OutOfRange {
            metric: Metric::AcInputPower,
            value: 65535.0,
        },
    ];
    let failure = Sample::from_poll(100, true, &poll).unwrap_err();
    assert_eq!(
        failure,
        Failure {
            ts: 100,
            stage: Stage::Decode,
            error: "battery_soc missing; ac_input_power out of range: 65535".into(),
        }
    );
    insert_failure(&conn, id, &failure).unwrap();
    assert_eq!(count(&conn, "sample"), 0);
    assert_eq!(count(&conn, "sample_extra"), 0);
    let stage: String = conn
        .query_row("SELECT stage FROM poll_failure", [], |row| row.get(0))
        .unwrap();
    assert_eq!(stage, "decode");
}

#[test]
fn three_failures_make_a_station_unreachable_and_a_sample_clears_it() {
    let db = TempDb::new("health");
    let mut conn = open_writer(&db.path()).unwrap();
    let id = sync_stations(&mut conn, &[station("router", Model::Eb3a, true)]).unwrap()["router"];
    assert_eq!(
        station_health(&conn, id).unwrap(),
        Health::Ok,
        "no polls yet"
    );

    let sample = |ts| Sample::from_poll(ts, true, &eb3a_poll()).unwrap();
    insert_sample(&mut conn, id, &sample(0)).unwrap();
    insert_failure(&conn, id, &failure(60)).unwrap();
    insert_failure(&conn, id, &failure(120)).unwrap();
    assert_eq!(station_health(&conn, id).unwrap(), Health::Ok, "2 failures");
    insert_failure(&conn, id, &failure(180)).unwrap();
    assert_eq!(
        station_health(&conn, id).unwrap(),
        Health::Unreachable { since: 60 }
    );
    insert_failure(&conn, id, &failure(240)).unwrap();
    assert_eq!(
        station_health(&conn, id).unwrap(),
        Health::Unreachable { since: 60 },
        "since stays at the first failure"
    );
    insert_sample(&mut conn, id, &sample(300)).unwrap();
    assert_eq!(station_health(&conn, id).unwrap(), Health::Ok);
}

#[test]
fn snapshot_stores_big_endian_words() {
    let db = TempDb::new("snapshot");
    let mut conn = open_writer(&db.path()).unwrap();
    let id = sync_stations(&mut conn, &[station("router", Model::Eb3a, true)]).unwrap()["router"];
    insert_snapshot(&conn, id, 3600, 70, &[0x00DD, 0x1234]).unwrap();
    let data: Vec<u8> = conn
        .query_row(
            "SELECT data FROM register_snapshot WHERE station_id = ?1 AND start_reg = 70",
            [id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(data, [0x00, 0xDD, 0x12, 0x34]);
}

#[test]
fn outage_opens_once_and_closes_once() {
    let db = TempDb::new("outage");
    let conn = open_writer(&db.path()).unwrap();
    let id = open_outage(&conn, "house", 1000, 60).unwrap();
    close_outage(&conn, id, 1600).unwrap();
    let row: (String, String, i64, Option<i64>, i64) = conn
        .query_row(
            "SELECT scope, kind, started_at, ended_at, precision_s FROM grid_event WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .unwrap();
    assert_eq!(row, ("house".into(), "outage".into(), 1000, Some(1600), 60));
    let again = close_outage(&conn, id, 1700).unwrap_err();
    assert!(again.to_string().contains("not open"), "{again}");
    assert!(close_outage(&conn, id + 1, 1700).is_err(), "unknown id");
}

/// One station-day of AC180 polls (the most extras: 6) against the v0.4 budget of
/// 10 MB per station-day (RULES.md).
#[test]
fn a_day_of_ac180_samples_fits_the_size_budget() {
    let db = TempDb::new("size");
    let mut conn = open_writer(&db.path()).unwrap();
    let id = sync_stations(&mut conn, &[station("mobile", Model::Ac180, false)]).unwrap()["mobile"];
    let values: BTreeMap<Metric, f64> = Model::Ac180
        .profile()
        .fields
        .iter()
        .map(|f| (f.metric, 1.0))
        .collect();
    for k in 0..1440 {
        let sample = Sample {
            ts: 1_760_000_000 + 60 * k,
            encrypted: true,
            values: values.clone(),
        };
        insert_sample(&mut conn, id, &sample).unwrap();
    }
    conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")
        .unwrap();
    assert_eq!(count(&conn, "sample_extra"), 1440 * 6);
    let bytes = std::fs::metadata(db.path()).unwrap().len();
    eprintln!("AC180 station-day: {bytes} bytes");
    assert!(bytes < 10_000_000, "{bytes} bytes");
}
