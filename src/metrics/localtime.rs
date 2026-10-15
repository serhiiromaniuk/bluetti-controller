//! Local time from `general.timezone` (ROADMAP.md §8.4 d, ADR-009): the
//! local-day bounds of the energy table and the "lasts until HH:MM" text.
//! Timestamps are unix seconds. Pure: the zone is parsed once by
//! `src/config.rs`, these functions only convert.

use jiff::Timestamp;
use jiff::tz::TimeZone;

/// The local day holding `ts`, as `[start, end)` in unix seconds. A DST day
/// is 23 h or 25 h long; a zone whose midnight is skipped starts at the first
/// valid instant.
pub fn local_day_bounds(ts: i64, tz: &TimeZone) -> Result<(i64, i64), jiff::Error> {
    let start = Timestamp::from_second(ts)?
        .to_zoned(tz.clone())
        .start_of_day()?;
    let end = start.tomorrow()?.start_of_day()?;
    Ok((start.timestamp().as_second(), end.timestamp().as_second()))
}

/// `ts` as local `HH:MM`.
pub fn local_hhmm(ts: i64, tz: &TimeZone) -> Result<String, jiff::Error> {
    Ok(Timestamp::from_second(ts)?
        .to_zoned(tz.clone())
        .strftime("%H:%M")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kyiv() -> TimeZone {
        TimeZone::get("Europe/Kyiv").unwrap()
    }

    // Hand-computed: 2026-03-28T00:00Z = 1767225600 (2026-01-01) + 86 d.
    const MAR_28_UTC: i64 = 1_774_656_000;
    // 2026-10-24T00:00Z = 1767225600 + 296 d.
    const OCT_24_UTC: i64 = 1_792_800_000;

    #[test]
    fn spring_forward_day_is_23_hours() {
        // 2026-03-29 in Kyiv: 00:00 EET (+02) = 03-28 22:00Z; next midnight is
        // EEST (+03) = 03-29 21:00Z.
        let start = MAR_28_UTC + 22 * 3600;
        let end = MAR_28_UTC + 86_400 + 21 * 3600;
        for ts in [start, start + 10 * 3600, end - 1] {
            assert_eq!(local_day_bounds(ts, &kyiv()).unwrap(), (start, end), "{ts}");
        }
        assert_eq!(end - start, 23 * 3600);
        assert_eq!(local_day_bounds(end, &kyiv()).unwrap().0, end);
    }

    #[test]
    fn fall_back_day_is_25_hours() {
        // 2026-10-25 in Kyiv: 00:00 EEST = 10-24 21:00Z; next midnight is EET
        // = 10-25 22:00Z.
        let start = OCT_24_UTC + 21 * 3600;
        let end = OCT_24_UTC + 86_400 + 22 * 3600;
        assert_eq!(
            local_day_bounds(start + 3600, &kyiv()).unwrap(),
            (start, end)
        );
        assert_eq!(end - start, 25 * 3600);
    }

    #[test]
    fn day_bounds_follow_local_midnight_not_utc() {
        // 2026-10-24T22:30Z is already 10-25 01:30 local.
        let (start, _) = local_day_bounds(OCT_24_UTC + 22 * 3600 + 1800, &kyiv()).unwrap();
        assert_eq!(start, OCT_24_UTC + 21 * 3600);
        assert_eq!(
            local_day_bounds(OCT_24_UTC, &TimeZone::UTC).unwrap(),
            (OCT_24_UTC, OCT_24_UTC + 86_400)
        );
    }

    #[test]
    fn hhmm_in_kyiv() {
        // 2026-01-01T00:00Z = 02:00 EET; 2026-07-01T12:34Z = 15:34 EEST.
        assert_eq!(local_hhmm(1_767_225_600, &kyiv()).unwrap(), "02:00");
        assert_eq!(local_hhmm(1_782_909_240, &kyiv()).unwrap(), "15:34");
    }

    #[test]
    fn out_of_range_timestamp_is_an_error() {
        assert!(local_hhmm(i64::MAX, &kyiv()).is_err());
        assert!(local_day_bounds(i64::MIN, &kyiv()).is_err());
    }
}
