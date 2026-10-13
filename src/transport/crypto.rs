//! [`CryptoGatt`]: the Modbus channel of an encrypted station (`ROADMAP.md`
//! §4.4, §5.4 step 4). It runs the key exchange over an inner [`Gatt`], then
//! seals every request in a session envelope and opens every reply, so
//! [`read_block`](super::read::read_block) sees plain Modbus frames.
//!
//! All randomness of the encrypted layer lives here: the ephemeral P-256
//! secret and every per-frame seed come from the OS RNG, so
//! `protocol::crypto` stays pure.

use std::time::Duration;

use tokio::time::Instant;

use super::{Gatt, TransportError};
use crate::protocol::crypto::handshake::{Handshake, Session, StaticKeys, Step};
use crate::protocol::crypto::{Layer, envelope_len};

/// Station messages of one handshake (1, 3, 4 and 6). The whole exchange is
/// bounded by this many request timeouts.
const STATION_MESSAGES: u32 = 4;

/// An inner [`Gatt`] with an established session. No `Debug`: it holds the
/// session key.
pub struct CryptoGatt<G> {
    inner: G,
    session: Session,
    /// Bytes of the next frame that arrived before it was complete.
    pending: Vec<u8>,
}

impl<G: Gatt + Send> CryptoGatt<G> {
    /// Runs the key exchange over `inner`, which is subscribed and has sent
    /// nothing yet. Each station message must arrive within `timeout`, the
    /// whole exchange within 4 × `timeout`. On any failure `inner` is
    /// disconnected before the error returns (golden rule #5).
    pub async fn establish(
        mut inner: G,
        keys: &StaticKeys,
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        let bound = timeout.saturating_mul(STATION_MESSAGES);
        let mut pending = Vec::new();
        let exchange = handshake(&mut inner, &mut pending, keys, timeout);
        let result = match tokio::time::timeout(bound, exchange).await {
            Ok(result) => result,
            Err(_) => Err(TransportError::Timeout(bound)),
        };
        match result {
            Ok(session) => Ok(Self {
                inner,
                session,
                pending,
            }),
            Err(err) => {
                if let Err(e) = inner.disconnect().await {
                    tracing::warn!(error = %e, "disconnect after a failed handshake failed");
                }
                Err(err)
            }
        }
    }
}

impl<G: Gatt + Send> Gatt for CryptoGatt<G> {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let envelope = self.seal(data)?;
        self.inner.write(&envelope).await
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let envelope = self.seal(data)?;
        self.inner.write_with_response(&envelope).await
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let envelope =
            next_frame(&mut self.inner, &mut self.pending, Layer::Session, timeout).await?;
        self.session
            .decrypt(&envelope)
            .map_err(TransportError::Crypto)
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.pending.clear();
        self.inner.disconnect().await
    }
}

impl<G> CryptoGatt<G> {
    /// A session envelope under a fresh random seed.
    fn seal(&self, modbus: &[u8]) -> Result<Vec<u8>, TransportError> {
        let mut seed = [0; 4];
        random(&mut seed)?;
        self.session
            .encrypt(seed, modbus)
            .map_err(TransportError::Crypto)
    }
}

/// Feeds station messages to a [`Handshake`] and sends its replies.
async fn handshake<G: Gatt>(
    inner: &mut G,
    pending: &mut Vec<u8>,
    keys: &StaticKeys,
    timeout: Duration,
) -> Result<Session, TransportError> {
    let mut ephemeral = [0; 32];
    random(&mut ephemeral)?;
    // A scalar outside 1..n (odds ~2^-32) fails this connection only.
    let mut exchange = Handshake::new(keys, &ephemeral).map_err(TransportError::Handshake)?;
    loop {
        let msg = next_frame(inner, pending, exchange.expecting(), timeout).await?;
        // shortcut: no resend of message 5 when message 6 is late (§4.4,
        // rustytti); add it if hardware runs lose message 6.
        match exchange
            .on_message(&msg)
            .map_err(TransportError::Handshake)?
        {
            Step::Reply { frame, acked: true } => inner.write_with_response(&frame).await?,
            Step::Reply {
                frame,
                acked: false,
            } => inner.write(&frame).await?,
            Step::Wait => {}
            Step::Done(session) => return Ok(session),
        }
    }
}

/// Reassembles the next complete `layer` frame from notifications within
/// `timeout`; bytes past it stay in `pending` for the next frame. A frame cut
/// off by the timeout is dropped, so it cannot prefix the next one.
async fn next_frame<G: Gatt>(
    inner: &mut G,
    pending: &mut Vec<u8>,
    layer: Layer,
    timeout: Duration,
) -> Result<Vec<u8>, TransportError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(len) = envelope_len(layer, pending)
            && pending.len() >= len
        {
            return Ok(pending.drain(..len).collect());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        match inner.next_notification(remaining).await {
            Ok(fragment) => pending.extend(fragment),
            Err(TransportError::Timeout(_)) => {
                pending.clear();
                // Report the whole timeout, not the remainder that ran out.
                return Err(TransportError::Timeout(timeout));
            }
            Err(err) => return Err(err),
        }
    }
}

fn random(buf: &mut [u8]) -> Result<(), TransportError> {
    getrandom::fill(buf).map_err(|err| TransportError::Rng(err.to_string()))
}

#[cfg(test)]
mod tests;
