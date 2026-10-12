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
fn frame_len_stops_at_five_on_exception() {
    // Exception vector `01 83 02 <crc>`: 5 bytes whatever count was asked.
    let frame = with_crc(&[0x01, 0x83, 0x02]);
    assert_eq!(frame.len(), 5);
    assert_eq!(expected_frame_len(&frame[..2], 40), Some(5));
    assert_eq!(expected_frame_len(&frame, 40), Some(5));
}

#[test]
fn frame_len_of_normal_reply() {
    assert_eq!(expected_frame_len(&[0x01, 0x03], 40), Some(85));
    assert_eq!(expected_frame_len(&[0x01, 0x03, 0x50, 0x00], 40), Some(85));
}

#[test]
fn frame_len_needs_two_bytes() {
    assert_eq!(expected_frame_len(&[], 40), None);
    assert_eq!(expected_frame_len(&[0x01], 40), None);
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

#[test]
fn rejects_short_frame() {
    assert_eq!(
        parse_read_response(&[0x01, 0x03, 0x00], 1),
        Err(FrameError::TooShort(3))
    );
}

#[test]
fn rejects_unexpected_function() {
    let frame = with_crc(&[0x01, 0x04, 0x02, 0x00, 0x01]);
    assert_eq!(
        parse_read_response(&frame, 1),
        Err(FrameError::UnexpectedFunction(0x04))
    );
}

#[test]
fn frame_errors_read_clearly() {
    let cases = [
        (FrameError::TooShort(3), "frame too short: 3 bytes"),
        (
            FrameError::BadCrc {
                expected: 0x1234,
                actual: 0xabcd,
            },
            "bad CRC: expected 0x1234, got 0xabcd",
        ),
        (
            FrameError::Exception(0x02),
            "device returned Modbus exception 0x02",
        ),
        (
            FrameError::UnexpectedFunction(0x04),
            "unexpected function code 0x04",
        ),
        (
            FrameError::LengthMismatch {
                expected: 85,
                actual: 7,
            },
            "length mismatch: expected 85 bytes, got 7",
        ),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
    let _: &dyn std::error::Error = &FrameError::TooShort(0);
}
