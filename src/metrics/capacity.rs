//! Usable battery capacity from observed discharges (ROADMAP.md §2.2, R1
//! decision): runs of 5-minute `rollup_5m` buckets with metered output and no
//! input, each long enough to drop the SOC by [`MIN_DROP`] points. Pure: no I/O.

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
