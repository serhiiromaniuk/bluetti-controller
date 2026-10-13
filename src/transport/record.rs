//! [`RecordingGatt`]: wraps any [`Gatt`] and tees every frame that crossed the
//! link into a [`Fixture`] (`ROADMAP.md` §11): above `CryptoGatt` for
//! `--record` (plaintext), below it for `--record-wire` (the wire layer).
//!
//! The recording is raw: the advert name carries the serial number. Save it
//! under the gitignored `captures/` and run it through
//! [`anonymise_fixture`](super::anonymise::anonymise_fixture) before it goes
//! to `tests/frames/` (RULES.md golden rule #3).

use std::time::{Duration, Instant};

use super::fixture::{Dir, Fixture, FixtureAdvert, FrameRecord, encode_hex};
use super::{Advert, Gatt, TransportError};

#[derive(Debug)]
pub struct RecordingGatt<G> {
    inner: G,
    fixture: Fixture,
    opened: Instant,
}

impl<G: Gatt + Send> RecordingGatt<G> {
    /// Starts the recording clock: `t_ms` counts from here.
    pub fn new(inner: G, advert: &Advert) -> Self {
        RecordingGatt {
            inner,
            fixture: Fixture {
                advert: FixtureAdvert {
                    name: advert.name.clone().unwrap_or_default(),
                    manufacturer_data: encode_hex(&advert.manufacturer_data),
                },
                ephemeral: None,
                frames: Vec::new(),
            },
            opened: Instant::now(),
        }
    }

    pub fn fixture(&self) -> &Fixture {
        &self.fixture
    }

    pub fn inner(&self) -> &G {
        &self.inner
    }

    /// Keeps the session's ephemeral secret with a wire recording, for the
    /// real-key replay test; the anonymisers drop it.
    pub fn set_ephemeral(&mut self, secret: &[u8; 32]) {
        self.fixture.ephemeral = Some(encode_hex(secret));
    }

    /// Only frames that crossed the link are kept, so a replay of the
    /// recording drives the same exchange.
    fn push(&mut self, dir: Dir, data: &[u8]) {
        self.fixture.frames.push(FrameRecord {
            t_ms: u64::try_from(self.opened.elapsed().as_millis()).unwrap_or(u64::MAX),
            dir,
            uuid: dir.uuid().to_owned(),
            hex: encode_hex(data),
        });
    }
}

impl<G: Gatt + Send> Gatt for RecordingGatt<G> {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.inner.write(data).await?;
        self.push(Dir::Write, data);
        Ok(())
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.inner.write_with_response(data).await?;
        self.push(Dir::Write, data);
        Ok(())
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        let data = self.inner.next_notification(timeout).await?;
        self.push(Dir::Notify, &data);
        Ok(data)
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.inner.disconnect().await
    }
}

#[cfg(test)]
mod tests;
