use super::*;

const TOL: f64 = 1e-3;

fn approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < TOL,
        "{actual} != {expected} (±{TOL})"
    );
}

/// One sample: SOC (if any), AC output and DC output watts.
fn sample(soc: Option<f64>, ac: f64, dc: f64) -> BTreeMap<Metric, f64> {
    let mut m = BTreeMap::from([(Metric::AcOutputPower, ac), (Metric::DcOutputPower, dc)]);
    if let Some(soc) = soc {
        m.insert(Metric::BatterySoc, soc);
    }
    m
}

fn view(rows: &[(i64, BTreeMap<Metric, f64>)]) -> Vec<(i64, &BTreeMap<Metric, f64>)> {
    rows.iter().map(|(ts, v)| (*ts, v)).collect()
}

#[test]
fn ewma_over_irregular_intervals_on_battery() {
    // α₁ = 1 − e^(−600/600) = 0.632121 → 100 + α₁·(200 − 100) = 163.2121
    // α₂ = 1 − e^(−300/600) = 0.393469 → 163.2121 + α₂·(50 − 163.2121) = 118.6666
    // remaining = 1000 × 50 / 100 × 0.85 = 425 Wh; runtime = 425 / 118.6666 = 3.58146 h
    // until = 900 + 3.58146 × 3600 = 13793.27 → 13793
    let rows = vec![
        (0, sample(Some(50.0), 100.0, 0.0)),
        (600, sample(None, 150.0, 50.0)),
        (900, sample(Some(50.0), 0.0, 50.0)),
    ];
    let got = forecast(&view(&rows), 1000.0, 0.85, GridState::Absent, None);
    let Forecast::OnBattery {
        load_w,
        remaining_wh,
        runtime_h,
        until_ts,
    } = got
    else {
        panic!("expected OnBattery, got {got:?}");
    };
    approx(load_w, 118.66658);
    approx(remaining_wh, 425.0);
    approx(runtime_h, 3.581463);
    assert_eq!(until_ts, 13793);
}

#[test]
fn on_grid_is_if_the_power_goes_now() {
    // Constant 100 W: EWMA = 100; remaining = 2000 × 80 / 100 × 0.85 = 1360 Wh → 13.6 h
    let rows = vec![
        (0, sample(Some(80.0), 60.0, 40.0)),
        (60, sample(Some(80.0), 100.0, 0.0)),
    ];
    let got = forecast(&view(&rows), 2000.0, 0.85, GridState::Present, None);
    let Forecast::OnGrid {
        load_w,
        remaining_wh,
        runtime_h,
    } = got
    else {
        panic!("expected OnGrid, got {got:?}");
    };
    approx(load_w, 100.0);
    approx(remaining_wh, 1360.0);
    approx(runtime_h, 13.6);
}

#[test]
fn tiny_nonzero_load_uses_the_one_watt_floor() {
    // 0.5 W read → max(0.5, 1) = 1 W; remaining = 100 × 10 / 100 × 1.0 = 10 Wh → 10 h
    let rows = vec![(0, sample(Some(10.0), 0.0, 0.5))];
    let got = forecast(&view(&rows), 100.0, 1.0, GridState::Present, None);
    let Forecast::OnGrid {
        load_w, runtime_h, ..
    } = got
    else {
        panic!("expected OnGrid, got {got:?}");
    };
    approx(load_w, 1.0);
    approx(runtime_h, 10.0);
}

#[test]
fn zero_watt_outputs_take_the_load_from_the_soc_slope() {
    // EB3A outage: 268.8 Wh × (89 − 87) / 100 = 5.376 Wh over 38 min = 0.63333 h → 8.48842 W
    // remaining = 268.8 × 87 / 100 × 0.85 = 198.7776 Wh → 23.4175 h; until = 2280 + 84303 = 86583
    // Δt starts at the observed fall 90 → 89 (t=0), not at the 90 plateau.
    let rows = vec![
        (-600, sample(Some(90.0), 0.0, 0.0)),
        (0, sample(Some(89.0), 0.0, 0.0)),
        (600, sample(Some(89.0), 0.0, 0.0)),
        (1140, sample(Some(88.0), 0.0, 0.0)),
        (1800, sample(None, 0.0, 0.0)),
        (2280, sample(Some(87.0), 0.0, 0.0)),
    ];
    let got = forecast(&view(&rows), 268.8, 0.85, GridState::Absent, None);
    let Forecast::OnBattery {
        load_w,
        remaining_wh,
        runtime_h,
        until_ts,
    } = got
    else {
        panic!("expected OnBattery, got {got:?}");
    };
    approx(load_w, 8.488421);
    approx(remaining_wh, 198.7776);
    approx(runtime_h, 23.4175);
    assert_eq!(until_ts, 86583);
}

#[test]
fn soc_slope_ends_at_the_first_sample_of_the_current_level() {
    // Fall to 90 at t=0, 88 first at t=1200, still 88 at t=1800: slope over 1200 s, not 1800 s.
    // 1000 × 2 / 100 / (1200 / 3600) = 60 W
    let rows = vec![
        (-600, sample(Some(91.0), 0.0, 0.0)),
        (0, sample(Some(90.0), 0.0, 0.0)),
        (1200, sample(Some(88.0), 0.0, 0.0)),
        (1800, sample(Some(88.0), 0.0, 0.0)),
    ];
    let got = forecast(&view(&rows), 1000.0, 1.0, GridState::Present, None);
    let Forecast::OnGrid { load_w, .. } = got else {
        panic!("expected OnGrid, got {got:?}");
    };
    approx(load_w, 60.0);
}

#[test]
fn idle_on_grid_plateau_does_not_dilute_the_soc_slope() {
    // 10 h flat at 90 on grid, then the outage: 89 at 36060, 87 first at 38340.
    // Δt = 2280 s (not 38340 s): 268.8 × 2 / 100 / 0.63333 h = 8.48842 W
    let rows = vec![
        (0, sample(Some(90.0), 0.0, 0.0)),
        (36000, sample(Some(90.0), 0.0, 0.0)),
        (36060, sample(Some(89.0), 0.0, 0.0)),
        (36660, sample(Some(89.0), 0.0, 0.0)),
        (37200, sample(Some(88.0), 0.0, 0.0)),
        (38340, sample(Some(87.0), 0.0, 0.0)),
    ];
    let got = forecast(&view(&rows), 268.8, 0.85, GridState::Absent, None);
    let Forecast::OnBattery { load_w, .. } = got else {
        panic!("expected OnBattery, got {got:?}");
    };
    approx(load_w, 8.488421);
}

#[test]
fn fewer_than_two_soc_steps_with_zero_watts_is_unknown() {
    let rows = vec![
        (0, sample(Some(89.0), 0.0, 0.0)),
        (1140, sample(Some(88.0), 0.0, 0.0)),
    ];
    assert_eq!(
        forecast(&view(&rows), 268.8, 0.85, GridState::Absent, None),
        Forecast::Unknown
    );
}

#[test]
fn a_fall_seen_from_the_window_edge_needs_three_soc_levels() {
    // 89 at the edge is the top plateau (its start is not a fall), so Δt starts
    // at 88: 88 → 87 is one step → Unknown, one SOC step later than 89 → 87.
    let edge = vec![
        (0, sample(Some(89.0), 0.0, 0.0)),
        (1140, sample(Some(88.0), 0.0, 0.0)),
        (2280, sample(Some(87.0), 0.0, 0.0)),
    ];
    assert_eq!(
        forecast(&view(&edge), 268.8, 0.85, GridState::Absent, None),
        Forecast::Unknown
    );
    // One 2-point jump 90 → 88 has no observed transition after the top.
    let jump = vec![
        (0, sample(Some(90.0), 0.0, 0.0)),
        (1140, sample(Some(88.0), 0.0, 0.0)),
    ];
    assert_eq!(
        forecast(&view(&jump), 268.8, 0.85, GridState::Absent, None),
        Forecast::Unknown
    );
}

#[test]
fn charging_with_zero_watts_on_grid_is_unknown_not_the_floor() {
    let rising = vec![
        (0, sample(Some(80.0), 0.0, 0.0)),
        (600, sample(Some(85.0), 0.0, 0.0)),
    ];
    assert_eq!(
        forecast(&view(&rising), 1000.0, 0.85, GridState::Present, None),
        Forecast::Unknown
    );
    // A fall that charging has since reversed is not "SOC falling": 90 → 85 → 87.
    let reversed = vec![
        (0, sample(Some(90.0), 0.0, 0.0)),
        (600, sample(Some(85.0), 0.0, 0.0)),
        (1200, sample(Some(87.0), 0.0, 0.0)),
    ];
    assert_eq!(
        forecast(&view(&reversed), 1000.0, 0.85, GridState::Present, None),
        Forecast::Unknown
    );
}

#[test]
fn missing_inputs_are_unknown() {
    assert_eq!(
        forecast(&[], 1000.0, 0.85, GridState::Absent, None),
        Forecast::Unknown
    );
    let no_soc = vec![(0, sample(None, 100.0, 0.0))];
    assert_eq!(
        forecast(&view(&no_soc), 1000.0, 0.85, GridState::Absent, None),
        Forecast::Unknown
    );
    let ok = vec![(0, sample(Some(50.0), 100.0, 0.0))];
    assert_eq!(
        forecast(&view(&ok), 1000.0, 0.85, GridState::Unknown, None),
        Forecast::Unknown
    );
}

/// The AC180 case: outputs meter 9 W, the pack gives 55 W. 1152 Wh × 69 % ×
/// 0.85 = 675.6 Wh at 55 W = 12.28 h, not the 75 h the meter implies.
#[test]
fn battery_side_load_overrides_the_metered_output() {
    let rows = [
        (0, sample(Some(69.0), 9.0, 0.0)),
        (60, sample(Some(69.0), 9.0, 0.0)),
    ];
    let Forecast::OnBattery {
        load_w, runtime_h, ..
    } = forecast(&view(&rows), 1152.0, 0.85, GridState::Absent, Some(55.0))
    else {
        panic!("on battery");
    };
    approx(load_w, 55.0);
    approx(runtime_h, 1152.0 * 0.69 * 0.85 / 55.0);
    // Below 1 W it is ignored: the metered 9 W stays.
    let Forecast::OnBattery { load_w, .. } =
        forecast(&view(&rows), 1152.0, 0.85, GridState::Absent, Some(0.5))
    else {
        panic!("on battery");
    };
    approx(load_w, 9.0);
}
