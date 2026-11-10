//! Security suite type 4, property-style (task `2026-11-07_web-security` step
//! 10a): `safe_next`, the Host rule and the Origin check over generated inputs
//! (ROADMAP.md §8.5 c, d, e). Table-driven generation, no extra dependency:
//! every string up to four symbols over an alphabet of the bytes that matter,
//! and the cartesian product of labelled host and port variants. `printable`
//! lives in the binary crate; its exhaustive test is in `cli/scan/tests.rs`.

use axum::http::HeaderValue;

use super::*;
use crate::web::auth::safe_next;

/// Every string of 0..=`max` symbols over `alphabet`.
fn strings(alphabet: &[&str], max: usize) -> Vec<String> {
    let mut all = vec![String::new()];
    let mut last = vec![String::new()];
    for _ in 0..max {
        last = last
            .iter()
            .flat_map(|s| alphabet.iter().map(move |a| format!("{s}{a}")))
            .collect();
        all.extend(last.iter().cloned());
    }
    all
}

/// A browser's view of a `Location` relative to `http://host/` (WHATWG URL,
/// simplified to be strict): leading/trailing C0 and space stripped, tab and
/// newlines removed, `\` read as `/`; any scheme or a leading `//` leaves the
/// site. Written apart from `safe_next`, so it is an independent oracle.
fn stays_on_site(location: &str) -> bool {
    let trimmed = location.trim_matches(|c: char| c <= ' ');
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let scheme = cleaned.split_once(':').is_some_and(|(s, _)| {
        s.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    });
    !scheme && !cleaned.starts_with("//")
}

#[test]
fn safe_next_never_leaves_the_site_over_generated_inputs() {
    let alphabet = [
        "/", "\\", "\t", "\n", "\r", " ", "a", ":", "%", ".", "@", "?", "#", "\0", "é", "\x7f",
    ];
    let mut kept = 0;
    let mut generated = strings(&alphabet, 4);
    generated.extend(strings(&alphabet, 3).iter().map(|s| format!("http:{s}")));
    for input in &generated {
        let out = safe_next(input);
        assert!(out == input || out == "/", "{input:?} -> {out:?}");
        // Sound: what is kept stays on the site and is a valid header value.
        assert!(stays_on_site(out), "{input:?} -> {out:?}");
        assert!(out.bytes().all(|b| b.is_ascii_graphic()), "{input:?}");
        // Complete: a visible-ASCII path on this site is kept.
        if input.starts_with('/') && input.bytes().all(|b| b.is_ascii_graphic()) {
            assert_eq!(out == input, stays_on_site(input), "{input:?}");
        }
        kept += usize::from(out == input && !input.is_empty() && input != "/");
    }
    assert!(generated.len() > 70_000, "{}", generated.len());
    // `/` then 1–3 of the 9 visible symbols, the first not `/` or `\`:
    // 7 + 7·9 + 7·81.
    assert_eq!(kept, 637);
}

/// Host-part variants and whether §8.5 e accepts them (`ideapad.lan` allowed).
const NAMES: &[(&str, bool)] = &[
    ("127.0.0.1", true),
    ("192.168.0.192", true),
    ("0.0.0.0", true),
    ("[::1]", true),
    ("[fe80::1]", true),
    ("[::ffff:127.0.0.1]", true),
    ("localhost", true),
    ("LOCALHOST", true),
    ("ideapad.lan", true),
    ("IdeaPad.LAN", true),
    ("", false),
    ("evil.example", false),
    ("localhost.", false),
    ("ideapad.lan.", false),
    ("ideapad.lan.evil.example", false),
    ("evil-ideapad.lan", false),
    ("xideapad.lan", false),
    ("127.0.0.1.nip.io", false),
    ("1.2.3", false),
    ("::1", false),
    ("[::1", false),
    ("[evil]", false),
    ("[::1]x", false),
    (" localhost", false),
    ("localhost ", false),
    ("l\u{43e}calhost", false),
    ("localhost\0", false),
    ("user@localhost", false),
];

/// Port suffixes and whether they are well-formed (1–5 ASCII digits).
const PORTS: &[(&str, bool)] = &[
    ("", true),
    (":80", true),
    (":0", true),
    (":65535", true),
    (":", false),
    (":123456", false),
    (":8a", false),
    (": 80", false),
    (":+80", false),
    (":\u{668}", false),
    (":80:80", false),
];

#[test]
fn the_host_rule_matches_its_table_over_every_name_and_port() {
    let allowed = vec!["ideapad.lan".to_owned()];
    let mut rows = 0;
    for (name, name_ok) in NAMES {
        for (port, port_ok) in PORTS {
            let host = format!("{name}{port}");
            let want = name_ok & port_ok;
            assert_eq!(host_allowed(&host, &allowed), want, "{host:?}");
            // Case never matters (§8.5 e compares case-insensitively).
            let upper = host.to_ascii_uppercase();
            assert_eq!(host_allowed(&upper, &allowed), want, "{upper:?}");
            rows += 1;
        }
        // No allow-list: only IP literals and `localhost` pass.
        let no_list = host_allowed(name, &[]);
        let listed = name.to_ascii_lowercase().contains("ideapad");
        assert_eq!(no_list, *name_ok && !listed, "{name:?}");
    }
    assert_eq!(rows, NAMES.len() * PORTS.len());
}

/// Mutations of a request's own origin; only the first two are that origin.
fn origins(host: &str) -> Vec<String> {
    vec![
        format!("http://{host}"),
        format!("https://{host}"),
        format!("HTTP://{host}"),
        format!("ws://{host}"),
        format!("http:/{host}"),
        format!("http:///{host}"),
        format!("http://{host}/"),
        format!("http://{host}."),
        format!("http://{host}:1"),
        format!("http://evil-{host}"),
        format!("http://{host}.evil.example"),
        format!("http://user@{host}"),
        format!(" http://{host}"),
        format!("http://{host} "),
        format!("https://{host}#"),
        "null".to_owned(),
        String::new(),
    ]
}

#[test]
fn the_origin_check_is_byte_exact_over_generated_pairs() {
    let mut pairs = 0;
    for (name, _) in NAMES.iter().filter(|(_, ok)| *ok) {
        for (port, _) in PORTS.iter().filter(|(_, ok)| *ok) {
            let host = format!("{name}{port}");
            for (i, origin) in origins(&host).into_iter().enumerate() {
                let mut headers = HeaderMap::new();
                headers.insert(header::HOST, HeaderValue::from_str(&host).unwrap());
                if let Ok(value) = HeaderValue::from_str(&origin) {
                    headers.insert(header::ORIGIN, value);
                }
                assert_eq!(same_origin(&headers), i < 2, "{host:?} / {origin:?}");
                pairs += 1;
            }
            // A missing Host never matches, whatever the Origin.
            let mut no_host = HeaderMap::new();
            let origin = HeaderValue::from_str(&format!("http://{host}")).unwrap();
            no_host.insert(header::ORIGIN, origin);
            assert!(!same_origin(&no_host), "{host:?}");
        }
    }
    assert!(pairs > 500, "{pairs}");
}
