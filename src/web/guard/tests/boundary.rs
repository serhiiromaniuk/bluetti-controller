//! Security suite type 3, boundary (task `2026-11-07_web-security` step 10a):
//! the lockout count, the lock window, the session lifetime and the span cap
//! at their exact edges, through the real router (ROADMAP.md §8.5 b, c;
//! SECURITY.md "Bounded queries and clean output").

use std::time::Duration;

use super::*;
use crate::web::auth::session::{LOCK, MAX_FAILURES, SESSION_TTL};

const FORM: &str = "Content-Type: application/x-www-form-urlencoded";
const UNAUTHORIZED: &str = "HTTP/1.1 401 Unauthorized";
const LOCKED: &str = "HTTP/1.1 429 Too Many Requests";
const SEE_OTHER: &str = "HTTP/1.1 303 See Other";

/// POSTs each password to `/login` in order; returns the status lines.
async fn tries(state: &Arc<AppState>, passwords: &[&str]) -> Vec<String> {
    let bodies: Vec<String> = passwords.iter().map(|p| format!("password={p}")).collect();
    with_server(state, move |addr| {
        bodies
            .iter()
            .map(|body| request_with(addr, "POST", "/login", &[FORM], body).0)
            .collect()
    })
    .await
}

#[tokio::test(flavor = "multi_thread")]
async fn four_failures_never_lock_and_the_fifth_does() {
    assert_eq!(MAX_FAILURES, 5);
    let (_db, state) = state("bnd-count", true, None);
    let four = ["wrong"; 4];
    assert_eq!(tries(&state, &four).await, [UNAUTHORIZED; 4]);
    assert_eq!(
        tries(&state, &["test"]).await,
        [SEE_OTHER],
        "4 failures: no lock"
    );
    // The count restarted at the success: 5 failures, then the lock.
    let five_then_right = ["wrong", "wrong", "wrong", "wrong", "wrong", "test"];
    let statuses = tries(&state, &five_then_right).await;
    assert_eq!(statuses[..5], [UNAUTHORIZED; 5]);
    assert_eq!(statuses[5], LOCKED);
}

#[tokio::test(start_paused = true)]
async fn the_lock_ends_exactly_after_60_seconds() {
    assert_eq!(LOCK, Duration::from_secs(60));
    let (_db, state) = state("bnd-lock", true, None);
    let auth = state.auth.as_ref().unwrap();
    for _ in 0..MAX_FAILURES {
        auth.lockout.begin();
    }
    tokio::time::advance(LOCK - Duration::from_secs(1)).await;
    assert_eq!(
        tries(&state, &["test"]).await,
        [LOCKED],
        "1 s before the end"
    );
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(tries(&state, &["test"]).await, [SEE_OTHER], "at the end");
}

#[tokio::test(start_paused = true)]
async fn a_session_lives_exactly_30_days() {
    let (_db, state) = state("bnd-ttl", true, None);
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();
    let call = || {
        vec![(
            "GET",
            "/api/status".to_owned(),
            vec![format!("Cookie: session={id}")],
        )]
    };
    let mut statuses = Vec::new();
    for step in [
        SESSION_TTL - Duration::from_secs(1),
        Duration::from_secs(1),
        Duration::from_secs(1),
    ] {
        tokio::time::advance(step).await;
        statuses.push(send_all(&state, call()).await.remove(0).0);
    }
    assert_eq!(statuses, ["HTTP/1.1 200 OK", UNAUTHORIZED, UNAUTHORIZED]);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_span_cap_and_the_window_order_hold_at_their_edges() {
    let (_db, state) = state("bnd-span", false, None);
    let cap = 366 * 86_400_i64;
    let t0 = 1_700_000_000_i64;
    let cases = [
        (format!("from=0&to={cap}"), true),
        (format!("from=0&to={}", cap + 1), false),
        (format!("from={t0}&to={}", t0 + cap), true),
        (format!("from={}&to={}", t0 - 1, t0 + cap), false),
        (format!("from={t0}&to={}", t0 + 1), true),
        (format!("from={t0}&to={t0}"), false),
        ("from=0&to=1".to_owned(), true),
        ("from=-1&to=1".to_owned(), false),
    ];
    // Every case on `/api/outages`; the cap rows on `/api/chart` too (its
    // `step` rule refuses windows shorter than a minute).
    let mut paths: Vec<(String, bool)> = cases
        .iter()
        .map(|(q, ok)| (format!("/api/outages?{q}"), *ok))
        .collect();
    paths.extend(
        cases[..4]
            .iter()
            .map(|(q, ok)| (format!("/api/chart?{q}"), *ok)),
    );
    let calls = paths
        .iter()
        .map(|(p, _)| ("GET", p.clone(), vec![]))
        .collect();
    let replies = send_all(&state, calls).await;
    for ((path, ok), (status, _, body)) in paths.iter().zip(replies) {
        let want = if *ok {
            "HTTP/1.1 200 OK"
        } else {
            "HTTP/1.1 400 Bad Request"
        };
        assert_eq!(status, want, "{path}: {body}");
    }
    for days in ["1", "30"] {
        let reply = send_all(
            &state,
            vec![("GET", format!("/api/energy?days={days}"), vec![])],
        )
        .await;
        assert_eq!(reply[0].0, "HTTP/1.1 200 OK", "days={days}");
    }
}
