//! Energy accounting (ROADMAP.md §7.4) and the `rollup_5m` row values (§6).
//!
//! One rule for rollups and daily totals (§7.4 M3): the segment between two
//! consecutive samples counts toward the window of its **later** sample, so the
//! caller passes the sample just before the window too. A segment longer than
//! 3 intervals is a gap: it adds no energy and no SOC change, and its length is
//! reported as missing coverage. Samples are `(ts, values)` in unix seconds,
//! oldest first; a missing power field counts as 0 W. Pure: no I/O.

use std::collections::BTreeMap;

use crate::protocol::device::Metric;

/// The `rollup_5m` bucket length in seconds.
pub const BUCKET_S: i64 = 300;

/// Energy over a window `[from, to)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Totals {
    /// AC + DC input, trapezoid-integrated.
    pub energy_in_wh: f64,
    /// AC + DC output; under-counts loads below the low-load floor (§4.3).
    pub energy_out_wh: f64,
    /// `capacity_wh × Δsoc / 100`, Δsoc summed over the counted segments.
    pub stored_delta_wh: f64,
    /// `in − out − stored`: an estimate, it absorbs the uncounted low loads.
    pub losses_estimate_wh: f64,
    /// Seconds of segments skipped as gaps.
    pub missing_s: i64,
}

/// The values of one `rollup_5m` row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rollup {
    /// Bucket start.
    pub ts: i64,
    /// Over the bucket samples that carry an SOC; `None` when none does.
    pub soc_avg: Option<f64>,
    pub soc_min: Option<f64>,
    pub ac_in_w_avg: f64,
    pub dc_in_w_avg: f64,
    pub ac_out_w_avg: f64,
    pub dc_out_w_avg: f64,
    pub energy_in_wh: f64,
    pub energy_out_wh: f64,
    /// Samples with ts in the bucket.
    pub n: u32,
}

/// Integrated sums over the segments whose later sample lies in `[from, to)`.
struct Sums {
    in_wh: f64,
    out_wh: f64,
    soc_delta: f64,
    missing_s: i64,
}

fn watts(values: &BTreeMap<Metric, f64>, metric: Metric) -> f64 {
    values.get(&metric).copied().unwrap_or(0.0)
}

fn power_in(values: &BTreeMap<Metric, f64>) -> f64 {
    watts(values, Metric::AcInputPower) + watts(values, Metric::DcInputPower)
}

fn power_out(values: &BTreeMap<Metric, f64>) -> f64 {
    watts(values, Metric::AcOutputPower) + watts(values, Metric::DcOutputPower)
}

fn integrate(
    samples: &[(i64, &BTreeMap<Metric, f64>)],
    interval_s: i64,
    from: i64,
    to: i64,
) -> Sums {
    let mut sums = Sums {
        in_wh: 0.0,
        out_wh: 0.0,
        soc_delta: 0.0,
        missing_s: 0,
    };
    for pair in samples.windows(2) {
        let ((t0, v0), (t1, v1)) = (pair[0], pair[1]);
        let dt = t1 - t0;
        if !(from..to).contains(&t1) || dt <= 0 {
            continue;
        }
        if dt > 3 * interval_s {
            sums.missing_s += dt;
            continue;
        }
        let hours = dt as f64 / 3600.0;
        sums.in_wh += (power_in(v0) + power_in(v1)) / 2.0 * hours;
        sums.out_wh += (power_out(v0) + power_out(v1)) / 2.0 * hours;
        if let (Some(s0), Some(s1)) = (v0.get(&Metric::BatterySoc), v1.get(&Metric::BatterySoc)) {
            sums.soc_delta += s1 - s0;
        }
    }
    sums
}

/// Energy totals for the segments whose later sample lies in `[from, to)`.
/// Pass the last sample before `from` too, or the first segment is lost.
pub fn totals(
    samples: &[(i64, &BTreeMap<Metric, f64>)],
    interval_s: i64,
    capacity_wh: f64,
    from: i64,
    to: i64,
) -> Totals {
    let sums = integrate(samples, interval_s, from, to);
    let stored_delta_wh = capacity_wh * sums.soc_delta / 100.0;
    Totals {
        energy_in_wh: sums.in_wh,
        energy_out_wh: sums.out_wh,
        stored_delta_wh,
        losses_estimate_wh: sums.in_wh - sums.out_wh - stored_delta_wh,
        missing_s: sums.missing_s,
    }
}

/// The `rollup_5m` values for `[bucket, bucket + BUCKET_S)`: averages, minimum
/// SOC and `n` over the samples in the bucket, energy over the segments ending
/// in it. Pass the last sample before the bucket too, so the segment that
/// crosses into it is counted. `None` when no sample lies in the bucket.
pub fn rollup(
    samples: &[(i64, &BTreeMap<Metric, f64>)],
    interval_s: i64,
    bucket: i64,
) -> Option<Rollup> {
    let end = bucket + BUCKET_S;
    let inside: Vec<_> = samples
        .iter()
        .filter(|(ts, _)| (bucket..end).contains(ts))
        .map(|(_, v)| *v)
        .collect();
    if inside.is_empty() {
        return None;
    }
    let n = inside.len() as f64;
    let avg = |metric| inside.iter().map(|v| watts(v, metric)).sum::<f64>() / n;
    let socs: Vec<f64> = inside
        .iter()
        .filter_map(|v| v.get(&Metric::BatterySoc).copied())
        .collect();
    let sums = integrate(samples, interval_s, bucket, end);
    Some(Rollup {
        ts: bucket,
        soc_avg: (!socs.is_empty()).then(|| socs.iter().sum::<f64>() / socs.len() as f64),
        soc_min: socs.iter().copied().reduce(f64::min),
        ac_in_w_avg: avg(Metric::AcInputPower),
        dc_in_w_avg: avg(Metric::DcInputPower),
        ac_out_w_avg: avg(Metric::AcOutputPower),
        dc_out_w_avg: avg(Metric::DcOutputPower),
        energy_in_wh: sums.in_wh,
        energy_out_wh: sums.out_wh,
        n: inside.len() as u32,
    })
}

#[cfg(test)]
mod tests;
