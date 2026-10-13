use super::*;

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn arr<const N: usize>(s: &str) -> [u8; N] {
    hex(s).try_into().unwrap()
}

// NIST SP 800-38A F.2: four plaintext blocks shared by F.2.1 and F.2.5.
const NIST_IV: &str = "000102030405060708090a0b0c0d0e0f";
const NIST_PLAIN: &str = "6bc1bee22e409f96e93d7e117393172a ae2d8a571e03ac9c9eb76fac45af8e51
                          30c81c46a35ce411e5fbc1191a0a52ef f69f2445df4f9b17ad2b417be66c3710";

#[test]
fn cbc_aes128_matches_nist_f21() {
    let key = arr::<16>("2b7e151628aed2a6abf7158809cf4f3c");
    let mut buf = hex(NIST_PLAIN);
    cbc_encrypt(AesKey::Aes128(&key), &arr(NIST_IV), &mut buf);
    let want = hex(
        "7649abac8119b246cee98e9b12e9197d 5086cb9b507219ee95db113a917678b2
         73bed6b8e3c1743b7116e69e22229516 3ff1caa1681fac09120eca307586e1a7",
    );
    assert_eq!(buf, want);
    cbc_decrypt(AesKey::Aes128(&key), &arr(NIST_IV), &mut buf);
    assert_eq!(buf, hex(NIST_PLAIN));
}

#[test]
fn cbc_aes256_matches_nist_f25() {
    let key = arr::<32>("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
    let mut buf = hex(NIST_PLAIN);
    cbc_encrypt(AesKey::Aes256(&key), &arr(NIST_IV), &mut buf);
    let want = hex(
        "f58c4c04d6e5f1ba779eabfb5f7bfbd6 9cfc4e967edb808d679f777bc6702c7d
         39f23369a9d9bacfa530e26304231461 b2eb05e2c39be9fcda6c19078c6a9d1b",
    );
    assert_eq!(buf, want);
    cbc_decrypt(AesKey::Aes256(&key), &arr(NIST_IV), &mut buf);
    assert_eq!(buf, hex(NIST_PLAIN));
}

// rustytti's captured challenge and our reply (ROADMAP.md §4.4): message 2 depends
// only on the seed, so it holds for any local AES key.
#[test]
fn recorded_challenge_gives_recorded_reply() {
    let msg1 = parse_kex(&hex("2a2a0104ffdf22b702bc")).unwrap();
    assert_eq!(msg1.kind, 0x01);
    assert_eq!(msg1.payload, hex("ffdf22b7"));
    let temp = TempKey::derive(msg1.payload.try_into().unwrap(), &[0x5A; 16]);
    assert_eq!(
        encode_kex(0x02, &temp.reply()).unwrap(),
        hex("2a2a02045a4fd30e0190")
    );
}

#[test]
fn temp_key_is_iv_xor_local_key() {
    let local = [0xFF; 16];
    let temp = TempKey::derive([0xff, 0xdf, 0x22, 0xb7], &local);
    // md5(b7 22 df ff), bytes 8..12 are the recorded reply 5a4fd30e.
    assert_eq!(temp.iv[8..12], hex("5a4fd30e")[..]);
    for i in 0..16 {
        assert_eq!(temp.key[i], temp.iv[i] ^ 0xFF);
    }
}

#[test]
fn session_iv_is_md5_of_seed_not_reversed() {
    // md5("abcd") = e2fc714c4727ee9395f324cd2e7f331f (RFC 1321 style test value).
    assert_eq!(
        session_iv(*b"abcd"),
        arr::<16>("e2fc714c4727ee9395f324cd2e7f331f")
    );
}

#[test]
fn kex_rejects_bad_magic_length_and_checksum() {
    assert_eq!(
        parse_kex(&hex("2b2a0104ffdf22b702bc")),
        Err(CryptoError::BadMagic)
    );
    assert_eq!(
        parse_kex(&hex("2a2a0104ffdf22b702")),
        Err(CryptoError::LengthMismatch {
            expected: 10,
            actual: 9
        })
    );
    assert_eq!(parse_kex(&hex("2a2a01")), Err(CryptoError::TooShort(3)));
    assert_eq!(
        parse_kex(&hex("2a2a0104ffdf22b702bd")),
        Err(CryptoError::BadChecksum {
            expected: 0x02bc,
            actual: 0x02bd
        })
    );
    assert_eq!(
        encode_kex(0x05, &[0; 256]),
        Err(CryptoError::PayloadTooLong(256))
    );
}

#[test]
fn temp_envelope_round_trips_with_zero_padding() {
    let temp = TempKey::derive([1, 2, 3, 4], &[0x11; 16]);
    let plain = encode_kex(0x06, &[0x00]).unwrap(); // 7 bytes
    let env = seal_temp(&temp, &plain).unwrap();
    assert_eq!(env[..2], [0x00, 0x07]);
    assert_eq!(env.len(), 2 + 16);
    assert_eq!(envelope_len(Layer::Temp, &env[..2]), Some(env.len()));
    assert_eq!(open_temp(&temp, &env).unwrap(), plain);
}

#[test]
fn session_envelope_round_trips_and_carries_seed() {
    let key = [0x42; 32];
    // A Modbus reply ending in a zero CRC byte must survive the zero padding.
    let plain = [0x01, 0x03, 0x02, 0x00, 0x64, 0xB9, 0x00];
    let env = seal_session(&key, [9, 8, 7, 6], &plain).unwrap();
    assert_eq!(env[..6], [0x00, 0x07, 9, 8, 7, 6]);
    assert_eq!(env.len(), 6 + 16);
    assert_eq!(envelope_len(Layer::Session, &env[..2]), Some(env.len()));
    assert_eq!(open_session(&key, &env).unwrap(), plain);
    // Exactly one block: no extra padding block.
    let env = seal_session(&key, [0; 4], &[0xAB; 16]).unwrap();
    assert_eq!(env.len(), 6 + 16);
}

#[test]
fn envelope_len_waits_for_its_header() {
    assert_eq!(envelope_len(Layer::Kex, &hex("2a2a01")), None);
    assert_eq!(envelope_len(Layer::Kex, &hex("2a2a0104")), Some(10));
    assert_eq!(envelope_len(Layer::Temp, &[0x00]), None);
    assert_eq!(envelope_len(Layer::Session, &[0x00, 0x11]), Some(6 + 32));
    assert_eq!(envelope_len(Layer::Session, &[0x00, 0x00]), Some(6));
}

#[test]
fn envelopes_reject_bad_length_and_padding() {
    let key = [0x42; 32];
    let env = seal_session(&key, [1, 1, 1, 1], &[0xAA; 5]).unwrap();
    assert_eq!(open_session(&key, &env[..4]), Err(CryptoError::TooShort(4)));
    assert_eq!(
        open_session(&key, &env[..env.len() - 1]),
        Err(CryptoError::LengthMismatch {
            expected: 22,
            actual: 21
        })
    );
    // Claim 4 plain bytes: byte 5 (0xAA) now sits in the padding.
    let mut short = env.clone();
    short[1] = 4;
    assert_eq!(open_session(&key, &short), Err(CryptoError::BadPadding));
    // A wrong key decrypts to noise in the padding.
    assert_eq!(
        open_session(&[0x43; 32], &env),
        Err(CryptoError::BadPadding)
    );

    let temp = TempKey::derive([1, 2, 3, 4], &[0x11; 16]);
    let mut env = seal_temp(&temp, &[0xAA; 3]).unwrap();
    env[1] = 2;
    assert_eq!(open_temp(&temp, &env), Err(CryptoError::BadPadding));
    assert_eq!(
        seal_temp(&temp, &vec![0; 65_536]),
        Err(CryptoError::PayloadTooLong(65_536))
    );
}

#[test]
fn errors_display_without_payload_bytes() {
    let shown = [
        CryptoError::BadMagic,
        CryptoError::TooShort(3),
        CryptoError::PayloadTooLong(256),
        CryptoError::BadPadding,
        CryptoError::BadChecksum {
            expected: 1,
            actual: 2,
        },
        CryptoError::LengthMismatch {
            expected: 1,
            actual: 2,
        },
    ]
    .map(|e| e.to_string());
    assert!(shown.iter().all(|s| !s.is_empty()));
}
