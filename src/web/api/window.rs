//! The `from`/`to` window of every windowed API route, capped (SECURITY.md
//! "Bounded queries and clean output").

use std::collections::HashMap;

use super::{ApiError, bad, int};

/// The longest window of every windowed route: a year, past any retention
/// (SECURITY.md "Bounded queries and clean output").
const MAX_SPAN_S: i64 = 366 * 86_400;

/// `from`/`to` in unix seconds: `to` defaults to `now`, `from` to `to − span`
/// (at least 0). `0 ≤ from < to` and `to − from ≤ MAX_SPAN_S`, else `400`.
pub(super) fn window(
    q: &HashMap<String, String>,
    now: i64,
    span: i64,
) -> Result<(i64, i64), ApiError> {
    let to = int(q, "to")?.unwrap_or(now);
    let from = match int(q, "from")? {
        Some(from) => from,
        None => to
            .checked_sub(span)
            .ok_or_else(|| bad(format!("to ({to}) is out of range")))?
            .max(0),
    };
    if from < 0 || from >= to {
        return Err(bad(format!("need 0 ≤ from < to, got {from} and {to}")));
    }
    to.checked_sub(from)
        .filter(|s| *s <= MAX_SPAN_S)
        .ok_or_else(|| bad(format!("the window must span ≤ {MAX_SPAN_S} s")))?;
    Ok((from, to))
}
