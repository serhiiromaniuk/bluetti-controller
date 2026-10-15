use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use tokio::sync::watch;
use tokio::time::Instant;

use crate::collector::Control;
use crate::store::open_reader;
use crate::store::temp_db::TempDb;
use crate::web::AppState;
use crate::web::tests::start;

/// A raw HTTP/1.1 POST with an optional `Authorization` header; returns the
/// status line and the body.
fn post(addr: SocketAddr, path: &str, auth: Option<&str>) -> (String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    let auth = auth.map_or(String::new(), |a| format!("Authorization: {a}\r\n"));
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (head.lines().next().unwrap().to_owned(), body.to_owned())
}

/// Serves a test state (with `token`, starting at `control`), sends each
/// `(path, auth)` POST in order, and returns the replies plus the final
/// `Control`.
async fn run(
    name: &str,
    token: Option<&str>,
    control: Control,
    posts: &[(&'static str, Option<&'static str>)],
) -> (Vec<(String, String)>, Control) {
    let db = TempDb::new(name);
    drop(crate::store::open_writer(&db.path()).unwrap());
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    state.token = token.map(str::to_owned);
    state.control = watch::channel(control).0;
    let rx = state.control.subscribe();
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
    let last = *rx.borrow();
    (replies, last)
}

const OK: &str = "HTTP/1.1 200 OK";

fn paused_until(body: &str) -> Option<i64> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    json["paused_until"].as_i64()
}

#[tokio::test(flavor = "multi_thread")]
async fn minutes_in_range_pause_and_zero_resumes() {
    let before = Instant::now();
    let (replies, last) = run(
        "pause-range",
        None,
        Control::Run,
        &[("/api/polling/pause?minutes=5", None)],
    )
    .await;
    assert_eq!(replies[0].0, OK);
    let at = paused_until(&replies[0].1).unwrap();
    assert!((at - (super::super::unix_now() + 300)).abs() <= 2, "{at}");
    let Control::Pause { until } = last else {
        panic!("{last:?}")
    };
    let left = until - before;
    assert!(left >= Duration::from_secs(300) && left < Duration::from_secs(302));

    let (replies, last) = run(
        "pause-zero",
        None,
        Control::Pause {
            until: Instant::now() + Duration::from_secs(600),
        },
        &[
            ("/api/polling/pause?minutes=0", None),
            ("/api/polling/pause?minutes=60", None),
        ],
    )
    .await;
    assert_eq!(
        replies[0],
        (OK.to_owned(), r#"{"paused_until":null}"#.to_owned())
    );
    assert_eq!(replies[1].0, OK);
    assert!(matches!(last, Control::Pause { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn bad_minutes_are_400_and_change_nothing() {
    let paths = [
        "/api/polling/pause?minutes=61",
        "/api/polling/pause?minutes=-1",
        "/api/polling/pause",
        "/api/polling/pause?minutes=abc",
        "/api/polling/pause?minutes=",
    ];
    let posts: Vec<_> = paths.iter().map(|p| (*p, None)).collect();
    let (replies, last) = run("pause-bad", None, Control::Run, &posts).await;
    for (status, body) in replies {
        assert_eq!(status, "HTTP/1.1 400 Bad Request");
        assert!(body.starts_with(r#"{"error":""#), "{body}");
    }
    assert_eq!(last, Control::Run);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_set_token_is_required() {
    let path = "/api/polling/pause?minutes=0";
    let (replies, last) = run(
        "pause-token",
        Some("s3cret"),
        Control::Pause {
            until: Instant::now() + Duration::from_secs(600),
        },
        &[
            (path, None),
            (path, Some("Bearer s3creT")),
            (path, Some("Bearer s3cret-longer")),
            (path, Some("s3cret")),
        ],
    )
    .await;
    for (status, body) in &replies {
        assert_eq!(status, "HTTP/1.1 401 Unauthorized");
        assert!(body.starts_with(r#"{"error":""#), "{body}");
    }
    assert!(matches!(last, Control::Pause { .. }), "{last:?}");

    let (replies, last) = run(
        "pause-token-ok",
        Some("s3cret"),
        Control::Run,
        &[("/api/polling/pause?minutes=1", Some("Bearer s3cret"))],
    )
    .await;
    assert_eq!(replies[0].0, OK);
    assert!(matches!(last, Control::Pause { .. }), "{last:?}");
}

#[test]
fn token_compare_checks_length_and_every_byte() {
    assert!(super::same(b"abc", b"abc"));
    assert!(!super::same(b"abc", b"abd"));
    assert!(!super::same(b"abc", b"ab"));
    assert!(!super::same(b"", b"a"));
}

/// A2: `serve` sends `Shutdown` while the server still runs; a POST then must
/// not overwrite it, or the collector never stops.
#[tokio::test(flavor = "multi_thread")]
async fn a_post_never_replaces_shutdown() {
    let (replies, last) = run(
        "pause-shutdown",
        None,
        Control::Shutdown,
        &[
            ("/api/polling/pause?minutes=5", None),
            ("/api/polling/pause?minutes=0", None),
        ],
    )
    .await;
    for (status, body) in &replies {
        assert_eq!(status, "HTTP/1.1 503 Service Unavailable");
        assert!(body.starts_with(r#"{"error":""#), "{body}");
    }
    assert_eq!(last, Control::Shutdown);
}
