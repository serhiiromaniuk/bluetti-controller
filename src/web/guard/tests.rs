use std::sync::Arc;

use super::*;
use crate::store::temp_db::TempDb;
use crate::store::{open_reader, open_writer};
use crate::web::tests::{header, request_with, start_shared};

/// Every protected route of `router()` as (method, path); `auth_guard` covers
/// them (§8.5 a).
const PROTECTED: &[(&str, &str)] = &[
    ("GET", "/"),
    ("POST", "/logout"),
    ("GET", "/telemetry"),
    ("GET", "/api/status"),
    ("GET", "/api/series"),
    ("GET", "/api/chart"),
    ("GET", "/api/outages"),
    ("GET", "/api/energy"),
    ("POST", "/api/polling/pause"),
    ("POST", "/api/control/output"),
    ("GET", "/api/schedule"),
    ("POST", "/api/schedule"),
    ("DELETE", "/api/schedule"),
    ("GET", "/events"),
];

/// The routes that answer without a credential (§8.5 a, Q6).
const PUBLIC: &[(&str, &str)] = &[
    ("GET", "/login"),
    ("POST", "/login"),
    ("GET", "/assets/{name}"),
    ("GET", "/healthz"),
];

const ORIGIN: &str = "Origin: http://localhost";

/// A test state over an empty db; `/events` ends at once (shutdown fired).
fn state(name: &str, login: bool, token: Option<&str>) -> (TempDb, Arc<AppState>) {
    let db = TempDb::new(name);
    drop(open_writer(&db.path()).unwrap());
    let mut state = AppState::for_tests(open_reader(&db.path()).unwrap());
    if login {
        state = state.with_test_password();
    }
    state.token = token.map(str::to_owned);
    state.allowed_hosts = vec!["ideapad.lan".into()];
    state.shutdown.send_replace(true);
    (db, Arc::new(state))
}

/// Sends each `(method, path, headers)` off the runtime; returns status line,
/// headers and body per request.
async fn send_all(
    state: &Arc<AppState>,
    calls: Vec<(&'static str, String, Vec<String>)>,
) -> Vec<(String, Vec<String>, String)> {
    let (addr, stop, server) = start_shared(Arc::clone(state)).await;
    let replies = tokio::task::spawn_blocking(move || {
        calls
            .into_iter()
            .map(|(method, path, headers)| {
                let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
                request_with(addr, method, &path, &headers, "")
            })
            .collect()
    })
    .await
    .unwrap();
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
    replies
}

fn concrete(path: &str) -> String {
    path.replace("{name}", "app.js")
}

fn session(state: &AppState) -> String {
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();
    format!("Cookie: session={id}")
}

#[test]
fn every_route_of_the_router_is_listed_once() {
    let source = include_str!("../mod.rs");
    let routes: Vec<&str> = source
        .split(".route(")
        .skip(1)
        .map(|rest| rest.trim_start().split('"').nth(1).expect("quoted path"))
        .collect();
    assert!(routes.len() >= 15, "{routes:?}");
    for path in &routes {
        let protected = PROTECTED.iter().any(|(_, p)| p == path);
        let public = PUBLIC.iter().any(|(_, p)| p == path);
        assert!(protected ^ public, "{path} must be in exactly one list");
    }
    for (_, path) in PROTECTED.iter().chain(PUBLIC) {
        assert!(routes.contains(path), "{path} is listed but not routed");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn without_a_credential_every_protected_route_is_refused() {
    let (_db, state) = state("guard-walk-none", true, None);
    let calls = PROTECTED
        .iter()
        .map(|(m, p)| (*m, format!("{p}?a=1&b=/x"), vec![ORIGIN.to_owned()]))
        .collect();
    let replies = send_all(&state, calls).await;
    for ((_, path), (status, headers, body)) in PROTECTED.iter().zip(replies) {
        if path.starts_with("/api/") || *path == "/events" {
            assert_eq!(status, "HTTP/1.1 401 Unauthorized", "{path}");
            assert_eq!(body, r#"{"error":"login required"}"#, "{path}");
        } else {
            assert_eq!(status, "HTTP/1.1 303 See Other", "{path}");
            let want = format!("/login?next={}%3Fa%3D1%26b%3D/x", path);
            assert_eq!(header(&headers, "location"), Some(want.as_str()));
        }
    }
}

/// Not refused by the guard: no 401, no 421, no redirect to the login and not
/// the guard's own 403 (a handler may still answer 400/403 for its own rules).
fn passed(path: &str, (status, headers, body): &(String, Vec<String>, String)) {
    assert!(
        !status.contains("401") && !status.contains("421"),
        "{path}: {status}"
    );
    assert!(!body.contains("cross-origin"), "{path}: {body}");
    let location = header(headers, "location").unwrap_or("");
    assert!(!location.starts_with("/login?"), "{path}: {location}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_cookie_or_the_bearer_passes_every_protected_route() {
    let (_db, state) = state("guard-walk-ok", true, Some("s3cret"));
    let mut calls = Vec::new();
    for (m, p) in PROTECTED {
        calls.push((*m, p.to_string(), vec![session(&state), ORIGIN.to_owned()]));
        // Scripts send the bearer and no Origin.
        calls.push((
            *m,
            p.to_string(),
            vec!["Authorization: Bearer s3cret".into()],
        ));
    }
    let replies = send_all(&state, calls).await;
    for (i, reply) in replies.iter().enumerate() {
        passed(PROTECTED[i / 2].1, reply);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn public_routes_answer_without_a_credential() {
    let (_db, state) = state("guard-public", true, None);
    let calls = PUBLIC
        .iter()
        .map(|(m, p)| (*m, concrete(p), vec![]))
        .collect();
    let replies = send_all(&state, calls).await;
    for ((_, path), (status, _, _)) in PUBLIC.iter().zip(replies) {
        // `POST /login` without a password is a 4xx form error, never 401/303.
        assert!(
            !status.contains("303") && !status.contains("421"),
            "{path}: {status}"
        );
        assert!(!status.contains("401"), "{path}: {status}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unsafe_method_needs_its_own_origin() {
    let (_db, state) = state("guard-origin", true, None);
    let pause = "/api/polling/pause?minutes=0";
    let host = "Host: 127.0.0.1:8080".to_owned();
    let with = |origin: &str| {
        let mut headers = vec![session(&state), host.clone()];
        if !origin.is_empty() {
            headers.push(format!("Origin: {origin}"));
        }
        ("POST", pause.to_owned(), headers)
    };
    let calls = vec![
        with("http://evil.example"),
        with("http://127.0.0.1:9999"),
        with("http://127.0.0.1"),
        with(""),
        with("null"),
        with("http://127.0.0.1:8080"),
        with("https://127.0.0.1:8080"),
    ];
    let replies = send_all(&state, calls).await;
    for (status, _, body) in &replies[..5] {
        assert_eq!(status, "HTTP/1.1 403 Forbidden");
        assert_eq!(body, r#"{"error":"cross-origin request refused"}"#);
    }
    for (status, _, _) in &replies[5..] {
        assert_eq!(status, "HTTP/1.1 200 OK");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_host_rule_refuses_unknown_names_but_not_healthz() {
    let (_db, state) = state("guard-host", true, None);
    let h = |host: &str| vec![format!("Host: {host}")];
    let calls = vec![
        ("GET", "/".to_owned(), h("evil.example")),
        ("POST", "/login".to_owned(), h("evil.example")),
        ("GET", "/login".to_owned(), h("other.lan:8080")),
        ("GET", "/login".to_owned(), vec![]),
        ("GET", "/login".to_owned(), h("192.168.0.5:8080")),
        ("GET", "/login".to_owned(), h("[::1]:8080")),
        ("GET", "/login".to_owned(), h("localhost")),
        ("GET", "/login".to_owned(), h("IDEAPAD.lan:8080")),
        ("GET", "/healthz".to_owned(), h("evil.example")),
    ];
    let replies = send_all(&state, calls).await;
    for (status, _, body) in &replies[..3] {
        assert_eq!(status, "HTTP/1.1 421 Misdirected Request");
        assert_eq!(body, r#"{"error":"unknown Host"}"#);
    }
    for (status, _, _) in &replies[3..] {
        assert_eq!(status, "HTTP/1.1 200 OK");
    }
}

#[test]
fn host_rule_edge_cases() {
    let allowed = vec!["ideapad.lan".to_owned()];
    for ok in [
        "127.0.0.1",
        "10.0.0.1:1",
        "[::1]",
        "[fe80::1]:8080",
        "LocalHost:80",
        "ideapad.lan",
    ] {
        assert!(host_allowed(ok, &allowed), "{ok}");
    }
    for bad in [
        "",
        ":8080",
        "::1",
        "[::1",
        "[::1]x",
        "[evil]:80",
        "1.2.3.4:",
        "1.2.3.4:123456",
        "1.2.3.4:8a",
        "1.2.3",
        "127.0.0.1.nip.io",
        "evil.example:80",
        "ideapad.lan.evil.example",
        "localhost.",
    ] {
        assert!(!host_allowed(bad, &allowed), "{bad}");
    }
    assert!(!host_allowed("ideapad.lan", &[]));
}

#[tokio::test(flavor = "multi_thread")]
async fn dev_mode_leaves_get_open_but_checks_host_and_origin() {
    let (_db, state) = state("guard-dev", false, None);
    let pause = "/api/polling/pause?minutes=0";
    let calls = vec![
        ("GET", "/api/status".to_owned(), vec![]),
        ("GET", "/".to_owned(), vec![]),
        ("POST", pause.to_owned(), vec![]),
        (
            "POST",
            pause.to_owned(),
            vec!["Origin: http://evil.example".into()],
        ),
        ("POST", pause.to_owned(), vec![ORIGIN.into()]),
        ("GET", "/".to_owned(), vec!["Host: evil.example".into()]),
    ];
    let replies = send_all(&state, calls).await;
    assert_eq!(replies[0].0, "HTTP/1.1 200 OK");
    assert_eq!(replies[1].0, "HTTP/1.1 200 OK");
    assert_eq!(replies[2].0, "HTTP/1.1 403 Forbidden");
    assert_eq!(replies[3].0, "HTTP/1.1 403 Forbidden");
    assert_eq!(replies[4].0, "HTTP/1.1 200 OK");
    assert_eq!(replies[5].0, "HTTP/1.1 421 Misdirected Request");
}

/// Dev mode with `web.token` set keeps the bearer rule for unsafe methods
/// (§8.5 h); moved here from the pause, control and schedule tests.
#[tokio::test(flavor = "multi_thread")]
async fn dev_mode_with_a_token_needs_the_bearer_for_unsafe_methods() {
    let (_db, state) = state("guard-dev-token", false, Some("s3cret"));
    let routes = [
        ("POST", "/api/polling/pause?minutes=0"),
        ("POST", "/api/control/output?station=x&output=ac&state=on"),
        ("POST", "/api/schedule?station=x&output=ac&on=07:00"),
        ("DELETE", "/api/schedule?station=x&output=ac"),
    ];
    let wrong = [
        "",
        "Bearer s3creT",
        "Bearer s3cret-longer",
        "s3cret",
        "Bearer ",
    ];
    let mut calls = Vec::new();
    for (m, p) in routes {
        for auth in wrong {
            let mut headers = vec![ORIGIN.to_owned()];
            if !auth.is_empty() {
                headers.push(format!("Authorization: {auth}"));
            }
            calls.push((m, p.to_owned(), headers));
        }
        calls.push((m, p.to_owned(), vec!["Authorization: Bearer s3cret".into()]));
    }
    let replies = send_all(&state, calls).await;
    for (i, (status, _, body)) in replies.iter().enumerate() {
        let path = routes[i / (wrong.len() + 1)].1;
        if i % (wrong.len() + 1) < wrong.len() {
            assert_eq!(status, "HTTP/1.1 401 Unauthorized", "{path}");
            assert_eq!(body, r#"{"error":"a valid bearer token is required"}"#);
        } else {
            passed(path, &replies[i]);
        }
    }
}

#[test]
fn token_compare_checks_length_and_every_byte() {
    assert!(same(b"abc", b"abc"));
    assert!(!same(b"abc", b"abd"));
    assert!(!same(b"abc", b"ab"));
    assert!(!same(b"", b"a"));
}

/// §8.5 f: the four headers on every response — protected, public, a 404 and
/// a Host refusal (the outermost layer wraps `host_guard`).
#[tokio::test(flavor = "multi_thread")]
async fn every_response_carries_the_security_headers() {
    let (_db, state) = state("guard-headers", true, None);
    let cookie = session(&state);
    let mut calls: Vec<(&str, String, Vec<String>)> = PROTECTED
        .iter()
        .map(|(m, p)| (*m, concrete(p), vec![cookie.clone(), ORIGIN.to_owned()]))
        .collect();
    for path in ["/login", "/assets/app.js", "/healthz", "/no-such-page"] {
        calls.push(("GET", path.to_owned(), vec![]));
    }
    calls.push(("GET", "/".to_owned(), vec!["Host: evil.example".to_owned()]));
    let replies = send_all(&state, calls.clone()).await;
    assert_eq!(replies[PROTECTED.len() + 3].0, "HTTP/1.1 404 Not Found");
    assert_eq!(
        replies[PROTECTED.len() + 4].0,
        "HTTP/1.1 421 Misdirected Request"
    );
    for ((_, path, _), (_, headers, _)) in calls.iter().zip(&replies) {
        assert_eq!(
            header(headers, "content-security-policy"),
            Some(
                "default-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'"
            ),
            "{path}"
        );
        assert_eq!(header(headers, "x-frame-options"), Some("DENY"), "{path}");
        assert_eq!(
            header(headers, "x-content-type-options"),
            Some("nosniff"),
            "{path}"
        );
        assert_eq!(
            header(headers, "referrer-policy"),
            Some("same-origin"),
            "{path}"
        );
    }
}
