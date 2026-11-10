//! Modbus-RTU framing used by Bluetti stations on the BLE GATT channel.
//!
//! Requests are written to characteristic `0xFF02`. Responses arrive as
//! notifications on `0xFF01` and can be split across several notifications,
//! so the transport reassembles them up to [`expected_frame_len`] bytes:
//! [`read_response_len`], except an exception reply (function byte has 0x80
//! set), which is always 5 bytes.
//! Registers are big-endian `u16`; the CRC is CRC-16/MODBUS, little-endian.

use std::fmt;

pub const SERVICE_UUID: u128 = 0x0000ff00_0000_1000_8000_00805f9b34fb;
pub const NOTIFY_UUID: u128 = 0x0000ff01_0000_1000_8000_00805f9b34fb;
pub const WRITE_UUID: u128 = 0x0000ff02_0000_1000_8000_00805f9b34fb;

const UNIT_ID: u8 = 0x01;
const READ_HOLDING: u8 = 0x03;
const WRITE_SINGLE: u8 = 0x06;
const EXCEPTION_FLAG: u8 = 0x80;

/// CRC-16/MODBUS (poly 0xA001 reflected, init 0xFFFF).
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= u16::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// Builds a "read holding registers" request for `count` registers at `start`.
pub fn read_request(start: u16, count: u16) -> [u8; 8] {
    let mut frame = [0u8; 8];
    frame[0] = UNIT_ID;
    frame[1] = READ_HOLDING;
    frame[2..4].copy_from_slice(&start.to_be_bytes());
    frame[4..6].copy_from_slice(&count.to_be_bytes());
    let crc = crc16(&frame[..6]);
    frame[6..8].copy_from_slice(&crc.to_le_bytes());
    frame
}

/// Length of a successful read response for `count` registers:
/// unit id, function code, byte count, data, CRC.
pub fn read_response_len(count: u16) -> usize {
    5 + 2 * usize::from(count)
}

/// Total length of the reply whose first bytes are `prefix`, for a request of
/// `count` registers: 5 for a Modbus exception (function byte has 0x80 set),
/// else [`read_response_len`]. `None` until the function byte has arrived.
pub fn expected_frame_len(prefix: &[u8], count: u16) -> Option<usize> {
    let function = *prefix.get(1)?;
    Some(if function & EXCEPTION_FLAG != 0 {
        5
    } else {
        read_response_len(count)
    })
}

/// Builds a "write single register" request (function 0x06) setting
/// `register` to `value`. Sent only behind the control gate (golden rule #2).
pub fn write_request(register: u16, value: u16) -> [u8; 8] {
    let mut frame = [0u8; 8];
    frame[0] = UNIT_ID;
    frame[1] = WRITE_SINGLE;
    frame[2..4].copy_from_slice(&register.to_be_bytes());
    frame[4..6].copy_from_slice(&value.to_be_bytes());
    let crc = crc16(&frame[..6]);
    frame[6..8].copy_from_slice(&crc.to_le_bytes());
    frame
}

/// Length of a successful write reply: the 8-byte echo of the request.
pub const WRITE_RESPONSE_LEN: usize = 8;

/// Total length of the write reply whose first bytes are `prefix`: 5 for a
/// Modbus exception, else [`WRITE_RESPONSE_LEN`]. `None` until the function
/// byte has arrived.
pub fn expected_write_len(prefix: &[u8]) -> Option<usize> {
    let function = *prefix.get(1)?;
    Some(if function & EXCEPTION_FLAG != 0 {
        5
    } else {
        WRITE_RESPONSE_LEN
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    TooShort(usize),
    BadCrc {
        expected: u16,
        actual: u16,
    },
    Exception(u8),
    UnexpectedFunction(u8),
    LengthMismatch {
        expected: usize,
        actual: usize,
    },
    /// A write reply whose register or value differs from the request.
    EchoMismatch,
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort(len) => write!(f, "frame too short: {len} bytes"),
            Self::BadCrc { expected, actual } => {
                write!(f, "bad CRC: expected {expected:#06x}, got {actual:#06x}")
            }
            Self::Exception(code) => write!(f, "device returned Modbus exception {code:#04x}"),
            Self::UnexpectedFunction(code) => write!(f, "unexpected function code {code:#04x}"),
            Self::LengthMismatch { expected, actual } => {
                write!(
                    f,
                    "length mismatch: expected {expected} bytes, got {actual}"
                )
            }
            Self::EchoMismatch => write!(f, "write reply does not echo the request"),
        }
    }
}

impl std::error::Error for FrameError {}

/// Checks length ≥ 5, the CRC and the function code (`function`, or its
/// exception) of a complete reply; returns the body without the CRC.
fn checked_body(frame: &[u8], function: u8) -> Result<&[u8], FrameError> {
    if frame.len() < 5 {
        return Err(FrameError::TooShort(frame.len()));
    }
    let (body, crc_bytes) = frame.split_at(frame.len() - 2);
    let actual = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
    let expected = crc16(body);
    if expected != actual {
        return Err(FrameError::BadCrc { expected, actual });
    }
    match body[1] {
        code if code == function => Ok(body),
        code if code == function | EXCEPTION_FLAG => Err(FrameError::Exception(body[2])),
        code => Err(FrameError::UnexpectedFunction(code)),
    }
}

/// Validates a complete read response and returns the register values.
pub fn parse_read_response(frame: &[u8], count: u16) -> Result<Vec<u16>, FrameError> {
    let body = checked_body(frame, READ_HOLDING)?;
    let expected_len = read_response_len(count);
    if frame.len() != expected_len || usize::from(body[2]) != 2 * usize::from(count) {
        return Err(FrameError::LengthMismatch {
            expected: expected_len,
            actual: frame.len(),
        });
    }
    Ok(body[3..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_be_bytes(pair))
        .collect())
}

/// Validates a complete write reply: CRC, function 0x06 (or its exception),
/// and an echo of exactly `register` and `value`.
pub fn parse_write_response(frame: &[u8], register: u16, value: u16) -> Result<(), FrameError> {
    checked_body(frame, WRITE_SINGLE)?;
    if frame.len() != WRITE_RESPONSE_LEN {
        return Err(FrameError::LengthMismatch {
            expected: WRITE_RESPONSE_LEN,
            actual: frame.len(),
        });
    }
    if frame[..WRITE_RESPONSE_LEN] != write_request(register, value) {
        return Err(FrameError::EchoMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
