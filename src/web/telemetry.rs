//! The raw telemetry page `/telemetry` (ROADMAP.md §8.1, §7.5, §8.4 k): every
//! stored metric per station with its latest value, its `verified` flag and a
//! server-side SVG sparkline, so the page needs no JS. Each station opens with
//! its usable-capacity estimate from observed discharges (R1, ROADMAP.md §2.2).

use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use rusqlite::Connection;

use crate::metrics::capacity;
use crate::metrics::energy::BUCKET_S as BUCKET_S_5M;
use crate::protocol::device::{Field, Metric};
use crate::store::read;
use crate::web::pages::local;
use crate::web::status::model_named;
use crate::web::{AppState, unix_now};

/// Sparkline bucket: 24 h / 30 min = 48 points at most (§8.4 k).
pub const BUCKET_S: i64 = 1800;
/// The sparkline's SVG viewBox.
const W: f64 = 120.0;
const H: f64 = 30.0;

#[derive(Template)]
#[template(path = "telemetry.html")]
struct Page {
    stations: Vec<StationView>,
}

struct StationView {
    name: String,
    model: String,
    health: Health,
    rows: Vec<Row>,
}

/// The R1 "Battery health" block (ROADMAP.md §2.2).
struct Health {
    /// The estimate line; `None` before the first qualifying discharge.
    estimate: Option<String>,
    /// A register the estimate rests on is not verified against the app.
    meter_unverified: bool,
    /// Newest first.
    segments: Vec<SegmentRow>,
}

struct SegmentRow {
    start: String,
    hours: String,
    soc: String,
    energy: String,
    capacity: String,
}

struct Row {
    key: String,
    value: String,
    at: String,
    verified: bool,
    points: String,
}

/// `GET /telemetry`: the page, or 500 when the read fails.
pub async fn page(State(state): State<Arc<AppState>>) -> Response {
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
            tracing::warn!(error = format!("{err:#}"), "telemetry page: render failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "telemetry page unavailable",
            )
                .into_response()
        }
    }
}

/// Renders `/telemetry` at `now` (unix seconds).
pub fn render(conn: &Connection, now: i64, state: &AppState) -> anyhow::Result<String> {
    let (from, to) = window(now);
    let mut stations = Vec::new();
    for station in read::stations(conn)? {
        let profile = model_named(&station.model).map(|m| m.profile());
        // The core columns of the newest sample, then every recent extra key.
        let mut latest: Vec<(String, Option<Metric>, i64, f64)> = Vec::new();
        if let Some((ts, values)) = read::latest_sample(conn, station.id)? {
            latest.extend(
                values
                    .into_iter()
                    .map(|(m, v)| (m.key().to_owned(), Some(m), ts, v)),
            );
        }
        for (key, (ts, v)) in read::latest_extras(conn, station.id)? {
            latest.push((key.clone(), Metric::from_key(&key), ts, v));
        }
        let mut rows = Vec::new();
        for (key, metric, ts, v) in latest {
            let points = match metric {
                Some(m) => {
                    let s = read::series(conn, station.id, m, from, to, BUCKET_S)?;
                    sparkline(&s.t, &s.v, from, to)
                }
                None => String::new(),
            };
            let unit = metric.map_or("", Metric::unit);
            rows.push(Row {
                key,
                value: format!("{v:.1} {unit}").trim_end().to_owned(),
                at: local(ts, &state.tz, "%a %H:%M")?,
                verified: profile.is_some_and(|p| is_verified(p.fields, metric)),
                points,
            });
        }
        // Battery side where the pack reports V and I (R1 amended 2026-10-16),
        // else the metered output from `rollup_5m`.
        let battery_side =
            profile.is_some_and(|p| p.fields.iter().any(|f| f.metric == Metric::PackCurrent));
        let segments = if battery_side {
            // At most 30 days: the scan is linear in raw samples (~140 ms per
            // station-month), the page budget is 1 s.
            let from = now - i64::from(state.raw_retention_days.min(30)) * 86_400;
            let points = read::battery_points(conn, station.id, from, now + 1)?;
            capacity::battery_segments(&points, 3 * state.interval_s)
        } else {
            // A full bucket less one missed poll (5 − 1 at the 60 s interval).
            let min_n = (BUCKET_S_5M / state.interval_s.max(1) - 1).max(1);
            capacity::segments(&read::discharge_buckets(conn, station.id, min_n)?)
        };
        let estimate = capacity::estimate(&segments, station.capacity_wh).map(|e| {
            let how = match battery_side {
                true => "battery side, pack V × I",
                false => "metered output; reads low at small loads",
            };
            format!(
                "≈ {:.0} Wh of {:.0} Wh nominal = {:.0} % ({how}; median of the newest {})",
                e.median_wh,
                station.capacity_wh,
                e.factor * 100.0,
                e.n
            )
        });
        let segments = segments
            .iter()
            .rev()
            .map(|s| {
                Ok(SegmentRow {
                    start: local(s.start, &state.tz, "%d %b %H:%M")?,
                    hours: format!("{:.1}", (s.end - s.start) as f64 / 3600.0),
                    soc: format!("{:.0} → {:.0} %", s.soc_from, s.soc_to),
                    energy: format!("{:.0}", s.energy_wh),
                    capacity: format!("{:.0}", s.capacity_wh),
                })
            })
            .collect::<anyhow::Result<_>>()?;
        let health = Health {
            estimate,
            // The registers the estimate rests on: pack V and I, or the output.
            meter_unverified: profile.is_some_and(|p| {
                let used: &[Metric] = match battery_side {
                    true => &[Metric::PackVoltage, Metric::PackCurrent],
                    false => &[Metric::AcOutputPower],
                };
                used.iter().any(|m| !is_verified(p.fields, Some(*m)))
            }),
            segments,
        };
        stations.push(StationView {
            name: station.name,
            model: station.model,
            health,
            rows,
        });
    }
    Ok(Page { stations }.render()?)
}

/// Whether `metric` is a verified field of the profile's `fields`; an extra key
/// that is no [`Metric`] is unverified.
pub fn is_verified(fields: &[Field], metric: Option<Metric>) -> bool {
    fields
        .iter()
        .any(|f| Some(f.metric) == metric && f.verified)
}

/// The sparkline window `[from, to)`: it ends just after `now` and starts on a
/// bucket boundary, so it spans at most 48 buckets of [`BUCKET_S`].
pub fn window(now: i64) -> (i64, i64) {
    let from = (now.div_euclid(BUCKET_S) + 1) * BUCKET_S - 86_400;
    (from, now + 1)
}

/// SVG `polyline` points of `(t, v)` over `[from, to)` in a `W × H` box: time
/// left to right, the series' min at the bottom and max at the top. A flat
/// series sits at mid-height.
pub fn sparkline(t: &[i64], v: &[f64], from: i64, to: i64) -> String {
    let (min, max) = v
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &x| {
            (lo.min(x), hi.max(x))
        });
    let span = (to - from).max(1) as f64;
    t.iter()
        .zip(v)
        .map(|(&t, &v)| {
            let x = (t - from) as f64 * W / span;
            let y = if max > min {
                (max - v) * H / (max - min)
            } else {
                H / 2.0
            };
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests;
