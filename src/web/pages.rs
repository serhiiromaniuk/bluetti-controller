//! The server-rendered status page `/` (ROADMAP.md §8.1, §8.3, §8.4 b–e, h–j):
//! the house banner, the station cards, the outage log, the energy table, the
//! chart slots `app.js` fills, and the pause form. Times and numbers are
//! formatted here, so the templates only place strings.

use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use rusqlite::Connection;

use crate::web::history::{self, EnergyDay, Outage};
use crate::web::status::{self, Card};
use crate::web::{AppState, unix_now};

/// The energy table's days (§8.4 d) and the outage log's rows (§8.4 e).
const DAYS: u32 = 7;
const OUTAGES: u32 = 20;

#[derive(Template)]
#[template(path = "status.html")]
struct Page {
    banner_class: &'static str,
    banner: String,
    paused: Option<String>,
    /// The newest sample of any station: unix seconds (0 = none) and local
    /// time; `app.js` ages it against `now` and flags it after `stale_s`.
    newest_ts: i64,
    newest_at: String,
    now: i64,
    stale_s: i64,
    cards: Vec<CardView>,
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
    /// The R1b extras line, e.g. `Pack 20.6 °C · mode standard`; empty = none.
    extras: String,
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
    count: String,
    without: String,
    longest: String,
}

/// `GET /`: the status page, or 500 when the read fails.
pub async fn status(State(state): State<Arc<AppState>>) -> Response {
    let html = tokio::task::spawn_blocking(move || {
        let conn = state
            .reader
            .lock()
            .map_err(|_| anyhow::anyhow!("reader lock poisoned"))?;
        render(&conn, unix_now(), &state)
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

/// Renders `/` at `now` (unix seconds).
pub fn render(conn: &Connection, now: i64, state: &AppState) -> anyhow::Result<String> {
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
    let newest = s.stations.iter().filter_map(|c| c.last_seen).max();
    let page = Page {
        banner_class,
        banner,
        paused: s
            .paused_until
            .map(|ts| local(ts, tz, "%H:%M"))
            .transpose()?,
        newest_ts: newest.unwrap_or(0),
        newest_at: match newest {
            Some(ts) => local(ts, tz, "%a %-d %b %H:%M")?,
            None => "never".to_owned(),
        },
        now,
        // The staleness rule of the grid vote (§8.4 b, amendment A1).
        stale_s: 3 * state.interval_s,
        cards: s
            .stations
            .iter()
            .map(|c| card(c, tz))
            .collect::<anyhow::Result<_>>()?,
        outages: history::outages(conn, OUTAGES, now)?
            .iter()
            .map(|o| outage(o, tz))
            .collect::<anyhow::Result<_>>()?,
        energy: energy_days.iter().flat_map(energy_rows).collect(),
        house: energy_days
            .iter()
            // Nothing monitored and no outage recorded: none was ruled out
            // either. A recorded outage (a long one drains every station) shows.
            .map(|d| match blind(d) && d.house.count == 0 {
                true => HouseRow {
                    day: d.day.clone(),
                    count: "—".to_owned(),
                    without: "no data".to_owned(),
                    longest: "—".to_owned(),
                },
                false => HouseRow {
                    day: d.day.clone(),
                    count: d.house.count.to_string(),
                    without: duration(d.house.seconds_without_power),
                    longest: duration(d.house.longest_s),
                },
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
        // The EB3A reads 0 W below its meter floor (ROADMAP §4.3), so a 0 W
        // output says "unknown load", not "no load". A known grid state means
        // the sample is fresh; a stale 0 W keeps the plain dash.
        forecast: if c.forecast == "—" && c.out_w == Some(0.0) && c.grid != "unknown" {
            "no forecast: output reads 0 W (no load, or below the meter floor)".to_owned()
        } else {
            c.forecast.clone()
        },
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
        extras: [
            c.pack_temp_c.map(|t| format!("Pack {t:.1} °C")),
            c.charging_mode.as_ref().map(|m| format!("mode {m}")),
            c.station_left.as_ref().map(|l| format!("station says {l}")),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · "),
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

/// No station has a sample that day.
fn blind(d: &EnergyDay) -> bool {
    d.stations.iter().all(|s| s.no_data)
}

fn energy_rows(d: &EnergyDay) -> Vec<EnergyRow> {
    // `+ 0.0` turns a rounded −0 into 0, so a balanced day never shows "-0".
    let wh = |v: f64| format!("{:.0}", v.round() + 0.0);
    let dashes = || ["—", "—", "—", "—"].map(String::from);
    // One row, not one per station, for a day nothing was monitored.
    if blind(d) {
        return vec![EnergyRow {
            day: d.day.clone(),
            station: "all stations".to_owned(),
            cells: dashes(),
            missing: "no data".to_owned(),
        }];
    }
    d.stations
        .iter()
        .map(|s| EnergyRow {
            day: d.day.clone(),
            station: s.station.clone(),
            cells: if s.no_data {
                dashes()
            } else {
                [
                    wh(s.energy_in_wh),
                    wh(s.energy_out_wh),
                    wh(s.stored_delta_wh),
                    wh(s.losses_estimate_wh),
                ]
            },
            missing: if s.no_data {
                "no data".to_owned()
            } else {
                duration(s.missing_s)
            },
        })
        .collect()
}

/// `ts` in `tz` with a `strftime` format.
pub(crate) fn local(ts: i64, tz: &TimeZone, fmt: &str) -> anyhow::Result<String> {
    Ok(Timestamp::from_second(ts)?
        .to_zoned(tz.clone())
        .strftime(fmt)
        .to_string())
}

/// `1 h 05 min`, `30 min` or `45 s`.
pub(crate) fn duration(s: i64) -> String {
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
