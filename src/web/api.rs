//! The JSON API (ROADMAP.md §8.2, §8.4 f). Query strings are parsed here, not
//! by axum's typed extractors, so every bad request answers `400` with
//! `{"error": "<message>"}`. Reads run on the read-only connection in
//! `spawn_blocking`.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{Value, json};

use super::{AppState, history, status as view, unix_now};
use crate::protocol::device::Metric;
use crate::store::read;

/// The default window of `/api/series` (§8.4 f).
const SERIES_DEFAULT_S: i64 = 24 * 3600;
/// The default window of `/api/outages` (§8.4 f).
const OUTAGES_DEFAULT_S: i64 = 7 * 86_400;
/// `/api/energy` never reaches further back than this many days (§8.4 f).
const ENERGY_MAX_DAYS: u32 = 30;
const ENERGY_DEFAULT_DAYS: u32 = 7;

type Params = Query<HashMap<String, String>>;

/// An API failure: `400` for a bad request, `500` when the read itself fails.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

fn bad(message: impl Into<String>) -> ApiError {
    ApiError {
        status: StatusCode::BAD_REQUEST,
        message: message.into(),
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        tracing::warn!(error = format!("{err:#}"), "api: read failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "database read failed".into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}

/// Runs `f` on the reader in `spawn_blocking`; its value becomes the JSON body.
async fn read<T, F>(state: Arc<AppState>, f: F) -> Result<Json<T>, ApiError>
where
    T: Serialize + Send + 'static,
    F: FnOnce(&Connection, &AppState) -> Result<T, ApiError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let conn = state
            .reader
            .lock()
            .map_err(|_| anyhow::anyhow!("reader lock poisoned"))?;
        f(&conn, &state)
    })
    .await
    .map_err(|err| ApiError::from(anyhow::Error::new(err)))?
    .map(Json)
}

/// The integer parameter `key`, `None` when absent.
fn int(q: &HashMap<String, String>, key: &str) -> Result<Option<i64>, ApiError> {
    q.get(key)
        .map(|v| {
            v.parse()
                .map_err(|_| bad(format!("{key} must be an integer, got {v:?}")))
        })
        .transpose()
}

/// `from`/`to` in unix seconds: `to` defaults to `now`, `from` to `to − span`.
fn window(q: &HashMap<String, String>, now: i64, span: i64) -> Result<(i64, i64), ApiError> {
    let to = int(q, "to")?.unwrap_or(now);
    let from = int(q, "from")?.unwrap_or(to.saturating_sub(span));
    if from >= to {
        return Err(bad(format!("from ({from}) must be before to ({to})")));
    }
    Ok((from, to))
}

/// `GET /api/status`: the `Status` the `/` page renders (§8.4 a).
pub async fn status(State(state): State<Arc<AppState>>) -> Result<Json<view::Status>, ApiError> {
    read(state, |conn, state| {
        Ok(view::status(conn, unix_now(), state)?)
    })
    .await
}

/// `GET /api/series?station=&metric=&from=&to=&step=`: one metric of one
/// station as `{"source", "t", "v"}`; over 48 h from `rollup_5m` where the
/// metric has a column there (§8.4 f).
pub async fn series(
    State(state): State<Arc<AppState>>,
    Query(q): Params,
) -> Result<Json<Value>, ApiError> {
    let name = q
        .get("station")
        .ok_or_else(|| bad("station is required"))?
        .clone();
    let key = q.get("metric").ok_or_else(|| bad("metric is required"))?;
    // `Metric::key()` names only: what app.js sends (amendment A6).
    let metric = Metric::from_key(key).ok_or_else(|| bad(format!("unknown metric {key:?}")))?;
    let (from, to) = window(&q, unix_now(), SERIES_DEFAULT_S)?;
    let step = int(&q, "step")?.unwrap_or(1);
    if step < 1 {
        return Err(bad(format!("step must be ≥ 1, got {step}")));
    }
    read(state, move |conn, _| {
        let station = read::stations(conn)?
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| bad(format!("unknown station {name:?}")))?;
        let s = read::series(conn, station.id, metric, from, to, step)?;
        Ok(json!({ "source": s.source.as_str(), "t": s.t, "v": s.v }))
    })
    .await
}

/// `GET /api/outages?from=&to=`: the events of every scope overlapping the
/// window (default the last 7 days), newest first (§8.4 f).
pub async fn outages(
    State(state): State<Arc<AppState>>,
    Query(q): Params,
) -> Result<Json<Vec<history::Outage>>, ApiError> {
    let now = unix_now();
    let (from, to) = window(&q, now, OUTAGES_DEFAULT_S)?;
    read(state, move |conn, _| {
        Ok(history::outages_between(conn, from, to, now)?)
    })
    .await
}

/// `GET /api/energy?days=N`: the energy table of the last `N` local days,
/// newest first; `N` in `1..=min(30, raw_retention_days)`, default `min(7, cap)` (§8.4 f).
pub async fn energy(
    State(state): State<Arc<AppState>>,
    Query(q): Params,
) -> Result<Json<Vec<history::EnergyDay>>, ApiError> {
    let cap = ENERGY_MAX_DAYS.min(state.raw_retention_days);
    let days = match q.get("days") {
        // Within the cap, so a retention under 7 days never reads gone days.
        None => ENERGY_DEFAULT_DAYS.min(cap),
        Some(v) => v
            .parse()
            .ok()
            .filter(|d| (1..=cap).contains(d))
            .ok_or_else(|| bad(format!("days must be in 1..={cap}, got {v:?}")))?,
    };
    read(state, move |conn, state| {
        Ok(history::energy(
            conn,
            days,
            unix_now(),
            state.interval_s,
            &state.tz,
        )?)
    })
    .await
}

#[cfg(test)]
mod tests;
