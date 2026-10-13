use std::collections::VecDeque;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use crate::protocol::crypto::handshake::HandshakeError;
use crate::protocol::crypto::handshake::sim::{self, HOST_SCALAR, LOCAL_AES_KEY, verifying_point};
use crate::protocol::modbus::crc16;
use crate::transport::read::{ReadPolicy, read_block};

const SEED: [u8; 4] = [0xff, 0xdf, 0x22, 0xb7];
const TIMEOUT: Duration = Duration::from_secs(5);

/// A station behind the step-7 simulator, as a [`Gatt`]: it answers the
/// handshake, then every read request with register value = address.
struct SimStation {
    sim: sim::SimStation,
    queue: VecDeque<Vec<u8>>,
    /// Notifications each session reply is split into.
    split: usize,
    /// Messages 3 and 4 arrive in one notification.
    merge_3_4: bool,
    /// Message 2 arrived as a Write Request.
    accepted: bool,
    connected: Arc<AtomicBool>,
}

impl SimStation {
    fn new() -> Self {
        Self {
            sim: sim::SimStation::new(SEED),
            queue: VecDeque::from([sim::SimStation::challenge(SEED)]),
            split: 1,
            merge_3_4: false,
            accepted: false,
            connected: Arc::new(AtomicBool::new(true)),
        }
    }

    fn check(&self) -> Result<(), TransportError> {
        if self.connected.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(TransportError::NotConnected)
        }
    }

    /// Answers one session-encrypted read request.
    fn answer(&mut self, envelope: &[u8]) {
        let session = self.sim.session();
        let request = session.decrypt(envelope).unwrap();
        let start = u16::from_be_bytes([request[2], request[3]]);
        let count = u16::from_be_bytes([request[4], request[5]]);
        let reply = session
            .encrypt([1, 2, 3, 4], &response(start, count))
            .unwrap();
        let chunks: Vec<_> = reply.chunks(reply.len().div_ceil(self.split)).collect();
        assert_eq!(chunks.len(), self.split);
        self.queue.extend(chunks.into_iter().map(<[u8]>::to_vec));
    }
}

impl Gatt for SimStation {
    async fn write(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.check()?;
        if !self.accepted {
            // A station drops message 2 sent as a Write Command (§4.4).
        } else if self.sim.session_key.is_none() {
            let msg6 = self.sim.on_our_key(data);
            self.queue.push_back(msg6);
        } else {
            self.answer(data);
        }
        Ok(())
    }

    async fn write_with_response(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.check()?;
        let (msg3, msg4) = self.sim.on_reply(data);
        self.accepted = true;
        if self.merge_3_4 {
            self.queue.push_back([msg3, msg4].concat());
        } else {
            self.queue.extend([msg3, msg4]);
        }
        Ok(())
    }

    async fn next_notification(&mut self, timeout: Duration) -> Result<Vec<u8>, TransportError> {
        self.check()?;
        match self.queue.pop_front() {
            Some(notification) => Ok(notification),
            None => {
                tokio::time::sleep(timeout).await;
                Err(TransportError::Timeout(timeout))
            }
        }
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected.store(false, Ordering::SeqCst);
        Ok(())
    }
}

/// A read response whose register values equal their addresses.
fn response(start: u16, count: u16) -> Vec<u8> {
    let mut frame = vec![0x01, 0x03, (2 * count) as u8];
    for address in start..start + count {
        frame.extend_from_slice(&address.to_be_bytes());
    }
    frame.extend_from_slice(&crc16(&frame).to_le_bytes());
    frame
}

fn policy() -> ReadPolicy {
    ReadPolicy {
        timeout: TIMEOUT,
        attempts: NonZeroU32::new(1).unwrap(),
        backoff: Duration::ZERO,
    }
}

#[tokio::test(start_paused = true)]
async fn read_block_returns_the_simulated_registers() {
    let keys = sim::keys();
    let mut gatt = CryptoGatt::establish(SimStation::new(), &keys, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(
        read_block(&mut gatt, 100, 3, &policy()).await.unwrap(),
        [100, 101, 102]
    );
    gatt.disconnect().await.unwrap();
    assert_eq!(gatt.write(&[1]).await, Err(TransportError::NotConnected));
}

#[tokio::test(start_paused = true)]
async fn envelope_split_across_three_notifications_reassembles() {
    let keys = sim::keys();
    let station = SimStation {
        split: 3,
        ..SimStation::new()
    };
    let mut gatt = CryptoGatt::establish(station, &keys, TIMEOUT)
        .await
        .unwrap();
    let values = read_block(&mut gatt, 1300, 20, &policy()).await.unwrap();
    assert_eq!(values, (1300..1320).collect::<Vec<u16>>());
}

#[tokio::test(start_paused = true)]
async fn two_messages_in_one_notification_are_split() {
    let keys = sim::keys();
    let station = SimStation {
        merge_3_4: true,
        ..SimStation::new()
    };
    let mut gatt = CryptoGatt::establish(station, &keys, TIMEOUT)
        .await
        .unwrap();
    assert_eq!(read_block(&mut gatt, 10, 1, &policy()).await.unwrap(), [10]);
}

#[tokio::test(start_paused = true)]
async fn silent_station_times_out_and_is_disconnected() {
    let keys = sim::keys();
    let station = SimStation::new();
    let connected = Arc::clone(&station.connected);
    let station = SimStation {
        queue: VecDeque::new(),
        ..station
    };
    let started = Instant::now();
    let err = CryptoGatt::establish(station, &keys, TIMEOUT)
        .await
        .err()
        .unwrap();
    assert_eq!(err, TransportError::Timeout(TIMEOUT));
    assert_eq!(started.elapsed(), TIMEOUT);
    assert!(!connected.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn bad_station_signature_fails_and_disconnects() {
    let wrong =
        StaticKeys::new(LOCAL_AES_KEY, &HOST_SCALAR, &verifying_point(&[0x44; 32])).unwrap();
    let station = SimStation::new();
    let connected = Arc::clone(&station.connected);
    let err = CryptoGatt::establish(station, &wrong, TIMEOUT)
        .await
        .err()
        .unwrap();
    assert_eq!(
        err,
        TransportError::Handshake(HandshakeError::BadStationSignature)
    );
    assert!(!connected.load(Ordering::SeqCst));
}
