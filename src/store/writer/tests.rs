use std::num::NonZeroU32;
use std::path::Path;

use super::*;
use crate::config::StationConfig;
use crate::protocol::device::Model;
use crate::store::temp_db::TempDb;
use crate::store::{Stage, open_reader, open_writer, sync_stations};

const P: i64 = 60;
const ON: f64 = 230.0;
const OFF: f64 = 0.0;
const RETENTION: NonZeroU32 = NonZeroU32::new(30).unwrap();

fn params() -> Params {
    debounced(2)
}

fn debounced(n: u32) -> Params {
    Params {
        voltage_min: 180.0,
        debounce: NonZeroU32::new(n).unwrap(),
        interval_s: P,
    }
}

/// Syncs `(name, grid_sensor)` stations into the file and returns them as the
/// writer sees them.
fn stations(path: &Path, list: &[(&str, bool)]) -> Vec<Station> {
    let configs: Vec<StationConfig> = list
        .iter()
        .map(|(name, grid_sensor)| StationConfig {
            name: (*name).into(),
            model: Model::Eb3a,
            address: "AA:BB:CC:DD:EE:FF".into(),
            grid_sensor: *grid_sensor,
        })
        .collect();
    let ids = sync_stations(&mut open_writer(path).unwrap(), &configs).unwrap();
    list.iter()
        .map(|(name, grid_sensor)| Station {
            id: ids[*name],
            name: (*name).into(),
            grid_sensor: *grid_sensor,
        })
        .collect()
}

fn volts(station: &Station, ts: i64, v: f64) -> Record {
    Record::Sample {
        station: station.id,
        sample: Sample {
            ts,
            encrypted: true,
            values: BTreeMap::from([(Metric::AcInputVoltage, v)]),
        },
    }
}

fn failed(station: &Station, ts: i64) -> Record {
    Record::Failure {
        station: station.id,
        failure: Failure {
            ts,
            stage: Stage::Connect,
            error: "timed out".into(),
        },
    }
}

/// One writer lifetime: restore from the file, take `records`, close the channel
/// and wait for the thread to return.
async fn run(path: &Path, stations: &[Station], records: Vec<Record>) {
    let conn = open_writer(path).unwrap();
    let writer = Writer::restore(conn, stations.to_vec(), params(), RETENTION).unwrap();
    let (tx, rx) = mpsc::channel(4);
    let handle = writer.spawn(rx);
    for record in records {
        tx.send(record).await.unwrap();
    }
    drop(tx);
    handle.await.unwrap();
}

/// Every `grid_event` row: (scope, started_at, ended_at, precision_s), by id.
fn events(path: &Path) -> Vec<(String, i64, Option<i64>, i64)> {
    let conn = open_reader(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT scope, started_at, ended_at, precision_s FROM grid_event ORDER BY id")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

/// One poll round per entry: every station's voltage at `ts`, in order.
fn rounds(stations: &[Station], rows: &[(i64, &[f64])]) -> Vec<Record> {
    rows.iter()
        .flat_map(|(ts, vs)| stations.iter().zip(*vs).map(|(s, v)| volts(s, *ts, *v)))
        .collect()
}

#[tokio::test]
async fn house_outage_survives_a_restart_as_one_event() {
    let db = TempDb::new("writer-restart");
    let s = stations(&db.path(), &[("a", true), ("b", true)]);
    let first = rounds(
        &s,
        &[
            (0, &[ON, ON]),
            (60, &[ON, ON]),
            (120, &[OFF, OFF]),
            (180, &[OFF, OFF]),
        ],
    );
    run(&db.path(), &s, first).await;
    let open = vec![
        ("a".into(), 120, None, P),
        ("b".into(), 120, None, P),
        ("house".into(), 120, None, P),
    ];
    assert_eq!(events(&db.path()), open);

    // A new writer on the same file: `a` sees grid again from 300.
    let second = rounds(
        &s,
        &[(240, &[OFF, OFF]), (300, &[ON, OFF]), (360, &[ON, OFF])],
    );
    run(&db.path(), &s, second).await;
    assert_eq!(
        events(&db.path()),
        vec![
            ("a".into(), 120, Some(300), P),
            ("b".into(), 120, None, P),
            ("house".into(), 120, Some(300), P),
        ]
    );
}

#[tokio::test]
async fn flapping_input_opens_no_event() {
    let db = TempDb::new("writer-flap");
    let s = stations(&db.path(), &[("a", true)]);
    let flap = [ON, ON, OFF, ON, OFF, ON, OFF, ON];
    let records = (0..)
        .zip(flap)
        .map(|(i, v)| volts(&s[0], i * P, v))
        .collect();
    run(&db.path(), &s, records).await;
    assert_eq!(events(&db.path()), vec![]);
}

#[test]
fn a_gap_is_unknown_and_opens_no_event() {
    let db = TempDb::new("writer-gap");
    let s = stations(&db.path(), &[("a", true), ("b", true)]);
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        RETENTION,
    )
    .unwrap();
    for record in rounds(&s, &[(0, &[ON, ON]), (60, &[ON, ON])]) {
        writer.handle(&record).unwrap();
    }
    for ts in (120..=420).step_by(60) {
        for station in &s {
            writer.handle(&failed(station, ts)).unwrap();
        }
    }
    let states: Vec<_> = writer.stations.iter().map(|t| t.grid.state()).collect();
    assert_eq!(states, vec![(GridState::Unknown, Some(60)); 2]);
    for record in rounds(&s, &[(480, &[ON, ON]), (540, &[ON, ON])]) {
        writer.handle(&record).unwrap();
    }
    assert_eq!(
        writer.stations[0].grid.state(),
        (GridState::Present, Some(480))
    );
    assert_eq!(events(&db.path()), vec![]);
}

/// Only the tick makes an unreachable sensor `Unknown`, so it stops voting
/// (§7.2) and the other sensor's `Absent` opens the house outage (A3, incl.
/// debounce 1).
#[test]
fn an_unreachable_sensor_stops_voting_after_the_tick() {
    for debounce in [1, 2] {
        let db = TempDb::new(&format!("writer-tick-{debounce}"));
        let s = stations(&db.path(), &[("a", true), ("b", true)]);
        let conn = open_writer(&db.path()).unwrap();
        let mut writer = Writer::restore(conn, s.clone(), debounced(debounce), RETENTION).unwrap();
        for record in rounds(&s, &[(0, &[ON, ON]), (60, &[ON, ON])]) {
            writer.handle(&record).unwrap();
        }
        for ts in (120..=240).step_by(60) {
            writer.handle(&failed(&s[0], ts)).unwrap();
            writer.handle(&volts(&s[1], ts, OFF)).unwrap();
        }
        assert_eq!(
            writer.house.state(),
            HouseState::Power,
            "debounce {debounce}"
        );
        writer.handle(&failed(&s[0], 300)).unwrap();
        assert_eq!(
            events(&db.path()),
            vec![("b".into(), 120, None, P), ("house".into(), 120, None, P)],
            "debounce {debounce}"
        );
    }
}

/// The restart splits a debounce run: the restored samples count toward it.
#[tokio::test]
async fn restore_replays_the_last_samples_in_order() {
    let db = TempDb::new("writer-replay");
    let s = stations(&db.path(), &[("a", true)]);
    run(
        &db.path(),
        &s,
        rounds(&s, &[(0, &[ON]), (60, &[ON]), (120, &[OFF])]),
    )
    .await;
    assert_eq!(events(&db.path()), vec![]);
    run(&db.path(), &s, rounds(&s, &[(180, &[OFF])])).await;
    assert_eq!(
        events(&db.path()),
        vec![("a".into(), 120, None, P), ("house".into(), 120, None, P)]
    );
}

#[tokio::test]
async fn an_outage_stays_open_through_unknown_and_ends_backdated() {
    let db = TempDb::new("writer-unknown");
    let s = stations(&db.path(), &[("a", true), ("router", false)]);
    let mut records = rounds(
        &s,
        &[
            (0, &[ON, ON]),
            (60, &[ON, ON]),
            (120, &[OFF, ON]),
            (180, &[OFF, ON]),
        ],
    );
    for ts in (240..=540).step_by(60) {
        records.push(failed(&s[0], ts));
        records.push(volts(&s[1], ts, ON));
    }
    records.extend(rounds(
        &s,
        &[(600, &[OFF, ON]), (660, &[ON, ON]), (720, &[ON, ON])],
    ));
    run(&db.path(), &s, records).await;
    // `router` is no sensor and always had grid: only `a` and the house record.
    assert_eq!(
        events(&db.path()),
        vec![
            ("a".into(), 120, Some(660), P),
            ("house".into(), 120, Some(660), P),
        ]
    );
}

#[tokio::test]
async fn stores_failures_and_snapshots_and_rejects_an_unknown_station() {
    let db = TempDb::new("writer-rows");
    let s = stations(&db.path(), &[("a", true)]);
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        RETENTION,
    )
    .unwrap();
    writer.handle(&failed(&s[0], 0)).unwrap();
    let snapshot = Record::Snapshot {
        station: s[0].id,
        ts: 0,
        start_reg: 10,
        values: vec![0x0102, 0x0304],
    };
    writer.handle(&snapshot).unwrap();
    let stray = Station {
        id: s[0].id + 1,
        ..s[0].clone()
    };
    assert!(writer.handle(&volts(&stray, 60, ON)).is_err());

    let conn = open_reader(&db.path()).unwrap();
    let count = |table: &str| -> i64 {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    };
    assert_eq!((count("poll_failure"), count("sample")), (1, 0));
    let data: Vec<u8> = conn
        .query_row("SELECT data FROM register_snapshot", [], |r| r.get(0))
        .unwrap();
    assert_eq!(data, vec![1, 2, 3, 4]);
}

#[test]
fn a_station_named_house_is_refused() {
    let db = TempDb::new("writer-house-name");
    let s = stations(&db.path(), &[("house", true)]);
    let err = Writer::restore(open_writer(&db.path()).unwrap(), s, params(), RETENTION).err();
    assert!(format!("{:#}", err.unwrap()).contains("house"));
}
