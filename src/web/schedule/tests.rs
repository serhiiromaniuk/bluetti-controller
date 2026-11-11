use tokio::sync::mpsc;

use super::*;
use crate::collector::Target;
use crate::config::StationConfig;
use crate::protocol::device::Model;
use crate::store::temp_db::TempDb;
use crate::store::{open_reader, open_writer, sync_stations};
use crate::web::tests::{get, send, start};

const BEARER: Option<&str> = Some("Bearer s3cret");

fn station(name: &str, model: Model) -> StationConfig {
    StationConfig {
        name: name.into(),
        model,
        address: "AA:BB:CC:00:00:00".into(),
        grid_sensor: false,
    }
}

/// Serves `router` (EB3A) and `office` (Elite 200 V2) over a temp db holding
/// `rows` (SQL), with `token`, control disabled, and a fake record receiver;
/// sends each `(method, path, auth)` in order. Returns the replies and every
/// record the writer would have received.
async fn run(
    name: &str,
    token: Option<&str>,
    rows: &str,
    calls: &[(&'static str, &'static str, Option<&'static str>)],
) -> (Vec<(String, String)>, Vec<Record>) {
    let db = TempDb::new(name);
    let mut conn = open_writer(&db.path()).unwrap();
    let configs = [
        station("router", Model::Eb3a),
        station("office", Model::Elite200V2),
    ];
    let ids = sync_stations(&mut conn, &configs).unwrap();
    conn.execute_batch(rows).unwrap();
    drop(conn);
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    state.token = token.map(str::to_owned);
    state.stations = configs
        .iter()
        .map(|config| Target {
            id: ids[&config.name],
            config: config.clone(),
        })
        .collect();
    let (records, mut queue) = mpsc::channel::<Record>(16);
    state.records = records.downgrade();
    let (addr, stop, server) = start(state).await;
    let calls = calls.to_vec();
    let replies = tokio::task::spawn_blocking(move || {
        calls
            .into_iter()
            .map(|(method, path, auth)| send(addr, method, path, auth))
            .collect()
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    drop(records);
    let mut seen = Vec::new();
    while let Some(record) = queue.recv().await {
        seen.push(record);
    }
    (replies, seen)
}

const SET: &str = "/api/schedule?station=router&output=ac&on=07:30&off=23:00";
const DELETE: &str = "/api/schedule?station=router&output=dc";

#[tokio::test(flavor = "multi_thread")]
async fn edits_without_a_configured_token_are_403() {
    let (replies, seen) = run(
        "schedule-403",
        None,
        "",
        &[("POST", SET, BEARER), ("DELETE", DELETE, None)],
    )
    .await;
    for (status, body) in replies {
        assert!(status.contains("403"), "{status}");
        assert_eq!(body, r#"{"error":"schedule edits need web.token"}"#);
    }
    assert!(seen.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn each_bad_field_is_400_naming_it() {
    let cases: &[(&'static str, &str)] = &[
        ("/api/schedule?station=nope&output=ac&on=07:00", "station"),
        ("/api/schedule?output=ac&on=07:00", "station"),
        (
            "/api/schedule?station=AA:BB:CC:00:00:00&output=ac&on=07:00",
            "station",
        ),
        ("/api/schedule?station=router&output=usb&on=07:00", "output"),
        ("/api/schedule?station=router&output=ac&on=24:00", "on"),
        ("/api/schedule?station=router&output=ac&on=7:00", "on"),
        ("/api/schedule?station=router&output=ac&off=23:60", "off"),
        ("/api/schedule?station=router&output=ac&off=ab:cd", "off"),
        ("/api/schedule?station=router&output=ac", "on or off"),
        (
            "/api/schedule?station=router&output=ac&on=&off=",
            "on or off",
        ),
        (
            "/api/schedule?station=router&output=ac&on=08:00&off=08:00",
            "on and off",
        ),
    ];
    let calls: Vec<_> = cases
        .iter()
        .map(|(path, _)| ("POST", *path, BEARER))
        .collect();
    let (replies, seen) = run("schedule-400", Some("s3cret"), "", &calls).await;
    for ((status, body), (path, field)) in replies.iter().zip(cases) {
        assert!(status.contains("400"), "{path}: {status}");
        assert!(body.contains(field), "{path}: {body}");
    }
    let (replies, _) = run(
        "schedule-400-delete",
        Some("s3cret"),
        "",
        &[
            ("DELETE", "/api/schedule?station=nope&output=ac", BEARER),
            ("DELETE", "/api/schedule?station=router&output=usb", BEARER),
        ],
    )
    .await;
    assert!(replies[0].0.contains("400") && replies[0].1.contains("station"));
    assert!(replies[1].0.contains("400") && replies[1].1.contains("output"));
    assert!(seen.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn valid_edits_are_202_and_send_the_record_while_control_is_disabled() {
    let (replies, seen) = run(
        "schedule-202",
        Some("s3cret"),
        "",
        &[
            ("POST", SET, BEARER),
            (
                "POST",
                "/api/schedule?station=router&output=dc&off=00:05",
                BEARER,
            ),
            ("DELETE", DELETE, BEARER),
        ],
    )
    .await;
    for (status, _) in &replies {
        assert!(status.contains("202"), "{status}");
    }
    assert_eq!(
        replies[0].1,
        r#"{"off":"23:00","on":"07:30","output":"ac","station":"router"}"#
    );
    assert_eq!(
        replies[1].1,
        r#"{"off":"00:05","on":null,"output":"dc","station":"router"}"#
    );
    assert_eq!(replies[2].1, r#"{"output":"dc","station":"router"}"#);
    assert_eq!(
        seen,
        [
            Record::ScheduleSet {
                station: 1,
                output: Output::Ac,
                on_min: Some(450),
                off_min: Some(1380),
            },
            Record::ScheduleSet {
                station: 1,
                output: Output::Dc,
                on_min: None,
                off_min: Some(5),
            },
            Record::ScheduleDelete {
                station: 1,
                output: Output::Dc,
            },
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn get_lists_the_entries_by_station_name() {
    // Station 9 is no longer in the config: its row is not listed.
    let rows = "INSERT INTO schedule VALUES (1, 'ac', 450, 1380, 1700000000);
                INSERT INTO schedule VALUES (1, 'dc', NULL, 5, NULL);
                INSERT INTO station VALUES (9, 'gone', 'EB3A', 268.0, 0);
                INSERT INTO schedule VALUES (9, 'ac', 60, NULL, NULL);";
    let db = TempDb::new("schedule-get");
    let mut conn = open_writer(&db.path()).unwrap();
    let configs = [station("router", Model::Eb3a)];
    let ids = sync_stations(&mut conn, &configs).unwrap();
    conn.execute_batch(rows).unwrap();
    drop(conn);
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    state.stations = vec![Target {
        id: ids["router"],
        config: configs[0].clone(),
    }];
    let (addr, stop, server) = start(state).await;
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/api/schedule"))
        .await
        .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert!(status.contains("200"), "{status}");
    assert_eq!(
        body,
        r#"[{"station":"router","output":"ac","on":"07:30","off":"23:00"},{"station":"router","output":"dc","on":null,"off":"00:05"}]"#
    );
}

#[test]
fn hh_mm_parses_and_formats() {
    assert_eq!(minutes("00:00"), Some(0));
    assert_eq!(minutes("23:59"), Some(1439));
    assert_eq!(minutes("12:5"), None);
    assert_eq!(minutes("+1:05"), None);
    assert_eq!(minutes("12:60"), None);
    assert_eq!(hh_mm(450), "07:30");
    assert_eq!(hh_mm(1439), "23:59");
}
