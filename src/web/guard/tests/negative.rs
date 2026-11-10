//! Security suite type 1, negative (task `2026-11-07_web-security` step 10a):
//! every missing, malformed, expired, revoked or forged credential is refused
//! (ROADMAP.md §8.5 b, g). Each table carries a passing control row, so a
//! refusal for the wrong reason cannot pass unseen.

use std::time::Duration;

use super::*;
use crate::web::auth::session::SESSION_TTL;

const STATUS: &str = "/api/status";
const OK: &str = "HTTP/1.1 200 OK";
const UNAUTHORIZED: &str = "HTTP/1.1 401 Unauthorized";

/// `GET /api/status` once per `(label, header lines)`; asserts the control row
/// (the last) passes and every other row is the login `401`.
async fn refused_but_last(state: &Arc<AppState>, cases: Vec<(&'static str, Vec<String>)>) {
    let calls = cases
        .iter()
        .map(|(_, headers)| ("GET", STATUS.to_owned(), headers.clone()))
        .collect();
    let replies = send_all(state, calls).await;
    let (control, refused) = replies.split_last().unwrap();
    assert_eq!(control.0, OK, "control row: {}", control.2);
    for ((label, _), (status, _, body)) in cases.iter().zip(refused) {
        assert_eq!(status, UNAUTHORIZED, "{label}");
        assert_eq!(body, r#"{"error":"login required"}"#, "{label}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_missing_empty_or_malformed_session_cookie_is_refused() {
    let (_db, state) = state("neg-cookie", true, None);
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();
    let cookie = |value: String| vec![format!("Cookie: {value}")];
    let forged = "0".repeat(64);
    let cases = vec![
        ("no Cookie header", vec![]),
        ("empty Cookie header", cookie(String::new())),
        ("empty session value", cookie("session=".into())),
        ("not hex", cookie("session=zz".into())),
        ("forged 64 hex chars", cookie(format!("session={forged}"))),
        (
            "uppercase copy",
            cookie(format!("session={}", id.to_uppercase())),
        ),
        ("one char short", cookie(format!("session={}", &id[..63]))),
        ("one char long", cookie(format!("session={id}0"))),
        ("other cookie name", cookie(format!("sessionid={id}"))),
        ("name with a prefix", cookie(format!("xsession={id}"))),
        ("space before =", cookie(format!("session ={id}"))),
        ("quoted value", cookie(format!("session=\"{id}\""))),
        (
            "the first session cookie decides",
            cookie(format!("session={forged}; session={id}")),
        ),
        (
            "control: the issued id",
            cookie(format!("theme=dark; session={id}")),
        ),
    ];
    refused_but_last(&state, cases).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_malformed_or_wrong_bearer_is_refused() {
    let (_db, state) = state("neg-bearer", true, Some("s3cret"));
    let auth = |value: &str| vec![format!("Authorization: {value}")];
    let cases = vec![
        ("scheme only", auth("Bearer")),
        ("empty token", auth("Bearer ")),
        ("lowercase scheme", auth("bearer s3cret")),
        ("uppercase scheme", auth("BEARER s3cret")),
        ("two spaces", auth("Bearer  s3cret")),
        ("no scheme", auth("s3cret")),
        ("Basic scheme", auth("Basic czNjcmV0")),
        ("one char short", auth("Bearer s3cre")),
        ("one char long", auth("Bearer s3cret0")),
        ("case differs", auth("Bearer S3cret")),
        (
            "the first Authorization header decides",
            vec![
                "Authorization: Bearer wrong".into(),
                "Authorization: Bearer s3cret".into(),
            ],
        ),
        ("control: the token", auth("Bearer s3cret")),
    ];
    refused_but_last(&state, cases).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn no_bearer_is_valid_when_no_token_is_set() {
    let (_db, state) = state("neg-no-token", true, None);
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();
    let cases = vec![
        ("any bearer", vec!["Authorization: Bearer s3cret".into()]),
        ("empty bearer", vec!["Authorization: Bearer ".into()]),
        ("control: a session", vec![format!("Cookie: session={id}")]),
    ];
    refused_but_last(&state, cases).await;
}

#[tokio::test(start_paused = true)]
async fn an_expired_session_is_refused() {
    let (_db, state) = state("neg-expired", true, None);
    let id = state.auth.as_ref().unwrap().sessions.issue().unwrap();
    let cookie = vec![format!("Cookie: session={id}")];
    let call = || vec![("GET", STATUS.to_owned(), cookie.clone())];
    tokio::time::advance(SESSION_TTL - Duration::from_secs(1)).await;
    assert_eq!(send_all(&state, call()).await[0].0, OK);
    tokio::time::advance(Duration::from_secs(1)).await;
    assert_eq!(send_all(&state, call()).await[0].0, UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_logged_out_session_is_refused() {
    let (_db, state) = state("neg-logout", true, None);
    let replies = with_server(&state, |addr| {
        let form = ["Content-Type: application/x-www-form-urlencoded"];
        let (_, headers, _) = request_with(addr, "POST", "/login", &form, "password=test");
        let set = header(&headers, "set-cookie").unwrap().to_owned();
        let cookie = format!("Cookie: {}", set.split(';').next().unwrap());
        let before = request_with(addr, "GET", STATUS, &[&cookie], "").0;
        let logout = request_with(addr, "POST", "/logout", &[&cookie, ORIGIN], "").0;
        let after = request_with(addr, "GET", STATUS, &[&cookie], "").0;
        [before, logout, after]
    })
    .await;
    assert_eq!(
        replies,
        [OK, "HTTP/1.1 303 See Other", UNAUTHORIZED].map(str::to_owned)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_page_without_a_valid_session_redirects_to_the_login() {
    let (_db, state) = state("neg-page", true, None);
    let calls = ["", "Cookie: session=", "Cookie: session=zz"]
        .into_iter()
        .map(|c| {
            let headers = if c.is_empty() { vec![] } else { vec![c.into()] };
            ("GET", "/telemetry".to_owned(), headers)
        })
        .collect();
    for (status, headers, _) in send_all(&state, calls).await {
        assert_eq!(status, "HTTP/1.1 303 See Other");
        assert_eq!(header(&headers, "location"), Some("/login?next=/telemetry"));
    }
}
