//! Server-sent events at `/events` (ROADMAP.md §8.2, §8.4 g): each committed
//! row the writer publishes, as a named event with the row as JSON.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_util::Stream;
use futures_util::stream::unfold;
use serde_json::{Map, Value, json};
use tokio::sync::broadcast::error::RecvError;

use super::AppState;
use crate::store::Update;
use crate::store::read::CORE;

/// `GET /events`: `sample`, `grid_event` and `poll_failure` events until
/// `state.shutdown` fires (so the graceful close never waits on a client).
/// A lagged client skips what it missed; a comment every 15 s keeps it alive.
pub async fn events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let start = (state.updates.subscribe(), state.shutdown.subscribe());
    let stream = unfold(start, |(mut updates, mut shutdown)| async move {
        loop {
            let received = tokio::select! {
                biased;
                // Also resolves at once when shutdown fired before subscribing.
                _ = shutdown.wait_for(|stop| *stop) => return None,
                received = updates.recv() => received,
            };
            match received {
                Ok(update) => return Some((Ok(event(&update)), (updates, shutdown))),
                Err(RecvError::Lagged(missed)) => {
                    tracing::debug!(missed, "/events client lagged, skipping");
                }
                Err(RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

/// The event named after the row's table, its columns as JSON data.
fn event(update: &Update) -> Event {
    let (name, data) = match update {
        Update::Sample { station, ts, core } => {
            let mut row = Map::new();
            row.insert("station_id".into(), json!(station));
            row.insert("ts".into(), json!(ts));
            for (metric, column) in CORE {
                if let Some(value) = core.get(&metric) {
                    row.insert(column.into(), json!(value));
                }
            }
            ("sample", Value::Object(row))
        }
        Update::GridEvent {
            scope,
            started_at,
            ended_at,
        } => (
            "grid_event",
            json!({"scope": scope, "started_at": started_at, "ended_at": ended_at}),
        ),
        Update::PollFailure { station, ts, stage } => (
            "poll_failure",
            json!({"station_id": station, "ts": ts, "stage": stage.as_str()}),
        ),
    };
    Event::default().event(name).data(data.to_string())
}

#[cfg(test)]
mod tests;
