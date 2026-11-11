//! The black-box harness of `tests/security_e2e.rs`: the built binary as a
//! child process and a raw-socket HTTP/1.1 client.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_bluetti-controller");
/// Throwaway credentials; distinctive, so a log hit cannot be a coincidence.
pub const PASSWORD: &str = "e2e-Pa55word-7f3a";
pub const TOKEN: &str = "e2e-token-91c4d2";
const FORM: &str = "Content-Type: application/x-www-form-urlencoded";

/// One raw HTTP/1.1 response.
pub struct Reply {
    pub status: String,
    pub head: Vec<String>,
    pub body: String,
}

impl Reply {
    pub fn code(&self) -> &str {
        self.status.split(' ').nth(1).unwrap_or_default()
    }

    /// Every value of the header `name` (lowercase).
    pub fn all(&self, name: &str) -> Vec<&str> {
        self.head
            .iter()
            .filter_map(|line| line.split_once(':'))
            .filter(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
            .collect()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.all(name).first().copied()
    }

    /// The `session=` id this reply sets.
    pub fn session(&self) -> String {
        let cookie = self.header("set-cookie").expect("a Set-Cookie");
        let value = cookie.strip_prefix("session=").expect(cookie);
        value.split(';').next().unwrap().to_owned()
    }
}

/// `hash-password` of [`PASSWORD`], from the binary itself.
fn hash() -> String {
    let mut child = Command::new(BIN)
        .arg("hash-password")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{PASSWORD}\n").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    text.lines().next().unwrap().to_owned()
}

/// `serve` as a child process; killed and its temp dir removed on drop.
pub struct Server {
    child: Child,
    pub addr: SocketAddr,
    dir: PathBuf,
    pub hash: String,
}

impl Server {
    pub fn start(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("bluetti-e2e-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let hash = hash();
        let config = dir.join("config.toml");
        let db = dir.join("data.db");
        fs::write(
            &config,
            format!(
                "[general]\npoll_interval_secs = 60\ntimezone = \"UTC\"\n\
                 [storage]\npath = \"{}\"\nraw_retention_days = 1\n\
                 [web]\nlisten = \"127.0.0.1:0\"\ntoken = \"{TOKEN}\"\npassword_hash = \"{hash}\"\n\
                 allowed_hosts = [\"ideapad.lan\"]\n\
                 [metrics]\ngrid_voltage_min = 180.0\ndebounce_samples = 2\nusable_capacity_factor = 0.85\n",
                db.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&config, fs::Permissions::from_mode(0o600)).unwrap();
        let log = fs::File::create(dir.join("serve.log")).unwrap();
        let child = Command::new(BIN)
            .arg("--config")
            .arg(&config)
            .arg("serve")
            .env_remove("RUST_LOG")
            // plain log lines, so `wait_ready` can read the bound port
            .env("NO_COLOR", "1")
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
            .unwrap();
        // `serve` binds port 0 itself, so parallel tests never share a port;
        // the real address comes from its `serving listen=…` log line.
        let mut server = Self {
            child,
            addr: SocketAddr::from(([127, 0, 0, 1], 0)),
            dir,
            hash,
        };
        server.wait_ready();
        server
    }

    fn wait_ready(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                panic!("serve exited ({status}):\n{}", self.logs());
            }
            if self.addr.port() == 0
                && let Some(addr) = bound(&self.logs())
            {
                self.addr = addr;
            }
            if self.addr.port() != 0
                && TcpStream::connect(self.addr).is_ok()
                && self.get("/healthz", &[]).code() == "200"
            {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "serve not ready:\n{}",
                self.logs()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// `serve` is still running (no crash on a hostile request).
    pub fn alive(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    pub fn logs(&self) -> String {
        fs::read_to_string(self.dir.join("serve.log")).unwrap_or_default()
    }

    pub fn origin(&self) -> String {
        format!("Origin: http://{}", self.addr)
    }

    /// Sends `bytes` as the whole request; an empty status when the server
    /// hung up without one.
    pub fn raw(&self, bytes: &[u8]) -> Reply {
        let mut stream = TcpStream::connect(self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        let _ = stream.write_all(bytes);
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        let text = String::from_utf8_lossy(&response);
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let mut lines = head.lines().map(str::to_owned);
        Reply {
            status: lines.next().unwrap_or_default(),
            head: lines.collect(),
            body: body.to_owned(),
        }
    }

    /// `Host: <addr>` unless `headers` brings its own.
    pub fn send(&self, method: &str, path: &str, headers: &[&str], body: &str) -> Reply {
        let mut request = format!("{method} {path} HTTP/1.1\r\n");
        if !headers
            .iter()
            .any(|h| h.to_ascii_lowercase().starts_with("host:"))
        {
            request.push_str(&format!("Host: {}\r\n", self.addr));
        }
        for header in headers {
            request.push_str(&format!("{header}\r\n"));
        }
        request.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ));
        self.raw(request.as_bytes())
    }

    pub fn get(&self, path: &str, headers: &[&str]) -> Reply {
        self.send("GET", path, headers, "")
    }

    pub fn login(&self, password: &str, next: &str, headers: &[&str]) -> Reply {
        let mut all = vec![FORM];
        all.extend_from_slice(headers);
        let body = format!("password={password}&next={next}");
        self.send("POST", "/login", &all, &body)
    }
}

/// The address in `serve`'s `serving listen=<addr>` log line.
fn bound(logs: &str) -> Option<SocketAddr> {
    let line = logs.lines().find(|l| l.contains(" serving "))?;
    let rest = line.split_once("listen=")?.1;
    rest.split_whitespace().next()?.parse().ok()
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.dir);
    }
}

pub fn assert_refused(reply: &Reply, codes: &[&str], label: &str) {
    assert!(
        codes.contains(&reply.code()),
        "{label}: {}\n{}",
        reply.status,
        reply.body
    );
    assert!(!reply.body.contains("stations"), "{label}: {}", reply.body);
}
