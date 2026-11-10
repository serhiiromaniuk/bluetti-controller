//! One output switch over a [`Gatt`] channel (`ROADMAP.md` §4.2, ADR-010):
//! write the 0x06 frame **once**, check its echo, then read the switch's
//! read-back register with the normal retry policy. The read-back is the truth.

use super::read::{ReadError, ReadPolicy, exchange, read_block};
use super::{Gatt, TransportError};
use crate::protocol::control::OutputSwitch;
use crate::protocol::modbus::{
    FrameError, expected_write_len, parse_write_response, write_request,
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The write reply: a Modbus exception, a bad CRC or a wrong echo.
    #[error(transparent)]
    Frame(#[from] FrameError),
    #[error("read-back: {0}")]
    ReadBack(#[from] ReadError),
    #[error("read-back register holds {got}, wanted {}", u16::from(*wanted))]
    ReadBackMismatch { wanted: bool, got: u16 },
}

/// Switches `switch` on (1) or off (0) and returns the read-back state.
/// The write frame is never retried: a timed-out write may have landed, so
/// only the read-back retries (golden rule #5). A read-back other than the
/// wanted value is [`WriteError::ReadBackMismatch`].
pub async fn set_output(
    gatt: &mut impl Gatt,
    switch: &OutputSwitch,
    on: bool,
    policy: &ReadPolicy,
) -> Result<bool, WriteError> {
    let (register, value) = (switch.register(), u16::from(on));
    let reply = exchange(
        gatt,
        &write_request(register, value),
        policy.timeout,
        expected_write_len,
    )
    .await?;
    parse_write_response(&reply, register, value)?;
    let got = read_block(gatt, switch.read_back(), 1, policy).await?[0];
    if got != value {
        return Err(WriteError::ReadBackMismatch { wanted: on, got });
    }
    Ok(on)
}

#[cfg(test)]
pub(crate) mod tests;
