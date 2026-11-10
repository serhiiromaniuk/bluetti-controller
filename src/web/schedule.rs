//! The schedule routes (ROADMAP.md §8.2, §6, §R3): `GET /api/schedule` lists
//! every entry; `POST` sets one station output's daily `on`/`off` times and
//! `DELETE` removes them. Edits go to the single writer as [`Record`]s and are
//! accepted while control is disabled; they need `web.token` to be set and
//! the bearer, so nobody on the LAN can save an entry that fires once the
//! owner enables control.

use std::collections::HashMap;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use serde::Serialize;
use serde_json::{Value, json};

use super::AppState;
use super::api::{ApiError, read};
use super::pause::{error, same};
use crate::collector::Target;
use crate::protocol::control::{Output, switches};
use crate::store::Record;
use crate::store::read::schedule_entries;

type Reply = (StatusCode, Json<Value>);
type Params = Query<HashMap<String, String>>;

/// One entry as the API shows it: times as `HH:MM` or `null`.
#[derive(Debug, Serialize)]
pub struct Entry {
    station: String,
    output: &'static str,
    on: Option<String>,
    off: Option<String>,
}

/// `GET /api/schedule` (open): every entry of a configured station, by
/// station id then output.
pub async fn list(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Entry>>, ApiError> {
    read(state, |conn, state| {
        Ok(schedule_entries(conn)?
            .into_iter()
            .filter_map(|e| {
                let target = state.stations.iter().find(|t| t.id == e.station_id)?;
                Some(Entry {
                    station: target.config.name.clone(),
                    output: e.output.key(),
                    on: e.on_min.map(hh_mm),
                    off: e.off_min.map(hh_mm),
                })
            })
            .collect())
    })
    .await
}

/// `POST /api/schedule?station=&output=&on=HH:MM&off=HH:MM`: an empty or
/// missing time is unset; at least one is set and `on ≠ off`. `202` with the
/// entry once the writer has it queued.
pub async fn set(State(state): State<Arc<AppState>>, headers: HeaderMap, query: Params) -> Reply {
    let (target, output) = match edit(&state, &headers, &query) {
        Ok(found) => found,
        Err(reply) => return reply,
    };
    let mut times = [None, None];
    for (slot, field) in times.iter_mut().zip(["on", "off"]) {
        match query.get(field).map(String::as_str) {
            None | Some("") => {}
            Some(text) => match minutes(text) {
                Some(m) => *slot = Some(m),
                None => {
                    return error(
                        StatusCode::BAD_REQUEST,
                        &format!("{field} must be HH:MM (00:00-23:59)"),
                    );
                }
            },
        }
    }
    let [on_min, off_min] = times;
    match (on_min, off_min) {
        (None, None) => return error(StatusCode::BAD_REQUEST, "set on or off"),
        (Some(on), Some(off)) if on == off => {
            return error(StatusCode::BAD_REQUEST, "on and off must differ");
        }
        _ => {}
    }
    let record = Record::ScheduleSet {
        station: target.id,
        output,
        on_min,
        off_min,
    };
    if let Err(reply) = queue(&state, record).await {
        return reply;
    }
    tracing::info!(station = %target.config.name, output = output.key(), ?on_min, ?off_min, "schedule set from the web");
    let entry = json!({
        "station": target.config.name,
        "output": output.key(),
        "on": on_min.map(hh_mm),
        "off": off_min.map(hh_mm),
    });
    (StatusCode::ACCEPTED, Json(entry))
}

/// `DELETE /api/schedule?station=&output=`: `202` once the writer has it queued.
pub async fn delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Params,
) -> Reply {
    let (target, output) = match edit(&state, &headers, &query) {
        Ok(found) => found,
        Err(reply) => return reply,
    };
    let record = Record::ScheduleDelete {
        station: target.id,
        output,
    };
    if let Err(reply) = queue(&state, record).await {
        return reply;
    }
    tracing::info!(station = %target.config.name, output = output.key(), "schedule deleted from the web");
    let entry = json!({ "station": target.config.name, "output": output.key() });
    (StatusCode::ACCEPTED, Json(entry))
}

/// The checks both edits share, in order: `web.token` set (`403`), the bearer
/// (`401`), a configured station name (never a MAC) and an output its model
/// switches (`400`).
fn edit<'a>(
    state: &'a AppState,
    headers: &HeaderMap,
    query: &Params,
) -> Result<(&'a Target, Output), Reply> {
    let Some(token) = &state.token else {
        return Err(error(
            StatusCode::FORBIDDEN,
            "schedule edits need web.token",
        ));
    };
    let given = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "));
    if !given.is_some_and(|given| same(given, token.as_bytes())) {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "a valid bearer token is required",
        ));
    }
    let name = query.get("station").map_or("", String::as_str);
    let Some(target) = state.stations.iter().find(|t| t.config.name == name) else {
        return Err(error(StatusCode::BAD_REQUEST, "unknown station"));
    };
    let output = query.get("output").and_then(|o| Output::from_key(o));
    let switched = output.filter(|&o| {
        switches(target.config.model)
            .iter()
            .any(|s| s.output() == o)
    });
    let Some(output) = switched else {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "output must be one this station switches (ac or dc)",
        ));
    };
    Ok((target, output))
}

/// Hands `record` to the writer; `503` once the collector has dropped its
/// sender (shutdown), since the state only holds a weak one.
async fn queue(state: &AppState, record: Record) -> Result<(), Reply> {
    let sent = match state.records.upgrade() {
        Some(records) => records.send(record).await.is_ok(),
        None => false,
    };
    if sent {
        Ok(())
    } else {
        Err(error(StatusCode::SERVICE_UNAVAILABLE, "shutting down"))
    }
}

/// `HH:MM` (two digits each, 00:00–23:59) to minutes after midnight.
fn minutes(text: &str) -> Option<u16> {
    let (h, m) = text.split_once(':')?;
    let digits = |s: &str| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(h) || !digits(m) {
        return None;
    }
    let (h, m) = (h.parse::<u16>().ok()?, m.parse::<u16>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

fn hh_mm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

#[cfg(test)]
mod tests;
