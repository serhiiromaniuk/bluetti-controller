//! The web server (ROADMAP.md §8): the pages, the JSON API, SSE and the pause
//! POST, one module each (§8.4 a). Handlers read through the read-only
//! connection in `spawn_blocking` (§5.3), never through the writer's.

use std::future::Future;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;
use tokio::net::TcpListener;

pub mod api;
pub mod assets;
pub mod history;
pub mod pages;
pub mod pause;
pub mod sse;
pub mod state;
pub mod status;
pub mod telemetry;

pub use state::AppState;

/// The `/healthz` body: always `ok` while the process runs; the age of the
/// newest sample of any station, `null` before the first successful poll.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Health {
    pub status: &'static str,
    pub last_poll_age_s: Option<i64>,
}

/// Every route of §8.1/§8.2 over `state`.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(pages::status))
        .route("/telemetry", get(telemetry::page))
        .route("/assets/{name}", get(assets::asset))
        .route("/api/status", get(api::status))
        .route("/api/series", get(api::series))
        .route("/api/outages", get(api::outages))
        .route("/api/energy", get(api::energy))
        .route("/api/polling/pause", post(pause::pause))
        .route("/events", get(sse::events))
        .route("/healthz", get(healthz))
        .with_state(state)
}

/// Serves `router` on `listener` until `shutdown` resolves, then lets open
/// requests finish (§5.3 graceful close).
pub async fn serve(
    listener: TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
        .context("web server")
}

/// 200 while the process runs, also when the DB read fails (§8.2: a slow BLE
/// round or a busy DB never makes the liveness check restart the process).
async fn healthz(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Health>) {
    let db = Arc::clone(&state.reader);
    let newest = tokio::task::spawn_blocking(move || {
        let conn = db
            .lock()
            .map_err(|_| anyhow::anyhow!("reader lock poisoned"))?;
        // Per station, so each max is one `(station_id, ts)` PK seek, not a scan.
        let ts: Option<i64> = conn.query_row(
            "SELECT max((SELECT max(ts) FROM sample WHERE station_id = s.id)) FROM station s",
            [],
            |row| row.get(0),
        )?;
        anyhow::Ok(ts)
    })
    .await;
    let newest = match newest {
        Ok(Ok(ts)) => ts,
        Ok(Err(err)) => {
            tracing::warn!(error = format!("{err:#}"), "healthz: read failed");
            None
        }
        Err(err) => {
            tracing::warn!(error = %err, "healthz: read task failed");
            None
        }
    };
    let health = Health {
        status: "ok",
        last_poll_age_s: newest.map(|ts| (unix_now() - ts).max(0)),
    };
    (StatusCode::OK, Json(health))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests;
