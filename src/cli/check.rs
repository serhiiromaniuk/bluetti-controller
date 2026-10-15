//! `check`: what `bluetti-update` runs before it switches a release
//! (`ROADMAP.md` §10.6). `main` has already loaded and validated the config
//! and the keys file (a failure there exits 1); this compares the database
//! schema with the binary's. Read-only, no BLE: safe next to a running `serve`.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::Path;

use bluetti_controller::config::Config;
use bluetti_controller::store::{SCHEMA_VERSION, schema_version};

/// The exit code when the database schema is newer than this binary.
pub const SCHEMA_NEWER: i32 = 2;
/// The exit code when the config file does not load or validate (`main`);
/// every other failure (keys, an unreadable database) exits 1.
pub const CONFIG_REJECTED: i32 = 4;

/// Prints the report and returns the exit code: 0 ok, [`SCHEMA_NEWER`].
pub fn run(config: &Config, keys: bool) -> anyhow::Result<i32> {
    println!("build {}", bluetti_controller::BUILD);
    println!(
        "config ok: {} station(s), keys {}",
        config.stations.len(),
        if keys { "ok" } else { "none" }
    );
    println!("database {}", config.storage.path.display());
    println!("health {}", health_url(config.web.listen));
    let (line, code) = verdict(&config.storage.path)?;
    println!("{line}");
    Ok(code)
}

/// Where this host reaches its own `/healthz`: the listen address, with an
/// unspecified IP (`0.0.0.0`, `::`) replaced by the loopback of its family.
fn health_url(listen: SocketAddr) -> String {
    let mut addr = listen;
    if addr.ip().is_unspecified() {
        addr.set_ip(match addr.ip() {
            IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
            IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
        });
    }
    format!("http://{addr}/healthz")
}

/// The schema line and exit code for the database at `path`.
fn verdict(path: &Path) -> anyhow::Result<(String, i32)> {
    Ok(match schema_version(path)? {
        None => (
            format!("schema: no database yet, this binary creates v{SCHEMA_VERSION}"),
            0,
        ),
        Some(db) if db > SCHEMA_VERSION => (
            format!("schema: database v{db} is newer than this binary's v{SCHEMA_VERSION}"),
            SCHEMA_NEWER,
        ),
        Some(db) => (
            format!("schema: database v{db}, binary v{SCHEMA_VERSION} (ok)"),
            0,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bluetti-check-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("data.db")
    }

    #[test]
    fn health_url_uses_loopback_for_an_unspecified_ip() {
        let url = |s: &str| health_url(s.parse().unwrap());
        assert_eq!(url("0.0.0.0:8080"), "http://127.0.0.1:8080/healthz");
        assert_eq!(url("[::]:9000"), "http://[::1]:9000/healthz");
        assert_eq!(url("192.168.1.5:8080"), "http://192.168.1.5:8080/healthz");
    }

    #[test]
    fn missing_current_and_newer_schema() {
        let path = temp("schema");
        assert_eq!(verdict(&path).unwrap().1, 0, "no file yet");
        assert!(!path.exists(), "check must not create the database");
        drop(bluetti_controller::store::open_writer(&path).unwrap());
        let (line, code) = verdict(&path).unwrap();
        assert_eq!(code, 0, "{line}");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", SCHEMA_VERSION + 1)
            .unwrap();
        drop(conn);
        let (line, code) = verdict(&path).unwrap();
        assert_eq!(code, SCHEMA_NEWER, "{line}");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
