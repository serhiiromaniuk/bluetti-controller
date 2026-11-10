//! Security suite type 7, black-box (task `2026-11-07_web-security` step 10a):
//! the built binary hashes a password with `hash-password`, then runs `serve`
//! on a loopback port from a temp config with no station (so no BLE), and the
//! exploit catalogue of `src/web/guard/tests/` is replayed over real HTTP from
//! a raw socket, so requests below `http::Request` (duplicate `Host`,
//! absolute-form) reach the server as an attacker sends them. The process log
//! is then searched for the password, the token, the hash and every session id
//! (ROADMAP.md §8.5).

#[path = "security_e2e/server.rs"]
mod server;

use server::{PASSWORD, Server, TOKEN, assert_refused};

const PAUSE: &str = "/api/polling/pause?minutes=5";
const HEADERS: [(&str, &str); 4] = [
    (
        "content-security-policy",
        "default-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
    ),
    ("x-frame-options", "DENY"),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "same-origin"),
];

#[test]
fn no_credential_path_method_or_bearer_trick_reaches_a_protected_handler() {
    let server = Server::start("bypass");
    for path in [
        "//api/status",
        "/api//status",
        "/api/../api/status",
        "/%2Fapi/status",
        "/api%2Fstatus",
        "/API/status",
        "/api/status/",
        "/api/status;x",
        "//telemetry",
    ] {
        assert_refused(&server.get(path, &[]), &["401", "404"], path);
    }
    for method in ["HEAD", "OPTIONS", "TRACE"] {
        assert_refused(
            &server.send(method, "/api/status", &[], ""),
            &["401", "404", "405"],
            method,
        );
    }
    let overridden = server.send(
        "POST",
        PAUSE,
        &["X-HTTP-Method-Override: GET", &server.origin()],
        "",
    );
    assert_eq!(overridden.code(), "401", "method override");
    let bearer = format!("Authorization: Bearer {TOKEN}");
    let lower = format!("Authorization: bearer {TOKEN}");
    let spaced = format!("Authorization: Bearer  {TOKEN}");
    let forged = format!("Cookie: session={}", "0".repeat(64));
    let tricks: [(&str, Vec<&str>); 6] = [
        ("bare Bearer", vec!["Authorization: Bearer"]),
        ("lowercase scheme", vec![&lower]),
        ("two spaces", vec![&spaced]),
        ("wrong token", vec!["Authorization: Bearer wrong"]),
        (
            "the first Authorization decides",
            vec!["Authorization: Bearer wrong", &bearer],
        ),
        ("forged session id", vec![&forged]),
    ];
    for (label, headers) in &tricks {
        let reply = server.get("/api/status", headers);
        assert_eq!(reply.code(), "401", "{label}");
        assert_eq!(reply.body, r#"{"error":"login required"}"#, "{label}");
    }
    let page = server.get("/telemetry", &[]);
    assert_eq!(page.code(), "303");
    assert_eq!(page.header("location"), Some("/login?next=/telemetry"));
    assert_eq!(server.get("/api/status", &[&bearer]).code(), "200");
    // Hostile spans with a valid bearer: a JSON 400, never a scan.
    for path in [
        "/api/outages?from=-9223372036854775808&to=9223372036854775807",
        "/api/chart?from=9223372036854775807&to=-9223372036854775808",
        "/api/series?station=x&metric=battery_soc&from=-1&to=1",
        "/api/energy?days=4294967296",
    ] {
        let reply = server.get(path, &[&bearer]);
        assert_eq!(reply.code(), "400", "{path}: {}", reply.body);
        assert!(reply.body.starts_with(r#"{"error":"#), "{path}");
        assert!(!reply.body.contains("unknown station"), "{path}");
    }
}

#[test]
fn rebinding_and_cross_site_requests_are_refused() {
    let server = Server::start("csrf");
    let id = server.login(PASSWORD, "/", &[]).session();
    let cookie = format!("Cookie: session={id}");
    let port = server.addr.port();
    let other_port = format!("Origin: http://127.0.0.1:{}", port + 1);
    let https_other = format!("Origin: https://127.0.0.1:{}", port + 1);
    let suffix = format!("Origin: http://evil-127.0.0.1:{port}");
    let referer = format!("Referer: http://{}/", server.addr);
    let csrf: [(&str, Vec<&str>); 6] = [
        ("no Origin", vec![]),
        ("Origin: null", vec!["Origin: null"]),
        ("other port", vec![&other_port]),
        ("other port and scheme", vec![&https_other]),
        ("suffix trick", vec![&suffix]),
        ("Referer only", vec![&referer]),
    ];
    for (label, extra) in &csrf {
        let mut headers = vec![cookie.as_str()];
        headers.extend(extra);
        let reply = server.send("POST", PAUSE, &headers, "");
        assert_eq!(reply.code(), "403", "{label}");
        assert_eq!(reply.body, r#"{"error":"cross-origin request refused"}"#);
    }
    let rebinding: [(&str, Vec<&str>); 6] = [
        ("foreign name", vec!["Host: evil.example"]),
        ("trailing dot", vec!["Host: ideapad.lan."]),
        ("prefix", vec!["Host: ideapad.lan.evil.example"]),
        (
            "X-Forwarded-Host ignored",
            vec!["Host: evil.example", "X-Forwarded-Host: localhost"],
        ),
        (
            "duplicate Host",
            vec!["Host: evil.example", "Host: localhost"],
        ),
        ("IP in a DNS name", vec!["Host: 127.0.0.1.nip.io"]),
    ];
    for (label, extra) in &rebinding {
        let mut headers = vec![cookie.as_str()];
        headers.extend(extra);
        assert_refused(&server.get("/api/status", &headers), &["421", "400"], label);
    }
    let absolute = server.raw(
        format!(
            "GET http://{}/api/status HTTP/1.1\r\nHost: evil.example\r\n{cookie}\r\nConnection: close\r\n\r\n",
            server.addr
        )
        .as_bytes(),
    );
    assert_refused(&absolute, &["421", "400"], "absolute-form");
    let no_host = server.raw(format!("GET /api/status HTTP/1.0\r\n{cookie}\r\n\r\n").as_bytes());
    assert_refused(&no_host, &["421", "400"], "no Host");
    for host in ["Host: IDEAPAD.LAN", "Host: [::1]:1", "Host: localhost"] {
        assert_eq!(
            server.get("/api/status", &[&cookie, host]).code(),
            "200",
            "{host}"
        );
    }
    let origin = server.origin();
    assert_eq!(
        server.send("POST", PAUSE, &[&cookie, &origin], "").code(),
        "200"
    );
    let bearer = format!("Authorization: Bearer {TOKEN}");
    assert_eq!(server.send("POST", PAUSE, &[&bearer], "").code(), "200");
}

#[test]
fn login_issues_a_fresh_strict_cookie_logout_revokes_it_and_no_secret_is_logged() {
    let server = Server::start("session");
    let mut ids = Vec::new();
    for next in [
        "%2F%2Fevil.example",
        "%2F%5Cevil.example",
        "%2F%09%2Fevil.example",
        "https%3A%2F%2Fevil.example",
        "javascript%3Aalert(1)",
        "%2F%0D%0ASet-Cookie%3A%20pwned%3D1",
    ] {
        let reply = server.login(PASSWORD, next, &[]);
        assert_eq!(reply.code(), "303", "{next}");
        assert_eq!(reply.header("location"), Some("/"), "{next}");
        assert!(reply.header("pwned").is_none(), "{next}");
        assert_eq!(reply.all("set-cookie").len(), 1, "{next}");
        ids.push(reply.session());
    }
    let wrong = server.login("wrong", "/", &[]);
    assert_eq!(wrong.code(), "401");
    assert!(wrong.header("set-cookie").is_none());

    let planted = format!("Cookie: session={}", "a".repeat(64));
    let login = server.login(PASSWORD, "%2Ftelemetry", &[&planted]);
    assert_eq!(login.code(), "303");
    assert_eq!(login.header("location"), Some("/telemetry"));
    let id = login.session();
    assert_eq!(id.len(), 64);
    assert!(id.bytes().all(|b| b.is_ascii_hexdigit()) && id != "a".repeat(64));
    assert!(!ids.contains(&id));
    assert_eq!(
        login.header("set-cookie").unwrap(),
        format!("session={id}; HttpOnly; SameSite=Strict; Path=/; Max-Age=2592000")
    );
    ids.push(id.clone());

    let cookie = format!("Cookie: session={id}");
    let origin = server.origin();
    let classes = [
        ("200 healthz", server.get("/healthz", &[])),
        ("200 page", server.get("/telemetry", &[&cookie])),
        ("200 asset", server.get("/assets/theme.js", &[])),
        ("303", server.get("/", &[])),
        ("401", server.login("wrong", "/", &[])),
        ("403", server.send("POST", PAUSE, &[&cookie], "")),
        ("404", server.get("/nope", &[&cookie])),
        (
            "421",
            server.get("/api/status", &[&cookie, "Host: evil.example"]),
        ),
    ];
    for (label, reply) in &classes {
        assert!(
            reply.status.contains(&label[..3]),
            "{label}: {}",
            reply.status
        );
        for (name, value) in HEADERS {
            assert_eq!(reply.header(name), Some(value), "{label}: {name}");
        }
    }

    let logout = server.send("POST", "/logout", &[&cookie, &origin], "");
    assert_eq!(logout.code(), "303");
    assert_eq!(
        logout.header("set-cookie").unwrap(),
        "session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
    );
    assert_eq!(server.get("/api/status", &[&cookie]).code(), "401");

    let bearer = format!("Authorization: Bearer {TOKEN}");
    assert_eq!(server.get("/api/status", &[&bearer]).code(), "200");
    let logs = server.logs();
    assert!(logs.contains("serving"), "{logs}");
    for secret in [PASSWORD, TOKEN, server.hash.as_str()]
        .into_iter()
        .chain(ids.iter().map(String::as_str))
    {
        assert!(!logs.contains(secret), "secret in the log:\n{logs}");
    }
    let salt = server.hash.split('$').nth(4).unwrap();
    assert!(!logs.contains(salt), "hash salt in the log");
}

#[test]
fn brute_force_locks_the_login_after_five_failures() {
    let server = Server::start("lockout");
    for attempt in 1..=5 {
        assert_eq!(server.login("wrong", "/", &[]).code(), "401", "{attempt}");
    }
    // The right password inside the lock window: refused, no cookie.
    let locked = server.login(PASSWORD, "/", &[]);
    assert_eq!(locked.code(), "429");
    assert!(locked.header("set-cookie").is_none());
    // A spoofed client address shares the one lock.
    let spoofed = server.login(PASSWORD, "/", &["X-Forwarded-For: 10.9.8.7"]);
    assert_eq!(spoofed.code(), "429");
}

#[test]
fn oversized_nul_and_non_utf8_input_is_refused_without_a_crash() {
    let mut server = Server::start("bytes");
    // 1 MiB fits the form limit: one verification, refused, counted.
    let big = server.login(&"a".repeat(1 << 20), "/", &[]);
    assert_eq!(big.code(), "401");
    // Over the limit: refused before the handler, never hashed nor counted.
    let huge = server.login(&"a".repeat(3 << 20), "/", &[]);
    assert!(
        huge.code() == "413" || huge.status.is_empty(),
        "{}",
        huge.status
    );
    for password in ["%00", "%FF%FE"] {
        assert_eq!(server.login(password, "/", &[]).code(), "401", "{password}");
    }
    let host = format!("Host: {}\r\n", server.addr);
    let raw = |line: &[u8], header: &[u8]| {
        let mut request = line.to_vec();
        request.extend_from_slice(host.as_bytes());
        request.extend_from_slice(header);
        request.extend_from_slice(b"Connection: close\r\n\r\n");
        request
    };
    let get = b"GET /api/status HTTP/1.1\r\n";
    let requests = [
        (
            "0xFF in the path",
            raw(b"GET /api/st\xffatus HTTP/1.1\r\n", b""),
        ),
        (
            "NUL in the path",
            raw(b"GET /api/status%00 HTTP/1.1\r\n", b""),
        ),
        ("0xFF bearer", raw(get, b"Authorization: Bearer \xff\r\n")),
        ("0xFF cookie", raw(get, b"Cookie: session=\xff\xfe\r\n")),
        ("NUL cookie", raw(get, b"Cookie: session=\x00\r\n")),
        (
            "0xFF Host",
            b"GET /api/status HTTP/1.1\r\nHost: local\xffhost\r\nConnection: close\r\n\r\n"
                .to_vec(),
        ),
    ];
    for (label, request) in &requests {
        let reply = server.raw(request);
        assert!(
            ["400", "401", "404", "421"].contains(&reply.code()),
            "{label}: {}",
            reply.status
        );
    }
    assert!(server.alive(), "serve crashed:\n{}", server.logs());
    assert_eq!(server.get("/healthz", &[]).code(), "200");
    // Three counted failures so far (1 MiB, NUL, non-UTF-8): two more lock.
    for _ in 0..2 {
        assert_eq!(server.login("wrong", "/", &[]).code(), "401");
    }
    assert_eq!(server.login(PASSWORD, "/", &[]).code(), "429");
}

#[test]
fn concurrent_wrong_logins_verify_at_most_five() {
    let server = Server::start("flood");
    let codes: Vec<String> = std::thread::scope(|scope| {
        let tries: Vec<_> = (0..10)
            .map(|_| scope.spawn(|| server.login("wrong", "/", &[]).code().to_owned()))
            .collect();
        tries.into_iter().map(|t| t.join().unwrap()).collect()
    });
    let count = |code: &str| codes.iter().filter(|c| *c == code).count();
    assert_eq!((count("401"), count("429")), (5, 5), "{codes:?}");
    let locked = server.login(PASSWORD, "/", &[]);
    assert_eq!(locked.code(), "429");
    assert!(locked.header("set-cookie").is_none());
}
