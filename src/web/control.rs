//! `POST /api/control/output?station=&output=ac|dc&state=on|off` (ROADMAP.md
//! §8.2, §R3): after the guard's credential check (§8.5), the control gate (golden rule #2), then the
//! request goes to the collector, which runs it between poll rounds and
//! answers with the read-back state.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde_json::{Value, json};
use tokio::sync::oneshot;

use super::AppState;
use super::pause::error;
use crate::collector::Control;
use crate::control::{Command, Refused, Request};
use crate::protocol::control::Output;

/// `403` while control is disabled, `400` for an unknown
/// station, an output the model does not list or a bad `state`. Then waits
/// at most one poll deadline + one interval for the collector: `504` on
/// timeout, `503` while shutting down, `502` with the error, else `200
/// {station, output, state}` with the read-back.
pub async fn output(
    State(state): State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>,
) -> (StatusCode, Json<Value>) {
    let name = query.get("station").map_or("", String::as_str);
    let Some(target) = state.stations.iter().find(|t| t.config.name == name) else {
        return error(StatusCode::BAD_REQUEST, "unknown station");
    };
    let Some(output) = query.get("output").and_then(|o| Output::from_key(o)) else {
        return error(StatusCode::BAD_REQUEST, "output must be ac or dc");
    };
    let on = match query.get("state").map(String::as_str) {
        Some("on") => true,
        Some("off") => false,
        _ => return error(StatusCode::BAD_REQUEST, "state must be on or off"),
    };
    let command = match Command::new(&state.control_config, &target.config, output, on) {
        Ok(command) => command,
        Err(refused @ Refused::Disabled) => {
            return error(StatusCode::FORBIDDEN, &refused.to_string());
        }
        Err(refused @ Refused::NotAllowed { .. }) => {
            return error(StatusCode::BAD_REQUEST, &refused.to_string());
        }
    };
    if *state.control.borrow() == Control::Shutdown {
        return error(StatusCode::SERVICE_UNAVAILABLE, "shutting down");
    }
    let (reply, answer) = oneshot::channel();
    let exchange = async {
        state.commands.send(Request { command, reply }).await.ok()?;
        answer.await.ok()
    };
    match tokio::time::timeout(wait(&state), exchange).await {
        Err(_) => error(StatusCode::GATEWAY_TIMEOUT, "no answer from the collector"),
        Ok(None) => error(StatusCode::SERVICE_UNAVAILABLE, "shutting down"),
        // The collector answers an error when shutdown cuts the request.
        Ok(Some(Err(_))) if *state.control.borrow() == Control::Shutdown => {
            error(StatusCode::SERVICE_UNAVAILABLE, "shutting down")
        }
        Ok(Some(Err(message))) => error(StatusCode::BAD_GATEWAY, &message),
        // The collector logs the switch with its read-back.
        Ok(Some(Ok(read_back))) => (
            StatusCode::OK,
            Json(json!({
                "station": name,
                "output": output.key(),
                "state": if read_back { "on" } else { "off" },
            })),
        ),
    }
}

/// One poll deadline (the interval shared by the stations, §5.4) plus one
/// interval: a request waits at most for the current round, then runs.
fn wait(state: &AppState) -> Duration {
    let interval = Duration::from_secs(state.interval_s.unsigned_abs());
    let stations = u32::try_from(state.stations.len().max(1)).unwrap_or(u32::MAX);
    interval / stations + interval
}

#[cfg(test)]
mod tests;
