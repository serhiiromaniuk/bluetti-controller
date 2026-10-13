//! [`ReplayGatt`]: a [`Gatt`] that replays a recorded [`Fixture`]. Each write
//! must equal the next recorded write; notifications are served in recorded
//! order. No real time passes: a missing notification times out at once.

use std::collections::VecDeque;
use std::time::Duration;

use super::fixture::{Dir, Fixture, FixtureError, decode_hex, encode_hex};
use super::{Gatt, TransportError};

#[derive(Debug)]
pub struct ReplayGatt {
    frames: VecDeque<(Dir, Vec<u8>)>,
    connected: bool,
}

impl ReplayGatt {
    /// Decodes every frame up front, so a bad fixture fails before the test runs.
    pub fn new(fixture: &Fixture) -> Result<ReplayGatt, FixtureError> {
        let frames = fixture
            .frames
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                if frame.uuid != frame.dir.uuid() {
                    return Err(FixtureError::WrongUuid {
                        index,
                        dir: frame.dir,
                        uuid: frame.uuid.clone(),
                    });
                }
                Ok((frame.dir, decode_hex(&frame.hex)?))
            })
            .collect::<Result<_, _>>()?;
        Ok(ReplayGatt {
            frames,
            connected: true,
        })
    }

    fn check_connected(&self) -> Result<(), TransportError> {
        if self.connected {
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }
}

impl Gatt for ReplayGatt {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.check_connected()?;
        let expected = match self.frames.front() {
            Some((Dir::Write, frame)) if frame == data => {
                self.frames.pop_front();
                return Ok(());
            }
            Some((Dir::Write, frame)) => encode_hex(frame),
            Some((Dir::Notify, _)) => "an unread notification first".to_owned(),
            None => "end of fixture".to_owned(),
        };
        Err(TransportError::UnexpectedWrite {
            expected,
            got: encode_hex(data),
        })
    }

    /// A fixture records no write kind, so both kinds replay alike.
    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.write(data).await
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        self.check_connected()?;
        match self.frames.front() {
            Some((Dir::Notify, _)) => Ok(self.frames.pop_front().expect("front checked").1),
            _ => Err(TransportError::Timeout(timeout)),
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
