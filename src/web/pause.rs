//! `POST /api/polling/pause` (ROADMAP.md §8.4 h): stops polling only, never
//! sends a frame to a station (golden rule #2) — it writes the collector's
//! `watch` channel and nothing else.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use serde_json::{Value, json};
use tokio::time::Instant;

use super::AppState;
use super::status::paused_until;
use crate::collector::Control;

/// `POST /api/polling/pause?minutes=N`: `1..=60` pauses polling for `N`
/// minutes, `0` resumes it; anything else is `400`. With `web.token` set,
/// `Authorization: Bearer <token>` is required (`401`). While the server
/// shuts down it is `503`: `Shutdown` is never replaced (amendment A2).
/// Replies `{"paused_until": <unix s|null>}`.
pub async fn pause(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    if let Some(token) = &state.token {
        let given = headers
            .get(AUTHORIZATION)
            .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "));
        if !given.is_some_and(|given| same(given, token.as_bytes())) {
            return error(StatusCode::UNAUTHORIZED, "a valid bearer token is required");
        }
    }
    let next = match query.get("minutes").map(|m| m.parse::<u64>()) {
        Some(Ok(0)) => Control::Run,
        Some(Ok(minutes @ 1..=60)) => Control::Pause {
            until: Instant::now() + Duration::from_secs(minutes * 60),
        },
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "minutes must be an integer in 0..=60",
            );
        }
    };
    // A2: `serve` sends `Shutdown` while the server still answers; replacing
    // it would keep the collector polling and hang the shutdown.
    let sent = state.control.send_if_modified(|current| {
        let open = *current != Control::Shutdown;
        if open {
            *current = next;
        }
        open
    });
    if !sent {
        return error(StatusCode::SERVICE_UNAVAILABLE, "shutting down");
    }
    tracing::info!(control = ?next, "polling control set from the web");
    (
        StatusCode::OK,
        Json(json!({ "paused_until": paused_until(next) })),
    )
}

fn error(status: StatusCode, message: &str) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": message })))
}

/// Constant-time equality (§8.4 h): the length first, then one XOR fold over
/// every byte, so the time does not tell how many leading bytes matched.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests;
