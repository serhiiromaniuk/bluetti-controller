use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::*;

// Hand-computed: 2026-01-10T00:00 EET (+02) = 2026-01-09T22:00Z
// = 1767225600 (2026-01-01T00:00Z) + 8 d + 22 h.
const MIDNIGHT: i64 = 1_767_225_600 + 8 * 86_400 + 22 * 3600;
const H: i64 = 3600;
const M: i64 = 60;

fn kyiv(ts: i64) -> Zoned {
    Timestamp::from_second(ts)
        .unwrap()
        .to_zoned(TimeZone::get("Europe/Kyiv").unwrap())
}

fn entry(on: Option<u16>, off: Option<u16>, last_due: Option<i64>) -> ScheduleEntry {
    ScheduleEntry {
        station_id: 1,
        output: Output::Ac,
        on_min: on,
        off_min: off,
        last_due,
    }
}

fn slot(on: bool, due_ts: i64) -> Due {
    Due {
        station_id: 1,
        output: Output::Ac,
        on,
        due_ts,
    }
}

const ON_7: Option<u16> = Some(7 * 60);
const OFF_23: Option<u16> = Some(23 * 60);

#[test]
fn midnight_constant_is_local_midnight() {
    assert_eq!(
        kyiv(MIDNIGHT).to_string(),
        "2026-01-10T00:00:00+02:00[Europe/Kyiv]"
    );
}

#[test]
fn due_exactly_at_off() {
    let entries = [entry(ON_7, OFF_23, Some(MIDNIGHT + 7 * H))];
    let now = kyiv(MIDNIGHT + 23 * H);
    assert_eq!(
        due(&entries, &now).unwrap(),
        [slot(false, MIDNIGHT + 23 * H)]
    );
    // One second earlier the 07:00 `on` is still the latest slot, already run.
    assert_eq!(due(&entries, &kyiv(MIDNIGHT + 23 * H - 1)).unwrap(), []);
}

#[test]
fn restart_after_a_missed_off_is_due_once() {
    let now = kyiv(MIDNIGHT + 23 * H + 30 * M);
    let yesterday_off = MIDNIGHT - H;
    let missed = [entry(None, OFF_23, Some(yesterday_off))];
    assert_eq!(
        due(&missed, &now).unwrap(),
        [slot(false, MIDNIGHT + 23 * H)]
    );
    let recorded = [entry(None, OFF_23, Some(MIDNIGHT + 23 * H))];
    assert_eq!(due(&recorded, &now).unwrap(), []);
}

#[test]
fn restart_after_a_recorded_run_is_not_due() {
    let entries = [entry(ON_7, OFF_23, Some(MIDNIGHT + 23 * H))];
    assert_eq!(due(&entries, &kyiv(MIDNIGHT + 23 * H + 5 * M)).unwrap(), []);
}

#[test]
fn both_missed_applies_only_the_latest() {
    let entries = [entry(ON_7, Some(22 * 60), Some(MIDNIGHT - 2 * H))];
    let now = kyiv(MIDNIGHT + 23 * H);
    assert_eq!(
        due(&entries, &now).unwrap(),
        [slot(false, MIDNIGHT + 22 * H)]
    );
}

#[test]
fn off_moved_earlier_is_due_at_once_when_past() {
    let now = kyiv(MIDNIGHT + 22 * H + 30 * M);
    let ran_on = Some(MIDNIGHT + 7 * H);
    assert_eq!(due(&[entry(ON_7, OFF_23, ran_on)], &now).unwrap(), []);
    let moved = [entry(ON_7, Some(22 * 60), ran_on)];
    assert_eq!(due(&moved, &now).unwrap(), [slot(false, MIDNIGHT + 22 * H)]);
}

#[test]
fn never_run_applies_yesterdays_latest_before_todays_times() {
    let current = ScheduleEntry {
        station_id: 2,
        ..entry(ON_7, OFF_23, Some(MIDNIGHT - H))
    };
    let entries = [entry(ON_7, OFF_23, None), current];
    let now = kyiv(MIDNIGHT + 6 * H);
    assert_eq!(due(&entries, &now).unwrap(), [slot(false, MIDNIGHT - H)]);
}

#[test]
fn a_time_in_the_dst_gap_resolves_after_it() {
    // Kyiv 2026-03-29: 03:00 EET jumps to 04:00 EEST, so 03:30 does not exist;
    // compatible disambiguation gives 04:30 EEST = 01:30Z.
    // 2026-03-29T00:00Z = 1767225600 + 87 d.
    let mar_29_utc = 1_767_225_600 + 87 * 86_400;
    let entries = [entry(None, Some(3 * 60 + 30), None)];
    let now = kyiv(mar_29_utc + 2 * H);
    assert_eq!(
        due(&entries, &now).unwrap(),
        [slot(false, mar_29_utc + H + 30 * M)]
    );
    // At 01:00Z (04:00 EEST) the gap slot is still ahead: yesterday's 03:30 EET.
    let before = kyiv(mar_29_utc + H);
    let yesterday = mar_29_utc - 86_400 + H + 30 * M;
    assert_eq!(due(&entries, &before).unwrap(), [slot(false, yesterday)]);
}

#[test]
fn a_time_past_23_59_is_an_error() {
    let entries = [entry(Some(24 * 60), None, None)];
    assert!(due(&entries, &kyiv(MIDNIGHT)).is_err());
}

// ---- the scheduler task (step 13b) ----

mod task {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use tokio::sync::{mpsc, watch};

    use super::*;
    use crate::collector::Target;
    use crate::config::{ControlConfig, StationConfig};
    use crate::control::{PAUSED, Request};
    use crate::protocol::device::Model;
    use crate::store::temp_db::TempDb;
    use crate::store::{Record, open_reader, open_writer, sync_stations};

    type Sent = Arc<Mutex<Vec<(String, Output, bool)>>>;

    /// A scheduler over a temp db holding `router` (EB3A) with one `ac` entry
    /// `on 07:00`, `off 23:00`, never run; a fake collector that logs every
    /// request and answers from `answers`, then `Ok(read-back)`.
    struct Rig {
        scheduler: Scheduler,
        sent: Sent,
        records: mpsc::Receiver<Record>,
        _records_tx: mpsc::Sender<Record>,
        id: i64,
        _db: TempDb,
    }

    fn rig(name: &str, enabled: bool, answers: Vec<Result<bool, String>>) -> Rig {
        let db = TempDb::new(name);
        let mut conn = open_writer(&db.path()).unwrap();
        let config = StationConfig {
            name: "router".into(),
            model: Model::Eb3a,
            address: "AA:BB:CC:00:00:00".into(),
            grid_sensor: false,
        };
        let id = sync_stations(&mut conn, std::slice::from_ref(&config)).unwrap()["router"];
        conn.execute(
            "INSERT INTO schedule (station_id, output, on_min, off_min) VALUES (?1, 'ac', 420, 1380)",
            [id],
        )
        .unwrap();
        drop(conn);
        let (commands, requests) = mpsc::channel(8);
        let (records_tx, records) = mpsc::channel(64);
        let sent = fake_collector(requests, answers);
        let scheduler = Scheduler::new(
            open_reader(&db.path()).unwrap(),
            vec![Target { id, config }],
            ControlConfig { enabled },
            commands,
            records_tx.downgrade(),
        );
        Rig {
            scheduler,
            sent,
            records,
            _records_tx: records_tx,
            id,
            _db: db,
        }
    }

    fn fake_collector(
        mut requests: mpsc::Receiver<Request>,
        answers: Vec<Result<bool, String>>,
    ) -> Sent {
        let sent = Sent::default();
        let log = Arc::clone(&sent);
        let mut answers = VecDeque::from(answers);
        tokio::spawn(async move {
            while let Some(Request { command, reply }) = requests.recv().await {
                let entry = (command.station().to_owned(), command.output(), command.on());
                log.lock().unwrap().push(entry);
                let _ = reply.send(answers.pop_front().unwrap_or(Ok(command.on())));
            }
        });
        sent
    }

    fn drain(records: &mut mpsc::Receiver<Record>) -> Vec<Record> {
        std::iter::from_fn(|| records.try_recv().ok()).collect()
    }

    fn sent(rig: &Rig) -> Vec<(String, Output, bool)> {
        rig.sent.lock().unwrap().clone()
    }

    const AT_23: i64 = MIDNIGHT + 23 * H;

    #[tokio::test]
    async fn a_due_entry_sends_one_request_and_one_run_record() {
        let mut rig = rig("sched-due", true, vec![]);
        let now = kyiv(AT_23 + 10);
        assert!(rig.scheduler.tick(&now).await);
        assert_eq!(sent(&rig), [("router".to_owned(), Output::Ac, false)]);
        let run = Record::ScheduleRun {
            station: rig.id,
            output: Output::Ac,
            due: AT_23,
        };
        assert_eq!(drain(&mut rig.records), [run]);
        // The writer never applied the UPDATE here: the in-memory success
        // still stops a repeat on the next tick.
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 70)).await);
        assert_eq!(sent(&rig).len(), 1);
        assert_eq!(drain(&mut rig.records), []);
    }

    #[tokio::test]
    async fn a_deleted_and_re_added_entry_does_not_inherit_the_old_slot() {
        let mut rig = rig("sched-readd", true, vec![]);
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 10)).await);
        // The writer stores the run; the next tick reads it back.
        let conn = open_writer(&rig._db.path()).unwrap();
        let set = |sql: &str| conn.execute(sql, [rig.id]).unwrap();
        set(&format!(
            "UPDATE schedule SET last_due = {AT_23} WHERE station_id = ?1"
        ));
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 70)).await);
        // Deleted and re-added as `on 08:00`: never run, so today's 08:00 is due.
        set("DELETE FROM schedule WHERE station_id = ?1");
        set("INSERT INTO schedule (station_id, output, on_min) VALUES (?1, 'ac', 480)");
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 130)).await);
        let router = |on| ("router".to_owned(), Output::Ac, on);
        assert_eq!(sent(&rig), [router(false), router(true)]);
        let run = |due| Record::ScheduleRun {
            station: rig.id,
            output: Output::Ac,
            due,
        };
        assert_eq!(drain(&mut rig.records), [run(AT_23), run(MIDNIGHT + 8 * H)]);
    }

    #[tokio::test]
    async fn a_failed_run_is_retried_next_tick_and_not_after_a_newer_slot() {
        let fail = || Err("no link".to_owned());
        let mut rig = rig("sched-retry", true, vec![fail(), fail()]);
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 10)).await);
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 70)).await);
        assert_eq!(drain(&mut rig.records), []);
        // Next day 07:10: the `on` slot supersedes the failed `off`.
        let next_on = MIDNIGHT + 31 * H;
        assert!(rig.scheduler.tick(&kyiv(next_on + 10 * M)).await);
        let router = |on| ("router".to_owned(), Output::Ac, on);
        assert_eq!(sent(&rig), [router(false), router(false), router(true)]);
        let run = Record::ScheduleRun {
            station: rig.id,
            output: Output::Ac,
            due: next_on,
        };
        assert_eq!(drain(&mut rig.records), [run]);
    }

    #[tokio::test]
    async fn pauses_do_not_count_and_the_11th_failure_sends_nothing() {
        let mut answers = vec![Err(PAUSED.to_owned()); 3];
        answers.extend(vec![Err("no link".to_owned()); MAX_FAILURES as usize]);
        let mut rig = rig("sched-cap", true, answers);
        for minute in 0..15 {
            assert!(rig.scheduler.tick(&kyiv(AT_23 + minute * M)).await);
        }
        assert_eq!(sent(&rig).len(), 3 + MAX_FAILURES as usize);
        assert_eq!(drain(&mut rig.records), []);
    }

    #[tokio::test]
    async fn disabled_control_sends_nothing() {
        let mut rig = rig("sched-off", false, vec![]);
        assert!(rig.scheduler.tick(&kyiv(AT_23 + 10)).await);
        assert_eq!(sent(&rig), []);
        assert_eq!(drain(&mut rig.records), []);
    }

    #[tokio::test]
    async fn run_ticks_at_once_and_stops_on_shutdown() {
        let rig = rig("sched-run", true, vec![]);
        let sent = Arc::clone(&rig.sent);
        let (stop, shutdown) = watch::channel(false);
        let tz = TimeZone::get("Europe/Kyiv").unwrap();
        let task = tokio::spawn(run(rig.scheduler, tz, shutdown));
        // An `on`/`off` entry never run is always due: the first tick sends.
        let first = async {
            while sent.lock().unwrap().is_empty() {
                tokio::task::yield_now().await;
            }
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), first)
            .await
            .expect("the first tick sends at once");
        stop.send_replace(true);
        tokio::time::timeout(std::time::Duration::from_secs(5), task)
            .await
            .expect("run stops on shutdown")
            .unwrap();
        assert_eq!(sent.lock().unwrap().len(), 1);
    }
}
