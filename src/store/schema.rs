//! Opening the database: the §6 pragmas, and the embedded migrations applied in
//! order, versioned by `PRAGMA user_version` (ROADMAP.md §6 "M3 decisions").

use std::path::Path;
use std::time::Duration;

use anyhow::{Context, bail, ensure};
use rusqlite::{Connection, OpenFlags};

/// Embedded migrations: entry `i` moves the schema from version `i` to `i + 1`.
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/001_init.sql")];

const BUSY_TIMEOUT: Duration = Duration::from_millis(5000);

/// Opens (or creates) the database for the single writer: WAL, `synchronous=NORMAL`,
/// `busy_timeout=5000`, then applies every migration the file has not seen yet.
pub fn open_writer(path: &Path) -> anyhow::Result<Connection> {
    let mut conn =
        Connection::open(path).with_context(|| format!("open database {}", path.display()))?;
    let mode: String =
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
    ensure!(
        mode.eq_ignore_ascii_case("wal"),
        "journal_mode is {mode}, not WAL"
    );
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    migrate(&mut conn)?;
    Ok(conn)
}

/// Opens an existing database read-only; any write through it fails.
pub fn open_reader(path: &Path) -> anyhow::Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags)
        .with_context(|| format!("open database {} read-only", path.display()))?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
    Ok(conn)
}

/// Applies the migrations past the file's `user_version`, one transaction each.
/// Crate-visible so tests can migrate an in-memory database (no WAL there).
pub(crate) fn migrate(conn: &mut Connection) -> anyhow::Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let version = version as usize;
    if version > MIGRATIONS.len() {
        bail!(
            "database schema version {version} is newer than this binary's {}",
            MIGRATIONS.len()
        );
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)
            .with_context(|| format!("migration {:03}", index + 1))?;
        tx.pragma_update(None, "user_version", (index + 1) as u32)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
