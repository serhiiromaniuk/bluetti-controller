use super::*;
use crate::store::temp_db::TempDb;

fn user_version(conn: &Connection) -> u32 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

#[test]
fn fresh_db_gets_every_table_and_version_1() {
    let db = TempDb::new("fresh");
    let conn = open_writer(&db.path()).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .unwrap();
    let tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        tables,
        [
            "grid_event",
            "poll_failure",
            "register_snapshot",
            "rollup_5m",
            "sample",
            "sample_extra",
            "station",
        ]
    );
    assert_eq!(user_version(&conn), 1);
    let mode: String = conn
        .pragma_query_value(None, "journal_mode", |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    let sync: i64 = conn
        .pragma_query_value(None, "synchronous", |row| row.get(0))
        .unwrap();
    assert_eq!(sync, 1, "synchronous=NORMAL");
}

#[test]
fn reopening_runs_no_migration_twice() {
    let db = TempDb::new("reopen");
    let conn = open_writer(&db.path()).unwrap();
    conn.execute(
        "INSERT INTO station (name, model, capacity_wh) VALUES ('router', 'EB3A', 268.8)",
        [],
    )
    .unwrap();
    drop(conn);
    // A second run of 001 would fail on `CREATE TABLE station` and lose nothing silently.
    let conn = open_writer(&db.path()).unwrap();
    assert_eq!(user_version(&conn), 1);
    let rows: i64 = conn
        .query_row("SELECT count(*) FROM station", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 1);
}

#[test]
fn reader_refuses_an_insert() {
    let db = TempDb::new("reader");
    let _writer = open_writer(&db.path()).unwrap();
    let reader = open_reader(&db.path()).unwrap();
    let insert = reader.execute(
        "INSERT INTO station (name, model, capacity_wh) VALUES ('router', 'EB3A', 268.8)",
        [],
    );
    assert!(insert.is_err(), "read-only connection accepted an INSERT");
    let rows: i64 = reader
        .query_row("SELECT count(*) FROM station", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows, 0);
}

#[test]
fn newer_schema_than_the_binary_is_refused() {
    let db = TempDb::new("newer");
    let conn = open_writer(&db.path()).unwrap();
    conn.pragma_update(None, "user_version", MIGRATIONS.len() as u32 + 1)
        .unwrap();
    drop(conn);
    let err = open_writer(&db.path()).unwrap_err();
    assert!(err.to_string().contains("newer than this binary"), "{err}");
}
