//! The schedule records (R3 slice 1, ROADMAP.md §6 "the schedule table").

use super::*;
use crate::protocol::control::Output;
use crate::store::read::{ScheduleEntry, schedule_entries};

fn writer(path: &Path, s: &[Station]) -> Writer {
    Writer::restore(
        open_writer(path).unwrap(),
        s.to_vec(),
        params(),
        RETENTION,
        quiet(),
    )
    .unwrap()
}

fn set(station: &Station, on_min: Option<u16>, off_min: Option<u16>) -> Record {
    Record::ScheduleSet {
        station: station.id,
        output: Output::Ac,
        on_min,
        off_min,
    }
}

fn entries(path: &Path) -> Vec<ScheduleEntry> {
    schedule_entries(&open_reader(path).unwrap()).unwrap()
}

#[test]
fn set_run_update_and_delete() {
    let db = TempDb::new("writer-schedule");
    let s = stations(&db.path(), &[("a", true)]);
    let mut w = writer(&db.path(), &s);
    let entry = |on_min, off_min, last_due| ScheduleEntry {
        station_id: s[0].id,
        output: Output::Ac,
        on_min,
        off_min,
        last_due,
    };

    w.handle(&set(&s[0], Some(420), Some(1380))).unwrap();
    assert_eq!(
        entries(&db.path()),
        vec![entry(Some(420), Some(1380), None)]
    );

    let run = Record::ScheduleRun {
        station: s[0].id,
        output: Output::Ac,
        due: 1_000,
    };
    w.handle(&run).unwrap();
    assert_eq!(
        entries(&db.path()),
        vec![entry(Some(420), Some(1380), Some(1_000))]
    );

    // An edit replaces both times and keeps `last_due`.
    w.handle(&set(&s[0], None, Some(1320))).unwrap();
    assert_eq!(
        entries(&db.path()),
        vec![entry(None, Some(1320), Some(1_000))]
    );

    let delete = Record::ScheduleDelete {
        station: s[0].id,
        output: Output::Ac,
    };
    w.handle(&delete).unwrap();
    assert_eq!(entries(&db.path()), vec![]);

    // A run after the delete is a no-op: no row comes back.
    w.handle(&run).unwrap();
    assert_eq!(entries(&db.path()), vec![]);
    // Schedule records are not samples: no grid event, no job side effect.
    assert_eq!(events(&db.path()), vec![]);
}

#[test]
fn entries_are_per_output_and_need_a_time() {
    let db = TempDb::new("writer-schedule-outputs");
    let s = stations(&db.path(), &[("a", true), ("b", false)]);
    let mut w = writer(&db.path(), &s);
    w.handle(&set(&s[1], Some(0), None)).unwrap();
    w.handle(&Record::ScheduleSet {
        station: s[0].id,
        output: Output::Dc,
        on_min: None,
        off_min: Some(1439),
    })
    .unwrap();
    let got: Vec<_> = entries(&db.path())
        .into_iter()
        .map(|e| (e.station_id, e.output))
        .collect();
    assert_eq!(got, vec![(s[0].id, Output::Dc), (s[1].id, Output::Ac)]);

    // The table's CHECK refuses an entry with neither time.
    assert!(w.handle(&set(&s[0], None, None)).is_err());
    // An unknown station is refused before any SQL.
    let err = w
        .handle(&Record::ScheduleDelete {
            station: 999,
            output: Output::Ac,
        })
        .unwrap_err();
    assert!(err.to_string().contains("not configured"), "{err}");
}
