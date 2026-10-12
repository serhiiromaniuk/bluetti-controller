//! Modbus-RTU framing used by Bluetti stations on the BLE GATT channel.
//!
//! Requests are written to characteristic `0xFF02`. Responses arrive as
//! notifications on `0xFF01` and can be split across several notifications,
//! so the transport reassembles them up to [`read_response_len`] bytes — except
//! an exception reply (function byte has 0x80 set), which is always 5 bytes.
//! Registers are big-endian `u16`; the CRC is CRC-16/MODBUS, little-endian.

use std::fmt;

pub const SERVICE_UUID: u128 = 0x0000ff00_0000_1000_8000_00805f9b34fb;
pub const NOTIFY_UUID: u128 = 0x0000ff01_0000_1000_8000_00805f9b34fb;
pub const WRITE_UUID: u128 = 0x0000ff02_0000_1000_8000_00805f9b34fb;

const UNIT_ID: u8 = 0x01;
const READ_HOLDING: u8 = 0x03;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    TooShort(usize),
    BadCrc { expected: u16, actual: u16 },
    Exception(u8),
    UnexpectedFunction(u8),
    LengthMismatch { expected: usize, actual: usize },
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
        }
    }
}

impl std::error::Error for FrameError {}

/// Validates a complete read response and returns the register values.
pub fn parse_read_response(frame: &[u8], count: u16) -> Result<Vec<u16>, FrameError> {
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
        READ_HOLDING => {}
        code if code == READ_HOLDING | EXCEPTION_FLAG => {
            return Err(FrameError::Exception(body[2]));
        }
        code => return Err(FrameError::UnexpectedFunction(code)),
    }
    let expected_len = read_response_len(count);
    if frame.len() != expected_len || usize::from(body[2]) != 2 * usize::from(count) {
        return Err(FrameError::LengthMismatch {
            expected: expected_len,
            actual: frame.len(),
        });
    }
    Ok(body[3..]
        .chunks_exact(2)
        .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_crc(body: &[u8]) -> Vec<u8> {
        let mut frame = body.to_vec();
        frame.extend_from_slice(&crc16(body).to_le_bytes());
        frame
    }

    #[test]
    fn crc_matches_reference_vectors() {
        assert_eq!(crc16(&[0x01, 0x03, 0x00, 0x00, 0x00, 0x0A]), 0xCDC5);
        assert_eq!(crc16(&[0x01, 0x03, 0x00, 0x00, 0x00, 0x01]), 0x0A84);
    }

    #[test]
    fn read_request_layout() {
        // EB3A main block: 40 registers from register 10.
        assert_eq!(
            read_request(10, 40),
            [0x01, 0x03, 0x00, 0x0A, 0x00, 0x28, 0x65, 0xD6]
        );
    }

    #[test]
    fn parses_registers() {
        let frame = with_crc(&[0x01, 0x03, 0x04, 0x00, 0x64, 0x01, 0x2C]);
        assert_eq!(parse_read_response(&frame, 2), Ok(vec![100, 300]));
    }

    #[test]
    fn rejects_bad_crc() {
        let mut frame = with_crc(&[0x01, 0x03, 0x02, 0x00, 0x01]);
        frame[4] ^= 0xFF;
        assert!(matches!(
            parse_read_response(&frame, 1),
            Err(FrameError::BadCrc { .. })
        ));
    }

    #[test]
    fn reports_exception() {
        let frame = with_crc(&[0x01, 0x83, 0x02]);
        assert_eq!(
            parse_read_response(&frame, 1),
            Err(FrameError::Exception(0x02))
        );
    }

    #[test]
    fn rejects_truncated_reassembly() {
        let frame = with_crc(&[0x01, 0x03, 0x02, 0x00, 0x01]);
        assert!(matches!(
            parse_read_response(&frame, 2),
            Err(FrameError::LengthMismatch {
                expected: 9,
                actual: 7
            })
        ));
    }
}
