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
