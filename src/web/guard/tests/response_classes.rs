//! Security suite type 6 (task `2026-11-07_web-security` step 10a): the four
//! §8.5 f headers on every response class the router can produce, the session
//! cookie's attributes, and no cookie on any other response (ROADMAP.md §8.5
//! b, f). `no_template_has_an_inline_script` (`pages/tests.rs`) covers the
//! templates.

use super::*;

const FORM: &str = "Content-Type: application/x-www-form-urlencoded";
/// Expected status, method, path, then the header lines and body.
type Row<'a> = (&'a str, &'a str, &'a str, (Vec<String>, String));

const CSP: &str = "default-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'";

#[tokio::test(flavor = "multi_thread")]
async fn every_response_class_carries_the_headers_and_only_logins_set_a_cookie() {
    let (_db, state) = state("sec-classes", true, None);
    let cookie = session(&state);
    let origin = ORIGIN.to_owned();
    let form = |body: &str| (vec![FORM.to_owned()], body.to_owned());
    let none = || (vec![], String::new());
    let with = |lines: &[&str]| {
        (
            lines.iter().map(|l| (*l).to_owned()).collect(),
            String::new(),
        )
    };
    let c = cookie.as_str();
    #[rustfmt::skip]
    let rows: Vec<Row> = vec![
        ("200 OK", "GET", "/", with(&[c])),
        ("200 OK", "GET", "/api/status", with(&[c])),
        ("200 OK", "GET", "/events", with(&[c])),
        ("200 OK", "GET", "/assets/app.js", none()),
        ("200 OK", "GET", "/healthz", none()),
        ("200 OK", "GET", "/login", none()),
        ("303 See Other", "GET", "/telemetry", none()),
        ("303 See Other", "POST", "/login", form("password=test&next=/telemetry")),
        ("400 Bad Request", "GET", "/api/series?station=x&from=1&to=0", with(&[c])),
        ("401 Unauthorized", "GET", "/api/status", none()),
        ("401 Unauthorized", "GET", "/events", none()),
        ("401 Unauthorized", "POST", "/login", form("password=a")),
        ("401 Unauthorized", "POST", "/login", form("password=b")),
        ("401 Unauthorized", "POST", "/login", form("password=c")),
        ("401 Unauthorized", "POST", "/login", form("password=d")),
        ("401 Unauthorized", "POST", "/login", form("password=e")),
        ("429 Too Many Requests", "POST", "/login", form("password=test")),
        ("403 Forbidden", "POST", "/api/polling/pause?minutes=0", with(&[c, "Origin: http://evil"])),
        ("404 Not Found", "GET", "/no-such-page", none()),
        ("404 Not Found", "GET", "/assets/no-such.js", none()),
        ("405 Method Not Allowed", "PUT", "/login", none()),
        ("415 Unsupported Media Type", "POST", "/login", (vec![], "password=x".into())),
        ("422 Unprocessable Entity", "POST", "/login", form("next=%2F")),
        ("421 Misdirected Request", "GET", "/", with(&[c, "Host: evil.example"])),
        ("421 Misdirected Request", "POST", "/login", (vec![FORM.into(), "Host: evil.example".into()], "password=test".into())),
        ("303 See Other", "POST", "/logout", with(&[c, &origin])),
    ];
    let calls: Vec<_> = rows
        .iter()
        .map(|(_, m, p, (h, b))| (m.to_string(), p.to_string(), h.clone(), b.clone()))
        .collect();
    let replies = with_server(&state, move |addr| {
        calls
            .iter()
            .map(|(method, path, headers, body)| {
                let headers: Vec<&str> = headers.iter().map(String::as_str).collect();
                request_with(addr, method, path, &headers, body)
            })
            .collect::<Vec<_>>()
    })
    .await;

    for ((want, method, path, _), (status, headers, _)) in rows.iter().zip(&replies) {
        let row = format!("{method} {path} ({want})");
        assert_eq!(status, &format!("HTTP/1.1 {want}"), "{row}");
        assert_eq!(
            header(headers, "content-security-policy"),
            Some(CSP),
            "{row}"
        );
        assert_eq!(header(headers, "x-frame-options"), Some("DENY"), "{row}");
        assert_eq!(
            header(headers, "x-content-type-options"),
            Some("nosniff"),
            "{row}"
        );
        assert_eq!(
            header(headers, "referrer-policy"),
            Some("same-origin"),
            "{row}"
        );
        let set_cookie = header(headers, "set-cookie");
        match (*method, *path, *want) {
            ("POST", "/login", "303 See Other") => {
                let value = set_cookie.expect("login sets the cookie");
                let (id, attributes) = value
                    .strip_prefix("session=")
                    .and_then(|v| v.split_once("; "))
                    .unwrap();
                assert_eq!(id.len(), 64, "{value}");
                assert!(
                    id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
                    "{value}"
                );
                assert_eq!(
                    attributes,
                    "HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000"
                );
            }
            ("POST", "/logout", _) => assert_eq!(
                set_cookie,
                Some("session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"),
            ),
            _ => assert_eq!(set_cookie, None, "{row}"),
        }
    }
    // The SSE stream is the event-stream type, under the same headers.
    assert_eq!(
        header(&replies[2].1, "content-type"),
        Some("text/event-stream")
    );
}
