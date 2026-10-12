//! One Modbus read over a [`Gatt`] channel (`ROADMAP.md` §5.4 step 5): write
//! [`read_request`], reassemble notifications up to [`expected_frame_len`]
//! within a per-request timeout, validate with [`parse_read_response`], and
//! retry a bounded number of times with backoff (§4.6).

use std::num::NonZeroU32;
use std::time::Duration;

use tokio::time::Instant;

use super::{Gatt, TransportError};
use crate::protocol::device::{Profile, Registers};
use crate::protocol::modbus::{FrameError, expected_frame_len, parse_read_response, read_request};

/// Timeout and retry bounds of one block read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadPolicy {
    /// Time from the request write to the last byte of the reply.
    pub timeout: Duration,
    /// Total tries, the first included.
    pub attempts: NonZeroU32,
    /// Pause before the first retry; doubles on each further retry.
    pub backoff: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Frame(#[from] FrameError),
}

impl ReadError {
    /// A lost or garbled reply may succeed on a retry. A Modbus exception is
    /// the station's answer, and a closed or out-of-order channel stays broken.
    fn is_retryable(&self) -> bool {
        match self {
            ReadError::Transport(err) => matches!(err, TransportError::Timeout(_)),
            ReadError::Frame(err) => !matches!(err, FrameError::Exception(_)),
        }
    }
}

/// Reads `count` holding registers from `start`, retrying per `policy`.
pub async fn read_block(
    gatt: &mut impl Gatt,
    start: u16,
    count: u16,
    policy: &ReadPolicy,
) -> Result<Vec<u16>, ReadError> {
    let mut backoff = policy.backoff;
    let mut attempt = 1;
    loop {
        let err = match read_once(gatt, start, count, policy.timeout).await {
            Ok(values) => return Ok(values),
            Err(err) => err,
        };
        if attempt >= policy.attempts.get() || !err.is_retryable() {
            return Err(err);
        }
        tracing::warn!(start, count, attempt, error = %err, "read failed, retrying");
        tokio::time::sleep(backoff).await;
        backoff = backoff.saturating_mul(2);
        drain(gatt).await;
        attempt += 1;
    }
}

/// Reads every register block of `profile` in order; any failed block fails
/// the whole read.
pub async fn read_profile(
    gatt: &mut impl Gatt,
    profile: &Profile,
    policy: &ReadPolicy,
) -> Result<Registers, ReadError> {
    let mut registers = Registers::default();
    for &(start, count) in profile.blocks {
        let values = read_block(gatt, start, count, policy).await?;
        registers.insert_block(start, &values);
    }
    Ok(registers)
}

async fn read_once(
    gatt: &mut impl Gatt,
    start: u16,
    count: u16,
    timeout: Duration,
) -> Result<Vec<u16>, ReadError> {
    gatt.write(&read_request(start, count)).await?;
    let deadline = Instant::now() + timeout;
    let mut frame = Vec::new();
    while expected_frame_len(&frame, count).is_none_or(|len| frame.len() < len) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(TransportError::Timeout(timeout).into());
        }
        let fragment = gatt
            .next_notification(remaining)
            .await
            .map_err(|err| match err {
                // Report the whole request timeout, not the remainder that ran out.
                TransportError::Timeout(_) => TransportError::Timeout(timeout),
                err => err,
            })?;
        frame.extend(fragment);
    }
    Ok(parse_read_response(&frame, count)?)
}

/// Drops notifications already queued, so a late fragment of the failed reply
/// cannot prefix the retried one.
async fn drain(gatt: &mut impl Gatt) {
    while gatt.next_notification(Duration::ZERO).await.is_ok() {}
}

#[cfg(test)]
mod tests;
