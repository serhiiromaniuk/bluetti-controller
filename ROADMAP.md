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
- **R1a Telemetry extras — postponed, not approved** (owner 2026-10-15: "we
  decided to postpone, because i not approved"). Read-only candidates found in
  §4.3, kept here so nobody re-derives them; build nothing until the owner
  approves:
  - in blocks already polled on the V2 stations (no extra BLE traffic): 103
    charging status, 104 / 105 the station's own time to full / to empty (min;
    the Elite dump matched our estimate, 249 vs 246 min);
  - one extra read every ~10 min per V2 station: 150–159 lifetime energy
    counters (u32, low word first, 0.1 kWh: DC out, AC out, PV, grid charge,
    feedback), 160 charging mode, 6003–6007 pack block (6007 temperature,
    °F suspected: 71 = 21.7 °C);
  - EB3A: nothing known beyond today's fields (§4.3 dump); an R1 sweep.
  A slow block must never fail the core poll (today one failed block fails the
  whole read, `transport::read::read_profile`).
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

**ADR-009 — Local time via `jiff` and the host tzdb.** *Context:* the energy
table needs local-day bounds and the forecast "lasts until HH:MM" needs local
time, both from `general.timezone` (§9); the server renders both (§8.4 d), so
browser-side `Intl` cannot do it. *Decision:* the `jiff` crate (0.2.38, Unlicense
OR MIT, pure Rust, default features) reads the host's tzdb (`/usr/share/zoneinfo`,
Debian `tzdata`) — offline, golden rule #1. `src/config.rs` parses the zone at
startup and fails fast on an unknown name. *Rejected:* `chrono-tz` (bundles the
whole tzdb into the binary); a fixed UTC offset (wrong across DST).
*Consequences:* one new runtime dependency; a host without `tzdata` fails at
startup with a clear error instead of rendering wrong days.

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
| 38 | AC output power (0 below a low-load floor, see below) | 1 | W | bt |
| 39 | DC output power | 1 | W | bt |
| 43 | battery SOC | 1 | % | bt |
| 48 | AC output on | bool | — | mq |
| 49 | DC output on | bool | — | mq |
| 77 | AC input voltage | 1 | V | S7 (bt/mq read 0.1; our dump on ARM 2197.02: whole volts) |
| 86 | DC input voltage | 0.01 | V | bt |
| 91 | pack count max | 1 | — | mq |

Controls (writes, R3 only): 3007 AC output, 3008 DC output, 3034 LED mode
(1 low / 2 high / 3 SOS / 4 off), 3060 power off, 3063 ECO, 3064 ECO shutdown time
(1–4 h), 3065 charging mode (0 standard / 1 silent / 2 turbo), 3066 power lifting.
Poll blocks used by mq: (10, 40), (70, 21), (3034, 1), (3060, 7).
**Not exposed by any library:** temperature, current, cell voltages, energy
counters. Pack registers 96–121 exist on AC300; unverified on EB3A — R1 sweep.
[S1][S7][S8]

**Our dump (CF-4, 2026-10-13, read-only, ARM 2197.02, `BLUETTF`).** Two dumps of
10–129 a minute apart (14:50, 14:51), grid on, SOC 100 %, AC outlet on and
powering the router (~9 W). Every field stays *unverified* until H1.

| Reg | Raw | Reading |
|---|---|---|
| 36–39 | 0, 0, 0, 0 | all powers 0 W, in both dumps |
| 48 / 49 | 1 / 0 | AC on, DC off |
| 71 / 77 | 220 → 222 | both track the mains voltage in whole volts; AC300's 0.1 scale would give 22.2 V, impossible on a 100–240 V input |
| 72–76, 78–90 | 0 | AC300's internal currents, powers and frequencies: not populated |
| 92 / 93 / 94 / 95 | 2310 / 2 → 12 / 100 / 71 | unknown (AC300: battery V and A ×0.1); no load tracking |
| 96–129 | 0 (116–119 masked) | — |

**Low-load floor (CF-4).** No register in the ranges the maps name for AC output
(36–39; AC300's 70–95 block) tracks a ~9 W load. In the grid outage (13:25–13:44)
reg 38 read 0 W on 20/20 polls while SOC fell 89 → 88 % in 19 min: 1 % of
268.8 Wh ≈ 2.7 Wh, ≈ 8.5 W (coarse: SOC has 1 % steps), so a load was real. Recommendation: keep
`ac_output_power` on 38 and treat 0 W as "below the floor" (floor unknown, > 9 W);
the runtime forecast must not read it as no load. H1 measures the floor.

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
MIT bridge exists (JordiGrasvi/bluetti-elite200v2-mqtt). [S9][S11][S14]

**Our dump (2026-10-13, read-only, firmware BMS 1068.11, on battery, grid off,
389 W AC load).** Every field stays *unverified* until H2 (app comparison).

| Reg | Raw | Reading | vs PR #49 / v2 |
|---|---|---|---|
| 100 / 101 / 102 | 3949 / 98 / 77 | 39.49 V, 9.8 A, 77 % | agrees; 389 W / 39.49 V = 9.85 A |
| 103 | 2 | charging status (discharging?) | v2 |
| 104 / 105 | 249 / 249 | minutes left: 77 % × 2073.6 Wh / 389 W ≈ 246 | agrees (104) |
| 110–115 | `Elite 200 V2` byte-swapped | device type | agrees |
| 140 / 142 / 144 / 146 | 0 / 389 / 0 / 0 | DC out, AC out, PV in, grid in (W) | agrees |
| 148 | 485 | unknown (apparent power, VA?) | — |
| 150–159 | 1, 0, 1959, 0, 0, 0, 2238, 0, 404, 0 | u32 low word first fits the v2 energy counters (0.1 kWh) | **contradicts** PR #49's 152 = battery V/100 (19.59 V vs 39.49 V) |
| 180–200 | — | Modbus exception 0x02 | — |
| 1300 / 1314 / 1315 | 0 / 0 / 0 | AC in Hz, V, A (grid off) | agrees (1315) |
| 1300–1419 | — | 40-register reads from 1300, 1340, 1380 fail (exception 0x02); 1300–1315 read singly | the poll block stays (1300, 15) |
| 1430 / 1431 / 1432 | 389 / 2300 / 16 | AC out W, 230.0 V, 1.6 A | agrees (1432) |
| 1460–1499 | — | exception 0x02 as one read; 1470 alone = 499 | agrees: 1470 = 49.9 Hz AC out |
| 1500 / 1510 / 1511 / 1512 | 499 / 483 / 2287 / 21 | AC out 49.9 Hz, ~VA, 228.7 V, 2.1 A | v2 |
| 6003 / 6004 / 6005 | 3950 / 98 / 77 | pack block echoes 100–102 | v2 |
| 6007 | 71 | temperature: 71 °F = 21.7 °C (room) | v2 °F suspected |

No dumped value contradicts `ELITE_200_V2` in `src/protocol/device.rs`; its
decode test uses the values above. Sweep a range only in blocks that avoid the
exception ranges: one bad address fails the whole read.

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
  anywhere. Phase-2 step 13 compared a hash of the point across two sessions
  per station: **different** on both the EB3A and the Elite 200 V2, so the
  station point is ephemeral (2026-10-13). The design never depends on it:
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
- **Ours (2026-10-13, `detect`):** EB3A and Elite 200 V2 both advertise
  `BLUETTF` and complete the §4.4 handshake; the EB3A keeps the V1 layout.
  Firmware: EB3A ARM 2197.02 / DSP 2196.01; Elite 200 V2 BMS 1068.11
  (register 6175). AC180 (phase-2 step 18): `BLUETTF`, V2 layout, BMS 1052.03
  (register 6175), handshake as above. See §14 Q5.

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
| p256 / aes / cbc / md-5 | 0.14.0 / 0.9.3 / 0.2.1 / 0.11.0 | locked in `Cargo.lock` (phase 2 step 6); RustCrypto, MIT OR Apache-2.0; p256 features `ecdh`, `ecdsa`; AES-128 temp + AES-256 session CBC, md5 IVs (§4.4); aes needs Rust ≥ 1.89 [S38] |
| uPlot (vendored JS) | 1.6.32 | 21.9 KB gzipped vs Chart.js 68.4 KB |
| jiff | 0.2.38 | Unlicense OR MIT; host tzdb, ADR-009 |

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
| `protocol::crypto` | BLUETTE handshake + AES-CBC wrapping, pure state machine | M2 ✓ |
| `transport` | `Gatt` trait; `BluerGatt` (BlueZ) and `ReplayGatt` (fixtures); connect/notify/write with timeouts; reassembly | M1 ✓ |
| `config` | load + validate TOML, fail fast | M1 ✓ |
| `cli` | `scan`, `detect`, `read`, `dump`, `serve` subcommands | M1 ✓ / M3 ✓ |
| `collector` | the poll loop: per station connect → (handshake) → read blocks → disconnect | M3 ✓ |
| `store` | SQLite: migrations, single writer, read-only pool, retention, rollups | M3 ✓ |
| `metrics` | grid state, outage events, runtime forecast, energy integration | M3 ✓ |
| `web` | axum routes, askama templates, SSE, embedded assets | M3 (`/healthz` stub) ✓ / M4 ✓ |

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

**M3 decisions (phase 4, 2026-10-13):**
- *Stages.* `discover` (the `find` step) · `connect` · `handshake` (told apart
  from `connect` by the error's typed source) · `read` · `decode`. A poll whose
  decode reports any `problems` (missing or implausible field) is a `decode`
  failure with the problems as its error, and stores **no** sample — the v0.3
  gate counted a poll OK only when every field was present and plausible.
  Recommendation: keep it strict; a half-valid sample would feed §7 bad input.
- *Per-round budget.* A round of n stations must fit one interval. Measured
  healthy poll: 4.0–5.0 s (EB3A, Elite, 2026-10-13). `find` takes the BlueZ
  cache, else scans and fails before `connect` when the station is not seen.
  A failed connect attempt costs 15 s + ≤ 5 s cleanup disconnect, with backoff
  2 s then 4 s between attempts. So a station that does not answer costs:
  in the cache, 3 × (15 + 5) + 2 + 4 = 66 s; not cached and not advertising,
  scan 15 s + 5 s D-Bus margin = 20 s (then `discover` fails); not cached but
  advertising, 20 + 66 = 86 s. 66–86 s > 60 s, so one silent station would cost
  the others their tick. Recommendation: every poll runs under a deadline of
  `poll_interval_secs / n_stations` (60 / 3 = 20 s, 4× a healthy poll). A
  fired deadline can cancel `connect` mid-attempt or after the link is up but
  before a GATT handle exists, so after **every** poll — success, error or
  deadline — the collector also disconnects the device by address through a
  `transport` call (bounded by `request_timeout_secs`, outside the deadline):
  no link outlives its poll (golden rule #5). A station already `unreachable`
  (§5.5) gets 1 connect attempt (≤ 15 + 5 s plus the find).
  *Smoke finding (phase-4 step 14, 2026-10-13):* BlueZ drops a station it no
  longer hears from its cache ~30 s after the disconnect (`TemporaryTimeout`),
  so at a 60 s interval the EB3A was never cached, every poll ran the full
  15 s scan and took ~20 s: 3 of 16 polls hit the deadline. Recommendation:
  the scan in `find` ends as soon as the station is heard (≤ 15 s, ~1–2 s
  when advertising), so an uncached poll costs ~6–7 s; an `unreachable`
  station scans too, else one dropped from the cache never recovers. The
  deadline still bounds both.
  Worst round: 3 × 20 s = 60 s = one interval; a round that still overruns
  skips the missed ticks (ticks at `start + k × interval`), never bursts.
  The deadline, not `connect_attempts`, bounds a connect that times out: one
  timed-out attempt (15 s + cleanup) leaves no room for a second inside 20 s.
  `connect_attempts = 3` still retries a connect that fails fast (an abort, a
  refusal). Recommendation: keep both; no per-attempt timeout derived from the
  deadline (phase-4 A1).

### 5.5 Error handling

- Transport errors are retried inside the poll; a failed poll is recorded, never
  fatal. Three consecutive failures mark the station `unreachable` (distinct from
  "grid lost").
- CRC/length errors are logged with the frame hex (MAC/SN masked) for debugging.
- A handshake failure is surfaced in the UI as "encryption failed — firmware
  change?" (§13 R2).
- *M3:* `unreachable` is derived, not stored: the station's newest 3 poll
  results (samples and failures by `ts`) are all failures. A sample clears it.
  Recommendation: no schema change; one indexed query per station.

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

**M3 decisions (phase 4, 2026-10-13)** — these refine the text above:
- *Migrations.* Embedded `migrations/NNN_*.sql` files (`include_str!`), applied
  in order inside one transaction each, versioned by `PRAGMA user_version`.
  Recommendation: no migration crate; the schema is small and single-writer.
- *Sample split.* The six core fields go to `sample`; every other decoded field
  goes to `sample_extra` under its `Metric::key()` (§7.5). A failed decode is
  a `poll_failure` only (§5.4). Recommendation: keep `sample` narrow and fixed;
  new fields need no migration.
- *Rollup and retention.* Each completed 5-minute bucket is rolled up within the
  next bucket, not after 30 days; the open current bucket never is. Progress =
  `max(rollup_5m.ts)` per station, and the `(station_id, ts)` primary key makes a
  repeat run a no-op, so a restart never double-counts. The nightly retention
  job deletes raw `sample`/`sample_extra` rows older than `raw_retention_days`
  only once their bucket is rolled up; `register_snapshot` and `poll_failure`
  after 90 days; `grid_event` never. `wal_checkpoint(TRUNCATE)` + `optimize`
  weekly. Recommendation: early rollups keep the M4 charts cheap and make
  rollup progress the only persisted cursor.
- *Restart.* Grid state is rebuilt from the open `grid_event` rows plus each
  station's last `debounce_samples` samples. An open event stays open through
  `unknown` and ends only when grid is seen again, backdated as in §7.1.
  Recommendation: no extra state table; the event rows are the state.
- *Snapshots.* The first successful poll of each UTC hour stores every profile
  block as one `register_snapshot` row per block (big-endian u16 data).
  Recommendation: hourly is enough to decode new fields later at ~24 rows per
  block per day.

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
- *M3:* the house outage log, its totals, the runtime forecast (§7.3) and the
  daily energy totals (§7.4) are pure functions over a window given as unix
  seconds `[from, to)`; an open event is clipped to `to`. They hold no state to
  persist. Local-day bounds from `general.timezone` arrive with the M4 energy
  table. Recommendation: no timezone crate in M3; nothing in M3 renders a day.

### 7.3 Runtime forecast

```
load_w      = EWMA(ac_out_w + dc_out_w), time constant ~10 min
remaining_wh = capacity_wh × soc / 100 × usable_factor     (default 0.85)
runtime_h   = remaining_wh / max(load_w, 1)
```

- **Low-load floor (§4.3, CF-4):** the EB3A reads 0 W below > 9 W. When a
  station discharges (SOC falling or no grid) and its outputs read 0 W, take
  `load_w` from the SOC slope (`capacity_wh × ΔSOC / 100 / Δt`, over ≥ 2 SOC
  steps) or show the runtime as unknown — never the 1 W floor.
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
  `energy_out_wh` under-counts loads below the low-load floor (§4.3, CF-4);
  the losses line below absorbs them, so label it an estimate.
- **Self-consumption / losses** ≈ `energy_in − energy_out − Δstored` — what UPS
  mode costs per day.
- Cross-check with the V2 lifetime energy counters (regs 150–159) once verified.
- *M3:* a segment between two consecutive samples counts toward the 5-minute
  bucket (and the day) of its **later** sample. A gap > 3 intervals adds no
  energy and counts as missing coverage. Within a gap-free run, a sample
  without SOC carries the last SOC forward, so Δsoc across it is kept.
  Recommendation: one rule for rollups and daily totals, so the two always agree.

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
  charts: SOC, output and input power across stations (uPlot), with a
  Grafana-style time picker and manual refresh (§8.4 j, 2026-10-15).
- **`/telemetry`:** every stored metric per station, verified and unverified,
  latest value and sparkline.
- **"Pause polling" button:** stops BLE polling for N minutes (max 60) so the
  official phone app can connect.

### 8.2 API

| Method | Path | Returns |
|---|---|---|
| GET | `/api/status` | current state of all stations + house |
| GET | `/api/series?station=&metric=&from=&to=&step=` | time series (raw or rollup by range) |
| GET | `/api/chart?from=&to=&step=` | every station's SOC, in W, out W on one axis per station, plus the window's outages (§8.4 j) |
| GET | `/api/outages?from=&to=` | grid events |
| GET | `/api/energy?days=7` | daily energy per station |
| GET | `/events` | SSE: `sample`, `grid_event`, `poll_failure` |
| POST | `/api/polling/pause?minutes=N` | pause polling (token if configured) |
| GET | `/healthz` | liveness + last successful poll age: JSON `{status, last_poll_age_s}`, 200 while the process runs (`last_poll_age_s` null before the first poll); recommended over 503-on-stale so a slow BLE round never restarts the process |

### 8.3 Constraints

- Server-rendered with askama; assets embedded in the binary; no JS build step;
  page weight < 100 KB.
- Binds to `web.listen`; LAN only; the host firewall blocks other networks.
- No auth for GET in the MVP; optional bearer `web.token` for the POST.

### 8.4 M4 design (phase 5, settled 2026-10-15)

Settles what §8.1–§8.3 leave open (RULES.md decision process). Each item is the
recommendation the code follows.

- **(a) Module layout.** `src/web/{mod,state,status,history,api,pages,telemetry,
  sse,pause,assets}.rs`, one responsibility each, ≤ ~400 lines; askama
  templates in `templates/`. `state.rs` holds `AppState` (reader, config view,
  `Control` sender, SSE sender, shutdown). One view model per page: `/` and
  `/api/status` serialise the same `Status` struct, so the page and the API
  never disagree.
- **(b) Grid state per station** (UI-derived, no new storage): an open
  station-scope `grid_event` → `Absent`; else no sample, or the newest sample
  older than 3 poll intervals → `Unknown`; else `Present`. House banner:
  `metrics::house::vote` over the `grid_sensor` stations' states; "since" =
  the open house event's `started_at`, else the newest closed house event's
  `ended_at`, else omitted. **Poll health** on the card: samples and
  `poll_failure` rows of the last 24 h, plus the newest failure's stage.
- **(c) Runtime forecast.** `metrics::runtime::forecast` over the station's
  last 2 h of `sample` rows, with `station.capacity_wh` (DB) and
  `metrics.usable_capacity_factor`, and the state from (b). `OnBattery` shows
  "lasts until HH:MM" (local, ADR-009), `OnGrid` "if the power goes now: N h",
  `Unknown` "—". SOC, powers and forecast carry an "unverified" badge while
  their fields are `verified: false` (H1–H3); no alert or gate uses them.
- **(d) Energy table.** The last 7 local days, today included as a partial
  day. Per station and day: `metrics::energy::totals` over the day's raw
  samples plus the one sample before the day, with `from`/`to` from
  `local_day_bounds`. Columns from `energy::Totals`: in, out, Δstored, losses
  (labelled "estimate", §7.4 — the UPS-mode self-consumption of §2.1 #5),
  missing coverage. Per day also `metrics::house::totals` over the house
  events: count, hours without power, longest. The helpers
  `local_day_bounds(ts, tz)` and `local_hhmm(ts, tz)` live in
  `src/metrics/localtime.rs` (DST days are 23/25 h), unit-tested.
- **(e) Outage log.** The newest 20 `grid_event` rows of every scope, newest
  `started_at` first; `ended_at NULL` shows as "ongoing" with its duration so far.
- **(f) API** (§8.2). Every bad parameter is `400` with `{"error": "<message>"}`;
  a failed database read is `500` with `{"error": "database read failed"}`.
  - `/api/status` → the `Status` of (a).
  - `/api/series`: `station` and `metric` required (unknown → 400); `from`
    default `to − 24 h`, `to` default now, `from < to`. Ranges ≤ 48 h read raw
    rows (`sample` columns, or `sample_extra` by `Metric::key()`). Beyond 48 h,
    metrics with a `rollup_5m` column (`soc` → `soc_avg`, the four powers →
    `*_avg`) read the rollup; others stay raw (extras exist only within their
    30-day retention). `step` (seconds, optional, ≥ 1) averages into
    `ts / step` buckets. Body is columnar: `{"source":"raw"|"rollup_5m",
    "t":[…],"v":[…]}`.
  - `/api/outages`: `from` default `to − 7 d`, `to` default now; events
    overlapping the window, newest first.
  - `/api/energy?days=N`: `1..=min(30, raw_retention_days)`, default
    `min(7, cap)`; the
    rows of (d). Beyond → 400: (d) reads raw rows only, and older days are gone.
- **(g) SSE `/events`.** The writer task publishes `sample`, `grid_event` and
  `poll_failure` (the row as JSON) on a `tokio::sync::broadcast` channel after
  each commit. A lagged client skips the missed messages (no close); keep-alive
  every 15 s; every stream ends when `state.shutdown` fires, so axum's graceful
  close never waits on an open `/events` client (`serve` fires it first).
- **(h) Pause** `POST /api/polling/pause?minutes=N`.
  - `N` in `1..=60` → `Control::Pause { until: now + N min }`; `0` →
    `Control::Run` (resume, §10.5); missing or other → `400`.
  - `web.token` set → `Authorization: Bearer <token>` required, else `401`;
    constant-time compare (fold XOR over the bytes, length checked first), no
    new crate. Token unset → open (§9, Q1): a pause only stops polling ≤ 60 min.
  - Reply `{"paused_until": <unix s|null>}`. Views read `paused_until` from
    `state.control.borrow()`: `Pause { until }` → `SystemTime::now() + (until −
    Instant::now())`, `null` once past.
  - No frame is ever sent to a station (golden rule #2).
- **(i) Assets.** Embedded with `include_bytes!`, served at `/assets/<name>`
  with `Cache-Control: max-age=86400`; unknown name → 404. uPlot **1.6.32**
  (git tag `1.6.32`, commit `e995b061`) vendored as `assets/uPlot.iife.min.js`
  (51 081 B) and `assets/uPlot.min.css` (1 857 B), its MIT `LICENSE` beside
  them, credited in `README.md`. No CDN, no external URL anywhere: the page
  works offline in an outage (golden rule #1). *Page weight* (uncompressed,
  100 KB = 100 000 B): 51 081 + 1 857 + `app.js` ≤ 12 000 + `app.css` ≤ 5 000 +
  HTML ≤ 18 000 = 87 938 B < 100 000 B. Over budget → trim HTML/JS, never the
  threshold; compression would add `tower-http` and needs an amendment here.
  *Rebalanced 2026-10-15 (owner — answered in the interactive session, recorded
  at the owner's request):* was JS 6 000 / CSS 4 000 / HTML 25 000; the
  dashboard (j) needs ~11 KB of JS while `/` renders ~7 KB of HTML. Same sum,
  same 100 KB limit; `web::tests` enforces each per-file cap.
- **(j) Front end** (reworked 2026-10-15 at the owner's request, task
  `2026-10-15_ui-dashboard`; lowest overhead first). Grafana-style:
  - Time picker: last 1 h … 30 d (default **12 h**), custom from/to, drag across
    a chart to zoom. URL state `?from=now-12h&to=now` (or ms epochs)
    `&refresh=5m`.
  - Refresh: **manual by default** (button); optional auto 1 m / 5 m / 15 m /
    30 m / 1 h, skipped while the tab is hidden, one catch-up when it shows
    again. A refresh is `GET /` (its `#live` and `#tables` swapped in) plus ONE
    `GET /api/chart`; nothing else goes over the network. The page no longer
    uses `/events` (the endpoint stays for API clients).
  - Three panels across stations — Battery %, Output W, Input W — with synced
    cursors, house-outage bands and a line break after a gap of > 3 steps.
    `/api/chart` raises `step` to ≤ 1000 buckets (a multiple of 300 s over
    48 h) and rounds values to 0.1.
  - Data age: "Last sample N ago" on the server clock; after 3 poll intervals
    it becomes a STALE banner. Days without samples read "no data", never 0.
  - Dark theme by default, a toggle kept in `localStorage`; no-JS shows cards
    and tables only. The pause form shows a token field only when `web.token`
    is set. One-column CSS below 600 px.
- **(k) `/telemetry`.** Every stored metric per station (the `sample`
  columns and the `sample_extra` keys), its latest value and its `verified`
  flag from the model's profile (`Field::verified`; unknown → unverified). A
  server-side inline-SVG sparkline over 24 h, averaged into ≤ 48 buckets
  (30 min), so the page needs no JS.

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

*Phase 5 (2026-10-15):* `web.token` unset leaves the pause POST **open**
(§8.4 h); set, it requires the bearer. `general.timezone` is parsed with `jiff`
at startup and an unknown zone fails fast (ADR-009). Paths are absolute: the
loader does not expand `~` or `%h`.

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

### 10.2 Install on the homelab host (human-only, after the MVP)

The first install, the migration from the dev laptop, every update and the
rollback are `deploy/HOMELAB.md`; the layout, the unit and the update flow
are §10.6. In short (all with sudo, the owner only): `apt install bluez curl`;
a system user `bluetti` in the `bluetooth` group; `/etc/bluetti-controller/`
root:bluetti 0750 with `config.toml` and `keys.toml` root:bluetti 0640;
`storage.path = "/var/lib/bluetti-controller/data.db"`. The D-Bus check runs
with the unit's identity: `systemd-run --pipe --wait -p User=bluetti -p
SupplementaryGroups=bluetooth busctl --system introspect org.bluez /org/bluez`.
The first release then goes in with `bluetti-update apply`, like every later one.

### 10.3 systemd unit (shipped in `deploy/` at M5)

The M5 run is on the dev laptop as a user-level unit (owner 2026-10-13,
§14 Q2): no `sudo`, no system unit. `loginctl enable-linger` keeps it running
without a login session.

```ini
# ~/.config/systemd/user/bluetti-controller.service
[Unit]
Description=bluetti-controller

[Service]
ExecStart=/usr/bin/systemd-inhibit --what=sleep:idle:handle-lid-switch --mode=block --who=bluetti-controller --why=monitoring %h/.local/bin/bluetti-controller serve --config %h/.config/bluetti-controller/config.toml
Restart=on-failure
RestartSec=10

[Install]
WantedBy=default.target
```

**M5 design (phase 5, settled 2026-10-15).**

- **Unit.** The block above is `deploy/bluetti-controller.service` (comments
  aside). `ExecStart` is wrapped in `systemd-inhibit
  --what=sleep:idle:handle-lid-switch`: logind's default
  `LidSwitchIgnoreInhibited=yes` ignores a plain `sleep` lock on lid close.
  `MainPID` is then `systemd-inhibit`; the service is its child, and
  `systemd-inhibit` returns the child's exit status, so `Restart=on-failure`
  still fires. Under linger polkit granted the lock without a session
  (phase-5 step 18: `systemd-inhibit --list` shows all three `what`s).
- **Install paths.** Binary `~/.local/bin/bluetti-controller`; config
  `~/.config/bluetti-controller/config.toml` (mode 600, copied from the local
  `config.toml`) with absolute `storage.path` =
  `~/.local/share/bluetti-controller/data.db` (a fresh DB) and absolute
  `crypto.keys_file`; `web.listen = "0.0.0.0:8080"`, no `web.token`.
  `loginctl enable-linger` (CF-6), or an H-row if polkit refuses.
- **Build.** `make build` = `cargo build --release --locked`. `make dist` →
  `dist/bluetti-controller-$(git describe --tags --always --dirty)-x86_64-linux.tar.gz`
  holding the binary, `deploy/bluetti-controller.service`,
  `config.example.toml` and `deploy/INSTALL.md`; `dist/` is gitignored.
- **1 h run (owner 2026-10-15, replaces the 7-day soak; phase-5 PLAN A7).**
  Starts at the unit's `ActiveEnterTimestamp` after linger is on; ends 3600 s
  later. No probe timer and no `soak_probe.sh`: the run is measured once, at
  its end.
  - **Polls** = successful polls per powered station over the hour, from the
    run's DB (`sample` vs `poll_failure` rows, read-only).
  - **RSS** = `VmHWM` of the service process (the child of `MainPID`,
    `pgrep -P`); **latency** = max `time_total` of 20 sequential `curl` GETs
    of `/` and of each API route on the host's LAN address. Gate (RULES.md
    v1.0): ≥ 95 % polls per powered station, ≤ 50 MB, ≤ 1 s, `make check`
    green. The phone look is an H-row (H9).
  - The unit keeps running after the run (the MVP in use). Long-run stability
    is proven on the homelab host after v1.0, installed interactively.

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
| Service status / logs (user unit) | `systemctl --user status bluetti-controller`; `journalctl --user -u bluetti-controller -f` |
| Service down after logout | `loginctl show-user "$USER" -p Linger` must be `yes` (`deploy/INSTALL.md` §4) |
| Laptop suspended during a run | `systemd-inhibit --list` must show `bluetti-controller` with `sleep:idle:handle-lid-switch` |
| Handshake fails on a station that worked | firmware update? Compare manufacturer data, read §13 R2 |

### 10.6 Releases and rollout on the homelab host (settled 2026-10-15)

Owner, 2026-10-15: move to the homelab host and have "a detailed and
sustainable mechanism of rolling out updates". No CI/CD (owner 2026-10-13):
the owner runs every step from the dev laptop. Every host command needs sudo,
so it is human-only (RULES.md); the agent writes and tests the tooling.

- **Version.** SemVer in `Cargo.toml`; a release is a `vX.Y.Z` tag on `main`
  with a `## X.Y.Z` section in `CHANGELOG.md`. The binary carries its build
  (`BLUETTI_BUILD` = `git describe --tags --always --dirty`, set by the
  Makefile; the crate version otherwise) in `--version` and `/healthz`
  `version`, so every running instance names the commit it came from.
- **`check` subcommand.** Loads and validates the config and the keys file,
  opens the database read-only and compares its schema version with the
  binary's. Never touches BLE, so it runs safely next to the live service.
  Exit 0 ok · 1 keys or DB unusable · 2 the DB schema is newer than this
  binary (an older binary refuses it: `store::schema::migrate`) · 4 the config
  is rejected (only then does a rollback restore a config).
- **Host layout.**
  - `/opt/bluetti-controller/releases/<build>/` — `bluetti-controller`,
    `bluetti-update`, the unit file, `CHANGELOG.md`; `current` and `previous`
    symlinks point at release directories (switched with `mv -T`, atomic).
  - `/etc/bluetti-controller/{config.toml,keys.toml}` — root:bluetti 0640.
  - `/var/lib/bluetti-controller/data.db` — systemd `StateDirectory`, owned
    by `bluetti`.
  - `/var/backups/bluetti-controller/` — root 0700, **outside** the service
    user's writable tree: `<UTC>-<build>.db` (+ `-wal`/`-shm`),
    `<UTC>-<build>.config.toml`, `<UTC>-pre-rollback.*`. Root never follows a
    symlink in the state dir: `bluetti-update` refuses to run when `data.db`,
    its `-wal`/`-shm` or the config is one (a compromised service must not
    steer root's copies, installs or deletes).
- **System unit** `deploy/bluetti-controller.system.service`: `User=bluetti`,
  `SupplementaryGroups=bluetooth` (BlueZ 5.82's policy already admits every
  local user, `context="default"`; the group covers stricter ones), `ExecStart=
  /opt/bluetti-controller/current/bluetti-controller --config
  /etc/bluetti-controller/config.toml serve`, `Restart=on-failure`,
  `StateDirectory=bluetti-controller`, `ProtectSystem=strict`,
  `ProtectHome=yes`, `PrivateTmp=yes`, `PrivateDevices=yes`,
  `NoNewPrivileges=yes`, `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6
  AF_BLUETOOTH`, `MemoryMax=128M`. No `systemd-inhibit`: the host never sleeps.
- **Ship.** `make release VERSION=X.Y.Z` (clean `main`, changelog section
  present → bump, `make check`, commit, tag, `make dist`). `make deploy
  HOST=<ssh name>` (`deploy/deploy.sh`) computes the tarball's sha256 on the
  laptop, copies it, and over `ssh -t` (the sudo prompt needs a tty) unpacks it
  under sudo into a root-owned `mktemp -d`, checks the hash against the
  laptop's value, and runs `./bluetti-update apply` from there — the newest
  update logic always runs, and nothing a non-root user can write is executed
  as root. `SHA256SUMS` inside the tarball proves the files, the laptop's hash
  proves the tarball.
- **`/healthz`** adds `version` (the build) and `samples_since_start` (samples
  this process stored, counted in memory from the writer's updates; at least,
  never fewer). The DB-wide `last_poll_age_s` cannot prove the new build
  polled: an older sample can land in the start's second, and a future-dated
  sample reads age 0.
- **`bluetti-update apply <dir>`** (root, `set -euo pipefail`, every path
  overridable for the stubbed test):
  1. verify `SHA256SUMS`; the release's build equal to `current` → nothing to
     do;
  2. preflight, nothing changed yet: the new binary's `check` against the live
     config and DB must exit 0 (1 = keys or DB unusable, 2 = a downgrade below the DB
     schema, 4 = config rejected → abort); free space ≥ 2 × (DB + WAL) for
     the backups and the release size for `releases/`. A build already in
     `releases/` (e.g. `previous`) is refused: `rollback` goes back to it
     without losing its saved config and `BACKUP` record;
  3. stop the unit; from here until the start in step 5 an `EXIT` trap puts
     `current`, `previous` and the old unit file back (`daemon-reload`) and
     starts the old build if anything fails (the trap runs every step even
     when one fails); it is cleared once the new build is started.
     `shopt -s inherit_errexit`: a failing copy inside `$(…)` aborts too;
  4. back up `data.db` (+ `-wal`/`-shm`) and `config.toml` to
     `/var/backups/bluetti-controller/<UTC>-<old build>.*` (`cp -p`: owner and
     mode kept); the release records that base name in `BACKUP`;
  5. install the release dir; `previous` ← old `current`; `current` ← new;
     install the release's unit file when it differs (`daemon-reload`); start;
  6. **health criterion:** within 300 s `/healthz` answers 200 with `version`
     = the new build **and** `samples_since_start` ≥ 1;
  7. pass → prune, summary, exit 0; fail → automatic rollback, exit 1.
  `--no-poll-check` drops the sample half of step 6 for a window when every
  station is off on purpose.
- **`bluetti-update rollback`** (also the automatic one): stop; `current` ←
  `previous`, `previous` ← the failed release; run the older binary's `check`:
  - exit 2 (the update migrated the schema) → move `data.db{,-wal,-shm}` aside
    as `<backups>/<UTC>-pre-rollback.*` (kept: nothing is destroyed), put the
    update's DB backup in place with `install -o bluetti -g bluetti -m 0640`,
    no stale `-wal`/`-shm` left; samples stored since are lost from the live
    DB (still in the pre-rollback copy), and the command says so;
  - exit 4 (the older binary rejects the config, e.g. a key the owner added
    for the new build) → put back the config that release last passed its
    health check with (`<release>/config.toml`, saved root 0600 on every
    healthy apply) with `install -o root -g bluetti -m 0640`, the live one kept
    as `<backups>/<UTC>-pre-rollback.config.toml`; re-check. (A backup taken
    at apply time would already hold the owner's edit.)
  - start — also when `check` still fails (any other exit): the old build is
    started anyway and left to systemd's restarts, exit 3 with the check
    output. Only a missing backup record or file (nothing to restore) stops
    with exit 3 before the start: starting would only crash-loop;
  - health criterion = `version` only (the previous build was healthy
    before; when the trigger was "no station polled", it would fail the same
    way). Still unhealthy → leave it running, exit 3 with a loud message and
    the journal command.
- **`check` runs as the service identity** (`runuser -u bluetti -g bluetti
  -G bluetooth`), in the preflight and in the rollback: a read-only open of a
  stopped WAL database creates `-wal`/`-shm`, and as root they would be
  root-owned and unwritable for the service; it also proves the service user
  can read the config and the keys. `deploy.sh`'s temp dir is root-owned 0755
  for that reason (readable, not writable, by others).
- **`status`**: current, previous, releases, backups, unit state, `/healthz`.
- **Reboot mid-update.** After the `current` switch, a reboot starts the new
  build without a health check; `status` shows it, and `rollback` works as
  usual. Before the switch, the old build simply starts again.
- **Retention:** the 5 newest releases and the 10 newest backup sets
  (including pre-rollback ones); `current` and `previous` are never pruned.
  The DB is bounded: ~0.3 MB/day per station (RULES.md v0.4) × 3 stations ×
  30 days of raw retention (`storage.raw_retention_days`) ≈ 27 MB plus the
  small 5-min rollups, so 10 sets stay ≲ 0.3–0.5 GB; the free-space preflight
  catches the rest.
- **Tested** by `deploy/test/update-test.sh` in `make check`: a temp root,
  stub `systemctl`/`curl`, fake builds; it proves apply-pass, a first install,
  apply-fail → rollback (with and without a schema move, and with a config
  the older build rejects), a failure after the stop (trap), manual rollback,
  a bad checksum, a failed preflight, and pruning.
- **One poller (golden rule #5).** The laptop's user unit is disabled before
  the homelab unit first starts; `deploy/HOMELAB.md` orders it so.

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
Phase 2 = M1 + M2 (merged by the owner 2026-10-13: the EB3A is encrypted; the
AC180 joined read-only at 14:05 the same day, so M1/M2 cover all three stations),
Phase 4 = M3, Phase 5 = M4 + M5 (M5: a 1 h run on the dev laptop as a
user-level systemd unit, owner 2026-10-15). **Human:** items below that compare with the app or unplug a
station are deferred to `HUMAN-CHECKLIST.md` and do not block
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

### M1 — BLE transport + CLI, EB3A (Phase 2, tagged with M2 as `v0.3-encryption`)

- [x] Add deps: tokio, bluer, clap, tracing, serde, toml, anyhow, thiserror;
      `libdbus-1-dev` on the dev host (human; CF-1 discharged).
- [x] `config.rs`: TOML schema (§9), validation, `config.example.toml`.
- [x] `transport::Gatt` trait; `BluerGatt`: find by MAC (fallback filtered
      discovery), connect with timeout/retries, subscribe `ff01`, write `ff02`,
      reassemble responses to the expected length, always disconnect.
- [x] `ReplayGatt` + `--record` to capture fixtures; anonymiser for MAC/SN.
- [x] CLI: `scan` (name, RSSI, model, encrypted flag), `detect <mac>` (model,
      protocol, encryption, device type and firmware registers), `read <mac>`
      (decoded metrics), `dump <mac> --from --to` (read-only sweep, golden rule #2).
- [x] `scan` + `detect` of the EB3A and Elite 200 V2: encryption flag and
      firmware in §14 Q5.
- [x] `scan` + `detect` of the AC180 (phase-2 step 18): encryption flag and
      firmware in §14 Q5.
- [x] EB3A acceptance run — 20 polls at 60 s (unattended): 20/20. App comparison
      and the `verified: true` flags it confirms → `HUMAN-CHECKLIST.md` H1;
      `ac_output_power` reads 0 W below a low-load floor → CF-4, step 21, §4.3.
- [x] Exception-aware reassembly length (loop/STATE.md CF-3).
- [x] Record an EB3A fixture; replay test for the full read path.
- [x] Wire `cargo llvm-cov` into `make check` (≥ 80% on `protocol`).

**Acceptance (RULES.md):** EB3A 19/20 polls OK (met, 20/20; `ac_output_power` →
CF-4 / step 21, a low-load floor in §4.3); SOC equal to the app, power ±5 W → H1.

### M2 — Encryption + V2 stations (Phase 2, merged) → `v0.3-encryption`

- [x] Read nhurman `encryption.py`, the bt port and `rustytti` side by side;
      write the handshake as a pure state machine in `protocol::crypto`.
- [x] Keys loaded from `crypto.keys_file`; refuse to start an encrypted station
      without keys (clear error).
- [x] Unit tests with vectors recorded from our EB3A and Elite 200 V2
      (anonymised, `tests/handshake_vectors.rs`).
- [x] Vectors from our AC180 (phase-2 step 19, `tests/frames/handshake-ac180.json`).
- [x] Transport runs the handshake when the advertisement says encrypted.
- [x] AC180 acceptance run (unattended): 20/20 (phase-2 step 20); app comparison
      and its `verified` flags → H3.
- [x] Elite 200 V2 `detect` + `dump` of 100–200, 1300–1520, 6000–6020;
      compared with PR #49 and v2 maps (§4.3); promotion to `verified` → H2.
- [x] Elite 200 V2 acceptance run (unattended): 20/20; app comparison → H2.
- [x] Fallback decision if blocked > 3 sessions: Python sidecar (ADR-002) —
      n/a, the handshake works on all three stations.

**Acceptance:** EB3A, Elite 200 V2 and AC180 (each 20/20) meet the
v0.3 poll criteria; app comparisons H1–H3.

### M3 — Collector, storage, metrics → `v0.4-collector` (Phase 4)

- [x] `store`: migrations (§6), single writer task, read-only connection,
      retention + rollup job.
- [x] `collector`: sequential poll loop, pause/shutdown via `watch`,
      `poll_failure` rows, unreachable state.
- [x] `metrics`: grid presence + debounce (§7.1), house outage log (§7.2),
      runtime forecast (§7.3), energy integration (§7.4), extras (§7.5).
- [x] Hourly `register_snapshot` for configured ranges.
- [x] `serve` subcommand (collector + store, web stub with `/healthz`).
- [x] Unit tests for every metric (flapping input, gaps, unknown state).
- [x] 12 h run: run 4 (owner, 2026-10-15; switch-off excluded, so the Elite
      was measured 2 h 31 min, 151/151), phase-4 LOG step 18d.
- [ ] **Human:** unplug a sensor station's AC input once (`HUMAN-CHECKLIST.md`
      H4; run 4 recorded a grid-loss event at 02:03:54 for the owner to judge:
      a real cut or the owner's switch-off).

**Acceptance:** ≥ 95% polls per station over 12 h; outage recorded ≤ 2 min after
the unplug; DB < 10 MB per station-day.

### M4 — Web UI (part of Phase 5)

- [x] axum router, askama templates, embedded assets (uPlot vendored).
- [x] Status page (§8.1), telemetry page, pause-polling button.
- [x] JSON API + SSE (§8.2); optional bearer token for the POST.
- [x] Page weight < 100 KB (65.7 KB, phase-5 step 14); the phone-browser look is
  `HUMAN-CHECKLIST.md` H9.
- [x] Handler tests against an in-memory DB.

### M5 — Deploy + 1 h run → `v1.0-mvp` (Phase 5)

- [x] `deploy/bluetti-controller.service` (user-level, §10.3), install notes
      (`deploy/INSTALL.md`; phase-5 step 16).
- [x] `make build` (release) and a `make dist` tarball (phase-5 step 16).
- [x] Install the user-level unit on the dev laptop (§10.3), linger on — done
      by the loop under the standing delegation (phase-5 step 18 LOG); the
      homelab host (§10.2) is installed interactively after the MVP (§14 Q2).
- [x] 1 h functional run of the installed unit (owner 2026-10-15, replaces the
      7-day soak): polls 100/100/98.3 %, RSS 12.3 MB, max page/API 2.1 ms
      (phase-5 step 21).
- [x] OUTCOME with measured numbers; tag `v1.0-mvp`.

**Acceptance (RULES.md v1.0):** 1 h, ≥ 95 % polls per powered station,
RSS ≤ 50 MB, status page and every API route ≤ 1 s on LAN, `make check` green.

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
| Q2 | Homelab host: model, internal BT or a USB dongle, which station powers it — partly answered 2026-10-13: M1–M4 run on the owner's dev laptop (internal BT); the homelab host arrives after MVP; the M5 run is on the dev laptop (owner 2026-10-13; 1 h, owner 2026-10-15) | — |
| Q3 | Region and outage queue (черга.підчерга) | R2 |
| Q4 | ~~Router powered from EB3A AC or DC 12 V?~~ — AC outlet (owner 2026-10-13) | — |
| Q5 | ~~Station firmware versions and the encryption flag of each~~ — EB3A `BLUETTF`, V1 layout, ARM 2197.02 / DSP 2196.01; Elite 200 V2 `BLUETTF`, V2 layout, BMS 1068.11 (register 6175) (`detect`, 2026-10-13); AC180 `BLUETTF`, V2 layout, BMS 1052.03 (register 6175) (`detect`, 2026-10-13, phase-2 step 18) | — |
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
| Telemetry extras: V2 time to full/empty, energy counters, pack temperature | R1a, postponed | owner did not approve yet (2026-10-15) |
| Outage schedules (YASNO / svitlo proxy) | R2 | owner's pick |
| Control, schedule-aware and staggered charging | R3 | owner's pick |
| Load recognition on the AC180 (fridge / boiler / PC by power signature) | idea | fun, not needed |
| Season summary ("212 h without power, 85 kWh delivered") | idea | cheap once R1 exists |
| Telegram bot / ntfy push | not planned | owner chose web UI only |
| Prometheus `/metrics` + Grafana | not planned | owner did not pick it |
| Bluetti cloud adapter | not planned | offline-first (golden rule #1) |
| Native Android app | not planned | web page works on the phone |
| ESP32 BLE proxies for range | not planned | revisit only if range fails (R5) |
