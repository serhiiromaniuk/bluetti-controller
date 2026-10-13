use super::*;

const MIN: f64 = 180.0;

fn params(debounce: u32) -> Params {
    Params {
        voltage_min: MIN,
        debounce: NonZeroU32::new(debounce).unwrap(),
        interval_s: 60,
    }
}

fn volts(v: f64) -> BTreeMap<Metric, f64> {
    BTreeMap::from([(Metric::AcInputVoltage, v), (Metric::AcInputPower, 0.0)])
}

const ON: f64 = 230.0;
const OFF: f64 = 0.0;

fn change(state: GridState, since: i64) -> Option<Change> {
    Some(Change { state, since })
}

#[test]
fn reading_uses_voltage_then_power_fallback() {
    assert_eq!(reading(&volts(180.0), MIN), Some(true));
    assert_eq!(reading(&volts(179.9), MIN), Some(false));
    // Voltage wins over power when both exist.
    let both = BTreeMap::from([(Metric::AcInputVoltage, 0.0), (Metric::AcInputPower, 50.0)]);
    assert_eq!(reading(&both, MIN), Some(false));
    let watts = |w| BTreeMap::from([(Metric::AcInputPower, w)]);
    assert_eq!(reading(&watts(1.0), MIN), Some(true));
    assert_eq!(reading(&watts(0.0), MIN), Some(false));
    assert_eq!(reading(&BTreeMap::new(), MIN), None);
}

#[test]
fn debounce_two_backdates_to_first_sample() {
    let mut grid = Grid::new(params(2));
    assert_eq!(grid.state(), (GridState::Unknown, None));
    assert_eq!(grid.observe(0, &volts(ON)), None);
    assert_eq!(grid.observe(60, &volts(ON)), change(GridState::Present, 0));
    assert_eq!(grid.observe(120, &volts(OFF)), None);
    assert_eq!(
        grid.observe(180, &volts(OFF)),
        change(GridState::Absent, 120)
    );
    assert_eq!(grid.state(), (GridState::Absent, Some(120)));
}

#[test]
fn flapping_on_off_on_never_flips() {
    let mut grid = Grid::new(params(2));
    grid.observe(0, &volts(ON));
    grid.observe(60, &volts(ON));
    for (i, v) in [OFF, ON, OFF, ON, OFF, ON].into_iter().enumerate() {
        let ts = 120 + 60 * i as i64;
        assert_eq!(grid.observe(ts, &volts(v)), None, "ts {ts}");
    }
    assert_eq!(grid.state(), (GridState::Present, Some(0)));
}

#[test]
fn gap_over_three_intervals_is_unknown_then_recovers() {
    let mut grid = Grid::new(params(2));
    grid.observe(0, &volts(OFF));
    grid.observe(60, &volts(OFF));
    // Exactly 3 intervals after the last sample is still known.
    assert_eq!(grid.tick(240), None);
    assert_eq!(grid.tick(241), change(GridState::Unknown, 60));
    assert_eq!(grid.tick(1000), None);
    // Recovery needs `debounce` fresh samples again, backdated to the first.
    assert_eq!(grid.observe(1000, &volts(OFF)), None);
    assert_eq!(
        grid.observe(1060, &volts(OFF)),
        change(GridState::Absent, 1000)
    );
}

#[test]
fn gap_inside_observe_drops_the_pending_run() {
    let mut grid = Grid::new(params(2));
    grid.observe(0, &volts(ON));
    grid.observe(60, &volts(ON));
    grid.observe(120, &volts(OFF));
    // The next sample comes 5 intervals later: Unknown first, the old OFF run is gone.
    assert_eq!(
        grid.observe(420, &volts(OFF)),
        change(GridState::Unknown, 120)
    );
    assert_eq!(
        grid.observe(480, &volts(OFF)),
        change(GridState::Absent, 420)
    );
}

#[test]
fn debounce_one_accepts_at_once_and_samples_without_grid_fields_are_skipped() {
    let mut grid = Grid::new(params(1));
    assert_eq!(grid.observe(0, &BTreeMap::new()), None);
    assert_eq!(grid.tick(10_000), None, "a skipped sample sets no last_ts");
    assert_eq!(grid.observe(0, &volts(ON)), change(GridState::Present, 0));
    assert_eq!(grid.observe(60, &volts(ON)), None);
    // After a gap, the resumed sample's own change wins over the Unknown one.
    assert_eq!(
        grid.observe(600, &volts(OFF)),
        change(GridState::Absent, 600)
    );
}
