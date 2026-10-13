//! `dump`: read-only sweep of a register range (golden rule #2: function
//! 0x03 only, through `read_request`). Chunks of at most [`MAX_CHUNK`]
//! registers with a pause between them (golden rule #5). A chunk the station
//! rejects is reported and skipped; a broken link ends the sweep.

use std::time::Duration;

use anyhow::bail;
use bluetti_controller::protocol::identity::is_serial;
use bluetti_controller::transport::TransportError;
use bluetti_controller::transport::read::{ReadError, ReadPolicy, read_block};

use super::{Context, session};

/// Registers per request; the largest profile block reads 40.
const MAX_CHUNK: u16 = 40;
const PAUSE: Duration = Duration::from_millis(500);

pub async fn run(ctx: &Context<'_>, target: &str, from: u16, to: u16) -> anyhow::Result<()> {
    if from > to {
        bail!("--from {from} is past --to {to}");
    }
    session::run(ctx, target, async |gatt, _| {
        let policy = ReadPolicy::from_ble(ctx.ble);
        let ranges = chunks(from, to, MAX_CHUNK);
        let mut unread = 0;
        // A reply to a timed-out chunk may still land after the next write.
        let mut after_timeout = false;
        for (i, &(start, count)) in ranges.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(PAUSE).await;
            }
            let last = start + (count - 1);
            let result = read_block(gatt, start, count, &policy).await;
            let suspect = std::mem::replace(
                &mut after_timeout,
                matches!(
                    result,
                    Err(ReadError::Transport(TransportError::Timeout(_)))
                ),
            );
            match result {
                Ok(values) => {
                    if suspect {
                        println!("{start:>5}-{last}: read after a timeout, may be its late reply");
                    }
                    for (register, value) in (start..=last).zip(values) {
                        println!("{}", row(register, value, ctx.anonymise));
                    }
                }
                Err(ReadError::Transport(err)) if !matches!(err, TransportError::Timeout(_)) => {
                    return Err(err.into());
                }
                Err(err) => {
                    unread += 1;
                    println!("{start:>5}-{last}: unread ({err})");
                }
            }
        }
        println!("{} chunk(s), {unread} unread", ranges.len());
        Ok(())
    })
    .await
}

/// `(start, count)` chunks covering `from..=to`, each at most `max` long.
fn chunks(from: u16, to: u16, max: u16) -> Vec<(u16, u16)> {
    let (to, max) = (u32::from(to), u32::from(max));
    (u32::from(from)..=to)
        .step_by(max as usize)
        .map(|start| {
            let count = (to - start + 1).min(max);
            (start as u16, count as u16)
        })
        .collect()
}

/// `register  0xhhhh  decimal  ascii`; under `--anonymise` the serial
/// registers of both layouts are masked, since the layout may be unknown.
fn row(register: u16, value: u16, anonymise: bool) -> String {
    if anonymise && is_serial(register, None) {
        return format!("{register:>5}  ******  masked");
    }
    let ascii: String = value
        .to_be_bytes()
        .iter()
        .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
        .collect();
    format!("{register:>5}  0x{value:04x}  {value:>5}  {ascii}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_cover_the_range_in_steps_of_at_most_max() {
        assert_eq!(chunks(100, 200, 40), [(100, 40), (140, 40), (180, 21)]);
        assert_eq!(chunks(10, 10, 40), [(10, 1)]);
        assert_eq!(chunks(0, 79, 40), [(0, 40), (40, 40)]);
        assert_eq!(chunks(65500, 65535, 40), [(65500, 36)]);
    }

    #[test]
    fn row_masks_serial_registers_of_both_layouts_only_when_anonymising() {
        assert_eq!(row(17, 0x4542, true), "   17  ******  masked");
        assert_eq!(row(117, 0x4542, true), "  117  ******  masked");
        assert_eq!(row(117, 0x4542, false), "  117  0x4542  17730  EB");
        assert_eq!(row(21, 0x0001, true), "   21  0x0001      1  ..");
    }
}
