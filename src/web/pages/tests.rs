use rusqlite::{Connection, params};

use super::*;
use crate::store::migrate;
use crate::web::AppState;
use crate::web::tests::{get, start};

/// router: EB3A 268.8 Wh, sensor; office: Elite200V2 2073.6 Wh, sensor.
fn db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate(&mut conn).unwrap();
    conn.execute_batch(
        "INSERT INTO station VALUES (1, 'router', 'EB3A', 268.8, 1);
         INSERT INTO station VALUES (2, 'office', 'Elite200V2', 2073.6, 1);",
    )
    .unwrap();
    conn
}

/// router: 61 samples 60 s apart ending at `now − 60`, SOC 50 %, 100 W in
/// (AC) and 100 W out (AC), grid 230 V.
fn fixture(now: i64) -> Connection {
    let conn = db();
    for i in 0..=60 {
        conn.execute(
            "INSERT INTO sample (station_id, ts, soc, ac_in_w, ac_out_w, ac_in_v, encrypted)
             VALUES (1, ?1, 50, 100, 100, 230, 1)",
            [now - 3660 + i * 60],
        )
        .unwrap();
    }
    conn
}

#[tokio::test(flavor = "multi_thread")]
async fn status_page_is_served_as_html() {
    let now = crate::web::unix_now();
    let (addr, stop, server) = start(AppState::for_tests(fixture(now))).await;
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert!(body.contains(r#"<meta name="viewport""#), "{body}");
    assert!(body.contains("if the power goes now: 1.2 h"), "{body}");
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

// 2026-10-15 (a Thursday) 09:00Z = 12:00 in Kyiv (EEST, +03); local midnight
// = 10-14 21:00Z, so the fixture's last hour lies inside today.
const NOW: i64 = 1_792_022_400 + 9 * 3600;

fn event(conn: &Connection, scope: &str, started_at: i64, ended_at: Option<i64>) {
    conn.execute(
        "INSERT INTO grid_event (scope, kind, started_at, ended_at, precision_s)
         VALUES (?1, 'outage', ?2, ?3, 60)",
        params![scope, started_at, ended_at],
    )
    .unwrap();
}

/// The fixture plus a closed house outage 09:30–10:00 local and an ongoing
/// `office` outage since 11:50 (office has no sample, so it votes unknown).
fn page(token: Option<&str>) -> String {
    let conn = fixture(NOW);
    event(&conn, "house", NOW - 9000, Some(NOW - 7200));
    event(&conn, "office", NOW - 600, None);
    let mut state = AppState::for_tests(Connection::open_in_memory().unwrap());
    state.token = token.map(str::to_owned);
    render(&conn, NOW, &state).unwrap()
}

#[test]
fn renders_hand_computed_values() {
    let html = page(None);
    // Router votes present → power, since the house event's end 10:00 local.
    assert!(
        html.contains(r#"<div class="banner present">Grid power since Thu 15 Oct 10:00</div>"#),
        "{html}"
    );
    // SOC 50 % of the newest sample; EWMA load 100 W: 268.8 × 0.5 × 0.9 / 100
    // = 1.2096 h → one decimal.
    assert!(html.contains(r#"<div class="big">50 %</div>"#));
    assert!(html.contains("In 100 W · Out 100 W"));
    assert!(html.contains("<p>if the power goes now: 1.2 h</p>"));
    assert!(html.contains(r#"<span class="badge warn" title="SOC, powers and forecast"#));
    assert!(html.contains("61 polls, 0 failures in 24 h"));
    // Outage log: newest first; 1800 s = 30 min, 600 s so far = 10 min.
    let ongoing =
        "<tr><td>office</td><td>2026-10-15 11:50</td><td>ongoing</td><td>10 min</td></tr>";
    let closed =
        "<tr><td>house</td><td>2026-10-15 09:30</td><td>2026-10-15 10:00</td><td>30 min</td></tr>";
    assert!(html.find(ongoing).unwrap() < html.find(closed).unwrap());
    // Today's energy: 60 segments of 1 min at 100 W in and out = 100 Wh each;
    // SOC flat → Δstored 0, losses 100 − 100 − 0 = 0; no gap.
    assert!(html.contains(
        "<tr><td>2026-10-15</td><td>router</td><td>100</td><td>100</td><td>0</td><td>0</td><td class=\"muted\">—</td><td class=\"muted\">—</td><td>0 s</td></tr>"
    ));
    assert!(html.contains("<tr><td>2026-10-15</td><td>1</td><td>30 min</td><td>30 min</td></tr>"));
    // Today office has no sample: its own dashes, never 0 Wh.
    assert!(html.contains(
        "<tr><td>2026-10-15</td><td>office</td><td>—</td><td>—</td><td>—</td><td>—</td><td class=\"muted\">—</td><td class=\"muted\">—</td><td>no data</td></tr>"
    ));
    // Yesterday no station has one: a single row, and no house "0 s".
    assert!(html.contains(
        "<tr><td>2026-10-14</td><td>all stations</td><td>—</td><td>—</td><td>—</td><td>—</td><td class=\"muted\">—</td><td class=\"muted\">—</td><td>no data</td></tr>"
    ));
    assert!(!html.contains("<td>2026-10-14</td><td>router</td>"));
    assert!(html.contains("<tr><td>2026-10-14</td><td>—</td><td>no data</td><td>—</td></tr>"));
    for slot in ["c-soc", "c-out_w", "c-in_w", "c-temp_c"] {
        assert!(html.contains(&format!(r#"<div class="chart" id="{slot}"></div>"#)));
    }
    // Newest sample NOW − 60 = 11:59 local; stale after 3 × 60 s.
    assert!(html.contains(&format!(
        r#"<p id="age" class="age" data-ts="{}" data-now="{NOW}" data-stale="180" data-at="Thu 15 Oct 11:59">"#,
        NOW - 60
    )));
    // Manual refresh by default: no live stream, auto-refresh starts off.
    assert!(
        !html.contains("EventSource") && html.contains(r#"<option value="">Auto: off</option>"#)
    );
    assert!(html.contains(r#"<option value="now-12h" selected>"#));
    assert!(html.contains(r#"<html lang="en" data-theme="dark">"#));
    assert!(
        html.contains(r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#)
    );
    assert!(html.contains(r#"<form id="pause">"#) && html.contains(r#"<output id="pause-reply">"#));
    assert!(!html.contains(r#"name="token""#));
    assert!(!html.contains("http://") && !html.contains("https://"));
    // §8.4 i: the HTML's share of the 100 KB page weight.
    assert!(html.len() <= 18_000, "{} B", html.len());
}

#[test]
fn token_field_only_with_a_token() {
    assert!(page(Some("s3cret")).contains(r#"<input name="token" type="password""#));
    assert!(!page(Some("s3cret")).contains("s3cret"));
}

#[test]
fn zero_output_says_why_there_is_no_forecast() {
    // router: SOC flat at 100 %, output 0 W (the EB3A meter floor, §4.3), so
    // neither the load nor the SOC slope gives a runtime.
    let conn = db();
    for i in 0..=10 {
        conn.execute(
            "INSERT INTO sample (station_id, ts, soc, ac_in_w, ac_out_w, ac_in_v, encrypted)
             VALUES (1, ?1, 100, 0, 0, 230, 1)",
            [NOW - 660 + i * 60],
        )
        .unwrap();
    }
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    let html = render(&conn, NOW, &state).unwrap();
    let why = "no forecast: output reads 0 W (no load, or below the meter floor)";
    assert!(html.contains(why), "{html}");
    // An hour later the 0 W is stale (grid unknown): the plain dash again.
    let html = render(&conn, NOW + 3600, &state).unwrap();
    assert!(!html.contains(why) && html.contains("<p>—</p>"), "{html}");
}

#[test]
fn a_recorded_outage_shows_on_a_day_without_samples() {
    // An open house outage since 10-13 12:00 local; no station sample at all.
    // 10-14 is a whole local day inside it: 1 outage, 24 h without power.
    let conn = db();
    event(&conn, "house", NOW - 2 * 86_400, None);
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    let html = render(&conn, NOW, &state).unwrap();
    assert!(
        html.contains(
            "<tr><td>2026-10-14</td><td>1</td><td>24 h 00 min</td><td>24 h 00 min</td></tr>"
        ),
        "{html}"
    );
    // 10-12 lies before the outage and holds no sample: unknown, not 0 s.
    assert!(html.contains("<tr><td>2026-10-12</td><td>—</td><td>no data</td><td>—</td></tr>"));
}

#[test]
fn r1b_extras_line_is_muted_and_marked_unverified() {
    let conn = db();
    conn.execute_batch(&format!(
        "INSERT INTO sample (station_id, ts, soc, ac_in_w, ac_out_w, encrypted)
         VALUES (2, {NOW}, 80, 0, 100, 1);
         INSERT INTO sample_extra VALUES (2, {NOW}, 'pack_temperature', 69);
         INSERT INTO sample_extra VALUES (2, {NOW}, 'charging_mode', 1);
         INSERT INTO sample_extra VALUES (2, {NOW}, 'time_to_empty', 249);"
    ))
    .unwrap();
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    let html = render(&conn, NOW, &state).unwrap();
    let line = "<p class=\"muted\">Pack 20.6 °C · mode silent · station says 4 h 09 min \
                <span class=\"badge warn\"";
    assert!(html.contains(line), "{html}");
    // The router has no extras: one such line only.
    assert_eq!(html.matches("<p class=\"muted\">Pack ").count(), 1);
}

#[test]
fn counter_columns_and_lifetime_line_for_a_v2_station() {
    // office (V2) counters at 10-14 23:00 local (NOW − 13 h): in 100 + 20,
    // out 80 + 10 kWh; today 11:00 (NOW − 1 h): in 100.4 + 20.1 = 120.5, out
    // 80.2 + 10 = 90.2. Today: in 500 Wh, out 200 Wh; office has no sample,
    // so its integrated cells stay dashes. Lifetime: out ÷ in = 90.2 ÷ 120.5
    // = 74.9 % → 75 %. router (EB3A) has no counters: dashes and no line.
    let conn = fixture(NOW);
    let keys = [
        "grid_in_energy",
        "pv_in_energy",
        "ac_out_energy",
        "dc_out_energy",
    ];
    for (ts, values) in [
        (NOW - 13 * 3600, [100.0, 20.0, 80.0, 10.0]),
        (NOW - 3600, [100.4, 20.1, 80.2, 10.0]),
    ] {
        for (key, v) in keys.into_iter().zip(values) {
            conn.execute(
                "INSERT INTO sample_extra VALUES (2, ?1, ?2, ?3)",
                params![ts, key, v],
            )
            .unwrap();
        }
    }
    let state = AppState::for_tests(Connection::open_in_memory().unwrap());
    let html = render(&conn, NOW, &state).unwrap();
    assert!(
        html.contains(
            r#"<td>office</td><td>—</td><td>—</td><td>—</td><td>—</td><td class="muted">500</td><td class="muted">200</td><td>no data</td>"#
        ),
        "{html}"
    );
    assert!(html.contains(
        r#"<td>router</td><td>100</td><td>100</td><td>0</td><td>0</td><td class="muted">—</td><td class="muted">—</td>"#
    ));
    assert!(html.contains(
        r#"<p class="muted">office: in 120.5 kWh, out 90.2 kWh, out ÷ in 75 % (station counters, unverified)</p>"#
    ));
    assert!(!html.contains("router: in"));
}
