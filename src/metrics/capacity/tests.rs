use super::*;

/// A bucket at `ts` with `soc_avg`, input and output energy in Wh.
fn b(ts: i64, soc: f64, in_wh: f64, out_wh: f64) -> Bucket {
    Bucket {
        ts,
        soc_avg: Some(soc),
        energy_in_wh: Some(in_wh),
        energy_out_wh: Some(out_wh),
    }
}

/// Six consecutive buckets from ts 0, SOC 90 → 60, 10 Wh out each, no input.
fn six() -> Vec<Bucket> {
    [90.0, 85.0, 80.0, 75.0, 70.0, 60.0]
        .iter()
        .enumerate()
        .map(|(i, &soc)| b(i as i64 * BUCKET_S, soc, 0.0, 10.0))
        .collect()
}

fn capacities(buckets: &[Bucket]) -> Vec<f64> {
    segments(buckets).iter().map(|s| s.capacity_wh).collect()
}

#[test]
fn a_run_is_hand_computed() {
    // 10+20+20+20+10 = 80 Wh, minus half of each end bucket = 70 Wh over a
    // 12-point drop: 70 / 12 × 100 = 583.33 Wh.
    let buckets: Vec<_> = [
        (80.0, 10.0),
        (77.0, 20.0),
        (74.0, 20.0),
        (71.0, 20.0),
        (68.0, 10.0),
    ]
    .iter()
    .enumerate()
    .map(|(i, &(soc, out))| b(i as i64 * BUCKET_S, soc, 0.0, out))
    .collect();
    let s = segments(&buckets);
    assert_eq!(s.len(), 1);
    assert_eq!((s[0].start, s[0].end), (0, 1200));
    assert_eq!((s[0].soc_from, s[0].soc_to), (80.0, 68.0));
    assert_eq!(s[0].energy_wh, 70.0);
    assert!((s[0].capacity_wh - 583.333).abs() < 0.001, "{s:?}");
}

#[test]
fn one_unbroken_run_is_one_segment() {
    // 60 − 10 = 50 Wh over 30 points.
    let caps = capacities(&six());
    assert_eq!(caps.len(), 1);
    assert!((caps[0] - 166.667).abs() < 0.001, "{caps:?}");
}

#[test]
fn a_missing_bucket_splits_the_run() {
    // Bucket 3 missing: [90, 85, 80] and [70, 60] → two 10-point drops, 20 and
    // 10 Wh → 200 and 100 Wh.
    let mut buckets = six();
    buckets.remove(3);
    assert_eq!(capacities(&buckets), [200.0, 100.0]);
}

#[test]
fn an_input_or_an_unmetered_bucket_splits_the_run() {
    for broken in [b(900, 75.0, 5.0, 10.0), b(900, 75.0, 0.0, 0.0)] {
        let mut buckets = six();
        buckets[3] = broken;
        assert_eq!(capacities(&buckets), [200.0, 100.0], "{broken:?}");
    }
    let mut buckets = six();
    buckets[3].soc_avg = None;
    assert_eq!(capacities(&buckets), [200.0, 100.0]);
}

#[test]
fn a_drop_below_ten_points_is_no_segment() {
    let buckets = [
        b(0, 50.0, 0.0, 10.0),
        b(300, 45.0, 0.0, 10.0),
        b(600, 40.1, 0.0, 10.0),
    ];
    assert!(segments(&buckets).is_empty());
    assert!(segments(&[]).is_empty());
}

fn seg(capacity_wh: f64) -> Segment {
    Segment {
        start: 0,
        end: 0,
        soc_from: 0.0,
        soc_to: 0.0,
        energy_wh: 0.0,
        capacity_wh,
    }
}

#[test]
fn the_estimate_is_the_median_of_the_newest_five() {
    assert_eq!(estimate(&[], 1000.0), None);
    let odd = estimate(&[seg(900.0), seg(700.0), seg(800.0)], 1000.0).unwrap();
    assert_eq!((odd.median_wh, odd.factor, odd.n), (800.0, 0.8, 3));
    let even = estimate(&[seg(900.0), seg(700.0)], 1000.0).unwrap();
    assert_eq!((even.median_wh, even.n), (800.0, 2));
    // The oldest two (100, 100) fall out: median of 500, 600, 700, 800, 900.
    let caps = [100.0, 100.0, 900.0, 500.0, 800.0, 600.0, 700.0];
    let window = estimate(&caps.map(seg), 1000.0).unwrap();
    assert_eq!((window.median_wh, window.n), (700.0, 5));
}
