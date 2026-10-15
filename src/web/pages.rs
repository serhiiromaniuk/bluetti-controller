//! The server-rendered status page `/` (ROADMAP.md §8.1, §8.3, §8.4 b–e, h–j):
//! the house banner, the station cards, the outage log, the energy table, the
//! chart slots `app.js` fills, and the pause form. Times and numbers are
//! formatted here, so the templates only place strings.

use std::sync::Arc;

use askama::Template;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::Connection;
use serde::Deserialize;

use crate::web::history::{self, EnergyDay, Outage};
use crate::web::status::{self, Card};
use crate::web::{AppState, unix_now};

/// The energy table's days (§8.4 d) and the outage log's rows (§8.4 e).
const DAYS: u32 = 7;
const OUTAGES: u32 = 20;

/// `?range=24h|7d` of `/` (§8.4 j); anything else is 24 h.
#[derive(Debug, Deserialize)]
pub struct RangeQuery {
    pub range: Option<String>,
}

#[derive(Template)]
#[template(path = "status.html")]
struct Page {
    banner_class: &'static str,
    banner: String,
    paused: Option<String>,
    cards: Vec<CardView>,
    week: bool,
    outages: Vec<OutageRow>,
    energy: Vec<EnergyRow>,
    house: Vec<HouseRow>,
    token: bool,
}

struct CardView {
    name: String,
    model: String,
    soc: String,
    soc_bar: f64,
    in_w: String,
    out_w: String,
    grid: &'static str,
    grid_class: &'static str,
    forecast: String,
    last_seen: String,
    health: String,
    health_class: &'static str,
    polls: String,
    encrypted: &'static str,
    unverified: bool,
}

struct OutageRow {
    scope: String,
    start: String,
    end: String,
    duration: String,
}

struct EnergyRow {
    day: String,
    station: String,
    cells: [String; 4],
    missing: String,
}

struct HouseRow {
    day: String,
    count: usize,
    without: String,
    longest: String,
}

/// `GET /`: the status page, or 500 when the read fails.
pub async fn status(State(state): State<Arc<AppState>>, Query(q): Query<RangeQuery>) -> Response {
    let week = q.range.as_deref() == Some("7d");
    let html = tokio::task::spawn_blocking(move || {
        let conn = state
            .reader
            .lock()
            .map_err(|_| anyhow::anyhow!("reader lock poisoned"))?;
        render(&conn, unix_now(), &state, week)
    })
    .await
    .map_err(anyhow::Error::from)
    .and_then(|r| r);
    match html {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::warn!(error = format!("{err:#}"), "status page: render failed");
            (StatusCode::INTERNAL_SERVER_ERROR, "status page unavailable").into_response()
        }
    }
}

/// Renders `/` at `now` (unix seconds); `week` = the 7 d chart range.
pub fn render(conn: &Connection, now: i64, state: &AppState, week: bool) -> anyhow::Result<String> {
    let tz = &state.tz;
    let s = status::status(conn, now, state)?;
    let (banner_class, what) = match s.house.state {
        "power" => ("present", "Grid power"),
        "outage" => ("absent", "Power outage"),
        _ => ("unknown", "Grid state unknown"),
    };
    let banner = match s.house.since {
        Some(ts) => format!("{what} since {}", local(ts, tz, "%a %-d %b %H:%M")?),
        None => what.to_owned(),
    };
    let energy_days = history::energy(conn, DAYS, now, state.interval_s, tz)?;
    let page = Page {
        banner_class,
        banner,
        paused: s
            .paused_until
            .map(|ts| local(ts, tz, "%H:%M"))
            .transpose()?,
        cards: s
            .stations
            .iter()
            .map(|c| card(c, tz))
            .collect::<anyhow::Result<_>>()?,
        week,
        outages: history::outages(conn, OUTAGES, now)?
            .iter()
            .map(|o| outage(o, tz))
            .collect::<anyhow::Result<_>>()?,
        energy: energy_days.iter().flat_map(energy_rows).collect(),
        house: energy_days
            .iter()
            .map(|d| HouseRow {
                day: d.day.clone(),
                count: d.house.count,
                without: duration(d.house.seconds_without_power),
                longest: duration(d.house.longest_s),
            })
            .collect(),
        token: state.token.is_some(),
    };
    Ok(page.render()?)
}

fn card(c: &Card, tz: &TimeZone) -> anyhow::Result<CardView> {
    let watts = |w: Option<f64>| w.map_or("—".to_owned(), |w| format!("{w:.0} W"));
    let mut polls = format!(
        "{} polls, {} failures in 24 h",
        c.samples_24h, c.failures_24h
    );
    if let Some(stage) = &c.last_failure_stage {
        polls += &format!("; last failure: {stage}");
    }
    let health = match c.unreachable_since {
        Some(ts) => format!("unreachable since {}", local(ts, tz, "%a %H:%M")?),
        None => c.health.to_owned(),
    };
    Ok(CardView {
        name: c.name.clone(),
        model: c.model.clone(),
        soc: c.soc.map_or("—".to_owned(), |v| format!("{v:.0} %")),
        soc_bar: c.soc.unwrap_or(0.0),
        in_w: watts(c.in_w),
        out_w: watts(c.out_w),
        grid: c.grid,
        grid_class: match c.grid {
            "present" => "ok",
            "absent" => "bad",
            _ => "warn",
        },
        forecast: c.forecast.clone(),
        last_seen: match c.last_seen {
            Some(ts) => local(ts, tz, "%a %-d %b %H:%M")?,
            None => "never".to_owned(),
        },
        health,
        health_class: if c.health == "ok" { "ok" } else { "bad" },
        polls,
        encrypted: match c.encrypted {
            Some(true) => "yes",
            Some(false) => "no",
            None => "—",
        },
        unverified: c.unverified,
    })
}

fn outage(o: &Outage, tz: &TimeZone) -> anyhow::Result<OutageRow> {
    const AT: &str = "%Y-%m-%d %H:%M";
    Ok(OutageRow {
        scope: o.scope.clone(),
        start: local(o.started_at, tz, AT)?,
        end: match o.ended_at {
            Some(ts) => local(ts, tz, AT)?,
            None => "ongoing".to_owned(),
        },
        duration: duration(o.duration_s),
    })
}

fn energy_rows(d: &EnergyDay) -> Vec<EnergyRow> {
    // `+ 0.0` turns a rounded −0 into 0, so a balanced day never shows "-0".
    let wh = |v: f64| format!("{:.0}", v.round() + 0.0);
    d.stations
        .iter()
        .map(|s| EnergyRow {
            day: d.day.clone(),
            station: s.station.clone(),
            cells: [
                wh(s.energy_in_wh),
                wh(s.energy_out_wh),
                wh(s.stored_delta_wh),
                wh(s.losses_estimate_wh),
            ],
            missing: duration(s.missing_s),
        })
        .collect()
}

/// `ts` in `tz` with a `strftime` format.
fn local(ts: i64, tz: &TimeZone, fmt: &str) -> anyhow::Result<String> {
    Ok(Timestamp::from_second(ts)?
        .to_zoned(tz.clone())
        .strftime(fmt)
        .to_string())
}

/// `1 h 05 min`, `30 min` or `45 s`.
fn duration(s: i64) -> String {
    let (h, m) = (s / 3600, s % 3600 / 60);
    if h > 0 {
        format!("{h} h {m:02} min")
    } else if s >= 60 {
        format!("{m} min")
    } else {
        format!("{s} s")
    }
}

#[cfg(test)]
mod tests;
