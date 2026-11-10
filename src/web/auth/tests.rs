use std::sync::Arc;
use std::sync::atomic::Ordering;

use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;

use super::{LoginForm, login, safe_next};
use crate::web::AppState;
use crate::web::auth::Auth;
use crate::web::tests::{Capture, header, request_with, start_shared};

const FORM: &str = "Content-Type: application/x-www-form-urlencoded";

fn state(auth: bool) -> Arc<AppState> {
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let state = AppState::for_tests(reader);
    Arc::new(if auth {
        state.with_test_password()
    } else {
        state
    })
}

fn auth(state: &AppState) -> &Auth {
    state.auth.as_ref().unwrap()
}

/// POSTs `body` to `path` with `extra` headers off the runtime.
async fn post(
    addr: std::net::SocketAddr,
    path: &'static str,
    extra: Vec<String>,
    body: &'static str,
) -> (String, Vec<String>, String) {
    tokio::task::spawn_blocking(move || {
        let mut headers = vec![FORM];
        headers.extend(extra.iter().map(String::as_str));
        request_with(addr, "POST", path, &headers, body)
    })
    .await
    .unwrap()
}

/// The session id a `Set-Cookie` header carries.
fn cookie_id(headers: &[String]) -> String {
    let cookie = header(headers, "set-cookie").expect("Set-Cookie");
    let id = cookie
        .strip_prefix("session=")
        .and_then(|rest| rest.split(';').next())
        .unwrap();
    id.to_owned()
}

#[test]
fn next_is_kept_only_for_a_path_on_this_site() {
    for ok in ["/", "/telemetry", "/telemetry?x=1&y=%2F", "/a/b"] {
        assert_eq!(safe_next(ok), ok);
    }
    for bad in [
        "",
        "evil.example",
        "http://evil.example/",
        "//evil.example",
        "/\\evil.example",
        "/\t/evil.example",
        "/\n/evil.example",
        "/ /evil.example",
        "/\x7f/evil",
        "/é",
    ] {
        assert_eq!(safe_next(bad), "/", "{bad:?}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn login_page_renders_the_form_with_a_safe_next() {
    let (addr, stop, server) = start_shared(state(true)).await;
    let (status, _, body) = tokio::task::spawn_blocking(move || {
        request_with(addr, "GET", "/login?next=%2Ftelemetry", &[], "")
    })
    .await
    .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert!(body.contains(r#"name="password""#), "{body}");
    assert!(body.contains(r#"name="next" value="/telemetry""#), "{body}");
    assert!(!body.contains("<script"), "no inline script: {body}");

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_right_password_sets_the_session_cookie() {
    let state = state(true);
    let (addr, stop, server) = start_shared(Arc::clone(&state)).await;

    let (status, headers, _) =
        post(addr, "/login", vec![], "password=test&next=%2Ftelemetry").await;
    assert_eq!(status, "HTTP/1.1 303 See Other");
    assert_eq!(header(&headers, "location"), Some("/telemetry"));
    let cookie = header(&headers, "set-cookie").unwrap();
    let id = cookie_id(&headers);
    assert_eq!(id.len(), 64);
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(
        cookie,
        format!("session={id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000")
    );
    assert!(auth(&state).sessions.check(&id));

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_wrong_password_is_401_with_the_form_and_no_cookie() {
    let (addr, stop, server) = start_shared(state(true)).await;

    let (status, headers, body) = post(addr, "/login", vec![], "password=tesT&next=%2F").await;
    assert_eq!(status, "HTTP/1.1 401 Unauthorized");
    assert!(header(&headers, "set-cookie").is_none());
    assert!(body.contains("Wrong password."), "{body}");
    assert!(body.contains(r#"name="password""#), "{body}");

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_flood_of_wrong_logins_verifies_at_most_five() {
    // Counted before verification, one permit (§8.5 c): 10 concurrent wrong
    // POSTs verify exactly 5 (the 5th starts the lock), the other 5 are 429.
    let state = state(true);
    let (addr, stop, server) = start_shared(Arc::clone(&state)).await;

    let tries: Vec<_> = (0..10)
        .map(|_| tokio::spawn(post(addr, "/login", vec![], "password=wrong")))
        .collect();
    let mut statuses = Vec::new();
    for t in tries {
        statuses.push(t.await.unwrap().0);
    }
    let count = |s: &str| statuses.iter().filter(|x| *x == s).count();
    assert_eq!(count("HTTP/1.1 401 Unauthorized"), 5, "{statuses:?}");
    assert_eq!(count("HTTP/1.1 429 Too Many Requests"), 5, "{statuses:?}");
    assert_eq!(auth(&state).verified.load(Ordering::Relaxed), 5);

    // Locked: even the right password is refused without a verification.
    let (status, headers, _) = post(addr, "/login", vec![], "password=test").await;
    assert_eq!(status, "HTTP/1.1 429 Too Many Requests");
    assert!(header(&headers, "set-cookie").is_none());
    assert_eq!(auth(&state).verified.load(Ordering::Relaxed), 5);

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_foreign_next_falls_back_to_the_root() {
    let (addr, stop, server) = start_shared(state(true)).await;

    for body in [
        "password=test&next=%2F%2Fevil.example",
        "password=test&next=%2F%5Cevil.example",
        "password=test&next=%2F%09%2Fevil.example",
        "password=test&next=http%3A%2F%2Fevil.example",
    ] {
        let (status, headers, _) = post(addr, "/login", vec![], body).await;
        assert_eq!(status, "HTTP/1.1 303 See Other", "{body}");
        assert_eq!(header(&headers, "location"), Some("/"), "{body}");
    }

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_revokes_the_session_and_clears_the_cookie() {
    let state = state(true);
    let (addr, stop, server) = start_shared(Arc::clone(&state)).await;

    let (_, headers, _) = post(addr, "/login", vec![], "password=test").await;
    let id = cookie_id(&headers);
    let other = auth(&state).sessions.issue().unwrap();
    assert!(auth(&state).sessions.check(&id));

    let cookie = format!("Cookie: theme=dark; session={id}");
    let origin = "Origin: http://localhost".to_owned();
    let (status, headers, _) = post(addr, "/logout", vec![cookie, origin], "").await;
    assert_eq!(status, "HTTP/1.1 303 See Other");
    assert_eq!(header(&headers, "location"), Some("/login"));
    assert_eq!(
        header(&headers, "set-cookie"),
        Some("session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0")
    );
    assert!(!auth(&state).sessions.check(&id));
    assert!(
        auth(&state).sessions.check(&other),
        "only that session ends"
    );

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn dev_mode_login_only_redirects() {
    let (addr, stop, server) = start_shared(state(false)).await;

    let (status, headers, _) = post(addr, "/login", vec![], "password=x&next=%2Ftelemetry").await;
    assert_eq!(status, "HTTP/1.1 303 See Other");
    assert_eq!(header(&headers, "location"), Some("/telemetry"));
    assert!(header(&headers, "set-cookie").is_none());
    let (status, headers, _) =
        tokio::task::spawn_blocking(move || request_with(addr, "GET", "/login", &[], ""))
            .await
            .unwrap();
    assert_eq!(status, "HTTP/1.1 303 See Other");
    assert_eq!(header(&headers, "location"), Some("/"));

    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn login_logs_name_the_outcome_never_the_password_or_hash() {
    // Current-thread runtime: the handler runs on this thread, under this
    // thread's subscriber. With one registered dispatcher, tracing-core lets a
    // callsite that another test's thread hits first keep that thread's empty
    // interest for good; a second live dispatcher turns that shortcut off.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let capture = Capture::default();
    let sink = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let password = "correct horse battery";
    let hash = super::password::hash(password).unwrap();
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let mut state = AppState::for_tests(reader);
    state.auth = Some(Auth::new(hash.clone()));
    let state = Arc::new(state);

    for (given, expected) in [
        ("wrong horse battery", StatusCode::UNAUTHORIZED),
        (password, StatusCode::SEE_OTHER),
    ] {
        let form = LoginForm {
            password: given.to_owned(),
            next: "/".to_owned(),
        };
        let response = login(State(Arc::clone(&state)), Form(form)).await;
        assert_eq!(response.status(), expected);
    }

    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("web login failed"), "{logs}");
    assert!(logs.contains("web login succeeded"), "{logs}");
    for secret in [password, "wrong horse battery", hash.as_str(), "$argon2id$"] {
        assert!(!logs.contains(secret), "{secret:?} in {logs}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dropped_login_keeps_its_slot_until_the_hash_ends() {
    // Amendment A3: five wrong logins, each dropped (the client hung up) as
    // soon as its verification started. The task owns the permit, so the
    // verifications still run one at a time (~19 MiB each, §8.5 c).
    use std::sync::atomic::Ordering::SeqCst;
    let state = state(true);
    for started in 1..=5 {
        let form = LoginForm {
            password: "wrong".to_owned(),
            next: String::new(),
        };
        let mut handler = Box::pin(login(State(Arc::clone(&state)), Form(form)));
        while auth(&state).verified.load(Ordering::Relaxed) < started {
            // `timeout` polls the handler once before it checks the deadline.
            let _ = tokio::time::timeout(std::time::Duration::ZERO, &mut handler).await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        drop(handler);
    }
    // The slot frees only once the last verification ends.
    let slot = Arc::clone(&auth(&state).verify_slot);
    let _permit = slot.acquire_owned().await.unwrap();
    let running = &auth(&state).running;
    assert_eq!(running.0.load(SeqCst), 0);
    assert_eq!(running.1.load(SeqCst), 1, "verifications overlapped");
}

#[test]
fn dev_mode_warns_once_that_the_login_is_off() {
    // Amendment A6, spec §8.5 h. See the login-log test for the second dispatcher.
    let _second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let capture = Capture::default();
    let sink = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    assert!(super::from_config(Some(super::TEST_HASH.to_owned())).is_some());
    assert!(capture.0.lock().unwrap().is_empty());
    assert!(super::from_config(None).is_none());
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert_eq!(logs.lines().count(), 1, "{logs}");
    assert!(logs.contains("WARN"), "{logs}");
    assert!(logs.contains("web login is off"), "{logs}");
}

/// Captures every log line of this thread until the guard drops (see
/// `login_logs_name_the_outcome_never_the_password_or_hash` for the second
/// dispatcher).
fn capture_logs() -> (
    Capture,
    tracing::Dispatch,
    tracing::subscriber::DefaultGuard,
) {
    let second = tracing::Dispatch::new(tracing::subscriber::NoSubscriber::default());
    let capture = Capture::default();
    let sink = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    (
        capture,
        second,
        tracing::subscriber::set_default(subscriber),
    )
}

#[tokio::test]
async fn a_failed_verification_task_is_a_failed_login() {
    let (capture, _second, _guard) = capture_logs();
    let task = tokio::spawn(std::future::pending::<bool>());
    task.abort();
    assert!(!super::joined(task.await), "a cancelled task is no login");
    assert!(super::joined(Ok(true)));
    assert!(!super::joined(Ok(false)));
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert_eq!(
        logs.matches("web login: verification task failed").count(),
        1,
        "{logs}"
    );
}

#[tokio::test]
async fn no_session_id_without_randomness_is_a_500_without_a_cookie() {
    let (capture, _second, _guard) = capture_logs();
    let mut reader = rusqlite::Connection::open_in_memory().unwrap();
    crate::store::migrate(&mut reader).unwrap();
    let mut state = AppState::for_tests(reader).with_test_password();
    state.auth.as_mut().unwrap().sessions =
        super::session::Sessions::with_rng(|_| Err(getrandom::Error::UNSUPPORTED));
    let form = LoginForm {
        password: "test".to_owned(),
        next: "/".to_owned(),
    };
    let response = login(State(Arc::new(state)), Form(form)).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(response.headers().get("set-cookie").is_none());
    assert!(response.headers().get("location").is_none());
    let logs = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("web login: no session id"), "{logs}");
    assert!(logs.contains("session id: "), "{logs}");
}
