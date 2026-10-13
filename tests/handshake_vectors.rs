//! Handshake vectors recorded from our own stations with `--record-wire` and
//! masked by `anonymise --wire` (`ROADMAP.md` §4.4 "Committable artefacts",
//! §11). The key-free test checks their structure; the synthetic round trip
//! in `protocol::crypto::handshake` covers the key math. The `#[ignore]` test
//! replays the raw capture with the local `keys.toml`.

use std::collections::BTreeMap;
use std::path::Path;

use bluetti_controller::config::CryptoConfig;
use bluetti_controller::protocol::crypto::handshake::{Handshake, Session, Step};
use bluetti_controller::protocol::crypto::{
    Layer, TempKey, encode_kex, envelope_len, open_temp, parse_kex,
};
use bluetti_controller::protocol::modbus::{parse_read_response, read_request, read_response_len};
use bluetti_controller::transport::fixture::{Dir, Fixture, decode_hex};
use md5::{Digest, Md5};

/// Fixture and the register count of its one `dump` chunk (EB3A 36–49,
/// Elite 100–110).
const FIXTURES: [(&str, u16); 2] = [("handshake-eb3a", 14), ("handshake-elite200v2", 11)];
/// Kex frame around a payload: magic, type, len, checksum.
const KEX_OVERHEAD: usize = 6;

/// Station messages in arrival order: 1 and 3 kex, 4 and 6 temp, then session.
fn station_layer(index: usize) -> Layer {
    [Layer::Kex, Layer::Kex, Layer::Temp, Layer::Temp]
        .get(index)
        .copied()
        .unwrap_or(Layer::Session)
}

/// Reassembles every station message; panics on bytes left over.
fn station_messages(fixture: &Fixture) -> Vec<Vec<u8>> {
    let mut stream: Vec<u8> = fixture
        .frames
        .iter()
        .filter(|f| f.dir == Dir::Notify)
        .flat_map(|f| decode_hex(&f.hex).unwrap())
        .collect();
    let mut messages = Vec::new();
    while !stream.is_empty() {
        let len = envelope_len(station_layer(messages.len()), &stream).expect("length field");
        assert!(len <= stream.len(), "truncated station message");
        messages.push(stream.drain(..len).collect());
    }
    messages
}

fn writes(fixture: &Fixture) -> Vec<Vec<u8>> {
    fixture
        .frames
        .iter()
        .filter(|f| f.dir == Dir::Write)
        .map(|f| decode_hex(&f.hex).unwrap())
        .collect()
}

fn plain_len(envelope: &[u8]) -> usize {
    u16::from_be_bytes([envelope[0], envelope[1]]).into()
}

/// An envelope of `plain_len` bytes, zero-padded to whole blocks, whose
/// ciphertext was masked to zeros.
fn assert_masked_envelope(envelope: &[u8], layer: Layer, plain: usize) {
    let header = if layer == Layer::Temp { 2 } else { 6 };
    assert_eq!(plain_len(envelope), plain);
    assert_eq!(envelope.len(), header + plain.div_ceil(16) * 16);
    assert!(
        envelope[header..].iter().all(|&b| b == 0),
        "ciphertext kept"
    );
}

#[test]
fn recorded_handshakes_have_the_documented_structure() {
    for (name, count) in FIXTURES {
        let path = Path::new("tests/frames").join(format!("{name}.json"));
        let fixture = Fixture::load(&path).unwrap();
        assert_eq!(fixture.ephemeral, None, "{name}");
        let station = station_messages(&fixture);
        let ours = writes(&fixture);

        // 1: the challenge; 2: our reply from its seed alone (the temp IV),
        // so it is recomputed here without any key.
        let challenge = parse_kex(&station[0]).unwrap();
        assert_eq!((challenge.kind, challenge.payload.len()), (0x01, 4));
        let seed: [u8; 4] = challenge.payload.try_into().unwrap();
        let reply = TempKey::derive(seed, &[0; 16]).reply();
        assert_eq!(ours[0], encode_kex(0x02, &reply).unwrap(), "{name}");
        // 3: accepted.
        assert_eq!(station[1], encode_kex(0x03, &[0x00]).unwrap());
        // 4 and 5: a kex frame of point ‖ signature (128 bytes) each; 5 may
        // have been resent, identical.
        let key_frame = KEX_OVERHEAD + 128;
        assert_masked_envelope(&station[2], Layer::Temp, key_frame);
        let resends = ours[1..]
            .iter()
            .take_while(|w| w[..2] == ours[1][..2])
            .count();
        assert!(
            (1..=2).contains(&resends),
            "{name}: {resends} message 5 writes"
        );
        for msg5 in &ours[1..=resends] {
            assert_eq!(msg5, &ours[1]);
            assert_masked_envelope(msg5, Layer::Temp, key_frame);
        }
        // 6: a one-byte status in the temp layer.
        assert_masked_envelope(&station[3], Layer::Temp, KEX_OVERHEAD + 1);

        // Session: one read request and its reply per chunk, a fresh seed each.
        let (requests, replies) = (&ours[1 + resends..], &station[4..]);
        assert_eq!((requests.len(), replies.len()), (1, 1), "{name}");
        for request in requests {
            assert_masked_envelope(request, Layer::Session, read_request(0, count).len());
        }
        for reply in replies {
            assert_masked_envelope(reply, Layer::Session, read_response_len(count));
        }
        let mut seeds: Vec<&[u8]> = requests.iter().chain(replies).map(|e| &e[2..6]).collect();
        seeds.sort();
        seeds.dedup();
        assert_eq!(
            seeds.len(),
            requests.len() + replies.len(),
            "{name}: seed reused"
        );
    }
}

/// `N` bytes from a `keys.toml` hex field.
fn hex_field<const N: usize>(table: &toml::Table, name: &str) -> [u8; N] {
    let text = table[name].as_str().expect("hex string");
    decode_hex(text).unwrap().try_into().expect("field length")
}

/// Replays every `captures/hs-*.json` through [`Handshake`] with the real
/// keys and the recorded ephemeral secret: our replies must equal the
/// recorded writes byte for byte (signing is deterministic), and every session
/// frame must decrypt to a valid Modbus frame. Logs only whether a station's
/// point was the same across its sessions (§4.4 "Station public key").
#[test]
#[ignore = "needs captures/hs-*.json from --record-wire and the local keys.toml"]
fn real_capture_replays_with_the_local_keys() {
    let keys_file = std::env::var("BLUETTI_KEYS").unwrap_or_else(|_| "keys.toml".into());
    let keys = CryptoConfig {
        keys_file: keys_file.clone().into(),
    }
    .load_keys()
    .unwrap();
    let table: toml::Table = std::fs::read_to_string(&keys_file)
        .unwrap()
        .parse()
        .unwrap();
    let local_aes_key = hex_field::<16>(&table, "local_aes_key");

    let mut captures: Vec<_> = std::fs::read_dir("captures")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("hs-") && name.ends_with(".json")
        })
        .collect();
    captures.sort();
    assert!(!captures.is_empty(), "no captures/hs-*.json");

    let mut points: BTreeMap<String, Vec<[u8; 16]>> = BTreeMap::new();
    for path in &captures {
        let capture = Fixture::load(path).unwrap();
        let ephemeral: [u8; 32] = decode_hex(capture.ephemeral.as_deref().expect("ephemeral"))
            .unwrap()
            .try_into()
            .unwrap();
        let mut handshake = Handshake::new(&keys, &ephemeral).unwrap();
        let mut stream = Vec::new();
        let mut session: Option<Session> = None;
        let mut last_reply: Option<Vec<u8>> = None;
        let (mut count, mut temp, mut index, mut decrypted) = (None, None, 0, 0);
        for frame in &capture.frames {
            let bytes = decode_hex(&frame.hex).unwrap();
            match (frame.dir, &session) {
                (Dir::Write, None) => {
                    assert_eq!(Some(&bytes), last_reply.as_ref(), "{path:?}: our reply");
                }
                (Dir::Write, Some(s)) => {
                    let request = s.decrypt(&bytes).unwrap();
                    let n = u16::from_be_bytes([request[4], request[5]]);
                    let start = u16::from_be_bytes([request[2], request[3]]);
                    assert_eq!(request, read_request(start, n), "{path:?}: request");
                    count = Some(n);
                    decrypted += 1;
                }
                (Dir::Notify, _) => stream.extend(bytes),
            }
            loop {
                let layer = handshake.expecting();
                let Some(len) = envelope_len(layer, &stream).filter(|&l| l <= stream.len()) else {
                    break;
                };
                let msg: Vec<u8> = stream.drain(..len).collect();
                if let Some(s) = &session {
                    let reply = s.decrypt(&msg).unwrap();
                    parse_read_response(&reply, count.expect("reply after a request")).unwrap();
                    decrypted += 1;
                    continue;
                }
                if index == 0 {
                    let seed = parse_kex(&msg).unwrap().payload.try_into().unwrap();
                    temp = Some(TempKey::derive(seed, &local_aes_key));
                }
                if index == 2 {
                    // Message 4: keep a hash of the station point, never the point.
                    let kex = parse_kex(&open_temp(temp.as_ref().unwrap(), &msg).unwrap()).unwrap();
                    let hash: [u8; 16] = Md5::digest(&kex.payload[..64]).into();
                    let station = path.file_stem().unwrap().to_string_lossy();
                    let station = station.rsplit_once('-').map_or(&*station, |(s, _)| s);
                    points.entry(station.to_owned()).or_default().push(hash);
                }
                index += 1;
                match handshake.on_message(&msg).unwrap() {
                    Step::Reply { frame, .. } => last_reply = Some(frame),
                    Step::Wait => {}
                    Step::Done(s) => session = Some(s),
                }
            }
        }
        assert!(session.is_some(), "{path:?}: handshake not finished");
        assert!(stream.is_empty(), "{path:?}: trailing bytes");
        assert!(decrypted >= 2, "{path:?}: no session frames");
        println!(
            "{}: handshake replayed, {decrypted} session frames decrypted",
            path.display()
        );
    }
    for (station, hashes) in points.iter().filter(|(_, h)| h.len() > 1) {
        let same = hashes.windows(2).all(|w| w[0] == w[1]);
        let verdict = if same { "same" } else { "different" };
        println!(
            "{station}: station point across {} sessions: {verdict}",
            hashes.len()
        );
    }
}
