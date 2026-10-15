//! The published updates (§8.4 g, phase-5 step 4).

use super::*;

/// §8.4 g: every committed `sample`, `grid_event` and `poll_failure` row is
/// published once, in commit order; a snapshot is not. A dropped receiver never
/// fails a record.
#[test]
fn publishes_one_update_per_committed_row() {
    let db = TempDb::new("writer-updates");
    let s = stations(&db.path(), &[("a", true)]);
    let (tx, mut rx) = broadcast::channel(16);
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        RETENTION,
        tx,
    )
    .unwrap();
    let id = s[0].id;
    let mut handle = |record: Record| {
        writer.handle(&record).unwrap();
        std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>()
    };
    let sample = |ts, v| Update::Sample {
        station: id,
        ts,
        core: BTreeMap::from([(Metric::AcInputVoltage, v)]),
    };
    let event = |scope: &str, ended_at| Update::GridEvent {
        scope: scope.into(),
        started_at: 120,
        ended_at,
    };

    // The extra (`pack_voltage`) is stored but not published: `core` only.
    let mut first = volts(&s[0], 0, ON);
    if let Record::Sample { sample, .. } = &mut first {
        sample.values.insert(Metric::PackVoltage, 51.2);
    }
    assert_eq!(handle(first), vec![sample(0, ON)]);
    assert_eq!(handle(volts(&s[0], 60, ON)), vec![sample(60, ON)]);
    assert_eq!(handle(volts(&s[0], 120, OFF)), vec![sample(120, OFF)]);
    // Debounce 2: the second OFF opens both events, backdated to 120.
    assert_eq!(
        handle(volts(&s[0], 180, OFF)),
        vec![sample(180, OFF), event("a", None), event("house", None)]
    );
    assert_eq!(handle(volts(&s[0], 240, ON)), vec![sample(240, ON)]);
    assert_eq!(
        handle(volts(&s[0], 300, ON)),
        vec![
            sample(300, ON),
            event("a", Some(240)),
            event("house", Some(240))
        ]
    );
    assert_eq!(
        handle(failed(&s[0], 360)),
        vec![Update::PollFailure {
            station: id,
            ts: 360,
            stage: Stage::Connect
        }]
    );
    let snapshot = Record::Snapshot {
        station: id,
        ts: 360,
        start_reg: 10,
        values: vec![1],
    };
    assert_eq!(handle(snapshot), vec![]);

    drop(handle);
    drop(rx);
    writer.handle(&volts(&s[0], 420, ON)).unwrap();
    let samples: i64 = open_reader(&db.path())
        .unwrap()
        .query_row("SELECT count(*) FROM sample", [], |r| r.get(0))
        .unwrap();
    assert_eq!(samples, 7);
}

/// A record whose insert fails publishes nothing.
#[test]
fn a_failed_insert_publishes_nothing() {
    let db = TempDb::new("writer-updates-failed");
    let s = stations(&db.path(), &[("a", true)]);
    let (tx, mut rx) = broadcast::channel(16);
    let mut writer = Writer::restore(
        open_writer(&db.path()).unwrap(),
        s.clone(),
        params(),
        RETENTION,
        tx,
    )
    .unwrap();
    for table in ["sample", "poll_failure"] {
        writer
            .conn
            .execute_batch(&format!(
                "CREATE TEMP TRIGGER fail_{table} BEFORE INSERT ON {table}
                 BEGIN SELECT RAISE(ABORT, 'injected'); END;"
            ))
            .unwrap();
    }
    assert!(writer.handle(&volts(&s[0], 0, ON)).is_err());
    assert!(writer.handle(&failed(&s[0], 60)).is_err());
    assert!(rx.try_recv().is_err());
}
