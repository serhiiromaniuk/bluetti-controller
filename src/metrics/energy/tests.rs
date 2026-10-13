use super::*;

const IV: i64 = 60;
const EPS: f64 = 1e-9;

fn close(got: f64, want: f64) {
    assert!((got - want).abs() < EPS, "got {got}, want {want}");
}

fn vals(pairs: &[(Metric, f64)]) -> BTreeMap<Metric, f64> {
    pairs.iter().copied().collect()
}

fn refs(owned: &[(i64, BTreeMap<Metric, f64>)]) -> Vec<(i64, &BTreeMap<Metric, f64>)> {
    owned.iter().map(|(ts, v)| (*ts, v)).collect()
}

#[test]
fn constant_100w_for_10_min_is_16_667_wh() {
    // 11 samples at 60 s: 10 segments × 100 W × 60 s = 60 000 Ws = 16.667 Wh.
    let owned: Vec<_> = (0..=10)
        .map(|i| (i * IV, vals(&[(Metric::AcInputPower, 100.0)])))
        .collect();
    let t = totals(&refs(&owned), IV, 1000.0, 0, 601);
    close(t.energy_in_wh, 100.0 * 600.0 / 3600.0);
    assert!((t.energy_in_wh - 16.667).abs() < 1e-3);
    close(t.energy_out_wh, 0.0);
    assert_eq!(t.missing_s, 0);
}

#[test]
fn ramp_0_to_120w_over_2_min_is_2_wh() {
    // (0+60)/2·60/3600 = 0.5 Wh, (60+120)/2·60/3600 = 1.5 Wh.
    let owned: Vec<_> = [0.0, 60.0, 120.0]
        .iter()
        .enumerate()
        .map(|(i, w)| (i as i64 * IV, vals(&[(Metric::AcOutputPower, *w)])))
        .collect();
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.energy_out_wh, 2.0);
    close(t.energy_in_wh, 0.0);
}

#[test]
fn ac_and_dc_add_up_and_missing_power_is_zero() {
    let owned = vec![
        (
            0,
            vals(&[(Metric::AcInputPower, 30.0), (Metric::DcInputPower, 30.0)]),
        ),
        // No power fields at all: counts as 0 W, so the segment is a 60→0 ramp.
        (60, vals(&[(Metric::BatterySoc, 50.0)])),
    ];
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.energy_in_wh, 30.0 * 60.0 / 3600.0);
}

#[test]
fn gap_over_3_intervals_adds_no_energy_and_reports_missing() {
    let w = || vals(&[(Metric::AcInputPower, 100.0)]);
    // 60→300 is 240 s > 180 s: skipped. 0→60 and 300→360 count; 360→540 is
    // exactly 3 intervals, so it counts too.
    let owned = vec![(0, w()), (60, w()), (300, w()), (360, w()), (540, w())];
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.energy_in_wh, 100.0 * (60.0 + 60.0 + 180.0) / 3600.0);
    assert_eq!(t.missing_s, 240);
}

#[test]
fn window_takes_segments_by_later_sample() {
    let w = || vals(&[(Metric::AcInputPower, 100.0)]);
    let owned = vec![(0, w()), (60, w()), (120, w())];
    // [60, 120): only the 0→60 segment (later sample 60) is in.
    let t = totals(&refs(&owned), IV, 1000.0, 60, 120);
    close(t.energy_in_wh, 100.0 * 60.0 / 3600.0);
    // [0, 60): no segment ends there; the lone first sample adds nothing.
    let t = totals(&refs(&owned), IV, 1000.0, 0, 60);
    close(t.energy_in_wh, 0.0);
}

#[test]
fn losses_are_in_minus_out_minus_stored() {
    // 1 h: in 100 W, out 50 W, SOC 50 → 52 % of 1000 Wh = +20 Wh stored.
    let owned: Vec<_> = (0..=60)
        .map(|i| {
            let soc = 50.0 + 2.0 * i as f64 / 60.0;
            let v = [
                (Metric::AcInputPower, 100.0),
                (Metric::DcOutputPower, 50.0),
                (Metric::BatterySoc, soc),
            ];
            (i * IV, vals(&v))
        })
        .collect();
    let t = totals(&refs(&owned), IV, 1000.0, 0, 3601);
    close(t.energy_in_wh, 100.0);
    close(t.energy_out_wh, 50.0);
    assert!((t.stored_delta_wh - 20.0).abs() < 1e-6);
    assert!((t.losses_estimate_wh - 30.0).abs() < 1e-6);
}

#[test]
fn soc_delta_skips_gap_segments() {
    let s = |soc| vals(&[(Metric::BatterySoc, soc)]);
    // 60→600 is a gap: its 10-point SOC drop is not counted; 0→60 (+1) and
    // 600→660 (−2) are.
    let owned = vec![(0, s(50.0)), (60, s(51.0)), (600, s(41.0)), (660, s(39.0))];
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.stored_delta_wh, -10.0);
    close(t.losses_estimate_wh, 10.0);
    assert_eq!(t.missing_s, 540);
}

#[test]
fn soc_carries_over_a_sample_without_soc() {
    let s = |soc: Option<f64>| soc.map_or_else(BTreeMap::new, |v| vals(&[(Metric::BatterySoc, v)]));
    // 89, None, 87 in one gap-free run: Δsoc = −2 → 1000 × −2 / 100 = −20 Wh.
    let owned = vec![(0, s(Some(89.0))), (60, s(None)), (120, s(Some(87.0)))];
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.stored_delta_wh, -20.0);
    // A gap resets the carry: 89 → gap → None → 87 counts nothing.
    let owned = vec![(0, s(Some(89.0))), (600, s(None)), (660, s(Some(87.0)))];
    let t = totals(&refs(&owned), IV, 1000.0, 0, 1000);
    close(t.stored_delta_wh, 0.0);
    assert_eq!(t.missing_s, 600);
}

#[test]
fn rollup_averages_min_and_count_over_bucket_samples() {
    let owned = vec![
        // Previous bucket: only its segment toward 300 matters, not its values.
        (
            240,
            vals(&[(Metric::BatterySoc, 99.0), (Metric::AcInputPower, 900.0)]),
        ),
        (
            300,
            vals(&[(Metric::BatterySoc, 50.0), (Metric::AcInputPower, 90.0)]),
        ),
        (
            360,
            vals(&[(Metric::BatterySoc, 48.0), (Metric::DcOutputPower, 30.0)]),
        ),
        (
            420,
            vals(&[(Metric::BatterySoc, 49.0), (Metric::AcInputPower, 0.0)]),
        ),
        // Next bucket: excluded.
        (600, vals(&[(Metric::BatterySoc, 1.0)])),
    ];
    let r = rollup(&refs(&owned), IV, 300).unwrap();
    assert_eq!(r.ts, 300);
    assert_eq!(r.n, 3);
    close(r.soc_avg.unwrap(), 49.0);
    close(r.soc_min.unwrap(), 48.0);
    close(r.ac_in_w_avg, 30.0);
    close(r.dc_in_w_avg, 0.0);
    close(r.ac_out_w_avg, 0.0);
    close(r.dc_out_w_avg, 10.0);
    // In: 240→300 (900+90)/2·60, 300→360 (90+0)/2·60, 360→420 0.
    close(r.energy_in_wh, (495.0 + 45.0) * 60.0 / 3600.0);
    // Out: 300→360 (0+30)/2·60, 360→420 (30+0)/2·60.
    close(r.energy_out_wh, 30.0 * 60.0 / 3600.0);
}

#[test]
fn segment_crossing_a_bucket_boundary_goes_to_later_bucket() {
    let w = || vals(&[(Metric::AcInputPower, 100.0)]);
    let owned = vec![(240, w()), (360, w())];
    let s = refs(&owned);
    // 240→360 crosses 300: all 120 s count toward the bucket of 360.
    close(rollup(&s, IV, 0).unwrap().energy_in_wh, 0.0);
    close(
        rollup(&s, IV, 300).unwrap().energy_in_wh,
        100.0 * 120.0 / 3600.0,
    );
    // Daily totals follow the same rule, so the buckets sum to the window.
    let t = totals(&s, IV, 1000.0, 0, 600);
    close(t.energy_in_wh, 100.0 * 120.0 / 3600.0);
}

#[test]
fn rollup_without_soc_or_samples() {
    let owned = vec![(0, vals(&[(Metric::AcInputPower, 10.0)]))];
    let r = rollup(&refs(&owned), IV, 0).unwrap();
    assert_eq!((r.soc_avg, r.soc_min, r.n), (None, None, 1));
    assert!(rollup(&refs(&owned), IV, 300).is_none());
}
