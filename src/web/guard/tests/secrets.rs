//! Security suite type 5, secret hygiene (task `2026-11-07_web-security` step
//! 10a): the bearer token, the password, its hash and the session ids never
//! reach a response body, a header other than the login's own `Set-Cookie`,
//! a redirect or a log line (golden rule #3, ROADMAP.md §8.5 c). Every
//! response class of the router is driven while `tracing` is captured.
//!
//! Elsewhere, not repeated: `Debug` of the config (`config/web.rs`
//! `debug_hides_token_and_hash`), config errors (`no_config_error_quotes_…`),
//! and the login handler's own log lines (`auth/tests.rs`). `AppState` and
//! `Auth` have no `Debug`.

use crate::web::auth::{Auth, password};
use crate::web::tests::Capture;

use super::*;

const PASSWORD: &str = "Pw-7c1e-horse-battery";
const TOKEN: &str = "tok-4f9a-SECRET-0b2d";
const FORM: &str = "Content-Type: application/x-www-form-urlencoded";

// Current-thread runtime: the server's tasks run on this thread, under this
// thread's subscriber (see `auth/tests.rs` for the second dispatcher).
#[tokio::test]
async fn no_secret_reaches_a_response_or_a_log_line() {
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let capture = Capture::default();
    let sink = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let hash = password::hash(PASSWORD).unwrap();
    let db = TempDb::new("sec-hygiene");
    drop(open_writer(&db.path()).unwrap());
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    state.auth = Some(Auth::new(hash.clone()));
    state.token = Some(TOKEN.to_owned());
    state.shutdown.send_replace(true);
    let state = Arc::new(state);
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();

    let bearer = format!("Authorization: Bearer {TOKEN}");
    let wrong = format!("Authorization: Bearer {TOKEN}x");
    let cookie = format!("Cookie: session={id}");
    let login = |password: &str| format!("password={password}&next=/telemetry");
    #[rustfmt::skip]
    let calls: Vec<(&str, &str, Vec<String>, String)> = vec![
        ("GET", "/api/status", vec![wrong.clone()], String::new()),
        ("GET", "/api/status", vec![bearer.clone()], String::new()),
        ("GET", "/events", vec![bearer.clone()], String::new()),
        ("POST", "/api/polling/pause?minutes=0", vec![bearer.clone()], String::new()),
        ("GET", "/api/series?station=x&from=1&to=0", vec![bearer.clone()], String::new()),
        ("GET", "/telemetry?x=1", vec![wrong.clone()], String::new()),
        ("GET", "/", vec![bearer.clone(), "Host: evil.example".into()], String::new()),
        ("POST", "/api/polling/pause", vec![cookie.clone(), "Origin: http://evil".into()], String::new()),
        ("POST", "/login", vec![FORM.into()], login(PASSWORD)),
        ("POST", "/login", vec![FORM.into()], login(&format!("{PASSWORD}x"))),
        ("POST", "/login", vec![FORM.into()], login("a")),
        ("POST", "/login", vec![FORM.into()], login("b")),
        ("POST", "/login", vec![FORM.into()], login("c")),
        ("POST", "/login", vec![FORM.into()], login("d")),
        ("POST", "/login", vec![FORM.into()], login(PASSWORD)),
        ("GET", "/login?next=/x", vec![], String::new()),
        ("GET", "/no-such-page", vec![bearer.clone()], String::new()),
        ("POST", "/logout", vec![cookie.clone(), "Origin: http://localhost".into()], String::new()),
    ];
    let replies = with_server(&state, move |addr| {
        calls
            .iter()
            .map(|(method, path, headers, body)| {
                let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
                request_with(addr, method, path, &headers, body)
            })
            .collect::<Vec<_>>()
    })
    .await;

    // The run reached every class the secrets could leak through.
    let statuses: Vec<&str> = replies.iter().map(|r| r.0.as_str()).collect();
    for want in ["200", "303", "400", "401", "403", "404", "421", "429"] {
        assert!(
            statuses.iter().any(|s| s.contains(want)),
            "{want}: {statuses:?}"
        );
    }
    let (salt, digest) = hash.rsplit_once('$').unwrap();
    let salt = salt.rsplit_once('$').unwrap().1;
    let mut issued = vec![id.clone()];
    for (status, headers, body) in &replies {
        let mut seen = format!("{status}\n{body}\n");
        for line in headers {
            match line.split_once(": ") {
                // The login's own cookie carries a session id by design.
                Some((name, value)) if name.eq_ignore_ascii_case("set-cookie") => {
                    let value = value.strip_prefix("session=").unwrap();
                    let new = value.split(';').next().unwrap();
                    issued.extend((!new.is_empty()).then(|| new.to_owned()));
                }
                _ => seen.push_str(line),
            }
        }
        for secret in [
            TOKEN,
            PASSWORD,
            hash.as_str(),
            salt,
            digest,
            "$argon2",
            id.as_str(),
        ] {
            assert!(!seen.contains(secret), "{secret:?} in {seen}");
        }
    }
    assert_eq!(issued.len(), 2, "one login succeeded: {issued:?}");

    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    for line in [
        "web login succeeded",
        "web login failed",
        "web login refused",
        "web logout",
    ] {
        assert!(logs.contains(line), "{line} not logged: {logs}");
    }
    let mut secrets = vec![
        TOKEN,
        PASSWORD,
        hash.as_str(),
        salt,
        digest,
        "$argon2",
        "Bearer",
    ];
    secrets.extend(issued.iter().map(String::as_str));
    for secret in secrets {
        assert!(!logs.contains(secret), "{secret:?} in {logs}");
    }
}
