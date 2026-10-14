//! The collector and the store writer end to end (`ROADMAP.md` §11, §12 M3),
//! on a temp database in virtual time, with no hardware. Each poll of a
//! replay station gets a fresh [`ReplayGatt`] over its recorded `read`
//! session; a fourth station always fails to connect. Expected values are the
//! ones `tests/replay.rs` hand-reads from the same fixtures.

use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bluetti_controller::collector::{Collector, Connect, Control, Progress, Target};
use bluetti_controller::config::StationConfig;
use bluetti_controller::metrics::grid::Params;
use bluetti_controller::protocol::device::{self, Metric, Model};
use bluetti_controller::store::{
    Health, Stage, Station, Writer, open_reader, open_writer, station_health, sync_stations,
};
use bluetti_controller::transport::Advert;
use bluetti_controller::transport::fixture::Fixture;
use bluetti_controller::transport::read::ReadPolicy;
use bluetti_controller::transport::replay::ReplayGatt;
use bluetti_controller::transport::station;
use rusqlite::Connection;
use tokio::sync::{mpsc, watch};

const INTERVAL: Duration = Duration::from_secs(60);
const ROUNDS: usize = 3;
/// Always fails to connect.
const ABSENT: &str = "absent";

fn policy() -> ReadPolicy {
    ReadPolicy {
        timeout: Duration::from_secs(5),
        attempts: NonZeroU32::new(1).unwrap(),
        backoff: Duration::ZERO,
    }
}

/// Station name → its recorded session.
struct Replay(BTreeMap<String, Fixture>);

impl Connect for Replay {
    type Gatt = ReplayGatt;

    /// A fresh replay per poll. The fixture opens with the `detect` reads
    /// that `read` ran first, so `detect` consumes them here and the collector
    /// reads the profile blocks that follow.
    async fn open(
        &self,
        station: &StationConfig,
        _retry: bool,
        progress: &mut Progress,
    ) -> Result<(ReplayGatt, bool), String> {
        progress.stage = Stage::Connect;
        let fixture = self.0.get(&station.name).ok_or("connect refused")?;
        let advert: Advert = fixture.advert().map_err(|e| e.to_string())?;
        let mut gatt = ReplayGatt::new(fixture).map_err(|e| e.to_string())?;
        station::detect(&mut gatt, &advert, &policy())
            .await
            .map_err(|e| e.to_string())?;
        let encrypted = device::is_encrypted(&advert.manufacturer_data).unwrap_or(false);
        Ok((gatt, encrypted))
    }

    async fn release(&self, _station: &StationConfig) {}
}

fn config(name: &str, model: Model) -> StationConfig {
    StationConfig {
        name: name.into(),
        model,
        address: "AA:BB:CC:00:00:00".into(),
        grid_sensor: true,
    }
}

/// A per-test directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Every stored value of one sample, core columns and extras, by metric key.
fn stored(conn: &Connection, station: i64, ts: i64) -> BTreeMap<String, f64> {
    let core = [
        ("soc", Metric::BatterySoc),
        ("ac_in_w", Metric::AcInputPower),
        ("dc_in_w", Metric::DcInputPower),
        ("ac_out_w", Metric::AcOutputPower),
        ("dc_out_w", Metric::DcOutputPower),
        ("ac_in_v", Metric::AcInputVoltage),
    ];
    let mut values = BTreeMap::new();
    for (column, metric) in core {
        let value: Option<f64> = conn
            .query_row(
                &format!("SELECT {column} FROM sample WHERE station_id = ?1 AND ts = ?2"),
                (station, ts),
                |row| row.get(0),
            )
            .unwrap();
        if let Some(value) = value {
            values.insert(metric.key().to_owned(), value);
        }
    }
    let mut extras = conn
        .prepare("SELECT metric, value FROM sample_extra WHERE station_id = ?1 AND ts = ?2")
        .unwrap();
    let rows = extras
        .query_map((station, ts), |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    values.extend(rows.map(Result::unwrap));
    values
}

fn by_key(values: &[(Metric, f64)]) -> BTreeMap<String, f64> {
    values
        .iter()
        .map(|(m, v)| (m.key().to_owned(), *v))
        .collect()
}

fn assert_values(got: &BTreeMap<String, f64>, want: &BTreeMap<String, f64>, name: &str) {
    assert_eq!(
        got.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>(),
        "{name}"
    );
    for (key, want) in want {
        assert!(
            (got[key] - want).abs() < 1e-9,
            "{name} {key}: {} != {want}",
            got[key]
        );
    }
}

#[tokio::test(start_paused = true)]
async fn replayed_rounds_land_samples_failures_and_unreachable() {
    let dir = TempDir(std::env::temp_dir().join(format!("bluetti-e2e-{}", std::process::id())));
    std::fs::create_dir_all(&dir.0).unwrap();
    let path = dir.0.join("collector.db");

    let fixture = |name: &str| {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/frames/{name}.json"));
        Fixture::load(&file).unwrap()
    };
    let configs = vec![
        config("router", Model::Eb3a),
        config("mobile", Model::Ac180),
        config("living-room", Model::Elite200V2),
        config(ABSENT, Model::Eb3a),
    ];
    let replay = Replay(BTreeMap::from([
        ("router".to_owned(), fixture("eb3a")),
        ("mobile".to_owned(), fixture("ac180")),
        ("living-room".to_owned(), fixture("elite200v2")),
    ]));

    let ids = sync_stations(&mut open_writer(&path).unwrap(), &configs).unwrap();
    let stations = configs
        .iter()
        .map(|c| Station {
            id: ids[&c.name],
            name: c.name.clone(),
            grid_sensor: c.grid_sensor,
        })
        .collect();
    let params = Params {
        voltage_min: 180.0,
        debounce: NonZeroU32::new(2).unwrap(),
        interval_s: 60,
    };
    let writer = Writer::restore(
        open_writer(&path).unwrap(),
        stations,
        params,
        NonZeroU32::new(30).unwrap(),
    )
    .unwrap();

    let (tx, rx) = mpsc::channel(1024);
    let collector = Collector {
        connect: replay,
        stations: configs
            .iter()
            .map(|c| Target {
                id: ids[&c.name],
                config: c.clone(),
            })
            .collect(),
        interval: INTERVAL,
        policy: policy(),
        records: tx,
    };
    let (control, watch_rx) = watch::channel(Control::Run);
    let collecting = tokio::spawn(collector.run(watch_rx));
    // Ticks at 0, 60 and 120 s; stop before the fourth.
    tokio::time::sleep(INTERVAL * 2 + INTERVAL / 2).await;
    control.send(Control::Shutdown).unwrap();
    collecting.await.unwrap();
    // The collector dropped its sender; the writer drains and returns, so
    // virtual time never waits on its blocking thread.
    writer.spawn(rx).await.unwrap();

    let conn = open_reader(&path).unwrap();
    let expected = [
        (
            "router",
            true,
            by_key(&[
                (Metric::DcInputPower, 0.0),
                (Metric::AcInputPower, 0.0),
                (Metric::AcOutputPower, 0.0),
                (Metric::DcOutputPower, 0.0),
                (Metric::BatterySoc, 90.0),
                (Metric::AcOutputOn, 1.0),
                (Metric::DcOutputOn, 0.0),
                (Metric::AcInputVoltage, 0.0),
                (Metric::DcInputVoltage, 0.0),
            ]),
        ),
        (
            "mobile",
            device::is_encrypted(&fixture("ac180").advert().unwrap().manufacturer_data)
                .unwrap_or(false),
            by_key(&[
                (Metric::PackVoltage, 33.09),
                (Metric::PackCurrent, 1.0),
                (Metric::BatterySoc, 93.0),
                (Metric::DcOutputPower, 0.0),
                (Metric::AcOutputPower, 0.0),
                (Metric::DcInputPower, 0.0),
                (Metric::AcInputPower, 0.0),
                (Metric::DcInputVoltage, 0.1),
                (Metric::AcInputFrequency, 0.0),
                (Metric::AcInputVoltage, 2.1),
                (Metric::AcOutputOn, 1.0),
                (Metric::DcOutputOn, 0.0),
            ]),
        ),
        (
            "living-room",
            device::is_encrypted(&fixture("elite200v2").advert().unwrap().manufacturer_data)
                .unwrap_or(false),
            by_key(&[
                (Metric::PackVoltage, 39.31),
                (Metric::PackCurrent, 9.9),
                (Metric::BatterySoc, 69.0),
                (Metric::DcOutputPower, 0.0),
                (Metric::AcOutputPower, 408.0),
                (Metric::DcInputPower, 0.0),
                (Metric::AcInputPower, 0.0),
                (Metric::AcInputFrequency, 0.0),
                (Metric::AcInputVoltage, 0.0),
            ]),
        ),
    ];
    for (name, encrypted, want) in &expected {
        let id = ids[*name];
        let rows: Vec<(i64, bool)> = conn
            .prepare("SELECT ts, encrypted FROM sample WHERE station_id = ?1 ORDER BY ts")
            .unwrap()
            .query_map([id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows.len(), ROUNDS, "{name} samples");
        for (ts, enc) in &rows {
            assert_eq!(enc, encrypted, "{name} encrypted");
            assert_values(&stored(&conn, id, *ts), want, name);
        }
        let failures: i64 = conn
            .query_row(
                "SELECT count(*) FROM poll_failure WHERE station_id = ?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(failures, 0, "{name} failures");
        assert_eq!(station_health(&conn, id).unwrap(), Health::Ok, "{name}");
    }

    let absent = ids[ABSENT];
    let failures: Vec<(i64, String)> = conn
        .prepare("SELECT ts, stage FROM poll_failure WHERE station_id = ?1 ORDER BY ts")
        .unwrap()
        .query_map([absent], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(failures.len(), ROUNDS);
    assert!(failures.iter().all(|(_, stage)| stage == "connect"));
    assert_eq!(
        station_health(&conn, absent).unwrap(),
        Health::Unreachable {
            since: failures[0].0
        }
    );
    let samples: i64 = conn
        .query_row(
            "SELECT count(*) FROM sample WHERE station_id = ?1",
            [absent],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(samples, 0);
}
