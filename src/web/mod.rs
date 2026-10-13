//! The web server (ROADMAP.md §8). M3 serves only `/healthz` (§8.2); the pages
//! and the JSON API arrive with M4. Handlers read through the read-only
//! connection in `spawn_blocking` (§5.3), never through the writer's.

use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use rusqlite::Connection;
use serde::Serialize;
use tokio::net::TcpListener;

type Db = Arc<Mutex<Connection>>;

/// The `/healthz` body: always `ok` while the process runs; the age of the
/// newest sample of any station, `null` before the first successful poll.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Health {
    pub status: &'static str,
    pub last_poll_age_s: Option<i64>,
}

/// The routes over `reader`, a connection from [`crate::store::open_reader`].
pub fn router(reader: Connection) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(Arc::new(Mutex::new(reader)))
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
async fn healthz(State(db): State<Db>) -> (StatusCode, Json<Health>) {
    let newest = tokio::task::spawn_blocking(move || {
        let conn = db
            .lock()
            .map_err(|_| anyhow::anyhow!("reader lock poisoned"))?;
        let ts: Option<i64> = conn.query_row("SELECT max(ts) FROM sample", [], |row| row.get(0))?;
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
