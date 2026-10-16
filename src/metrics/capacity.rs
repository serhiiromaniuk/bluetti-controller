//! Usable battery capacity from observed discharges (ROADMAP.md §2.2, R1
//! decision): runs of 5-minute `rollup_5m` buckets with metered output and no
//! input, each long enough to drop the SOC by [`MIN_DROP`] points; on stations
//! with pack V and I, runs of raw samples counted from the battery side
//! ([`battery_segments`]). Pure: no I/O.

use super::energy::BUCKET_S;

/// The smallest SOC drop, in points, that makes a segment: SOC has 1-point
/// steps, so 10 points is ±10 %.
pub const MIN_DROP: f64 = 10.0;
/// The estimate is the median of this many newest segments.
pub const RECENT: usize = 5;

/// The `rollup_5m` columns the estimate needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bucket {
    /// Bucket start, unix seconds.
    pub ts: i64,
    pub soc_avg: Option<f64>,
    pub energy_in_wh: Option<f64>,
    pub energy_out_wh: Option<f64>,
}

impl Bucket {
    /// No input and metered output: a zero-output bucket (the EB3A's low-load
    /// floor, §4.3 CF-4) would hide energy, so it never counts.
    fn discharging(&self) -> bool {
        self.soc_avg.is_some()
            && self.energy_in_wh == Some(0.0)
            && self.energy_out_wh.is_some_and(|wh| wh > 0.0)
    }
}

/// One qualifying discharge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// First and last bucket start; the energy spans their midpoints.
    pub start: i64,
    pub end: i64,
    pub soc_from: f64,
    pub soc_to: f64,
    /// Output between the first and the last bucket's midpoint.
    pub energy_wh: f64,
    /// `energy_wh` per 100 SOC points.
    pub capacity_wh: f64,
}

/// The median of the newest [`RECENT`] segments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub median_wh: f64,
    /// `median_wh / nominal`: the suggested `usable_capacity_factor`.
    pub factor: f64,
    /// Segments the median covers.
    pub n: usize,
}

/// Every qualifying discharge in `buckets` (oldest first), oldest first: a
/// maximal run of consecutive discharging buckets whose `soc_avg` drops by at
/// least [`MIN_DROP`] points.
pub fn segments(buckets: &[Bucket]) -> Vec<Segment> {
    buckets
        .chunk_by(|a, b| b.ts == a.ts + BUCKET_S && a.discharging() && b.discharging())
        .filter(|run| run[0].discharging())
        .filter_map(|run| {
            let (first, last) = (run[0], run[run.len() - 1]);
            let (soc_from, soc_to) = (first.soc_avg?, last.soc_avg?);
            let drop = soc_from - soc_to;
            if drop < MIN_DROP {
                return None;
            }
            let out = |b: &Bucket| b.energy_out_wh.unwrap_or(0.0);
            let energy_wh = run.iter().map(out).sum::<f64>() - (out(&first) + out(&last)) / 2.0;
            Some(Segment {
                start: first.ts,
                end: last.ts,
                soc_from,
                soc_to,
                energy_wh,
                capacity_wh: energy_wh / drop * 100.0,
            })
        })
        .collect()
}

/// One raw sample seen from the battery (ROADMAP.md §2.2, R1 amended
/// 2026-10-16): SOC, the summed input power and pack V × I.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub ts: i64,
    pub soc: f64,
    pub in_w: f64,
    pub battery_w: f64,
}

/// Every qualifying discharge in `points` (oldest first), counted from the
/// battery side: a maximal run with no input and gaps of at most `max_gap_s`,
/// whose SOC drops by at least [`MIN_DROP`] points. The energy is the left
/// Riemann sum of `battery_w` over the run, so it includes the inverter's own
/// draw and loads below the output meter's floor.
pub fn battery_segments(points: &[Point], max_gap_s: i64) -> Vec<Segment> {
    let idle = |p: &Point| p.in_w == 0.0;
    points
        .chunk_by(|a, b| b.ts - a.ts <= max_gap_s && idle(a) && idle(b))
        .filter(|run| idle(&run[0]))
        .filter_map(|run| {
            let (first, last) = (run[0], run[run.len() - 1]);
            let drop = first.soc - last.soc;
            if drop < MIN_DROP {
                return None;
            }
            let energy_wh: f64 = run
                .windows(2)
                .map(|w| w[0].battery_w * (w[1].ts - w[0].ts) as f64 / 3600.0)
                .sum();
            Some(Segment {
                start: first.ts,
                end: last.ts,
                soc_from: first.soc,
                soc_to: last.soc,
                energy_wh,
                capacity_wh: energy_wh / drop * 100.0,
            })
        })
        .collect()
}

/// The mean battery-side power of `points` (the last few minutes, oldest
/// first) for the runtime forecast: only with no input anywhere (the pack
/// current is unsigned, so charging would read as load), at least 3 points,
/// and the newest at or after `fresh_from`.
pub fn battery_load_w(points: &[Point], fresh_from: i64) -> Option<f64> {
    let newest = points.last()?;
    if points.len() < 3 || newest.ts < fresh_from || points.iter().any(|p| p.in_w != 0.0) {
        return None;
    }
    Some(points.iter().map(|p| p.battery_w).sum::<f64>() / points.len() as f64)
}

/// The median capacity of the newest [`RECENT`] of `segments` (oldest first);
/// `None` without any.
pub fn estimate(segments: &[Segment], nominal_wh: f64) -> Option<Estimate> {
    let recent = &segments[segments.len().saturating_sub(RECENT)..];
    let mut wh: Vec<f64> = recent.iter().map(|s| s.capacity_wh).collect();
    wh.sort_by(f64::total_cmp);
    let mid = wh.len() / 2;
    let median_wh = match wh.len() {
        0 => return None,
        n if n % 2 == 0 => (wh[mid - 1] + wh[mid]) / 2.0,
        _ => wh[mid],
    };
    Some(Estimate {
        median_wh,
        factor: median_wh / nominal_wh,
        n: wh.len(),
    })
}

#[cfg(test)]
mod tests;
