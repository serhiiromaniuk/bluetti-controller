//! The status view model (ROADMAP.md §8.1, §8.4 a–c): the house banner, one
//! card per station with its runtime forecast, and `paused_until`. `/` and
//! `/api/status` serialise the same struct, so the field names are the API.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tokio::time::Instant;

use crate::collector::Control;
use crate::metrics::grid::GridState;
use crate::metrics::house::{self, HouseState};
use crate::metrics::localtime::{local_day_bounds, local_hhmm};
use crate::metrics::runtime::{self, Forecast};
use crate::protocol::device::{Metric, Model};
use crate::store::read::{self, StationRow, Values};
use crate::store::{Health, station_health};
use crate::web::AppState;
use crate::web::pages::duration;

/// An R1a extra older than this (two 10-min slow slots) is not shown (R1b).
const EXTRA_MAX_AGE_S: i64 = 20 * 60;
/// How far back the forecast reads samples (§8.4 c).
const FORECAST_WINDOW_S: i64 = 2 * 3600;
/// The poll-health window on the card (§8.4 b).
const HEALTH_WINDOW_S: i64 = 24 * 3600;
/// The fields behind SOC, the powers and the forecast (the Q2 badge).
const BADGED: [Metric; 5] = [
    Metric::BatterySoc,
    Metric::AcInputPower,
    Metric::DcInputPower,
    Metric::AcOutputPower,
    Metric::DcOutputPower,
];

/// The whole `/` page and the `/api/status` body.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Status {
    pub house: Banner,
    /// One card per station row, by id.
    pub stations: Vec<Card>,
    /// Unix seconds the collector is paused until; `None` = polling (§8.4 h).
    pub paused_until: Option<i64>,
}

/// The house banner (§7.2, §8.4 b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Banner {
    /// `"power"`, `"outage"` or `"unknown"`.
    pub state: &'static str,
    /// Unix seconds: an outage's `started_at`, or power's newest closed house
    /// event `ended_at`; `None` when not known.
    pub since: Option<i64>,
}

/// One station's card (§8.1, §8.4 b–c).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Card {
    pub name: String,
    /// The `station.model` string, e.g. `EB3A`.
    pub model: String,
    /// Battery % of the newest sample.
    pub soc: Option<f64>,
    /// `ac_in_w + dc_in_w` of the newest sample; `None` when both are missing.
    pub in_w: Option<f64>,
    /// `ac_out_w + dc_out_w` of the newest sample; `None` when both are missing.
    pub out_w: Option<f64>,
    /// `"present"`, `"absent"` or `"unknown"` (§8.4 b, amendment A1).
    pub grid: &'static str,
    /// "lasts until HH:MM" (local; "Sat 17 Oct HH:MM" on another day) / "if the power goes now: N h" (one
    /// decimal) / "—" (§7.3).
    pub forecast: String,
    /// Unix seconds of the newest sample.
    pub last_seen: Option<i64>,
    /// `"ok"` or `"unreachable"` (`store::station_health`).
    pub health: &'static str,
    /// The first of the trailing failures while unreachable.
    pub unreachable_since: Option<i64>,
    /// Samples in the last 24 h.
    pub samples_24h: i64,
    /// `poll_failure` rows in the last 24 h.
    pub failures_24h: i64,
    /// The newest failure's stage (`discover`, `connect`, …), any age.
    pub last_failure_stage: Option<String>,
    /// Whether the newest sample came over an encrypted link.
    pub encrypted: Option<bool>,
    /// SOC, powers and forecast rest on `verified: false` fields (Q2 badge).
    pub unverified: bool,
    /// R1b, unverified: pack temperature in °C, one decimal (read as °F).
    pub pack_temp_c: Option<f64>,
    /// R1b, unverified: `standard`, `silent`, `turbo`, or the raw number.
    pub charging_mode: Option<String>,
    /// R1b, unverified: the station's own `time_to_empty`, e.g. `4 h 09 min`;
    /// only while discharging (output > 0 W, input = 0 W).
    pub station_left: Option<String>,
}

/// Builds the status view at `now` (unix seconds) from the read connection.
pub fn status(conn: &Connection, now: i64, state: &AppState) -> anyhow::Result<Status> {
    let mut cards = Vec::new();
    let mut sensors = Vec::new();
    for row in read::stations(conn)? {
        let (card, grid) = card(conn, now, state, &row)?;
        if row.grid_sensor {
            sensors.push(grid);
        }
        cards.push(card);
    }
    Ok(Status {
        house: banner(conn, &sensors)?,
        stations: cards,
        paused_until: paused_until(*state.control.borrow()),
    })
}

/// Amendment A1: no sample, or the newest older than 3 intervals → `Unknown`
/// (checked first, so an unreachable station never votes `Absent`); else an
/// open station-scope event → `Absent`; else `Present`.
fn grid_state(
    conn: &Connection,
    now: i64,
    interval_s: i64,
    name: &str,
    newest: Option<i64>,
) -> anyhow::Result<GridState> {
    Ok(match newest {
        Some(ts) if now - ts <= 3 * interval_s => match read::open_event(conn, name)? {
            Some(_) => GridState::Absent,
            None => GridState::Present,
        },
        _ => GridState::Unknown,
    })
}

/// An open house event wins (outage since its start); otherwise the vote, with
/// power "since" the newest closed house event's end.
fn banner(conn: &Connection, sensors: &[GridState]) -> anyhow::Result<Banner> {
    if let Some(open) = read::open_event(conn, "house")? {
        return Ok(Banner {
            state: "outage",
            since: Some(open.started_at),
        });
    }
    Ok(match house::vote(sensors) {
        HouseState::Power => Banner {
            state: "power",
            since: conn.query_row(
                "SELECT max(ended_at) FROM grid_event WHERE scope = 'house'",
                [],
                |row| row.get(0),
            )?,
        },
        // No open house event yet: the collector has not confirmed a start.
        HouseState::Outage => Banner {
            state: "outage",
            since: None,
        },
        HouseState::Unknown => Banner {
            state: "unknown",
            since: None,
        },
    })
}

fn card(
    conn: &Connection,
    now: i64,
    state: &AppState,
    row: &StationRow,
) -> anyhow::Result<(Card, GridState)> {
    let latest = read::latest_sample(conn, row.id)?;
    let last_seen = latest.as_ref().map(|(ts, _)| *ts);
    let grid = grid_state(conn, now, state.interval_s, &row.name, last_seen)?;

    let from = now - FORECAST_WINDOW_S;
    // `read::samples` adds the one sample before `from`; the window is 2 h only.
    let window: Vec<(i64, Values)> = read::samples(conn, row.id, from, now + 1)?
        .into_iter()
        .filter(|(ts, _)| *ts >= from)
        .collect();
    let refs: Vec<(i64, &Values)> = window.iter().map(|(ts, v)| (*ts, v)).collect();
    let forecast = match runtime::forecast(&refs, row.capacity_wh, state.usable_factor, grid) {
        Forecast::OnBattery { until_ts, .. } => {
            let at = local_hhmm(until_ts, &state.tz)?;
            // A5: past local midnight the time alone would read as today.
            if local_day_bounds(until_ts, &state.tz)? == local_day_bounds(now, &state.tz)? {
                format!("lasts until {at}")
            } else {
                let day = Timestamp::from_second(until_ts)?
                    .to_zoned(state.tz.clone())
                    .strftime("%a %-d %b")
                    .to_string();
                format!("lasts until {day} {at}")
            }
        }
        Forecast::OnGrid { runtime_h, .. } => format!("if the power goes now: {runtime_h:.1} h"),
        Forecast::Unknown => "—".to_owned(),
    };

    let (health, unreachable_since) = match station_health(conn, row.id)? {
        Health::Ok => ("ok", None),
        Health::Unreachable { since } => ("unreachable", Some(since)),
    };
    let day = now - HEALTH_WINDOW_S;
    let count = |table: &str| -> rusqlite::Result<i64> {
        conn.query_row(
            &format!("SELECT count(*) FROM {table} WHERE station_id = ?1 AND ts > ?2"),
            (row.id, day),
            |r| r.get(0),
        )
    };
    let last_failure_stage = conn
        .query_row(
            "SELECT stage FROM poll_failure WHERE station_id = ?1 ORDER BY ts DESC LIMIT 1",
            [row.id],
            |r| r.get(0),
        )
        .optional()?;
    let encrypted = last_seen
        .map(|ts| {
            conn.query_row(
                "SELECT encrypted FROM sample WHERE station_id = ?1 AND ts = ?2",
                (row.id, ts),
                |r| r.get(0),
            )
        })
        .transpose()?;

    let values = latest.map(|(_, v)| v).unwrap_or_default();
    let sum = |a, b| match (values.get(&a), values.get(&b)) {
        (None, None) => None,
        (x, y) => Some(x.unwrap_or(&0.0) + y.unwrap_or(&0.0)),
    };
    let in_w = sum(Metric::AcInputPower, Metric::DcInputPower);
    let out_w = sum(Metric::AcOutputPower, Metric::DcOutputPower);
    let discharging = matches!((in_w, out_w), (Some(i), Some(o)) if i == 0.0 && o > 0.0);
    let extras = fresh_extras(conn, row.id, now)?;
    let extra = |m: Metric| extras.get(m.key()).copied();
    let card = Card {
        name: row.name.clone(),
        model: row.model.clone(),
        soc: values.get(&Metric::BatterySoc).copied(),
        in_w,
        out_w,
        grid: match grid {
            GridState::Present => "present",
            GridState::Absent => "absent",
            GridState::Unknown => "unknown",
        },
        forecast,
        last_seen,
        health,
        unreachable_since,
        samples_24h: count("sample")?,
        failures_24h: count("poll_failure")?,
        last_failure_stage,
        encrypted,
        unverified: !verified(&row.model),
        pack_temp_c: extra(Metric::PackTemperature)
            .map(|f| ((f - 32.0) * 50.0 / 9.0).round() / 10.0),
        charging_mode: extra(Metric::ChargingMode).map(mode_name),
        station_left: extra(Metric::TimeToEmpty)
            .filter(|&min| discharging && min <= 60_000.0)
            .map(|min| duration((min * 60.0) as i64)),
    };
    Ok((card, grid))
}

/// The newest value of each extra key no older than [`EXTRA_MAX_AGE_S`].
fn fresh_extras(
    conn: &Connection,
    station_id: i64,
    now: i64,
) -> anyhow::Result<BTreeMap<String, f64>> {
    Ok(read::latest_extras(conn, station_id)?
        .into_iter()
        .filter(|(_, (ts, _))| now - ts <= EXTRA_MAX_AGE_S)
        .map(|(key, (_, v))| (key, v))
        .collect())
}

/// `charging_mode` (160) by the R3 map of register 2020, assumed (ROADMAP §2.2 R1b).
fn mode_name(raw: f64) -> String {
    match raw as i64 {
        0 => "standard".to_owned(),
        1 => "silent".to_owned(),
        2 => "turbo".to_owned(),
        _ => format!("{raw}"),
    }
}

/// Whether every [`BADGED`] field of the model is verified; an unknown model
/// string is unverified.
fn verified(model: &str) -> bool {
    let Some(model) = model_named(model) else {
        return false;
    };
    model
        .profile()
        .fields
        .iter()
        .filter(|f| BADGED.contains(&f.metric))
        .all(|f| f.verified)
}

/// The model stored as `name` in the `station` table; `None` for any other string.
pub(crate) fn model_named(name: &str) -> Option<Model> {
    [Model::Eb3a, Model::Ac180, Model::Elite200V2]
        .into_iter()
        .find(|m| m.name() == name)
}

/// `Pause { until }` as unix seconds (§8.4 h); `None` once past or not paused.
pub(crate) fn paused_until(control: Control) -> Option<i64> {
    let Control::Pause { until } = control else {
        return None;
    };
    let left = until.checked_duration_since(Instant::now())?;
    let at = (SystemTime::now() + left).duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(at.as_secs()).ok()
}

#[cfg(test)]
mod tests;
