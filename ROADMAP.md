# bluetti-controller — roadmap, spec and research

The single source for **what** we build, **why**, **how**, and **in which order**.
Governance (golden rules, phase gates, git) lives in [`RULES.md`](./RULES.md); this
file is the governing spec that tasks cite as `ROADMAP.md §<n>`.

- **Status (2026-10-13):** Phase 1 / M0 *Foundation* closed, tag `v0.1-foundation` — research done, spec written,
  repo scaffolded, protocol layer coded and unit-tested, `make check` green.
  RULES.md and this ROADMAP approved by the owner (2026-10-13). The §14 answers
  are still needed before any hardware work (Phase 2).
- **Repo:** `serhiiromaniuk/bluetti-controller` (private).
- **Conventions in this file:** *unverified* means a fact comes from a single
  community source or a search snippet and still needs a hardware check. Every
  research fact carries a source in §15.

---

## Contents

1. Goal and context
2. Scope: MVP, later, not planned
3. Decisions (ADR log)
4. Research findings
5. Architecture
6. Data model
7. Derived metrics (algorithms)
8. Web UI and HTTP API
9. Configuration
10. Deployment and operations
11. Testing strategy
12. Milestones (detailed task lists)
13. Risk register
14. Open questions for the owner
15. Sources
- Appendix A — Station facts and operating notes
- Appendix B — Idea backlog

---

## 1. Goal and context

### 1.1 Problem

Scheduled and emergency power outages in Ukraine will continue for months. The
household runs on three Bluetti LiFePO4 power stations:

| Station | Role | Placement | Capacity |
|---|---|---|---|
| EB3A | Internet router (25–30 h runtime today) | permanent, plugged in (UPS mode) | 268.8 Wh |
| AC180 | PC, gas boiler, fridge — moved between rooms as needed | mobile | 1152 Wh |
| Elite 200 V2 | TV + PS5 | permanent, plugged in (UPS mode) | 2073.6 Wh |

The official Bluetti phone app shows one station at a time, needs the phone
nearby, keeps no useful history and cannot answer the questions that matter
during an outage: *when did the power go, how long will each station last at
the current load, how much energy did each load use?*

### 1.2 Goal

A tiny always-on service on a low-power homelab PC that reads all three stations
over local Bluetooth LE, keeps a history, and answers those questions on one web
page — **with no internet and no Bluetti cloud**, because outages also take the
internet down.

### 1.3 Constraints

- **Offline-first:** works with the internet down (RULES.md golden rule #1).
- **Low power:** the host is an x86_64 laptop or mini-PC that idles at 3–10 W.
- **Minimal:** one Rust binary, one SQLite file, one web page, no JS build step.
- **Safe:** read-only towards the stations in the MVP (golden rule #2).
- **Private:** no keys, MACs or serial numbers in the repo (golden rule #3).

### 1.4 Owner's answers (planning session, 2026-10-13)

| Question | Answer |
|---|---|
| Stations in the MVP | EB3A, AC180, Elite 200 V2 (all three) |
| MVP features | outage log, runtime forecast, energy accounting, any extra useful telemetry (temperature etc.) |
| After the MVP | outage-schedule integration, device control, battery health |
| Interface | minimal web UI only (no Telegram, no ntfy, no CLI dashboard) |
| Host | x86_64 laptop / mini-PC |
| Elite 200 V2 access | decided by research → local BLE (§3, ADR-002) |
| Repo | private, English |
| APK reverse-engineering | only if it is the easier path → it is not (§3, ADR-003) |

---

## 2. Scope

### 2.1 MVP (v1.0-mvp)

1. **Polling:** every 60 s, read each station over BLE (plaintext or encrypted).
2. **Storage:** every reading in SQLite; raw for 30 days, 5-minute rollups forever.
3. **Outage log:** when the grid disappeared and returned, per station and for the
   house; durations; daily/weekly totals.
4. **Runtime forecast:** per station, "lasts until HH:MM at the current load", and
   "if the power goes now" while on grid.
5. **Energy accounting:** energy in/out per station per day, self-consumption of
   the stations in UPS mode.
6. **Extra telemetry:** every additional field a station exposes (pack voltage,
   current, frequencies, energy counters, temperatures where available) stored
   for later use; hourly raw register snapshots so new fields can be decoded
   retroactively.
7. **Web UI:** one LAN page with station cards, grid banner, outage log, energy
   table, 24 h / 7 d charts, and a "pause polling" button so the phone app can
   connect.
8. **Ops:** systemd service on Debian, survives reboots and BLE hiccups.

### 2.2 After the MVP (planned)

- **R1 Battery health & telemetry discovery** — usable-capacity estimate from
  observed discharges, SOH/temperature on V2 stations, register sweeps on EB3A.
- **R2 Outage schedules** — YASNO (Kyiv, Dnipro) or the svitlo proxy for other
  regions: next planned outage, "will this station last through it?".
- **R3 Device control** — AC/DC on/off, charging mode; schedule-aware charging
  (Turbo before a planned outage, Silent otherwise); staggered charging when the
  grid returns so the breaker does not trip. Needs auth on the UI.

### 2.3 Not planned (decided, revisit only on request)

Notifications (Telegram, ntfy), Prometheus `/metrics`, Grafana, Bluetti cloud
access, a native Android app, ESP32 BLE proxies, multi-user auth, HTTPS on the
LAN. Each is listed in Appendix B with the reason it was parked.

---

## 3. Decisions (ADR log)

Each decision: context → decision → consequences. Do not relitigate without new
evidence; record a superseding ADR instead.

**ADR-001 — Pure Rust, single binary.** *Context:* owner asked for a minimal Rust
app with a DB on a low-power host. *Decision:* one Rust crate, one process, no
sidecars in the MVP. *Consequences:* the BLE protocol and the encryption
handshake are (re)implemented in Rust; a Python sidecar remains the fallback if
the Rust port stalls (ADR-002).

**ADR-002 — Read all three stations over local BLE; port the community handshake.**
*Context:* AC180 is encrypted on firmware since ~2024-07; Elite 200 V2 is
reported both encrypted and not. Options: (a) official cloud API — dead without
internet, its "BLE mode" still fetches a per-device key from the cloud; (b)
official crypto module — closed x86_64 `.so` for Python, needs a per-SN licence
CSV from support, no verified success story; (c) the community re-implementation
of the handshake (ECDH P-256 + AES-CBC with public static keys), MIT-licensed,
validated on AC180. *Decision:* (c), ported to Rust with crates `p256`, `aes`,
`cbc`, `md-5`. Encryption is detected per station from BLE manufacturer data on
every scan. *Consequences:* fully offline; no licence process; risk that a
firmware update rotates the static keys (§13 R2). Fallback order: Python sidecar
with `bluetti-bt-lib` → official module with licence CSV → cloud (remote only).

**ADR-003 — No APK reverse-engineering.** *Context:* owner asked whether
decompiling the official app would be more powerful. *Findings:* the static keys,
the handshake, the V1 maps and most of the V2 map are already public (MIT code);
the official cloud endpoints are in Bluetti's own MIT Home Assistant source.
Decompilation under EU Directive 2009/24/EC Art. 6 / Ukraine Law 2811-IX Art. 25 is
permitted only for information not readily available — which it is.
*Decision:* do not decompile. Fill gaps by (1) register sweeps on our own
stations, (2) Android HCI snoop logs of the official app talking to our own
stations, (3) the APK only as a last resort for a specific missing field.
*Consequences:* faster start, no legal grey zone.

**ADR-004 — Stack.** `tokio` + `bluer` (BlueZ D-Bus) + `axum` + `askama` +
`rusqlite` (bundled SQLite) + vendored `uPlot`, `toml` + `serde`, `clap`,
`tracing`. *Rejected:* `btleplug` (cross-platform layer we don't need, connect by
MAC needs a scan, open BlueZ abort bug); `sqlx` (async pool and macros, no gain
for one writer); `htmx` (v4 moved SSE to an extension — churn for three fetch
calls); `maud` (pins axum-core); Chart.js (3× the size of uPlot). Versions in
§4.9.

**ADR-005 — Storage layout.** SQLite in WAL mode, one writer task, a read-only
connection for the web. Core metrics in a wide `sample` table, everything else
in a long `sample_extra` table, 5-minute rollups after 30 days, migrations via
`PRAGMA user_version` + embedded SQL files (no `refinery`: it lags `rusqlite` and
causes duplicate `libsqlite3-sys` links).

**ADR-006 — UI.** Server-rendered HTML (askama) + plain `fetch` + `EventSource`
for live updates + uPlot for charts. No auth in the MVP (read-only, LAN-only);
an optional bearer token protects the only POST (pause polling). R3 control makes
the token mandatory.

**ADR-007 — Keys and identifiers outside the repo.** The static encryption keys
are public in other projects but are vendor material; they live in a local keys
file referenced by the config. MACs and serials live in the local config only.
Committed fixtures are anonymised. This keeps the repo publishable later.

**ADR-008 — Host power and placement.** Power the homelab host from the Elite
200 V2 (or its own battery), **not** from the EB3A: the router station would drop
from ~27 h to ~16 h runtime (§10.1). Place the host (or a USB BLE dongle on an
extension) within BLE range of all three stations.

---

## 4. Research findings

### 4.1 BLE GATT layout and naming

- Service `0000ff00-0000-1000-8000-00805f9b34fb`; notify characteristic `ff01`
  (responses), write characteristic `ff02` (requests); CCCD `2902`. [S1][S2][S3]
- Advertised name = `<MODEL><serial digits>`, e.g. `EB3A…`, `AC180…`. Elite 200 V2
  advertises `E200V2<SN>` [S10] or `Elite 200 V2<SN>` [S11]; its register type
  string is `Elite 200 V2` [S9]. AC180P is a different model (`AC180P…`).
- **Encryption flag:** manufacturer data under company id `0x4C42` ("BL") starts
  with ASCII `BLUETTI` (plaintext) or `BLUETTE` / `BLUETTF` (encrypted). Read it on
  every scan — encryption is firmware-driven, not model-driven. [S6][S12]

### 4.2 Frame format (plaintext)

- Modbus-RTU: `[unit=0x01][function][payload][CRC16 little-endian]`.
- Read holding registers: `01 03 <start:u16 BE> <count:u16 BE> <crc>` (8 bytes).
- Response: `01 03 <byte count> <data: count × u16 BE> <crc>`, length `5 + 2·count`.
- Write single `0x06`, write multiple `0x10` (post-MVP, golden rule #2).
- Exception: function | 0x80, one byte exception code.
- CRC-16/MODBUS: reflected poly 0xA001, init 0xFFFF. [S4][S5]
- **Fragmentation:** the 8-byte request fits one write; responses arrive split
  across several notifications and must be reassembled to the expected length.
  Writes larger than MTU are split to `mtu − 3` (min 20), write-without-response.
  [S2][S5]
- Implemented and tested in `src/protocol/modbus.rs` (M0).

### 4.3 Register maps

Values are big-endian `u16`; `scale` multiplies the raw value. *Source columns:*
bt = `bluetti-bt-lib` (validated per its README), mq = `bluetti_mqtt`,
v2 = `v2_device.py` (APK-derived generic V2 map; offsets there are **byte**
offsets, register = base + offset / 2).

#### EB3A — protocol V1 (plaintext, validated in bt)

| Reg | Field | Scale | Unit | Source |
|---|---|---|---|---|
| 10–15 | device type (ASCII, 6 regs) | — | — | bt |
| 17–20 | serial number (4 regs) | — | — | bt |
| 23, 25 | ARM / DSP firmware version | — | — | bt |
| 36 | DC input power | 1 | W | bt |
| 37 | AC input power | 1 | W | bt |
| 38 | AC output power | 1 | W | bt |
| 39 | DC output power | 1 | W | bt |
| 43 | battery SOC | 1 | % | bt |
| 48 | AC output on | bool | — | mq |
| 49 | DC output on | bool | — | mq |
| 77 | AC input voltage | 0.1 | V | bt |
| 86 | DC input voltage | 0.01 | V | bt |
| 91 | pack count max | 1 | — | mq |

Controls (writes, R3 only): 3007 AC output, 3008 DC output, 3034 LED mode
(1 low / 2 high / 3 SOS / 4 off), 3060 power off, 3063 ECO, 3064 ECO shutdown time
(1–4 h), 3065 charging mode (0 standard / 1 silent / 2 turbo), 3066 power lifting.
Poll blocks used by mq: (10, 40), (70, 21), (3034, 1), (3060, 7).
**Not exposed by any library:** temperature, current, cell voltages, energy
counters. Pack registers 96–121 exist on AC300; unverified on EB3A — R1 sweep.
[S1][S7][S8]

#### AC180 — protocol V2 (encrypted on current firmware, validated in bt)

| Reg | Field | Scale | Unit | Source |
|---|---|---|---|---|
| 100 | pack voltage | 0.01 | V | v2, *unverified* |
| 101 | pack current | 0.1 | A | v2, *unverified*; unsigned magnitude (Elite 200 V2: +11.2 A discharging 445 W, 2026-10-13) |
| 102 | battery SOC | 1 | % | bt |
| 103 | pack charging status | — | — | v2, *unverified* |
| 104 / 105 | time to full / time to empty | — | min? | v2, *unverified* |
| 110–115 | device type (byte-swapped ASCII) | — | — | bt |
| 116–119 | serial number | — | — | bt |
| 140 | DC output power | 1 | W | bt (v2 says u32 — word order to confirm) |
| 142 | AC output power | 1 | W | bt |
| 144 | DC/PV input power | 1 | W | bt |
| 146 | AC (grid) input power | 1 | W | bt |
| 150–159 | energy counters u32: DC out, AC out, PV charge, grid charge, feedback | 0.1 | kWh | v2, *unverified* |
| 160 | charging mode | enum | — | v2, *unverified* |
| 1213 | DC input voltage | 0.1 | V | bt |
| 1214 | DC input current | 0.1 | A | bt |
| 1300 | AC input frequency | 0.1 | Hz | bt |
| 1314 | AC input voltage | 0.1 | V | bt |
| 1315 | AC input current | 0.1 | A | bt |
| 1500 | AC output frequency | 0.1 | Hz | bt |
| 1511 | AC output voltage | 0.1 | V | bt |
| 2011 / 2012 | AC / DC output on | bool | — | bt |
| 6003 / 6004 | pack voltage / current (pack block) | — | — | v2, *unverified* |
| 6007 | pack average temperature | ? (°F suspected) | — | v2, *unverified* |
| 6175 | BMS firmware version | — | — | bt |

Controls (R3 only): 2011 AC, 2012 DC, 2014–2016 ECO DC (switch, time, min power),
2017–2019 ECO AC, 2020 charging mode, 2021 power lifting. [S1][S9][S13]

#### Elite 200 V2 — protocol V2 (*provisional*)

No merged profile upstream. PR #49 in bt (open since 2026-02-10) proposes the base
V2 map plus: 104 time remaining, 152 battery voltage /100 (conflicts with the
energy counter at 152 in v2), 1315 AC input current, 1432 AC output current, 1470
AC output frequency. Generic V2 readers (nhurman fork) read it. An Elite-specific
MIT bridge exists (JordiGrasvi/bluetti-elite200v2-mqtt). Everything is
*unverified* until our M2 dump. [S9][S11][S14]

The M0 code (`src/protocol/device.rs`) implements these maps with a
`verified` flag per field.

### 4.4 Encryption (V2 / BLUETTE stations)

Read at pinned commits, all MIT: nhurman `encryption.py` [S35], the
`bluetti-bt-lib` port of it (same code + fragment reassembly) [S36], `rustytti`
`crypto.rs`/`session.rs` (Rust, p256/aes/cbc/md-5, AC50P) [S37]. The three agree
byte for byte; the static values are identical in all three (compared by hash).
The GPL `x3ccd4828/bluetti-encryption` was not opened.

**Framing.** Three shapes on the wire (ff01 notify / ff02 write, §4.1):

- *Kex frame* (plaintext): `2A 2A` · type u8 · len u8 · payload[len] ·
  checksum u16 BE = wrapping sum of the type, len and payload bytes. A
  notification that starts with `2A 2A` is a kex frame; test this only when the
  reassembly buffer is empty, since a continuation fragment can start with those
  bytes by chance.
- *Temp envelope*: plain_len u16 BE · AES-128-CBC ciphertext. No magic, no
  seed; the IV is the per-session temp IV.
- *Session envelope*: plain_len u16 BE · seed[4] · AES-256-CBC ciphertext,
  IV = `md5(seed)` (seed **not** reversed), a fresh random seed per frame, both
  directions.
- Padding: zeros up to a 16-byte multiple (not PKCS#7); strip it with plain_len.
  Envelope size = header (2 or 6) + `ceil(plain_len/16)*16`, so an envelope that
  spans several notifications is reassembled by length [S36][S37].

**Sequence** (the station speaks first: subscribe to ff01, then the challenge
arrives as a notification) [S35][S37]:

| # | Dir | Wire shape | Plaintext | Key / static value used |
|---|---|---|---|---|
| 1 | st→us | kex frame | type `01`, len 4, seed[4] | none |
| 2 | us→st | kex frame, ATT **Write Request** | type `02`, len 4, temp_iv[8..12] | none: temp_iv = `md5(reverse(seed))`; temp_key = temp_iv XOR *local AES key* |
| 3 | st→us | kex frame | type `03`, len 1, `00` = accepted | none |
| 4 | st→us | temp envelope wrapping a kex frame | type `04`, len `80`: station point[64] · sig[64] | *station verifying key* checks sig over point ‖ temp_iv |
| 5 | us→st | temp envelope wrapping a kex frame, Write Command | type `05`, len `80`: our ephemeral point[64] · sig[64] | *host signing key* signs our point ‖ temp_iv |
| 6 | st→us | temp envelope wrapping a kex frame | type `06`, len 1, `00` = accepted | session_key = ECDH(our ephemeral secret, station point) |
| 7+ | both | session envelope | Modbus RTU frame (§4.2) with its own CRC | session_key |

- Curve P-256 (secp256r1). Points are raw X‖Y (64 bytes, SEC1 uncompressed
  without the `04`). Signatures are ECDSA-SHA256, raw r‖s (2×32 bytes BE), not
  DER. The session key is the raw 32-byte ECDH x-coordinate used directly as an
  **AES-256** key (no KDF); only the temporary key is AES-128.
- Write type (rustytti, matching the official app): message 2 must be a Write
  Request (the station drops a Write Command); 5 and later are Write Commands.
  If message 6 has not arrived ~1 s after message 5, resend the **identical**
  message 5 ciphertext once; a new ephemeral pair is not accepted. rustytti
  waits 8 s for message 1, 5 s for 4, 2.5 s + 1.5 s for 6, and after the
  handshake sends a session-encrypted `01` ping the way the app does; nhurman
  and bluetti-bt-lib skip the ping and still read AC180-class stations.
- **Static values, by role** (shared by every station, extracted from the app
  and firmware, not per station): *local AES key* (16 B), XORed into the temp
  key at step 2; *host signing key* (P-256 private scalar, 32 B), signs message 5;
  *station verifying key* (P-256 public key), verifies message 4. The DER SPKI
  prefix for P-256 that nhurman also carries is a public ASN.1 constant: p256's
  SEC1 parsing does not need it.
- **`BLUETTE` vs `BLUETTF`:** no source has a different handshake per flag.
  nhurman's `is_device_using_encryption` returns true only when the
  manufacturer data equals `BLUETTF` exactly (constant `ENCRYPTED_ESP32_HEX`);
  `BLUETTE` is declared but unused. bluetti-bt-lib ignores the advert and tries
  encrypted first, then plaintext; rustytti matches on the name. The meaning of
  the E/F split is unknown. We treat both as encrypted (§4.1) and `detect` logs
  the flag; a `BLUETTE` station that sends no challenge is a finding, not a retry.
- **MAC / SN:** no handshake message carries either. The signatures bind only
  to the point and the session's temp IV. Session payloads may carry the SN in
  Modbus registers, which the existing anonymiser masks.
- **Station public key:** it travels inside the temp layer, so anyone with the
  public *local AES key* and the sniffed seed can read it. It is signed with the
  station's static signing key, the counterpart of the *station verifying key*.
  All three sources run a full handshake on every connection and never cache
  the point; whether the station's point changes between sessions is not stated
  anywhere. Step 13 settles it by comparing a hash of the point across two
  sessions (only same/different is logged). The design never depends on it:
  verify the signature and run ECDH on every connection.
- Open risks: a high-S station signature is accepted by nhurman (OpenSSL DER
  verify); p256 verify behaviour on high-S is untested here, though rustytti
  works on AC50P with p256 0.13. The checksum sum cannot overflow u16 (max body
  130 × 255).

**`keys.toml` schema** (gitignored, at the repo root for development, path from
`[crypto].keys_file` §9; `deny_unknown_fields`; hex without `0x` or separators,
either case; the loader names a bad field, never its value):

| Field | Hex chars | Content |
|---|---|---|
| `local_aes_key` | 32 | 16-byte AES key XORed with `md5(reverse(seed))` |
| `host_signing_key` | 64 | 32-byte P-256 private scalar, big-endian |
| `station_verifying_key` | 130 | 65-byte SEC1 uncompressed point, starts `04` (the last 65 bytes of nhurman's 91-byte DER form) |

**Committable artefacts** (golden rule #3). Allowed: messages 1–3 verbatim (a
random seed, IV bytes and a 1-byte status: no key material); the header of every
envelope (plain_len, and the seed for session envelopes) and its total length,
with the ciphertext replaced by zeros of the same length; derived facts (type
sequence, lengths, same/different). Never committed: message 4 or 6 ciphertext
(the seed and the public local AES key decrypt them, exposing the station
point), the station point or our ephemeral key in any form, the temp key, the
session key, any static value, MAC/SN, and decrypted payloads that have not been
through the anonymiser. Raw captures stay in gitignored `captures/`.

**Recommendation:** port the state machine from `rustytti` (Rust, MIT) [S37],
cross-checked against nhurman [S35] and credited in `README.md`. Use the
RustCrypto crates in §4.9 at their current majors (rustytti is on the previous
generation: p256 0.13, aes 0.8, cbc 0.1, md-5 0.10), with block-wise CBC and our
own zero padding. Keep `protocol/crypto` pure: the ephemeral secret and the
session seeds are inputs, and the RNG lives in `transport/`. Messages 2 and 5
need a Write Request vs a Write Command, so the `Gatt` trait needs both write
kinds. Test split: (1) `make check` uses **synthetic keys**: test-generated
static values and a simulated station drive a full round trip, plus the bad
signature, out-of-order and truncated cases; NIST SP 800-38A F.2.1/F.2.5 CBC
vectors [S39]. (2) **Key-independent recorded vectors**, also in `make check`:
rustytti's captured challenge `2a2a0104ffdf22b702bc` → reply
`2a2a02045a4fd30e0190` (message 2 depends only on the seed; recomputed here),
plus step 13's structure-only fixtures. (3) A **real-key `#[ignore]` test**
replays a gitignored capture with the local `keys.toml`. It decrypts messages
4 and 6 with the temp key, verifies the station signature, and checks that our
message 5 verifies; a full session decrypt needs that run's ephemeral secret,
which is stored only in `captures/`.

Re-implementations: `nhurman/bluetti_mqtt` (Python, MIT), `bluetti-bt-lib` port
(Python, MIT), `j4m3z0r/bluetti-monitor` (Python, MIT), `rustytti` (Rust, MIT,
AC50P only) — **use `rustytti` and nhurman as references**;
`x3ccd4828/bluetti-encryption` is GPLv3 and must not be copied. [S6][S16][S17][S35][S36][S37]

Official alternative: `bluetti-official/bluetti-bluetooth-lib` 1.0.0 — closed
x86_64 SWIG Python extension (`ble_crypt_link_handler` status 0→4,
`encrypt_data`, `decrypt_data`), needs `bluetti_device_licence.csv` per serial from
support; no LICENSE file; no confirmed success. Not used (ADR-002). [S18]

### 4.5 Which station is encrypted

- **EB3A:** no report of encryption (absence unverified) → plaintext expected.
- **AC180:** encrypted on units/firmware since ~2024-07; "latest firmware" needs
  encryption (2025-07); a plaintext AC180P was seen in 2026-06. [S12][S19]
- **Elite 200 V2:** conflicting (#131 scanned encrypted; bt #86 says no) → detect.

### 4.6 Polling behaviour and BLE reliability

- **One GATT connection per station.** While the phone app is connected (even in
  the background) the station stops advertising — force-quit the app. [S5][S17]
- bluetti_mqtt: 5 s reply timeout, 5 retries, 5 s sleep after disconnect; an
  `AT+NAME?` notification means a bad connection. [S2]
- Home Assistant users: AC180 stable only at a 60 s poll / 120 s timeout / 10
  retries, still drops after hours. [S19]
- A killed client process can leave the BlueZ link up ("device not found") —
  `bluetoothctl disconnect <mac>`; stale GATT cache → `bluetoothctl remove`. [S5][S20]
- Onboard Wi-Fi/BT combo chips often abort connections (HCI 0x3e); a USB adapter
  (CSR8510, BCM20702) on a USB 2.0 port with a short extension is more stable;
  BT 5.x gives no advantage here. [S20][S21]
- Connect latency: ~60 ms to several seconds depending on scan parameters;
  serialise connections to multiple devices. [S20][S22]

### 4.7 Official cloud (not used)

`bluetti-official/bluetti-home-assistant` (MIT, v1.0.5, 2026-09-16): OAuth2
(`/oauth2/grant`, `/oauth2/token`, hosts from a remote profile), REST
`/api/bluiotdata/ha/v2/devices`, `/ha/v1/deviceStates`, `/ha/v1/fulfillment`,
push via STOMP over WebSocket. Elite 200 V2 supported, AC180 not listed. No
public docs, rate limits or ToS. Dead without internet. [S23]

### 4.8 Prior art in Rust

| Project | What | Licence | Use |
|---|---|---|---|
| `rustytti` 0.1.1 (crates.io) | btleplug + p256 + aes; V2 handshake + Modbus; AC50P profile | MIT | reference for M2 |
| `v0l/battery-control` crates/bluetti | plaintext V1 (EB3A, AC300…) | none stated | read-only reference |
| `x3ccd4828/bluetti-encryption` | no_std handshake | GPLv3 | do not copy |
| `tab-liu/ble_gui` | Slint desktop tool | — | ignore |

None covers all three of our stations. [S16][S24]

### 4.9 Rust stack (crates.io, 2026-10-13)

| Crate | Version | Notes |
|---|---|---|
| tokio | 1.53.2 | rt-multi-thread, macros, signal, time, sync |
| bluer | 0.17.4 | features `bluetoothd`; needs `libdbus-1-dev`; wrap every GATT op in a timeout, retry connect 2–3×; open issues #167 (intermittent "connection refused"), #182 (`write()` can hang) |
| axum | 0.8.9 | built-in SSE, graceful shutdown |
| askama | 0.16.1 | `Html(tpl.render()?)` |
| rusqlite | 0.40.2 | `bundled` |
| serde / serde_json | 1.0.229 / 1.0.154 | |
| toml | 1.1.8 | |
| clap | 4.6.7 | derive |
| tracing / tracing-subscriber | 0.1.44 / 0.3.23 | env-filter, stdout → journald |
| anyhow / thiserror | 1.0.104 / 2.0.21 | |
| p256 / aes / cbc / md-5 | 0.14.0 / 0.9.3 / 0.2.1 / 0.11.0 | RustCrypto, MIT OR Apache-2.0; p256 features `ecdh`, `ecdsa`; AES-128 temp + AES-256 session CBC, md5 IVs (§4.4); aes needs Rust ≥ 1.89 [S38] |
| uPlot (vendored JS) | 1.6.32 | 21.9 KB gzipped vs Chart.js 68.4 KB |

Toolchain: rustup with Rust 1.99.0 pinned in `rust-toolchain.toml` (rustfmt +
clippy included); the distro `rustc` 1.93.1 is shadowed by `~/.cargo/bin`. [S25]

### 4.10 Outage-schedule sources (for R2)

- **YASNO** (Kyiv, Dnipro only), undocumented public JSON, no auth,
  `Cache-Control: max-age=300`. Base
  `https://app.yasno.ua/api/blackout-service/public/shutdowns/`:
  `addresses/v2/regions`; `regions/{r}/dsos/{d}/planned-outages` → per group
  (`"1.1"`…) `today`/`tomorrow` slots `{start, end}` in minutes from midnight,
  `type: NotPlanned|Definite`, `status: ScheduleApplies|NoOutages|WaitingForSchedule|EmergencyShutdowns`;
  `probable-outages?regionId&dsoId` (weekly); address → group via
  `addresses/v2/streets|houses|group`. Reference client: `ha-yasno-outages`
  (polls 15 min, keeps cache on error). [S26]
- **svitlo proxy** (unofficial Cloudflare Worker, 16 regions incl. Kyiv region,
  Lviv, Odesa, Kharkiv): half-hour slots, 1 = on, 2 = off, queues 1.1–6.2, ETag,
  poll 600 s; the author warns the free tier is near its limits. [S27]
- energy-ua.info and DTEK portals: scraping behind anti-bot protection — avoid.
- No national API. Queues are `group.subgroup` (черга.підчерга).

### 4.11 Homelab host

- Idle draw: N100 mini-PC 5.5–8 W at the wall; laptops ~3–10 W with the lid
  closed. Measure the real unit. [S28]
- Debian 13 ships BlueZ 5.82 (Debian 12: 5.66); Home Assistant recommends BlueZ
  ≥ 5.63 and kernel ≥ 5.15.62. [S21][S29]
- Debian's D-Bus policy lets any local user act as a BLE client; adapter resets
  need root / `CAP_NET_ADMIN`. [S30]
- Laptop: `HandleLidSwitch=ignore` and `HandleLidSwitchExternalPower=ignore` in
  logind; `powertop --auto-tune`; if the BT dongle misbehaves under autosuspend,
  TLP `USB_EXCLUDE_BTUSB=1` or `btusb.enable_autosuspend=0`; battery charge
  thresholds via TLP where the vendor supports them. [S31]

---

## 5. Architecture

### 5.1 Components

```
+---------------------------- bluetti-controller (one process) ----------------------------+
|                                                                                          |
|  config.rs --> collector (poll loop, 1 task) --> transport (bluer, the ONLY BlueZ user)  |
|                    |          ^                       |                                  |
|                    |          | frames                v                                  |
|                    |       protocol (pure): modbus, crypto, device maps, decode           |
|                    v                                                                     |
|              store (SQLite, 1 writer task) <---- metrics (outages, runtime, energy)      |
|                    |                                                                     |
|              web (axum): HTML pages, JSON API, SSE  <---- broadcast of new samples        |
+------------------------------------------------------------------------------------------+
        BlueZ (D-Bus) <-> BLE adapter <-> EB3A / AC180 / Elite 200 V2
```

### 5.2 Modules (`src/`)

| Module | Responsibility | Milestone |
|---|---|---|
| `protocol::modbus` | frames, CRC, response parsing | M0 ✓ |
| `protocol::device` | model detection, encryption flag, register maps, decoding | M0 ✓ |
| `protocol::crypto` | BLUETTE handshake + AES-CBC wrapping, pure state machine | M2 |
| `transport` | `Gatt` trait; `BluerGatt` (BlueZ) and `ReplayGatt` (fixtures); connect/notify/write with timeouts; reassembly | M1 |
| `config` | load + validate TOML, fail fast | M1 |
| `cli` | `scan`, `detect`, `read`, `dump`, `serve` subcommands | M1 / M3 |
| `collector` | the poll loop: per station connect → (handshake) → read blocks → disconnect | M3 |
| `store` | SQLite: migrations, single writer, read-only pool, retention, rollups | M3 |
| `metrics` | grid state, outage events, runtime forecast, energy integration | M3 |
| `web` | axum routes, askama templates, SSE, embedded assets | M4 |

### 5.3 Concurrency and lifecycle

- `tokio` runtime. One **collector** task iterates stations **sequentially**
  (golden rule #5), sleeping until the next interval; a `watch` channel carries
  shutdown and pause.
- Readings go over an `mpsc` channel to the **store writer** task, which owns the
  only write connection and runs `metrics` on each new sample.
- A `broadcast` channel pushes new samples and events to SSE clients.
- Web handlers read via a second, read-only SQLite connection in
  `spawn_blocking`.
- Shutdown on SIGINT/SIGTERM: stop polling, disconnect any open link, flush the
  writer, close the server gracefully.

### 5.4 One poll, step by step

1. Ensure the station is known to BlueZ (by MAC from config; short filtered
   discovery if BlueZ has not seen it).
2. Read the advertisement: name → model check, manufacturer data → encrypted?
3. Connect (timeout 15 s, up to 3 attempts with backoff).
4. Subscribe to `ff01`; if encrypted, run the handshake (M2).
5. For each register block of the profile: write the read request, reassemble
   the response (timeout 5 s), validate CRC/length, collect registers.
6. Disconnect (always, also on error).
7. Decode → `Sample` (core metrics + extras) → store. On failure → `poll_failure`
   row with the stage that failed.

### 5.5 Error handling

- Transport errors are retried inside the poll; a failed poll is recorded, never
  fatal. Three consecutive failures mark the station `unreachable` (distinct from
  "grid lost").
- CRC/length errors are logged with the frame hex (MAC/SN masked) for debugging.
- A handshake failure is surfaced in the UI as "encryption failed — firmware
  change?" (§13 R2).

---

## 6. Data model

SQLite file at `storage.path`; `PRAGMA journal_mode=WAL; synchronous=NORMAL;
busy_timeout=5000`; weekly `wal_checkpoint(TRUNCATE)` + `optimize`.

```sql
-- migrations/001_init.sql
CREATE TABLE station (
  id           INTEGER PRIMARY KEY,
  name         TEXT NOT NULL UNIQUE,      -- from config, e.g. "router"
  model        TEXT NOT NULL,             -- EB3A | AC180 | Elite200V2
  capacity_wh  REAL NOT NULL,
  grid_sensor  INTEGER NOT NULL DEFAULT 0 -- counts towards the house outage state
);

-- one row per successful poll; core metrics only
CREATE TABLE sample (
  station_id INTEGER NOT NULL REFERENCES station(id),
  ts         INTEGER NOT NULL,            -- unix seconds, UTC
  soc        REAL,                        -- %
  ac_in_w    REAL, dc_in_w  REAL,
  ac_out_w   REAL, dc_out_w REAL,
  ac_in_v    REAL,
  encrypted  INTEGER NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;

-- every other decoded field, verified or not (extra telemetry)
CREATE TABLE sample_extra (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  metric     TEXT NOT NULL,               -- Metric::key()
  value      REAL NOT NULL,
  PRIMARY KEY (station_id, ts, metric)
) WITHOUT ROWID;

CREATE TABLE poll_failure (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  stage      TEXT NOT NULL,               -- discover|connect|handshake|read|decode
  error      TEXT NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;

-- hourly raw register blocks, for decoding new fields later (90-day retention)
CREATE TABLE register_snapshot (
  station_id INTEGER NOT NULL,
  ts         INTEGER NOT NULL,
  start_reg  INTEGER NOT NULL,
  data       BLOB NOT NULL,               -- big-endian u16 values
  PRIMARY KEY (station_id, ts, start_reg)
) WITHOUT ROWID;

-- grid presence per station and for the house; ended_at NULL = ongoing
CREATE TABLE grid_event (
  id          INTEGER PRIMARY KEY,
  scope       TEXT NOT NULL,              -- 'house' or a station name
  kind        TEXT NOT NULL,              -- 'outage'
  started_at  INTEGER NOT NULL,
  ended_at    INTEGER,
  precision_s INTEGER NOT NULL            -- = poll interval at detection
);

-- 5-minute rollups, kept forever
CREATE TABLE rollup_5m (
  station_id    INTEGER NOT NULL,
  ts            INTEGER NOT NULL,         -- bucket start
  soc_avg REAL, soc_min REAL,
  ac_in_w_avg REAL, dc_in_w_avg REAL, ac_out_w_avg REAL, dc_out_w_avg REAL,
  energy_in_wh  REAL, energy_out_wh REAL,
  n             INTEGER NOT NULL,
  PRIMARY KEY (station_id, ts)
) WITHOUT ROWID;
```

**Volume:** 60 s interval → 1440 samples/station/day → ~43k raw rows per station
in the 30-day window, ~105k rollup rows per station-year. The DB stays in the tens
of MB.

**Retention job (nightly):** roll up samples older than 30 days into
`rollup_5m`, delete them; delete `register_snapshot` older than 90 days;
`poll_failure` older than 90 days. Events are kept forever.

---

## 7. Derived metrics

### 7.1 Grid presence per station

- Present when `ac_in_v ≥ grid_voltage_min` (default 180 V). Fallback when the
  voltage field is missing: `ac_in_w > 0`.
- A change of state is accepted after `debounce_samples` consecutive samples
  (default 2). The event time is backdated to the first sample in the new state.
- No successful poll for > 3 intervals → state `unknown`, never "outage".

### 7.2 House outage log

- Stations with `grid_sensor = true` (default: EB3A and Elite 200 V2 — both stay
  plugged in; the AC180 moves) vote.
- **Outage starts** when every reachable sensor station reports no grid.
- **Outage ends** when any sensor station reports grid again.
- If all sensors are unreachable, the house state is `unknown`.
- Precision = poll interval; stored with each event.
- Daily/weekly totals: number of outages, total hours without power, longest.

### 7.3 Runtime forecast

```
load_w      = EWMA(ac_out_w + dc_out_w), time constant ~10 min
remaining_wh = capacity_wh × soc / 100 × usable_factor     (default 0.85)
runtime_h   = remaining_wh / max(load_w, 1)
```

- On battery: show "lasts until HH:MM".
- On grid: show "if the power goes now: N h".
- `usable_factor` covers inverter losses and the BMS cut-off; R1 calibrates it
  per station from observed full discharges.
- Where a station reports its own time-to-empty (V2 regs 104/105, unverified),
  show it next to ours once verified.

### 7.4 Energy accounting

- Integrate power over time with the trapezoid rule between consecutive samples;
  skip gaps longer than 3 intervals (counted as missing coverage).
- Per station per day: `energy_in_wh` (AC + DC input), `energy_out_wh` (AC + DC
  output), `Δstored_wh = capacity_wh × Δsoc / 100`.
- **Self-consumption / losses** ≈ `energy_in − energy_out − Δstored` — what UPS
  mode costs per day.
- Cross-check with the V2 lifetime energy counters (regs 150–159) once verified.

### 7.5 Extra telemetry

- Every decoded field outside the core goes to `sample_extra` (with its
  `verified` flag known from the profile).
- Unverified fields are shown on a "raw telemetry" page only; never in alerts or
  the forecast (RULES.md conventions).

---

## 8. Web UI and HTTP API

### 8.1 Pages

- **`/` status:** house grid banner (state + since when); one card per station
  (name, model, SOC bar, in/out W, grid state, runtime forecast, last seen, poll
  health, encrypted yes/no); outage log (last 20); energy table (last 7 days);
  charts: SOC and power per station, 24 h with a 7 d toggle (uPlot).
- **`/telemetry`:** every stored metric per station, verified and unverified,
  latest value and sparkline.
- **"Pause polling" button:** stops BLE polling for N minutes (max 60) so the
  official phone app can connect.

### 8.2 API

| Method | Path | Returns |
|---|---|---|
| GET | `/api/status` | current state of all stations + house |
| GET | `/api/series?station=&metric=&from=&to=&step=` | time series (raw or rollup by range) |
| GET | `/api/outages?from=&to=` | grid events |
| GET | `/api/energy?days=7` | daily energy per station |
| GET | `/events` | SSE: `sample`, `grid_event`, `poll_failure` |
| POST | `/api/polling/pause?minutes=N` | pause polling (token if configured) |
| GET | `/healthz` | liveness + last successful poll age |

### 8.3 Constraints

- Server-rendered with askama; assets embedded in the binary; no JS build step;
  page weight < 100 KB.
- Binds to `web.listen`; LAN only; the host firewall blocks other networks.
- No auth for GET in the MVP; optional bearer `web.token` for the POST.

---

## 9. Configuration

`/etc/bluetti-controller/config.toml` (never committed; a sanitised
`config.example.toml` is added in M1):

```toml
[general]
poll_interval_secs = 60
timezone = "Europe/Kyiv"

[storage]
path = "/var/lib/bluetti-controller/data.db"
raw_retention_days = 30

[web]
listen = "0.0.0.0:8080"
# token = "change-me"            # required for POST endpoints when set

[ble]
adapter = "hci0"
connect_timeout_secs = 15
request_timeout_secs = 5
connect_attempts = 3

[crypto]
# Static keys for encrypted (BLUETTE) stations, ROADMAP.md §4.4. Never commit.
keys_file = "/etc/bluetti-controller/keys.toml"

[metrics]
grid_voltage_min = 180.0
debounce_samples = 2
usable_capacity_factor = 0.85

[control]
enabled = false                    # RULES.md golden rule #2 — never default true

[[station]]
name = "router"
model = "EB3A"
address = "AA:BB:CC:DD:EE:01"
grid_sensor = true

[[station]]
name = "mobile"
model = "AC180"
address = "AA:BB:CC:DD:EE:02"
grid_sensor = false

[[station]]
name = "living-room"
model = "Elite200V2"
address = "AA:BB:CC:DD:EE:03"
grid_sensor = true
```

---

## 10. Deployment and operations

### 10.1 Host

- x86_64 laptop or mini-PC, Debian 13 (BlueZ 5.82), within BLE range of all
  three stations (or a USB BT adapter on a short extension).
- **Power it from the Elite 200 V2 or its own battery, not the EB3A.** The
  router alone draws ~9 W (≈240 Wh usable / 27 h); a 6 W host would cut that to
  ~16 h. On the Elite, 6 W is ~144 Wh/day (≈7%).
- Laptop: lid switch ignored, `powertop --auto-tune`, battery charge thresholds
  if supported.

### 10.2 Install (human-only steps, M5)

```bash
sudo apt install bluez libdbus-1-0      # runtime; build host also needs libdbus-1-dev pkg-config
sudo useradd --system --home /var/lib/bluetti-controller bluetti
sudo install -m 0755 target/release/bluetti-controller /usr/local/bin/
sudo install -d -m 0750 -o bluetti /etc/bluetti-controller
sudo install -m 0640 -o bluetti config.toml keys.toml /etc/bluetti-controller/
busctl --system introspect org.bluez /org/bluez   # as the service user: D-Bus access works
```

### 10.3 systemd unit (shipped in `deploy/` at M5)

```ini
[Unit]
Description=bluetti-controller
After=bluetooth.service network-online.target
Wants=bluetooth.service

[Service]
User=bluetti
SupplementaryGroups=bluetooth
StateDirectory=bluetti-controller
ExecStart=/usr/local/bin/bluetti-controller serve --config /etc/bluetti-controller/config.toml
Restart=on-failure
RestartSec=10
ProtectSystem=strict
ProtectHome=yes
NoNewPrivileges=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```

### 10.4 Pre-flight checklist (before any hardware run)

- [ ] Official app force-quit on every phone near the stations.
- [ ] `bluetoothctl show` → adapter powered; `rfkill list` → not blocked.
- [ ] Station firmware versions noted from the app (device info).
- [ ] `control.enabled = false` in the config.
- [ ] Captures go to `captures/` (gitignored), anonymised before becoming fixtures.

### 10.5 Runbook

| Symptom | Action |
|---|---|
| "device not found" after a crash | `bluetoothctl disconnect <mac>` |
| Repeated GATT errors on one station | `bluetoothctl remove <mac>`, let discovery re-add it |
| Adapter stuck | replug the USB dongle (note: may renumber `hci0` → `hci1`) |
| Station unreachable | check the phone app is closed; move the dongle; pause/resume polling |
| Handshake fails on a station that worked | firmware update? Compare manufacturer data, read §13 R2 |

---

## 11. Testing strategy

- **Unit (pure):** framing, CRC, model detection, encryption flag, decoding with
  hand-computed values, handshake against recorded vectors, metrics arithmetic
  (outage debounce, gaps, EWMA, trapezoid integration).
- **Replay:** `Gatt` trait with `ReplayGatt` driven by fixtures
  `tests/frames/<model>.json` → `[{t_ms, dir, uuid, hex}]`, recorded with
  `--record`, anonymised (MAC/SN masked) before commit.
- **Hardware:** `#[ignore]` tests that need `BLUETTI_TEST_MAC`; run by a human.
- **Acceptance:** scripts that measure the RULES.md gate numbers from the DB
  (poll success ratio, outage detection delay, DB size, RSS).
- **Coverage:** `cargo llvm-cov` from M1, ≥ 80% on `protocol` and `metrics`.
- **No CI/CD** (owner's decision): `make check` runs locally before every commit.

---

## 12. Milestones

Effort is in focused sessions (~2–4 h). Phases map to RULES.md: Phase 1 = M0,
Phase 2 = M1 + M2 (merged by the owner 2026-10-13: the EB3A is encrypted; M2
covers the EB3A and Elite 200 V2, the AC180 is a `HUMAN-CHECKLIST.md` row),
Phase 4 = M3, Phase 5 = M4 + M5 (M5 soak on the dev laptop as a user-level
systemd unit). **Human:** items below that compare with the app, unplug a
station or need the AC180 are deferred to `HUMAN-CHECKLIST.md` and do not block
(RULES.md "Deferred physical checks"); read-only station access is unattended.

### M0 — Foundation → `v0.1-foundation` (Phase 1)

- [x] Research: protocol, register maps, encryption, cloud, Rust stack, host,
      outage schedules (§4).
- [x] Decisions recorded (§3).
- [x] This ROADMAP; RULES.md, AGENTS.md, loop, task ledger, subagents, Makefile,
      `.gitignore` (agent-readiness scaffold). No CI/CD by decision.
- [x] `src/protocol/modbus.rs`: request builder, CRC-16/MODBUS, response parser.
- [x] `src/protocol/device.rs`: model from BLE name, encryption flag, register
      maps (EB3A, AC180, provisional Elite 200 V2), decoding with plausibility.
- [x] 11 unit tests green (`cargo test`).
- [x] rustup + `rust-toolchain.toml` (1.99.0, rustfmt, clippy); `make check` green locally.
- [x] Owner approved RULES.md: golden rules, phase table, thresholds (2026-10-13).
- [x] Owner approved this ROADMAP (2026-10-13). §14 Q2–Q8 stay open; none blocks M0.

**Acceptance:** `make check` green; owner sign-off recorded in the task LOG.

### M1 — BLE transport + CLI, EB3A → `v0.2-ble` (Phase 2)

- [ ] Add deps: tokio, bluer, clap, tracing, serde, toml, anyhow, thiserror;
      `libdbus-1-dev` on the dev host (human).
- [ ] `config.rs`: TOML schema (§9), validation, `config.example.toml`.
- [ ] `transport::Gatt` trait; `BluerGatt`: find by MAC (fallback filtered
      discovery), connect with timeout/retries, subscribe `ff01`, write `ff02`,
      reassemble responses to the expected length, always disconnect.
- [ ] `ReplayGatt` + `--record` to capture fixtures; anonymiser for MAC/SN.
- [ ] CLI: `scan` (name, RSSI, model, encrypted flag), `detect <mac>` (model,
      protocol, encryption, device type and firmware registers), `read <mac>`
      (decoded metrics), `dump <mac> --from --to` (read-only sweep, golden rule #2).
- [ ] Run `scan` near all three stations (read-only, may run unattended); record
      which are encrypted and their firmware versions (§14 Q5).
- [ ] EB3A acceptance run — 20 polls at 60 s (unattended); **Human:** compare with
      the official app and sign off;
      mark the fields it confirms `verified: true` (all start `false`).
- [ ] Exception-aware reassembly length (loop/STATE.md CF-3).
- [ ] Record an EB3A fixture; replay test for the full read path.
- [ ] Wire `cargo llvm-cov` into `make check` (≥ 80% on `protocol`).

**Acceptance (RULES.md):** EB3A 19/20 polls OK, SOC equal to the app, power ±5 W.

### M2 — Encryption + V2 stations → `v0.3-encryption` (Phase 3)

- [ ] Read nhurman `encryption.py`, the bt port and `rustytti` side by side;
      write the handshake as a pure state machine in `protocol::crypto`.
- [ ] Keys loaded from `crypto.keys_file`; refuse to start an encrypted station
      without keys (clear error).
- [ ] Unit tests with vectors recorded from our AC180 (anonymised).
- [ ] Transport runs the handshake when the advertisement says encrypted.
- [ ] **Human:** AC180 acceptance run (20 polls vs app); mark confirmed fields `verified: true`.
- [ ] **Human:** Elite 200 V2 `detect` + `dump` of 100–200, 1300–1520, 6000–6020;
      compare with PR #49 and v2 maps; promote confirmed fields to `verified`.
- [ ] **Human:** Elite 200 V2 acceptance run.
- [ ] Fallback decision if blocked > 3 sessions: Python sidecar (ADR-002).

**Acceptance:** AC180 and Elite 200 V2 meet the v0.3 criteria.

### M3 — Collector, storage, metrics → `v0.4-collector` (Phase 4)

- [ ] `store`: migrations (§6), single writer task, read-only connection,
      retention + rollup job.
- [ ] `collector`: sequential poll loop, pause/shutdown via `watch`,
      `poll_failure` rows, unreachable state.
- [ ] `metrics`: grid presence + debounce (§7.1), house outage log (§7.2),
      runtime forecast (§7.3), energy integration (§7.4), extras (§7.5).
- [ ] Hourly `register_snapshot` for configured ranges.
- [ ] `serve` subcommand (collector + store, web stub with `/healthz`).
- [ ] Unit tests for every metric (flapping input, gaps, unknown state).
- [ ] **Human:** 24 h run; unplug a sensor station's AC input once.

**Acceptance:** ≥ 95% polls per station over 24 h; outage recorded ≤ 2 min after
the unplug; DB < 10 MB per station-day.

### M4 — Web UI (part of Phase 5)

- [ ] axum router, askama templates, embedded assets (uPlot vendored).
- [ ] Status page (§8.1), telemetry page, pause-polling button.
- [ ] JSON API + SSE (§8.2); optional bearer token for the POST.
- [ ] Page weight < 100 KB; works on a phone browser.
- [ ] Handler tests against an in-memory DB.

### M5 — Deploy + soak → `v1.0-mvp` (Phase 5)

- [ ] `deploy/bluetti-controller.service` (§10.3), install notes (§10.2).
- [ ] `make build` (release) and a `make dist` tarball.
- [ ] **Human:** install on the homelab host (§10.2), power it per ADR-008.
- [ ] **Human:** 7-day unattended soak; record interventions, RSS, page time.
- [ ] OUTCOME with measured numbers; tag `v1.0-mvp`.

**Acceptance:** 7 days, ≤ 1 manual intervention, RSS ≤ 50 MB, page ≤ 1 s on LAN.

### R1 — Battery health & telemetry discovery (post-MVP)

- [ ] EB3A register sweep 0–8000 (read-only) to look for temperature/cells.
- [ ] Verify V2 pack block 6000+ (temperature, SOH) on AC180 / Elite.
- [ ] Usable capacity per station from full observed discharges; feed back into
      `usable_capacity_factor`.
- [ ] Trend page: capacity estimate over months.

### R2 — Outage schedules (post-MVP)

- [ ] Owner gives region + queue (§14 Q3).
- [ ] `schedule` adapter: YASNO (Kyiv/Dnipro) or svitlo proxy; poll 15 min;
      cache last good schedule (works offline with the last copy).
- [ ] UI: next planned outage; per station "lasts through it: yes/no".
- [ ] Planned-vs-actual comparison in the outage log.

### R3 — Device control (post-MVP)

- [ ] Write path (0x06) with an allow-list per model; `control.enabled` +
      mandatory token (golden rule #2).
- [ ] Confirm write frames for encrypted stations (bt PR #59 / forks).
- [ ] UI toggles: AC/DC output, charging mode.
- [ ] Schedule-aware charging: Turbo before a planned outage when SOC < 100%,
      Silent otherwise.
- [ ] Staggered charging after the grid returns (one station in Turbo at a time,
      keeps the total under the 16 A breaker).

---

## 13. Risk register

| # | Risk | L | I | Mitigation |
|---|---|---|---|---|
| R1 | Elite 200 V2 map/encryption differ from the provisional profile | H | M | runtime detection; M2 dump; PR #49 and JordiGrasvi references |
| R2 | A firmware update rotates the static keys or changes the handshake | M | H | detect handshake failure, show it in the UI; don't update station firmware without checking community reports; fallback chain ADR-002 |
| R3 | BlueZ/BLE instability (aborts, hangs, stale links) | H | M | timeouts on every op, bounded retries, always disconnect, USB adapter on USB 2.0 + extension, runbook §10.5 |
| R4 | Phone app and service fight for the single connection | H | L | pause-polling button; force-quit the app |
| R5 | AC180 moves out of BLE range | M | L | `unreachable` state, never counted as an outage; not a grid sensor |
| R6 | EB3A exposes no temperature/energy over BLE | H | L | accept; R1 sweep |
| R7 | Host drains the router's station | M | M | ADR-008: host on Elite 200 V2 / own battery |
| R8 | Vendor objects to the static keys | L | M | keys never committed (ADR-007); own devices, interoperability only |
| R9 | `bluer` slow maintenance / blocking bugs | M | M | adapter boundary; `btleplug` as a drop-in alternative behind `Gatt` |
| R10 | SQLite damage on sudden power loss | L | M | WAL + `synchronous=NORMAL`; host on a UPS station |
| R11 | False outages from a flapping input | M | M | voltage threshold + debounce + multi-station vote |
| R12 | Debian toolchain too old for a crate's MSRV | L | L | toolchain pinned in `rust-toolchain.toml`, bumped deliberately |

---

## 14. Open questions for the owner

| # | Question | Blocks |
|---|---|---|
| Q1 | ~~Confirm or edit the golden rules, phase table and thresholds in RULES.md~~ — approved 2026-10-13 | — |
| Q2 | Homelab host: model, internal BT or a USB dongle, which station powers it — partly answered 2026-10-13: M1–M4 run on the owner's dev laptop (internal BT); the homelab host arrives after MVP; the M5 soak runs on the dev laptop (owner 2026-10-13) | — |
| Q3 | Region and outage queue (черга.підчерга) | R2 |
| Q4 | ~~Router powered from EB3A AC or DC 12 V?~~ — AC outlet (owner 2026-10-13) | — |
| Q5 | Station firmware versions and the encryption flag of each (the `scan` output) | M1/M2 |
| Q6 | ~~Names/roles for the stations~~ — defaults approved by the owner 2026-10-13 (phase 2 PLAN Q5): EB3A `router`, AC180 `mobile`, Elite 200 V2 `living-room` | — |
| Q7 | ~~Poll interval~~ — 60 s (owner 2026-10-13) | — |
| Q8 | ~~LAN or VPN~~ — LAN only (owner 2026-10-13) | — |

---

## 15. Sources

| Id | Source |
|---|---|
| S1 | https://github.com/Patrick762/bluetti-bt-lib (const.py, devices/eb3a.py, devices/ac180.py, base_devices/) |
| S2 | https://github.com/warhammerkid/bluetti_mqtt (bluetooth/client.py, core/commands.py, core/devices/eb3a.py) |
| S3 | https://github.com/j4m3z0r/bluetti-monitor/blob/main/PROTOCOL.md |
| S4 | https://github.com/warhammerkid/bluetti_mqtt/blob/main/bluetti_mqtt/core/commands.py |
| S5 | https://github.com/j4m3z0r/bluetti-monitor (README, const.py) |
| S6 | https://github.com/nhurman/bluetti_mqtt/blob/main/bluetti_mqtt/bluetooth/encryption.py |
| S7 | https://github.com/bluetti-community/bluetti-registers/blob/main/bluetooth/eb3a.json |
| S8 | https://github.com/warhammerkid/bluetti_mqtt/blob/main/CHANGELOG.md (0.10.0) |
| S9 | https://github.com/Patrick762/bluetti-bt-lib/pull/49 |
| S10 | https://github.com/Patrick762/hassio-bluetti-bt/issues/200 |
| S11 | https://github.com/warhammerkid/bluetti_mqtt/issues/131 |
| S12 | https://github.com/Patrick762/hassio-bluetti-bt/issues/120 |
| S13 | https://github.com/nhurman/bluetti_mqtt/blob/main/bluetti_mqtt/core/devices/v2_device.py |
| S14 | https://github.com/JordiGrasvi/bluetti-elite200v2-mqtt |
| S15 | https://github.com/Patrick762/bluetti-bt-lib (encryption port), PR #59 (encrypted writes) |
| S16 | https://gitlab.com/spock/rustytti |
| S17 | https://github.com/x3ccd4828/bluetti-encryption (GPLv3), https://github.com/ytmytm/Bluetti_ESP32_Bridge (GPLv3) |
| S18 | https://github.com/bluetti-official/bluetti-bluetooth-lib |
| S19 | https://github.com/Patrick762/hassio-bluetti-bt/issues/176, https://github.com/Patrick762/bluetti-bt-lib/issues/86 |
| S20 | https://bleak.readthedocs.io/en/latest/troubleshooting.html |
| S21 | https://www.home-assistant.io/integrations/bluetooth/ |
| S22 | https://devzone.nordicsemi.com/f/nordic-q-a/28165/time-cost-to-establish-ble-connection |
| S23 | https://github.com/bluetti-official/bluetti-home-assistant (README, api/, issue #174) |
| S24 | https://github.com/v0l/battery-control, https://github.com/tab-liu/ble_gui |
| S25 | https://crates.io/crates/bluer, https://github.com/bluez/bluer/issues, https://crates.io/crates/btleplug, https://github.com/deviceplug/btleplug/issues/412, https://docs.rs/axum/latest/axum/response/sse/, https://askama.rs/en/stable/frameworks.html, https://github.com/bigskysoftware/htmx/releases/tag/v4.0.0, https://crates.io/crates/rusqlite, https://sqlite.org/wal.html, https://sqlite.org/pragma.html#pragma_user_version, https://bundlephobia.com/package/uplot@1.6.32, https://bundlephobia.com/package/chart.js@4.5.1 |
| S26 | https://github.com/denysdovhan/ha-yasno-outages, https://app.yasno.ua/api/blackout-service/public/shutdowns/addresses/v2/regions |
| S27 | https://github.com/chaichuk/svitlo_live, https://dtek-api.svitlo-proxy.workers.dev/ |
| S28 | https://minipclab.com/blog/mini-pc-power-consumption-guide |
| S29 | https://packages.debian.org/search?keywords=bluez&searchon=names&exact=1&suite=all&section=all |
| S30 | https://sources.debian.org/src/bluez/5.82-1.1/src/bluetooth.conf/ |
| S31 | https://linrunner.de/tlp/settings/network.html, https://linrunner.de/tlp/faq/usb.html, https://linrunner.de/tlp/settings/battery.html, https://manpages.debian.org/trixie/systemd/logind.conf.5.en.html |
| S32 | https://legislation.gov.uk/eudr/2009/24/article/6, https://zakon.rada.gov.ua/laws/show/2811-20 |
| S33 | Bluetti product pages and reviews: https://shop.bluettipower.com/pages/ac180, https://www.storagereview.com/review/portable-power-meets-lab-grade-reliability-bluetti-elite-200-v2-review, https://www.bluettipower.com.au/products/bluetti-eb3a-portable-power-station |
| S34 | Bluetti community forum threads: "UPS Problem on EB3A", "EB3A fan always on in UPS mode", "Introduce an option for a 80% charge limit", "Encryption and decryption module provides APIs", "Official Bluetti Device API Survey" (community.bluettipower.com) |
| S35 | https://github.com/nhurman/bluetti_mqtt/blob/b746f17c1f6c/bluetti_mqtt/bluetooth/encryption.py (MIT; also `bluetooth/__init__.py`, `manager.py`) |
| S36 | https://github.com/Patrick762/bluetti-bt-lib/tree/0e11a80484a1 (MIT; `bluetti_bt_lib/bluetooth/encryption.py`, `device_reader.py`, `device_recognizer.py`) |
| S37 | https://gitlab.com/spock/rustytti/-/tree/d2c651d54dcf (MIT; `src/crypto.rs`, `src/session.rs`, `src/timeouts.rs`) |
| S38 | https://crates.io/crates/p256/0.14.0, https://crates.io/crates/aes/0.9.3, https://crates.io/crates/cbc/0.2.1, https://crates.io/crates/md-5/0.11.0 (crates.io API, 2026-10-13) |
| S39 | https://csrc.nist.gov/pubs/sp/800/38/a/final (Appendix F.2, CBC example vectors) |

---

## Appendix A — Station facts and operating notes

From the planning session (sources S33, S34). Useful for forecasts and alerts.

| | EB3A | AC180 | Elite 200 V2 |
|---|---|---|---|
| Capacity | 268.8 Wh LFP | 1152 Wh LFP | 2073.6 Wh LFP |
| Inverter | 600 W (1200 W surge) | 1800 W (2700 W power lifting) | 2600 W |
| UPS switchover | ~20–30 ms (user measurements) | ≤ 20 ms | ≤ 15 ms (official table; "10 ms" in some marketing) |
| Cycle life | 2500+ to 80% | 3500+ to 80% | 6000+ to 80% |
| AC charging | 268 W standard / 350 W turbo | Silent 260 W / Standard ~1000 W / Turbo ~1440 W | up to 1800 W input (charge + bypass) |
| Charge limit in app | no | no | yes (Custom UPS mode) |

**24/7 UPS use is fine** for all three (LiFePO4 + bypass). Notes:

- **EB3A:** in UPS mode it tops up 99→100% every ~35–40 min, runs the fan and
  gets warm (~38 °C); some units cut AC at low load. ECO mode turns AC off when
  the load is < 10 W for > 4 h — disable it for the router. After a full
  discharge it may not restore its output by itself when the grid returns.
- **Router on DC:** the EB3A has DC5521 12 V outputs; powering a 12 V router
  directly avoids inverter losses (estimated +30–60% runtime) and has no
  switchover at all. Check the router's PSU label (12 V, centre positive).
- **AC180:** ATX PSUs hold up ≥ 16–17 ms, so 20 ms is marginal for a PC under
  load. Gas boilers with ionisation flame sensing may lock out on a floating
  neutral when on battery (fix: N-PE bonding by an electrician). Forum reports
  of shutdowns with Grid Self-Adaptation on at 100%.
- **Elite 200 V2:** Standard UPS mode keeps 100%; after the outage season use
  Custom mode with a ~80–90% limit.
- **All:** bypass does not stabilise voltage — a voltage relay in the panel
  protects loads when the grid returns. Charging all three in Turbo at once
  (~1440 + 1800 + 350 W) plus household load can trip a 16 A breaker — the R3
  staggered-charging feature exists for this.

## Appendix B — Idea backlog

| Idea | Status | Why |
|---|---|---|
| Outage log, runtime forecast, energy accounting | MVP | owner's picks |
| Extra telemetry + raw register snapshots | MVP | owner asked for "any useful data" |
| Pause-polling button | MVP | the phone app needs the single BLE connection |
| Battery health / capacity trend | R1 | owner's pick |
| Outage schedules (YASNO / svitlo proxy) | R2 | owner's pick |
| Control, schedule-aware and staggered charging | R3 | owner's pick |
| Load recognition on the AC180 (fridge / boiler / PC by power signature) | idea | fun, not needed |
| Season summary ("212 h without power, 85 kWh delivered") | idea | cheap once R1 exists |
| Telegram bot / ntfy push | not planned | owner chose web UI only |
| Prometheus `/metrics` + Grafana | not planned | owner did not pick it |
| Bluetti cloud adapter | not planned | offline-first (golden rule #1) |
| Native Android app | not planned | web page works on the phone |
| ESP32 BLE proxies for range | not planned | revisit only if range fails (R5) |
