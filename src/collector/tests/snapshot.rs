//! Hourly register snapshots (§6, step 11).

use super::*;

/// (station id, ts, start register, values) per snapshot record.
fn snapshots(records: &[Record]) -> Vec<(i64, i64, u16, Vec<u16>)> {
    records
        .iter()
        .filter_map(|record| match record {
            Record::Snapshot {
                station,
                ts,
                start_reg,
                values,
            } => Some((*station, *ts, *start_reg, values.clone())),
            _ => None,
        })
        .collect()
}

#[tokio::test(start_paused = true)]
async fn the_first_successful_poll_of_each_utc_hour_sends_a_snapshot_set() {
    let (collector, _state, mut rx) = collector(&[("a", Behave::Fill(0))]);
    let (_tx, control) = watch::channel(Control::Run);
    // 62 polls over 3660 s: at least two UTC hours, at least one with two polls.
    run_for(collector, control, 3700).await;
    let records = drain(&mut rx);
    let polls = summary(&records);
    assert_eq!(polls.len(), 62);
    let mut firsts: Vec<i64> = Vec::new();
    for &(_, ts, _) in &polls {
        if firsts
            .last()
            .is_none_or(|first| first.div_euclid(3600) != ts.div_euclid(3600))
        {
            firsts.push(ts);
        }
    }
    assert!(firsts.len() >= 2, "{firsts:?}");
    // One set per hour, at its first poll: every EB3A block, as read.
    let expected: Vec<_> = firsts
        .iter()
        .flat_map(|&ts| [(1, ts, 10, vec![0; 40]), (1, ts, 70, vec![0; 21])])
        .collect();
    assert_eq!(snapshots(&records), expected);
}

#[tokio::test(start_paused = true)]
async fn a_failed_poll_sends_no_snapshot() {
    let (collector, _state, mut rx) = collector(&[
        ("a", Behave::Fail(Stage::Connect)),
        ("b", Behave::Silent),
        ("c", Behave::Fill(0xFFFF)),
    ]);
    let (_tx, control) = watch::channel(Control::Run);
    run_for(collector, control, 130).await;
    let records = drain(&mut rx);
    assert_eq!(summary(&records).len(), 9);
    assert!(snapshots(&records).is_empty());
}

#[test]
fn a_new_hour_is_due_once() {
    let mut last = None;
    let due: Vec<_> = [3599, 3600, 3660, 7199, 7200, 7200]
        .into_iter()
        .map(|ts| new_hour(&mut last, ts))
        .collect();
    assert_eq!(due, [true, true, false, false, true, false]);
}
