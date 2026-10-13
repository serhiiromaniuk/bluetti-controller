use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::*;
use crate::metrics::grid::{Grid, Params};
use crate::protocol::device::Metric;

use GridState::{Absent, Present, Unknown};

const P: i64 = 60;

fn grid(debounce: u32) -> Grid {
    Grid::new(Params {
        voltage_min: 180.0,
        debounce: NonZeroU32::new(debounce).unwrap(),
        interval_s: P,
    })
}

fn volts(v: f64) -> BTreeMap<Metric, f64> {
    BTreeMap::from([(Metric::AcInputVoltage, v)])
}

const ON: f64 = 230.0;
const OFF: f64 = 0.0;

fn opened(start: i64) -> Option<Event> {
    Some(Event {
        start,
        end: None,
        precision_s: P,
    })
}

fn closed(start: i64, end: i64) -> Option<Event> {
    Some(Event {
        start,
        end: Some(end),
        precision_s: P,
    })
}

#[test]
fn vote_counts_only_reachable_sensors() {
    assert_eq!(vote(&[]), HouseState::Unknown);
    assert_eq!(vote(&[Unknown, Unknown]), HouseState::Unknown);
    assert_eq!(vote(&[Unknown, Absent]), HouseState::Outage);
    assert_eq!(vote(&[Absent, Absent]), HouseState::Outage);
    assert_eq!(vote(&[Absent, Present]), HouseState::Power);
    assert_eq!(vote(&[Unknown, Present]), HouseState::Power);
}

#[test]
fn one_sensor_unreachable_while_other_has_no_grid_is_outage() {
    let (mut a, mut b) = (grid(2), grid(2));
    let mut house = House::new(P, None);
    for ts in [0, 60] {
        a.observe(ts, &volts(ON));
        b.observe(ts, &volts(ON));
        assert_eq!(house.update(&[a.state(), b.state()]), None);
    }
    // B stops answering after 60; A loses grid from 120 (accepted at 180).
    for ts in [120, 180, 240] {
        a.observe(ts, &volts(OFF));
        b.tick(ts);
        // B stays Present until > 3 intervals after its last sample.
        assert_eq!(house.update(&[a.state(), b.state()]), None, "ts {ts}");
    }
    a.observe(300, &volts(OFF));
    b.tick(300);
    assert_eq!(b.state(), (Unknown, Some(60)));
    // A has shown no grid since 120; B was last seen with grid at 60.
    assert_eq!(house.update(&[a.state(), b.state()]), opened(120));
    assert_eq!(house.state(), HouseState::Outage);
    a.observe(360, &volts(ON));
    b.tick(360);
    assert_eq!(house.update(&[a.state(), b.state()]), None);
    a.observe(420, &volts(ON));
    b.tick(420);
    assert_eq!(house.update(&[a.state(), b.state()]), closed(120, 360));
    assert_eq!(house.state(), HouseState::Power);
}

#[test]
fn both_unreachable_is_unknown_with_no_new_event() {
    let mut house = House::new(P, None);
    let up = (Present, Some(0));
    assert_eq!(house.update(&[up, up]), None);
    let lost = (Unknown, Some(60));
    assert_eq!(house.update(&[lost, lost]), None);
    assert_eq!(house.state(), HouseState::Unknown);
    // An outage already open stays open through unknown.
    assert_eq!(
        house.update(&[(Absent, Some(500)), (Absent, Some(520))]),
        opened(520)
    );
    assert_eq!(house.update(&[(Unknown, Some(560)), lost]), None);
    assert_eq!(house.state(), HouseState::Outage);
    assert_eq!(
        house.update(&[(Unknown, Some(560)), (Present, Some(900))]),
        closed(520, 900)
    );
}

#[test]
fn restart_resumes_the_open_event() {
    // The writer restores the open `grid_event` row's start after a restart.
    let mut house = House::new(P, Some(1000));
    assert_eq!(house.state(), HouseState::Outage);
    assert_eq!(house.update(&[(Unknown, None), (Unknown, None)]), None);
    assert_eq!(
        house.update(&[(Present, Some(2000)), (Absent, Some(1900))]),
        closed(1000, 2000)
    );
}

#[test]
fn end_never_precedes_start() {
    let mut house = House::new(P, Some(1000));
    assert_eq!(house.update(&[(Present, Some(400))]), closed(1000, 1000));
}

#[test]
fn flapping_sensor_input_makes_no_spurious_event() {
    let (mut a, mut b) = (grid(2), grid(2));
    let mut house = House::new(P, None);
    let mut feed = |ts: i64, va: f64, vb: f64| {
        a.observe(ts, &volts(va));
        b.observe(ts, &volts(vb));
        house.update(&[a.state(), b.state()])
    };
    assert_eq!(feed(0, ON, ON), None);
    assert_eq!(feed(60, ON, ON), None);
    // B loses grid for good; A flaps and never accepts a change.
    for (i, va) in [OFF, ON, OFF, ON].into_iter().enumerate() {
        let ts = 120 + 60 * i as i64;
        assert_eq!(feed(ts, va, OFF), None, "ts {ts}");
    }
    assert_eq!(feed(360, OFF, OFF), None);
    assert_eq!(feed(420, OFF, OFF), opened(360));
    // A flaps back on once: the outage holds.
    assert_eq!(feed(480, ON, OFF), None);
    assert_eq!(feed(540, OFF, OFF), None);
    assert_eq!(feed(600, ON, OFF), None);
    assert_eq!(feed(660, ON, OFF), closed(360, 600));
}

#[test]
fn totals_clip_events_to_the_window() {
    let ev = |start, end| Event {
        start,
        end,
        precision_s: P,
    };
    let events = [
        ev(-100, Some(300)),  // ends at `from`: outside
        ev(0, Some(600)),     // clipped to 300..600 = 300
        ev(1000, Some(1300)), // 300
        ev(2000, None),       // open, clipped to 2000..2500 = 500
        ev(2500, None),       // starts at `to`: outside
    ];
    assert_eq!(
        totals(&events, 300, 2500),
        Totals {
            count: 3,
            seconds_without_power: 1100,
            longest_s: 500,
        }
    );
    // Both open events overlap [5000, 6000): 1000 s each.
    assert_eq!(
        totals(&events, 5000, 6000),
        Totals {
            count: 2,
            seconds_without_power: 2000,
            longest_s: 1000,
        }
    );
    assert_eq!(totals(&[], 0, 100), Totals::default());
}
