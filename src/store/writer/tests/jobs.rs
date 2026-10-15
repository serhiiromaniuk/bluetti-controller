//! The §6 job triggers (amendment A7) and the late-sample check (A8).

use super::*;

const DAY: i64 = 86_400;

fn sample_ts(path: &Path) -> Vec<i64> {
    let conn = open_reader(path).unwrap();
    let mut stmt = conn.prepare("SELECT ts FROM sample ORDER BY ts").unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn wal_bytes(path: &Path) -> u64 {
    let mut wal = path.as_os_str().to_owned();
    wal.push("-wal");
    std::fs::metadata(wal).map_or(0, |m| m.len())
}

/// Retention runs on the first record of each UTC day, compaction on the first
/// of each week (epoch weeks), and neither runs again inside its period.
#[test]
fn retention_runs_daily_and_compaction_weekly() {
    let db = TempDb::new("writer-jobs");
    let s = stations(&db.path(), &[("a", true)]);
    let one_day = NonZeroU32::new(1).unwrap();
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        one_day,
        quiet(),
    )
    .unwrap();
    for ts in (0..=600).step_by(60) {
        writer.handle(&volts(&s[0], ts, ON)).unwrap();
    }
    assert!(wal_bytes(&db.path()) > 0);

    // Day 1: retention deletes the rolled-up rows older than one day (< 600).
    writer.handle(&volts(&s[0], DAY + 600, ON)).unwrap();
    assert_eq!(sample_ts(&db.path()), vec![600, DAY + 600]);
    // Same day: no second run, though 600 is past the cutoff now.
    writer.handle(&volts(&s[0], DAY + 1200, ON)).unwrap();
    assert_eq!(sample_ts(&db.path()), vec![600, DAY + 600, DAY + 1200]);
    assert!(wal_bytes(&db.path()) > 0, "no compaction inside week 0");

    // Week 1 (day 7): retention runs again and compaction empties the WAL.
    writer.handle(&volts(&s[0], 7 * DAY, ON)).unwrap();
    assert_eq!(sample_ts(&db.path()), vec![7 * DAY]);
    assert_eq!(wal_bytes(&db.path()), 0);
}

/// A sample in a bucket the rollup has passed is stored, and flagged.
#[test]
fn a_sample_behind_the_rollup_is_stored_and_flagged() {
    let db = TempDb::new("writer-late");
    let s = stations(&db.path(), &[("a", true)]);
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        RETENTION,
        quiet(),
    )
    .unwrap();
    for ts in (0..=600).step_by(60) {
        writer.handle(&volts(&s[0], ts, ON)).unwrap();
    }
    // Buckets 0 and 300 are rolled up; 600 is still open.
    assert_eq!(rolled_past(&writer.conn, s[0].id, 599).unwrap(), Some(300));
    assert_eq!(rolled_past(&writer.conn, s[0].id, 600).unwrap(), None);
    writer.handle(&volts(&s[0], 290, ON)).unwrap();
    assert!(sample_ts(&db.path()).contains(&290));
}
