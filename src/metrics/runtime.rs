//! Runtime forecast (ROADMAP.md §7.3), including the low-load floor (§4.3).
//!
//! A pure function over one station's window of samples, oldest first; it
//! holds no state to persist (§7.2 M3 note).

use std::collections::BTreeMap;

use crate::metrics::grid::GridState;
use crate::protocol::device::Metric;

/// The EWMA time constant for the load, in seconds (~10 min, §7.3).
const TAU_S: f64 = 600.0;

/// How long one station's battery lasts at its current load.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Forecast {
    /// Not enough data: no SOC, grid state unknown, or outputs at 0 W with no
    /// usable SOC slope. Never a runtime from the 1 W floor.
    Unknown,
    /// On battery: lasts until `until_ts` (unix seconds).
    OnBattery {
        load_w: f64,
        remaining_wh: f64,
        runtime_h: f64,
        until_ts: i64,
    },
    /// On grid: the runtime if the power goes now.
    OnGrid {
        load_w: f64,
        remaining_wh: f64,
        runtime_h: f64,
    },
}

/// The forecast at the newest SOC sample of `samples` (oldest first; a
/// missing output field counts as 0 W, a sample without SOC is skipped for SOC).
///
/// - `load_w` = EWMA of `ac_out_w + dc_out_w` with `α = 1 − e^(−Δt/600)`.
/// - `remaining_wh = capacity_wh × soc / 100 × usable_factor`, and
///   `runtime_h = remaining_wh / max(load_w, 1)`.
/// - **Outputs read 0 W** = the newest sample's `ac_out_w + dc_out_w` is ≤ 0.
///   Then the load comes from the SOC slope (rules on `soc_slope_w`), else the
///   forecast is `Unknown`.
/// - `until_ts` is counted from the newest SOC sample, the moment the
///   remaining energy was read.
/// - `battery_w`, the pack's own recent draw while nothing charges it
///   ([`super::capacity::battery_load_w`]), overrides the output-based load:
///   it includes the inverter's own draw and loads under the meter's floor.
pub fn forecast(
    samples: &[(i64, &BTreeMap<Metric, f64>)],
    capacity_wh: f64,
    usable_factor: f64,
    grid: GridState,
    battery_w: Option<f64>,
) -> Forecast {
    let Some(&(ts, soc)) = soc_points(samples).last() else {
        return Forecast::Unknown;
    };
    let Some(&(_, newest)) = samples.last() else {
        return Forecast::Unknown;
    };
    let load_w = if let Some(w) = battery_w.filter(|w| *w >= 1.0) {
        w
    } else if output_w(newest) <= 0.0 {
        match soc_slope_w(samples, capacity_wh) {
            Some(w) => w,
            None => return Forecast::Unknown,
        }
    } else {
        ewma_load_w(samples).max(1.0)
    };
    let remaining_wh = capacity_wh * soc / 100.0 * usable_factor;
    let runtime_h = remaining_wh / load_w;
    match grid {
        GridState::Unknown => Forecast::Unknown,
        GridState::Present => Forecast::OnGrid {
            load_w,
            remaining_wh,
            runtime_h,
        },
        GridState::Absent => Forecast::OnBattery {
            load_w,
            remaining_wh,
            runtime_h,
            until_ts: ts + (runtime_h * 3600.0).round() as i64,
        },
    }
}

fn output_w(values: &BTreeMap<Metric, f64>) -> f64 {
    let get = |m| values.get(&m).copied().unwrap_or(0.0);
    get(Metric::AcOutputPower) + get(Metric::DcOutputPower)
}

/// EWMA of the output power over irregular samples, seeded with the first.
fn ewma_load_w(samples: &[(i64, &BTreeMap<Metric, f64>)]) -> f64 {
    let mut iter = samples.iter();
    let Some(&(mut prev_ts, first)) = iter.next() else {
        return 0.0;
    };
    let mut load = output_w(first);
    for &(ts, values) in iter {
        let dt = (ts - prev_ts).max(0) as f64;
        let alpha = 1.0 - (-dt / TAU_S).exp();
        load += alpha * (output_w(values) - load);
        prev_ts = ts;
    }
    load
}

fn soc_points(samples: &[(i64, &BTreeMap<Metric, f64>)]) -> Vec<(i64, f64)> {
    samples
        .iter()
        .filter_map(|&(ts, v)| v.get(&Metric::BatterySoc).map(|soc| (ts, *soc)))
        .collect()
}

/// The load implied by the falling SOC, `capacity_wh × ΔSOC / 100 / Δt`.
///
/// - **SOC falling** = the run of SOC samples ending at the newest one in
///   which SOC never rises (a rise anywhere ends the run, so a fall that
///   charging has since reversed does not count).
/// - Δt is measured between observed downward transitions: from the first
///   sample below the run's top level to the first sample at the newest
///   level. The top plateau is skipped, because its start was a rise or the
///   window edge, never a fall (an idle on-grid plateau must not dilute Δt).
/// - **≥ 2 SOC steps** = the SOC at Δt's start minus the newest SOC is ≥ 2
///   percentage points.
///
/// `None` when that drop is under 2 steps or Δt ≤ 0.
fn soc_slope_w(samples: &[(i64, &BTreeMap<Metric, f64>)], capacity_wh: f64) -> Option<f64> {
    let points = soc_points(samples);
    let &(_, latest) = points.last()?;
    let mut start = points.len() - 1;
    while start > 0 && points[start - 1].1 >= points[start].1 {
        start -= 1;
    }
    let top = points[start].1;
    let from = start + points[start..].iter().position(|&(_, soc)| soc < top)?;
    let (t0, high) = points[from];
    let (t1, _) = *points[from..].iter().find(|(_, soc)| *soc == latest)?;
    let drop = high - latest;
    let dt_h = (t1 - t0) as f64 / 3600.0;
    (drop >= 2.0 && dt_h > 0.0).then(|| capacity_wh * drop / 100.0 / dt_h)
}

#[cfg(test)]
mod tests;
