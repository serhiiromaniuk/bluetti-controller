//! The request guard (ROADMAP.md §8.5 a, d, e, g, h): two
//! `axum::middleware::from_fn_with_state` layers on the router, so no handler
//! checks credentials itself. `host_guard` wraps every route but `/healthz`;
//! `auth_guard` is the protected routes' `route_layer`.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};

use super::AppState;
use super::auth::session_id;
use super::pause::error;

/// `421` unless the `Host` header passes the rule of §8.5 e: an IP literal,
/// `localhost` or a `web.allowed_hosts` entry, the port ignored. `/healthz`
/// is exempt (Q6: `bluetti-update` curls it by DNS name).
pub async fn host_guard(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    if req.uri().path() == "/healthz" {
        return next.run(req).await;
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok());
    if host.is_some_and(|host| host_allowed(host, &state.allowed_hosts)) {
        next.run(req).await
    } else {
        error(StatusCode::MISDIRECTED_REQUEST, "unknown Host").into_response()
    }
}

/// The credential and CSRF checks of §8.5 b, d, h, in order: a valid bearer
/// passes; with a login, no valid session is `401` (`/api/*`, `/events`) or
/// `303 /login?next=` (pages); in dev mode an unsafe method needs the bearer
/// when `web.token` is set; an unsafe method without the bearer needs an
/// `Origin` equal to the request's own origin (`403`).
pub async fn auth_guard(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let headers = req.headers();
    if bearer_ok(headers, state.token.as_deref()) {
        return next.run(req).await;
    }
    let unsafe_method = matches!(
        *req.method(),
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    );
    match &state.auth {
        Some(auth) => {
            if !session_id(headers).is_some_and(|id| auth.sessions.check(id)) {
                return refuse(&req);
            }
        }
        None if unsafe_method && state.token.is_some() => {
            return error(StatusCode::UNAUTHORIZED, "a valid bearer token is required")
                .into_response();
        }
        None => {}
    }
    if unsafe_method && !same_origin(headers) {
        return error(StatusCode::FORBIDDEN, "cross-origin request refused").into_response();
    }
    next.run(req).await
}

/// `Host`'s host part is an IP literal, `localhost` or an allowed name
/// (case-insensitive); the port is not compared (§8.5 e).
pub fn host_allowed(host: &str, allowed: &[String]) -> bool {
    if let Some(rest) = host.strip_prefix('[') {
        // `[v6]` or `[v6]:port`
        return rest.split_once(']').is_some_and(|(v6, tail)| {
            v6.parse::<Ipv6Addr>().is_ok()
                && (tail.is_empty() || tail.strip_prefix(':').is_some_and(port))
        });
    }
    let name = match host.rsplit_once(':') {
        Some((name, p)) if port(p) => name,
        Some(_) => return false,
        None => host,
    };
    name.parse::<Ipv4Addr>().is_ok()
        || name.eq_ignore_ascii_case("localhost")
        || allowed.iter().any(|a| a.eq_ignore_ascii_case(name))
}

/// 1–5 ASCII digits.
fn port(p: &str) -> bool {
    (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit())
}

/// `Authorization: Bearer <web.token>`, compared in constant time; no token
/// configured means no bearer is valid.
fn bearer_ok(headers: &HeaderMap, token: Option<&str>) -> bool {
    let given = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "));
    match (token, given) {
        (Some(token), Some(given)) => same(given, token.as_bytes()),
        _ => false,
    }
}

/// `Origin` is `http://<Host>` or `https://<Host>`, byte for byte (§8.5 d).
fn same_origin(headers: &HeaderMap) -> bool {
    let (Some(origin), Some(host)) = (headers.get(header::ORIGIN), headers.get(header::HOST))
    else {
        return false;
    };
    let (origin, host) = (origin.as_bytes(), host.as_bytes());
    [&b"http://"[..], b"https://"]
        .iter()
        .any(|scheme| origin.strip_prefix(*scheme) == Some(host))
}

/// No credential (§8.5 g): JSON `401` for the API and SSE, else a redirect to
/// the login with the path and query as `next`.
fn refuse(req: &Request) -> Response {
    let path = req.uri().path();
    if path.starts_with("/api/") || path == "/events" {
        return error(StatusCode::UNAUTHORIZED, "login required").into_response();
    }
    let target = req.uri().path_and_query().map_or(path, |pq| pq.as_str());
    Redirect::to(&format!("/login?next={}", encode(target))).into_response()
}

/// Percent-encodes every byte but unreserved ones and `/`.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// Constant-time equality (§8.4 h): the length first, then one XOR fold over
/// every byte, so the time does not tell how many leading bytes matched.
pub(super) fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests;
