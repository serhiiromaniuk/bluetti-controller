use tokio::sync::{mpsc, watch};

use super::*;
use crate::collector::Target;
use crate::config::{ControlConfig, StationConfig};
use crate::protocol::device::Model;
use crate::store::open_reader;
use crate::store::temp_db::TempDb;
use crate::web::tests::{post, start};

const BEARER: Option<&str> = Some("Bearer s3cret");

/// What the fake collector does with a request.
#[derive(Clone, Copy)]
enum Fake {
    /// Answers the wanted state as the read-back.
    ReadBack,
    /// Answers this error.
    Fail(&'static str),
    /// Keeps the request and never answers.
    Silent,
}

fn target(id: i64, name: &str, model: Model) -> Target {
    Target {
        id,
        config: StationConfig {
            name: name.into(),
            model,
            address: "AA:BB:CC:00:00:00".into(),
            grid_sensor: false,
        },
    }
}

/// Serves a state with token `s3cret`, `router` (EB3A) and `office` (Elite
/// 200 V2), control `enabled`, a 1 s interval and `fake` reading the command
/// channel; sends each POST in order. Returns the replies and the
/// `(station, output, on)` of every request the fake received.
async fn run(
    name: &str,
    enabled: bool,
    control: Control,
    fake: Fake,
    posts: &[(&'static str, Option<&'static str>)],
) -> (Vec<(String, String)>, Vec<(String, &'static str, bool)>) {
    let db = TempDb::new(name);
    drop(crate::store::open_writer(&db.path()).unwrap());
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    state.token = Some("s3cret".into());
    state.interval_s = 1;
    state.control = watch::channel(control).0;
    state.control_config = ControlConfig { enabled };
    state.stations = vec![
        target(1, "router", Model::Eb3a),
        target(2, "office", Model::Elite200V2),
    ];
    let (commands, mut requests) = mpsc::channel::<Request>(8);
    state.commands = commands;
    let collector = tokio::spawn(async move {
        let (mut seen, mut kept) = (Vec::new(), Vec::new());
        while let Some(Request { command, reply }) = requests.recv().await {
            seen.push((
                command.station().to_owned(),
                command.output().key(),
                command.on(),
            ));
            match fake {
                Fake::ReadBack => drop(reply.send(Ok(command.on()))),
                Fake::Fail(message) => drop(reply.send(Err(message.into()))),
                Fake::Silent => kept.push(reply),
            }
        }
        seen
    });
    let (addr, stop, server) = start(state).await;
    let posts = posts.to_vec();
    let replies = tokio::task::spawn_blocking(move || {
        posts
            .into_iter()
            .map(|(path, auth)| post(addr, path, auth))
            .collect()
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    (replies, collector.await.unwrap())
}

const AC_OFF: &str = "/api/control/output?station=router&output=ac&state=off";

#[tokio::test(flavor = "multi_thread")]
async fn without_the_right_bearer_is_401_and_sends_nothing() {
    let (replies, seen) = run(
        "control-401",
        true,
        Control::Run,
        Fake::ReadBack,
        &[
            (AC_OFF, None),
            (AC_OFF, Some("Bearer wrong")),
            (AC_OFF, Some("s3cret")),
        ],
    )
    .await;
    for (status, body) in replies {
        assert!(status.contains("401"), "{status}");
        assert!(body.contains("bearer"), "{body}");
    }
    assert!(seen.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn disabled_control_is_403_and_sends_nothing() {
    let (replies, seen) = run(
        "control-403",
        false,
        Control::Run,
        Fake::ReadBack,
        &[(AC_OFF, BEARER)],
    )
    .await;
    assert!(replies[0].0.contains("403"), "{}", replies[0].0);
    assert_eq!(replies[0].1, r#"{"error":"control is disabled"}"#);
    assert!(seen.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_station_output_or_state_is_400() {
    let (replies, seen) = run(
        "control-400",
        true,
        Control::Run,
        Fake::ReadBack,
        &[
            (
                "/api/control/output?station=nope&output=ac&state=on",
                BEARER,
            ),
            ("/api/control/output?output=ac&state=on", BEARER),
            (
                "/api/control/output?station=router&output=usb&state=on",
                BEARER,
            ),
            (
                "/api/control/output?station=router&output=ac&state=1",
                BEARER,
            ),
            // A MAC is never a station.
            (
                "/api/control/output?station=AA:BB:CC:00:00:00&output=ac&state=on",
                BEARER,
            ),
            // The Elite lists no switch until H16 (§4.3).
            (
                "/api/control/output?station=office&output=ac&state=on",
                BEARER,
            ),
        ],
    )
    .await;
    let bodies: Vec<&str> = replies.iter().map(|(_, b)| b.as_str()).collect();
    for (status, _) in &replies {
        assert!(status.contains("400"), "{status}");
    }
    assert!(bodies[0].contains("unknown station"));
    assert!(bodies[2].contains("ac or dc"));
    assert!(bodies[3].contains("on or off"));
    assert!(bodies[5].contains("no ac output switch"), "{}", bodies[5]);
    assert!(seen.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn an_allowed_request_returns_the_read_back() {
    let (replies, seen) = run(
        "control-200",
        true,
        Control::Run,
        Fake::ReadBack,
        &[
            (AC_OFF, BEARER),
            (
                "/api/control/output?station=router&output=dc&state=on",
                BEARER,
            ),
        ],
    )
    .await;
    assert!(replies[0].0.contains("200"), "{}", replies[0].0);
    assert_eq!(
        replies[0].1,
        r#"{"output":"ac","state":"off","station":"router"}"#
    );
    assert_eq!(
        replies[1].1,
        r#"{"output":"dc","state":"on","station":"router"}"#
    );
    assert_eq!(
        seen,
        [
            ("router".into(), "ac", false),
            ("router".into(), "dc", true)
        ]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_collector_error_is_502() {
    let (replies, _) = run(
        "control-502",
        true,
        Control::Run,
        Fake::Fail("polling paused"),
        &[(AC_OFF, BEARER)],
    )
    .await;
    assert!(replies[0].0.contains("502"), "{}", replies[0].0);
    assert_eq!(replies[0].1, r#"{"error":"polling paused"}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn no_answer_within_deadline_plus_interval_is_504() {
    let started = std::time::Instant::now();
    let (replies, seen) = run(
        "control-504",
        true,
        Control::Run,
        Fake::Silent,
        &[(AC_OFF, BEARER)],
    )
    .await;
    // 1 s / 2 stations + 1 s.
    assert!(started.elapsed() >= Duration::from_millis(1500));
    assert!(replies[0].0.contains("504"), "{}", replies[0].0);
    assert_eq!(seen.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn shutting_down_is_503_and_sends_nothing() {
    let (replies, seen) = run(
        "control-503",
        true,
        Control::Shutdown,
        Fake::ReadBack,
        &[(AC_OFF, BEARER)],
    )
    .await;
    assert!(replies[0].0.contains("503"), "{}", replies[0].0);
    assert!(seen.is_empty());
}
