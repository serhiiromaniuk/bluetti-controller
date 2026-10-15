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
    let (status, body) = tokio::task::spawn_blocking(move || get(addr, "/?range=7d"))
        .await
        .unwrap();
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert!(body.contains(r#"<meta name="viewport""#), "{body}");
    assert!(body.contains("if the power goes now: 1.2 h"), "{body}");
    assert!(body.contains(r#"<a href="?range=7d" data-range="7d" class="on">"#));
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
    render(&conn, NOW, &state, false).unwrap()
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
        "<tr><td>2026-10-15</td><td>router</td><td>100</td><td>100</td><td>0</td><td>0</td><td>0 s</td></tr>"
    ));
    assert!(html.contains("<tr><td>2026-10-15</td><td>1</td><td>30 min</td><td>30 min</td></tr>"));
    assert!(html.contains(r#"<div class="chart" data-chart-station="office"></div>"#));
    assert!(html.contains(r#"<a href="?range=24h" data-range="24h" class="on">"#));
    assert!(
        html.contains(r#"<meta name="viewport" content="width=device-width, initial-scale=1">"#)
    );
    assert!(html.contains(r#"<form id="pause">"#) && html.contains(r#"<output id="pause-reply">"#));
    assert!(!html.contains(r#"name="token""#));
    assert!(!html.contains("http://") && !html.contains("https://"));
    // §8.4 i: the HTML's share of the 100 KB page weight.
    assert!(html.len() <= 25_000, "{} B", html.len());
}

#[test]
fn token_field_only_with_a_token() {
    assert!(page(Some("s3cret")).contains(r#"<input name="token" type="password""#));
    assert!(!page(Some("s3cret")).contains("s3cret"));
}
