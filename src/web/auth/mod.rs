//! The web login (ROADMAP.md ADR-011, §8.5 b–c): argon2id hashing, the
//! in-memory session store and the `/login` and `/logout` handlers. The guard
//! that checks the session on every other route is `crate::web::guard`.

pub mod password;
pub mod session;

use std::sync::Arc;

use askama::Template;
use axum::Form;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{AppendHeaders, Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

pub use session::Auth;
use session::{Attempt, SESSION_TTL};

use super::AppState;

/// The session cookie's name (§8.5 b).
pub const COOKIE: &str = "session";

/// The argon2id PHC string of the throwaway password `"test"` (never an
/// owner password), for tests that need a login.
#[cfg(test)]
pub(crate) const TEST_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$XYoC+IyxSMsuKhoGWisQFA$FYfUAw0rj87Ga04AeeadC8HcSCwM0Q++Dl9U0gVQ67w";

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage<'a> {
    next: &'a str,
    error: &'a str,
}

/// `GET /login?next=`.
#[derive(Deserialize)]
pub struct NextQuery {
    #[serde(default)]
    next: String,
}

/// The `POST /login` form. No `Debug`: it holds the password.
#[derive(Deserialize)]
pub struct LoginForm {
    password: String,
    #[serde(default)]
    next: String,
}

/// `next` when it is a path on this site (§8.5 c), else `/`: it starts with
/// one `/`, the byte after it is neither `/` nor `\`, and every byte is visible
/// ASCII. The last rule stops `/<TAB>/evil`, which a browser reads as `//evil`
/// once it strips the tab (amendment A2).
pub fn safe_next(next: &str) -> &str {
    let bytes = next.as_bytes();
    let ok = bytes.first() == Some(&b'/')
        && !matches!(bytes.get(1), Some(b'/' | b'\\'))
        && bytes.iter().all(u8::is_ascii_graphic);
    if ok { next } else { "/" }
}

/// The `session` cookie's value from the request's `Cookie` headers.
pub fn session_id(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .find_map(|pair| pair.trim().strip_prefix("session="))
}

/// `GET /login`: the form. In dev mode (no `web.password_hash`) there is
/// nothing to log in to, so it redirects to `next`.
pub async fn login_page(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NextQuery>,
) -> Response {
    let next = safe_next(&query.next);
    if state.auth.is_none() {
        return Redirect::to(next).into_response();
    }
    page(StatusCode::OK, next, "")
}

/// `POST /login` (§8.5 c): the attempt is counted before verification, a lock
/// answers `429`, verification runs one at a time off the runtime; success
/// sets the session cookie and redirects to `next`, failure is `401`.
pub async fn login(State(state): State<Arc<AppState>>, Form(form): Form<LoginForm>) -> Response {
    let next = safe_next(&form.next);
    let Some(auth) = &state.auth else {
        return Redirect::to(next).into_response();
    };
    if auth.lockout.begin() == Attempt::Locked {
        tracing::info!("web login refused: locked after repeated failures");
        return page(
            StatusCode::TOO_MANY_REQUESTS,
            next,
            "Too many failed logins. Wait a minute and try again.",
        );
    }
    // The semaphore is never closed, so `acquire` cannot fail. The permit
    // moves into the blocking task: dropping this future (the client hung up)
    // leaves it held until the hash is done (amendment A3).
    let Ok(permit) = Arc::clone(&auth.verify_slot).acquire_owned().await else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };
    #[cfg(test)]
    auth.verified
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    #[cfg(test)]
    let running = Arc::clone(&auth.running);
    let hash = auth.hash.clone();
    let password = form.password;
    let ok = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        #[cfg(test)]
        let _running = Running::start(&running);
        password::verify(&hash, &password)
    })
    .await
    .unwrap_or_else(|err| {
        tracing::warn!(error = %err, "web login: verification task failed");
        false
    });
    if !ok {
        tracing::info!("web login failed: wrong password");
        return page(StatusCode::UNAUTHORIZED, next, "Wrong password.");
    }
    auth.lockout.succeeded();
    match auth.sessions.issue() {
        Ok(id) => {
            tracing::info!("web login succeeded");
            let cookie = format!(
                "{COOKIE}={id}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
                SESSION_TTL.as_secs()
            );
            (
                AppendHeaders([(header::SET_COOKIE, cookie)]),
                Redirect::to(next),
            )
                .into_response()
        }
        Err(err) => {
            tracing::warn!(error = format!("{err:#}"), "web login: no session id");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// `POST /logout` (§8.5 b): revokes the request's session and clears the
/// cookie, then redirects to `/login`.
pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let (Some(auth), Some(id)) = (&state.auth, session_id(&headers)) {
        auth.sessions.revoke(id);
        tracing::info!("web logout");
    }
    let cookie = format!("{COOKIE}=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0");
    (
        AppendHeaders([(header::SET_COOKIE, cookie)]),
        Redirect::to("/login"),
    )
        .into_response()
}

/// Counts one running verification for the amendment A3 test: `.0` now
/// running, `.1` the peak.
#[cfg(test)]
struct Running<'a>(&'a (std::sync::atomic::AtomicU32, std::sync::atomic::AtomicU32));

#[cfg(test)]
impl<'a> Running<'a> {
    fn start(counts: &'a (std::sync::atomic::AtomicU32, std::sync::atomic::AtomicU32)) -> Self {
        use std::sync::atomic::Ordering::SeqCst;
        let now = counts.0.fetch_add(1, SeqCst) + 1;
        counts.1.fetch_max(now, SeqCst);
        Self(counts)
    }
}

#[cfg(test)]
impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// The login state from `web.password_hash`. Without one (dev mode, §8.5 h)
/// every GET is open, so this logs one `warn!`; `serve` calls it once.
pub fn from_config(hash: Option<String>) -> Option<Auth> {
    if hash.is_none() {
        tracing::warn!("web login is off: web.password_hash is unset (dev mode)");
    }
    hash.map(Auth::new)
}

fn page(status: StatusCode, next: &str, error: &str) -> Response {
    match (LoginPage { next, error }).render() {
        Ok(html) => (status, Html(html)).into_response(),
        Err(err) => {
            tracing::warn!(error = %err, "login page: render failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "login page unavailable").into_response()
        }
    }
}

#[cfg(test)]
mod tests;
