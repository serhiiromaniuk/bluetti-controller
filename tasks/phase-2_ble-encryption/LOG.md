# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes. `loop/where.sh`
> computes step position from `PLAN.md` and the last result from this file's newest
> `## ` heading — so restating an entry in a pointer file is prose every later
> iteration pays to re-read.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns). Bullets, not
> narrative. Cite a spec/doc section, never re-quote it. Record the decision and
> the evidence, not the reasoning that produced them. The shape below is the shape.

## YYYY-MM-DD HH:MM — Session start
Context loaded per `loop/where.sh --json`. Starting at step <N>.

<!-- Entry template — copy, fill, keep it under 40 lines:

## Step <N> <✓|blocked> YYYY-MM-DD — <the headline result, one line>

**Changed:** <file/module — what, one line each>

**Check:** <the check that proves this step> — failed first for <the intended
reason>, then passed. <or, when it is green-on-arrival: the mutation that proves
the assertion has teeth, in one line>

**Evidence:** written below this entry by `loop/step-done.sh` (the gate tail,
fenced in `gate:begin/gate:end`). Add by hand only what no script captures: a
smoke result, measured acceptance numbers — the decisive lines.

**reviewer:** <n> CRITICAL / <n> HIGH / <n> MEDIUM · or `batched` (low-risk,
reviewed with the next batch before the push)
- CRITICAL|HIGH <id>: <finding> → fixed in <where>.
- MEDIUM <id>: <finding> → deferred to PLAN `## Amendments` A<n>.

**Decision:** <a choice the next iteration must not relitigate · none>
**Carry-forward:** <raised/discharged in loop/STATE.md · none>
**Next:** step <N+1> — <title>.

Commit: via `loop/step-done.sh --commit "<type>(<scope>): <subject>"`   ·   Marker: <<LOOP:CONTINUE>>
-->

## Plan ✓ 2026-10-13 — PLAN.md written (17 steps) and audited

**Changed:** `PLAN.md` — 17 steps, parallel group A (1–4), Q1–Q5 all `delegated 2026-10-13`; old steps 9–16 and A1–A8 carried (mapping in Risks).

**Check:** `plan-reviewer` audit — 0 CRITICAL / 2 HIGH / 4 MEDIUM / 1 LOW; all acted on before the commit:
- HIGH F1 (step 13 committed a session secret + station key, against golden rule #3 and BRIEF done-when) → fixed: step 13 commits key-free envelope structure only; step 1 decides the committable-artefact rule and whether the station key is ephemeral.
- HIGH F2 (key scan armed only at step 8, after steps 6–7 port key-bearing sources) → fixed: step 5 writes the local `keys.toml` and proves the scan refuses its values; step 8 only loads keys.
- MEDIUM F3 (replay `detect` vs `read` recording) → fixed: step 12 `read` runs `detect` + `poll` in one session.
- MEDIUM F4 (no owner for an EB3A V2 re-map) → fixed in step 11.
- MEDIUM F5 (step 12 oversized) → `--record-wire` moved to step 13.
- MEDIUM F6 (step 1 check lacks the fixture decision) → fixed in step 1's check. LOW F7 → step 9 check wording.

**reviewer (batch `@{u}..HEAD` = 001de51, 1f2809b, 63ca4e8):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM → deferred to PLAN `## Amendments` A9 (ROADMAP §12 M1/M2/M5 bodies stale), A10 (H-rows lack the `verified` follow-through), A11 ("production deploys" undefined vs user-level units); all homed at step 17.

**Decision:** none beyond the delegated Q1–Q5 in PLAN.
**Carry-forward:** none.
**Next:** parallel group A — steps 1–4.

Commit: `docs(loop): plan phase-2 encryption task` · Marker: <<LOOP:CONTINUE>>

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:30:48Z

```text

test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-f1d778b563e65a98)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-10-13 — handshake designed in ROADMAP §4.4 from 3 MIT sources

**Changed:** `ROADMAP.md` §4.4 rewritten (sequence table, static keys by role, `keys.toml` schema, committable-artefact rule, test split, recommendation), §4.9 crypto row, sources S35–S39 (worktree branch, commit a7d96b5).

**Check:** sources pinned + licences MIT: nhurman `bluetti_mqtt` b746f17c1f6c, Patrick762 `bluetti-bt-lib` 0e11a80484a1, `rustytti` d2c651d54dcf (GitLab); GPL repo never opened. Added-line hex grep `[0-9a-fA-F]{32,}` → 0. Crates: p256 0.14.0, aes 0.9.3, cbc 0.2.1, md-5 0.11.0.
- Findings: station opens kex (type 01, 4-byte seed); temp layer AES-128 (md5(reverse(seed)) XOR local key); session layer AES-256 from raw ECDH x; zero padding; no MAC/SN in any message; BLUETTE/BLUETTF same handshake in all sources; station-key ephemerality unknown → step 13.

**Decision:** keys.toml = `local_aes_key` (32 hex), `host_signing_key` (64 hex), `station_verifying_key` (130 hex, SEC1 `04…`).
**Next:** see group close below.

## Step 2 ✓ 2026-10-13 — A2 + A3: config Debug masks address/token, metrics range-checked

**Changed:** `src/config.rs` (329 lines) — hand-written `Debug` for `StationConfig` (`<masked>`) and `WebConfig` (`<set>`/`None`); `validate` rejects factor outside (0,1] and non-finite/≤0 `grid_voltage_min` (commit ed4a881).

**Check:** failing first: `debug_masks_address_and_token ... FAILED` (`assertion failed: !format!("{config:?}").contains("DD:EE:01")`), `rejects_bad_metrics ... FAILED`; then `cargo test --lib config::` ok 11 passed. A2, A3 → fixed.

## Step 3 ✓ 2026-10-13 — A5 + A6: backoff pinned, anonymiser hardened

**Changed:** `Cargo.toml` dev-dep tokio `test-util`; `read/tests.rs` `backoff_doubles_between_retries` (start_paused, elapsed == 3 s); `anonymise.rs` masks both serial layouts for every model, refuses replies after an unanswered request (commit 085223e + review fix below).

**Check:** failing first: A5 `no method named start_paused` before test-util; A6 `fixture_masks_both_serial_layouts_for_every_model ... FAILED` (`serial at 116 leaked`). Mutation: deleting `backoff.saturating_mul(2)` → `left: 2s right: 3s`. Step text's two-filter command does not parse; ran `cargo test --lib -- transport::read transport::anonymise` → ok 23 passed. A5, A6 → fixed. Cargo.lock unchanged.

## Step 4 ✓ 2026-10-13 — Metric::range bounds + pure problems()

**Changed:** `src/protocol/device.rs` (309 lines) `Metric::range()` per metric with reasons (SOC 0–100, power 0–5000 W, AC V 0–300, Hz 0–70, DC V 0–150, flags 0–1, pack 0–60 V, current ±100 A); `Problem {Missing, OutOfRange}`, `problems(profile, registers)`; `decode` unchanged (commit 221140b).

**Check:** stub returning `vec![]` failed 3 new tests (`left: [] right: [OutOfRange { metric: BatterySoc, value: 101.0 }, …]`); then ok 14 passed; protocol coverage 100%.

## Group A close 2026-10-13 — merged, batched review

- Worktree branches p2-step1..4; `loop/merge-gate.sh` → `MERGE GATE OK` on the merged tree; cherry-picked linear onto main; worktrees removed.
- **reviewer (batch 5f40a76..HEAD):** INTENT 1–4 satisfied; 0 CRITICAL / 1 HIGH / 1 MEDIUM.
  - HIGH H1 `anonymise.rs:137`: an unanswered NON-serial request + same-count serial request shifts pairing, serial reply masked against wrong range → fixed: any unanswered request sets `stale`; test replaced with the shifted case (failed first at tests.rs:242, then ok 13 passed). Second pass on the fix: no findings.
  - MEDIUM M1 `device.rs:143` PackCurrent sign vs u16 decode → deferred to A14.
- Writer findings → A12 (session AES-256, zero padding; step 6), A13 (Write Request for msg 2; steps 9–10).
- Env note: cargo lives in `~/.cargo/bin`, not on the loop shell's PATH.

**Carry-forward:** none. **Next:** step 5 — A7 + key scan in `loop/step-done.sh`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:42:23Z

```text

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-6a10f69a4d815929)

running 2 tests
test cli::scan::tests::unknown_station_still_prints ... ok
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 5 ✓ 2026-10-13 — scan refuses `-`/`_` MACs and keys.toml values; local keys.toml written

**Changed:**
- `loop/step-done.sh` `builtin_scan`: MAC regex `[0-9A-Fa-f]{2}([:_-])([0-9A-Fa-f]{2}\1){4}[0-9A-Fa-f]{2}` (one separator per MAC); exemption `^AA([:_-])BB\1CC\1`, case-insensitive (A7).
- Same function: when `<main worktree>/keys.toml` exists (via `git rev-parse --git-common-dir`), count added lines holding any of its ≥32-hex values (`grep -ciF`, case-insensitive); refuse on >0; values never printed.
- `keys.toml` (gitignored, mode 0600, not committed): §4.4 schema from nhurman/bluetti_mqtt `b746f17c1f6c` `encryption.py` (LICENSE: MIT). Lengths 32/64/130 hex; `station_verifying_key` = last 65 B of the 91-B DER, whose 26-B prefix equals the public P-256 SPKI header (checked, True). Fetched copy deleted.

**Check:**
- `bash -n loop/step-done.sh` → ok; `git check-ignore -q keys.toml` → ignored.
- Scratch repo (plain bash, GNU grep 3.12, copy of keys.toml): `12-34-…` refused (1 MAC), `12_34_…` refused (1), `12:34:…` refused (1), `AA-BB-CC-DD-EE-01` pass, `aa_bb_cc_dd_ee_02` pass, mixed separators pass (not a MAC), uppercased real 64-hex value refused (1 line), clean line pass. Scratch dir removed.
- Pitfall: the interactive tool shell's `grep` is a ugrep wrapper without `\1` support — first scratch run errored and passed everything; the script runs under plain bash and gets GNU grep.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 2 MEDIUM. M1 (empty pattern list passes silently) → deferred to A15; M2 (byte-array / split-line key forms pass) → deferred to A16. Both homed at step 6, before any crypto port commits.

**Carry-forward:** none. **Next:** step 6 — crypto envelope (fix A15 + A16 first in that increment).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:45:42Z

```text

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-6a10f69a4d815929)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## A15 + A16 ✓ 2026-10-13 — key scan refuses unparsed keys.toml and split/byte-array key forms

**Changed:**
- `loop/step-done.sh` `builtin_scan`: keys.toml values parsed single- or double-quoted, optional `0x`; an empty parse (no hex, unreadable file) refuses with "keys.toml present but no hex values parsed" (A15).
- Patterns are hex chars 1–32 and 3–34 of each value, lowercased; matched against all added lines joined, with `0x`/`\x` and every non-hex char removed, so byte arrays and keys split over lines are caught (A16). Counts only, never values.

**Check:**
- `bash -n` ok. Scratch repo, dummy keys, plain bash + GNU grep: clean PASS; uppercase 64-hex REFUSED; `[0x.., …]` split over 2 lines REFUSED; `b"\X..…"` REFUSED; point without `04` REFUSED; sha256 checksum line PASS; single-quoted and `0x`-prefixed keys.toml REFUSED; no-hex and unreadable keys.toml REFUSED (A15 message); no keys.toml PASS. Real keys.toml parses non-empty (clean index PASS). Scratch removed.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM. Noted limits within A16's ask (fragments <30 hex, decimal arrays, partial parse) — not findings.

**Carry-forward:** none. **Next:** step 6 — crypto envelope.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:49:55Z

```text

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-6a10f69a4d815929)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 6 ✓ 2026-10-13 — crypto envelope, pure: kex frame, temp + session envelopes, temp key

**Changed:**
- `Cargo.toml`: aes 0.9.3, cbc 0.2.1, md-5 0.11.0, p256 0.14.0 (`ecdh`,`ecdsa`, no defaults; used from step 7); comment now "pure: std + RustCrypto, no I/O or async"; `rust-version` 1.85 → 1.89 (aes 0.9.3 needs 1.89).
- `Cargo.lock`: via `cargo build` (365 lines, insertions only) instead of `make lock` (`generate-lockfile` would bump every other dependency).
- MSRV bump armed two clippy lints in old code, fixed mechanically: `modbus.rs` `as_chunks::<2>`, `fixture.rs` `is_multiple_of`.
- `src/protocol/crypto/mod.rs`: `CryptoError` (no payload bytes), `TempKey::derive` (iv = md5(reverse(seed)), key = iv XOR local key, no `Debug`), `reply()`, `session_iv`, private block-wise CBC for AES-128 and AES-256 (A12).
- `src/protocol/crypto/frame.rs`: `encode_kex`/`parse_kex` (magic, exact length, checksum), `Layer` + `envelope_len`, `seal`/`open` for temp and session envelopes, zero padding checked on open.
- `README.md` credits: crypto ported from rustytti, cross-checked against nhurman (both MIT).

**Check:**
- Failing first (mutation): padding check disabled + seed reversal removed → 3 FAILED (bad padding, recorded vector, temp key); restored → `cargo test --lib protocol::crypto` 11 passed.
- Vectors: NIST SP 800-38A F.2.1 (AES-128) and F.2.5 (AES-256) both ways; rustytti challenge `2a2a0104…02bc` → reply `2a2a02045a4fd30e0190`.
- `make check` green; `src/protocol/` 99.71% (crypto/frame.rs 99.2%, mod.rs 100%).
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM. A12 → fixed (both widths).

**Carry-forward:** none. **Next:** step 7 — handshake state machine.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:53:12Z

```text

test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-871f4c6d900bafc2)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.71% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 7 ✓ 2026-10-13 — handshake state machine, pure: StaticKeys, Handshake, Session

**Changed:**
- `src/protocol/crypto/handshake.rs` (new, `pub mod handshake`): `StaticKeys::new` (redacted `Debug`, bad key named by field); `Handshake::new(keys, ephemeral)` + `on_message` → `Step::{Reply{frame, acked}, Wait, Done(Session)}`; `expecting()` gives the reassembly `Layer`; `HandshakeError` typed (OutOfOrder, BadPayloadLength, Rejected, BadStationPoint, BadStationSignature, InvalidKey, Finished, Frame); any error ends the handshake. Signed message = point ‖ temp IV (§4.4; re-checked against rustytti `crypto.rs` d2c651d, fetched to /tmp and deleted). No RNG, no I/O: ephemeral secret and session seeds are inputs.
- `src/protocol/crypto/handshake/sim.rs` (`#[cfg(test)] pub(crate)`): station simulator with test-generated keys, for step 10's `SimStation`.
- `src/protocol/crypto/handshake/tests.rs`: 10 tests — round trip (equal session keys, both directions decrypt, msg 2 acked, msg 5 not), bad signature, wrong station key, out-of-order (1 and 4), truncated (kex + temp), bad length, rejected status, off-curve point, rustytti msg-2 vector, invalid keys + `Debug`.
- `README.md`: credit names `handshake.rs`.

**Check:**
- `cargo test --lib protocol::crypto::handshake` 10 passed.
- Mutation: signature over point only + status check removed → 3 FAILED (round trip, rejection, off-curve); restored → 10 passed.
- `make check` green; `src/protocol/` 99.81%.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM. High-S acceptance stays the §4.4 open risk (real-key test, step 13).

**Carry-forward:** none. **Next:** step 8 — keys via the config module.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:59:24Z

```text

test result: ok. 93 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-871f4c6d900bafc2)

running 2 tests
test cli::scan::tests::unknown_station_still_prints ... ok
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.81% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 8 ✓ 2026-10-13 — static keys via the config module

**Changed:**
- `src/config/keys.rs` (new, `mod keys` in `config.rs`): `CryptoConfig::load_keys() -> StaticKeys` reads `keys_file` in the §4.4 schema; `Config::static_keys()` fails with "encrypted station needs `[crypto] keys_file`" when `[crypto]` is absent (step 11 calls it).
- Unknown-field refusal done by hand over a `toml::Table`, not serde `deny_unknown_fields`: serde/toml errors can quote the value. Errors name field + expected digit count; TOML syntax errors give the line number only.
- Hex: exactly 2·N ASCII hex digits, either case (rejects `0x` and the `+` that `from_str_radix` accepts); invalid P-256 scalar/point → `invalid key: <field>`.
- `config.example.toml`: `[crypto]` comment documents the three fields and their hex lengths.
- Local `config.toml` (gitignored): `keys_file` was the `/etc` example path; now the repo-root `keys.toml` (delegated, Q1).

**Check:**
- `cargo test --lib config::` 17 passed (6 new: handshake to `Done` against the step-7 simulator with loaded keys, hex byte order, bad length/`0x`/`+`/off-curve without quoting the value, unknown/missing/mistyped field, syntax-error line, missing `[crypto]`/file).
- Failing first (mutation): hexdigit check removed → `rejects_bad_field_without_quoting_it` FAILED; restored → pass.
- Real keys: scratch test (removed) loaded local `config.toml` → `static_keys()` ok; first failed on the old `/etc` path (no such file), passed after the local fix.
- `git check-ignore` keys.toml + config.toml: both ignored; `git status --short` lists neither.
- `make check` green; `src/protocol/` 99.81%.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM. Its "real key not loaded" gap is covered by the scratch load above.

**Carry-forward:** none. **Next:** step 9 — `BluerGatt` + A8 + `detect` part 1.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:03:23Z

```text

test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-871f4c6d900bafc2)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.81% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 9 ✓ 2026-10-13 — BluerGatt, A8, `detect` part 1: both stations send the §4.4 challenge

**Changed:**
- `src/transport/bluer/gatt.rs` (new): `find(ble, mac)` = BlueZ cache, then `scan` for `connect_timeout_secs`; `connect(ble, advert)` = `connect_attempts` tries at 2 s doubling backoff, resolve ff00/ff01/ff02, subscribe via `notify_io`; `BluerGatt: Gatt` (write = Write Command). Every D-Bus call under a timeout (`bounded`); a failed connect or setup disconnects; error texts MAC-masked (D-Bus paths hold the address).
- `src/transport/mod.rs`: `TransportError::Ble(String)`; re-exports `BluerGatt`, `connect`, `find`.
- A8 `src/transport/bluer/scan.rs`: shared `advert_of`; a device whose property read fails is skipped with `tracing::debug!` (masked); adapter calls keep `?`.
- `src/cli/detect.rs` (+ `detect/tests.rs`): `detect <station|mac>` prints scan line + `protocol=` + `flag=`, then connected → first-notification length + 4 bytes → disconnected; disconnect runs before the result is used. `main.rs` passes config stations.
- `tests/hardware.rs`: `#[ignore]` test on `BLUETTI_TEST_MAC`: find, connect, first notification, disconnect, `NotConnected` after; encrypted → starts `2a2a0104`.
- A13 not here: `shortcut:` comment in `write`; Write Request lands with `CryptoGatt` (step 10).

**Check:**
- `cargo test --bin bluetti-controller detect` 5 passed. Tests written with the code; mutation: `resolve` returning the target → `station_name_resolves_to_its_address` FAILED; restored → pass.
- `grep -rn bluer src/` outside `src/transport/bluer/`: only `src/transport/mod.rs` lines 4 (doc), 11 (`mod`), 17–18 (re-exports).
- Pre-flight §10.4: adapter powered, rfkill unblocked, `control.enabled = false`, `captures/` ignored; app not checked (owner's phone) — stations advertised, so not connected.
- Hardware (`--config config.toml --anonymise`, MACs also sed-masked):
  - `detect router`: `EB3A… -57 dBm EB3A encrypted=yes protocol=V1 flag=BLUETTF` → connected → `first notification: 10 bytes, starts 2a2a0104` → disconnected, exit 0.
  - `detect living-room`: `Elite200V2 encrypted=yes protocol=V2 flag=BLUETTF` (from BlueZ cache, RSSI `?`) → same 10-byte `2a2a0104` → disconnected, exit 0.
  - `BLUETTI_TEST_MAC=<router> cargo test --test hardware -- --ignored`: 1 passed.
- Finding: both stations speak first with kex type 01, len 4 (10 B = 2+1+1+4+2), as §4.4 says.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 2 MEDIUM, deferred to A17 (no `Drop`/Ctrl-C disconnect fallback) and A18 (`disconnect` retry after a failed D-Bus call reports `Ok`), both home step 10.

**Carry-forward:** none. **Next:** step 10 — `CryptoGatt` (+ A13, A17, A18).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:11:13Z

```text
test cli::detect::tests::unknown_target_passes_through_as_an_address ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-2d8db31929c076b8)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.81% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 10 ✓ 2026-10-13 — `CryptoGatt` over any `Gatt`, A13 + A18

**Changed:**
- `src/transport/crypto.rs` (new): `CryptoGatt::establish(inner, keys, timeout)` runs the step-7 `Handshake`; each station message reassembled by `envelope_len(expecting())` under `timeout`, the whole exchange under 4 × `timeout`; any failure disconnects `inner` first. Then `write` = session envelope under a fresh seed; `next_notification` = one reassembled session envelope, decrypted. Bytes past a frame stay pending; a frame cut off by a timeout is dropped.
- RNG: `getrandom::fill` (getrandom 0.4.3, already locked via uuid; the source `OsRng` wraps) for the ephemeral scalar and per-frame seeds; `protocol/` untouched.
- `TransportError`: `Handshake`, `Crypto`, `Rng` variants (no key/payload bytes).
- A13: `Gatt::write_with_response` (ATT Write Request). `BluerGatt` → `WriteOp::Request`; `RecordingGatt` forwards + records; `ReplayGatt` = `write` (fixtures hold no write kind); handshake message 2 uses it.
- A18: `BluerGatt.disconnect_owed`, cleared only after a D-Bus `Disconnect` succeeds; a failed one is retried by the next `disconnect`.
- A17 moved to step 11 (it rewrites connect + `detect`); still open.
- `shortcut:` no message-5 resend (§4.4 rustytti); add if hardware loses message 6.
- `sim.rs`: `LOCAL_AES_KEY`, `HOST_SCALAR` `pub(crate)` for the transport double.

**Check:**
- `cargo test --lib transport::crypto` 5 passed: registers over `CryptoGatt<SimStation>`; 20-register reply split across 3 notifications; messages 3+4 in one notification; silent station → `Timeout(5s)` after exactly 5 s (paused clock), disconnected; wrong station key → `BadStationSignature`, disconnected.
- `SimStation` drops message 2 sent as a Write Command (§4.4), so the success tests pin A13.
- Mutations: message 2 via `write` → 4 FAILED; `drain(..len)` → `mem::take` → `two_messages_in_one_notification_are_split` FAILED; restored → pass.
- `grep -rn bluer src/` outside `src/transport/bluer/`: only `src/transport/mod.rs` doc + `mod` + re-exports.
- `make check` green; `src/protocol/` 99.81%.
- Noted: A7 (fixed by step 5) still reads `disposition: open`; settle at the close.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
  - MEDIUM: message-5 resend skipped (`crypto.rs:113`) → deferred to PLAN `## Amendments` A19, home step 11.

**Decision:** none. **Carry-forward:** none. **Next:** step 11 — encrypted connect + `detect` part 2 (+ A17, A19).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:18:03Z

```text
test cli::detect::tests::unknown_target_passes_through_as_an_address ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-e834033cf2500684)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.81% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 11 ✓ 2026-10-13 — encrypted `detect` reads both stations; handshake proven on hardware, A17 + A19

**Changed:**
- `src/transport/link.rs` (new): `transport::open(ble, advert, keys)` → `Link::{Plain, Encrypted}` (`Gatt` by delegation; RPITIT trait is not `dyn`). Unknown flag or encrypted-without-keys refused before any connect.
- `src/transport/station.rs` (new): `detect(&mut impl Gatt, &Advert, &ReadPolicy) -> Identity {device_type, layout, firmware}`. Tries the advertised model's layout first (V1 10–15 / V2 110–115 byte-swapped), the other on no printable type; firmware best-effort (V1 23/25, V2 6175; bt `VersionField` low word first, /100), a failed one logged + left out.
- `ReadPolicy::from_ble`: request timeout, 3 tries, 1 s backoff (§4.6).
- `src/cli/detect.rs`: open → `station::detect` raced against `ctrl_c` → disconnect → identity line; first-notification header removed. `main.rs` loads keys whenever `[crypto]` is set (fail fast).
- A17: `impl Drop for BluerGatt` spawns a bounded disconnect while `disconnect_owed` (`shortcut:` best effort on runtime shutdown). SIGINT raced only after `open`; during connect/handshake the default handler still ends the process.
- A19: `crypto.rs` keeps message 5; no message 6 within `min(1 s, timeout)` → resend the identical bytes once, then wait the full timeout.

**Check:**
- `cargo test --lib transport::station transport::link transport::crypto`: 14 passed (5 station over `ReplayGatt`, 3 link refusals, 6 crypto incl. new `lost_message_5_is_resent_once_after_one_second`: handshake done at exactly 1 s, paused clock).
- Mutations: resend write removed → `lost_message_5…` FAILED; layout order `[V1, V1]` → `v1_station_without_a_v1_type_falls_back_to_v2` FAILED; restored → pass.
- Pre-flight §10.4: adapter powered, rfkill unblocked, `control.enabled = false`.
- Hardware (`--config config.toml --anonymise`, MACs sed-masked):
  - `detect router`: `EB3A encrypted=yes protocol=V1 flag=BLUETTF` → `connected, session established` → `device type=EB3A layout=V1 firmware: arm=2197.02 dsp=2196.01` → disconnected, exit 0.
  - `detect living-room`: `Elite200V2 encrypted=yes protocol=V2 flag=BLUETTF` → session established → `device type=Elite 200 V2 layout=V2 firmware: bms=1068.11` → disconnected, exit 0.
- Finding: the encrypted EB3A keeps the V1 layout, so no `EB3A` profile switch. Firmware decoding unverified (H-row via step 14 / §14 Q5).
- `link.rs` imports `BluerGatt`/`connect` via the `transport` re-exports; `grep -rn bluer src/` outside `src/transport/bluer/` + `mod.rs`: only a doc comment in `station.rs`.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM, all deferred (home step 12):
  - SIGINT during `open` unhandled → A20; A17 back to open (partly fixed).
  - 1 s early wait drops a straddling message 6 fragment → A21.
  - identity map/decoders belong in `protocol/` → A22.

**Decision:** none. **Carry-forward:** none. **Next:** step 12 — CLI `read`/`dump`/recording (+ A14, A17, A20–A22).


<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:25:41Z

```text
test cli::detect::tests::unknown_target_passes_through_as_an_address ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-e834033cf2500684)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.81% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 12 ✓ 2026-10-13 — CLI `read` + `dump` + `--record` on hardware; A14, A17, A20–A22 fixed

**Changed:**
- `src/transport/station.rs`: `poll(gatt, profile, policy) -> Poll {values, problems}` = `read_profile` + `decode` + `problems`.
- `src/protocol/identity.rs` (new, A22): identity layouts V1/V2, `layouts`, `is_serial` (moved from `anonymise.rs`), `ascii`, `version`; tests in `identity/tests.rs`. `station.rs` keeps the reads only.
- `src/cli/session.rs` (new): find → `open` → body → disconnect → save capture. SIGINT handler installed before the scan (A20): aborts the scan, is held through `open`, cancels the body; every path past `open` disconnects. `--record` only under `captures/` (relative, no `..`).
- `src/cli/read.rs`: detect + poll in one session, one line per field with `verified=`, then one per problem; any problem → `Err` → exit 1.
- `src/cli/dump.rs`: `--from --to`, chunks ≤ 40, 500 ms pause; `--anonymise` masks 17–20 and 116–119; an exception/timeout chunk is reported and skipped, a broken link ends the sweep.
- `src/cli/{mod,detect}.rs`, `main.rs`: `Context` (config + `--anonymise` + global `--record`); `detect` reuses `session`; `scan --record` refused.
- A21 `src/transport/crypto.rs`: `next_frame` keeps a partial frame on timeout; message 5 is resent only when nothing of message 6 arrived; `next_notification` clears the stale partial itself.
- A14 `device.rs`: `PackCurrent` range `0..=100` (unsigned magnitude); ROADMAP §4.3 row 101 annotated.

**Check:**
- New tests: `read` report + verdict (exit code), `dump` chunks (incl. 65500–65535) + row masking, capture-path guard, identity ×4, `message_6_straddling_the_resend_mark_is_kept_whole`.
- Failing first: the straddling test against HEAD's `crypto.rs` → panicked; new → pass.
- `grep -rn 'pub fn .*request' src/` → only `src/protocol/modbus.rs:37: pub fn read_request`.
- Pre-flight §10.4: powered, rfkill unblocked, `control.enabled = false`.
- Hardware (`--config config.toml --anonymise`, MACs sed-masked):
  - `read router`: EB3A V1 arm=2197.02 dsp=2196.01; soc 94 %, ac_output_on 1, all powers 0 W, ac_input_voltage 0 V; exit 0.
  - `read living-room`: Elite 200 V2; pack 39.51 V, **pack_current 11.20 A**, soc 86 %, ac_output 445 W, ac_input 0 V/0 Hz; exit 0. 445 W / 39.51 V = 11.26 A, so register 101 is an unsigned magnitude while discharging (A14).
  - `dump router --from 10 --to 90`: 3 chunks, 0 unread; 17–20 masked; 43 = 94 (soc), 48 = 1 (AC on), 71 = 230.
  - `--record /tmp/x.json` refused; `--record captures/…` 6 frames, first write `0103000a0006…` (plaintext); file removed.
  - SIGINT right after the advert line: `connected, session established` → `disconnected` → `Error: interrupted`, exit 1; `bluetoothctl devices Connected` → 0.
- Observation for H1: EB3A AC output on but 0 W, AC input 0 V on both stations (grid off at the time?); unverified map, app comparison decides.

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM: late reply to a timed-out `dump` chunk can pass as the next chunk's answer (`dump.rs:40`) → deferred to PLAN `## Amendments` A23, home step 14.

**Decision:** register 101 (pack current) is an unsigned magnitude (A14, hardware). **Carry-forward:** none. **Next:** step 13 — key-free handshake vectors (`--record-wire`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:34:32Z

```text
test cli::detect::tests::unknown_target_passes_through_as_an_address ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-e834033cf2500684)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13 ✓ 2026-10-13 — key-free handshake vectors from both stations; station point is ephemeral

**Changed:**
- `transport/fixture.rs`: `Fixture.ephemeral: Option<String>` (wire captures only, skipped when `None`); `Fixture::save` (moved from `RecordingGatt`).
- `transport/link.rs`: `Link` wraps `Wire = RecordingGatt<BluerGatt>` (wire teed in memory); ephemeral drawn before connect and kept in the tee; `Link::wire()`.
- `transport/crypto.rs`: `establish_with(…, ephemeral)`, pub `ephemeral()`, `inner()`; `record.rs`: `inner()`, `set_ephemeral()`.
- `transport/anonymise/wire.rs` (new): `anonymise_wire` — messages 1–3 verbatim, envelope headers kept (plain_len; + seed for session), ciphertext zeroed at same length, trailing partial zeroed, ephemeral dropped; refuses captures not opening with the kex. `anonymise_fixture` drops `ephemeral` too.
- CLI: global `--record-wire` (captures/ only, like `--record`); `anonymise <in> <out> [--wire]` (no station).
- `tests/handshake_vectors.rs`: key-free structure test over `tests/frames/handshake-{eb3a,elite200v2}.json` (kex 01/02/03, msg 2 recomputed from the seed, temp plain_len 134/7, session plain_len 8 / `read_response_len`, padding, zero ciphertext, distinct seeds); `#[ignore]` real-key replay of `captures/hs-*.json`.
- ROADMAP §4.4 "Station public key": result recorded (different → ephemeral).

**Check:**
- Unit: 4 `anonymise::wire` tests + `wire_below_the_session_records_the_handshake_and_anonymises` (CryptoGatt over a teed SimStation).
- Mutation: session header 6→2 in `wire.rs` → `keeps_messages_1_to_3_and_envelope_headers_only` FAILED; raw capture swapped into the fixture → structure test panicked at "ciphertext kept". Both restored, green.
- Pre-flight §10.4: rfkill unblocked, `control.enabled = false`.
- Hardware (`--anonymise`): `dump router --from 36 --to 49 --record-wire captures/hs-eb3a-{1,2}.json` → 1 chunk, 0 unread, 43 = 93 (soc); EB3A capture 1 shows message 5 sent twice (resend path, A19, live). `dump living-room --from 100 --to 110 --record-wire captures/hs-elite200v2-{1,2}.json` → 0 unread.
- Ignored test `cargo test --test handshake_vectors -- --include-ignored`: 4 captures replayed byte for byte, 2 session frames each decrypted to valid Modbus; station point across 2 sessions: **different** (EB3A, Elite).
- Grep of real MAC (both forms), SN (3 cuts), keys.toml values, ephemeral, raw manufacturer data over `tests/frames/` → 0 matches (count only).

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM (it could not run cargo; gate and grep evidence are mine, above).

**Decision:** station P-256 point is per-session (ROADMAP §4.4). **Carry-forward:** none. **Next:** step 14 — Elite 200 V2 map + §14 Q5 (+ A1, A23).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T09:53:22Z

```text
test recorded_handshakes_have_the_documented_structure ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-e834033cf2500684)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 14 ✓ 2026-10-13 — Elite 200 V2 dump agrees with `ELITE_200_V2`; §14 Q5 filled; A1, A23 settled

**Changed:**
- `transport/read.rs`: `read_once` drains queued notifications before every write (was: between retries only) — A23.
- `transport/read/tests.rs`: `late_reply_to_an_earlier_read_is_drained_first`.
- `protocol/device/tests.rs`: `decodes_elite_200_v2_dump` on the hand-read values below; profile unchanged (no contradiction), all fields `verified: false`.
- ROADMAP §4.3 Elite dump table, §4.5 "Ours", §14 Q5 struck with flag + firmware. PLAN A1, A23 → fixed.

**Check:**
- A23 test failed first (`UnexpectedWrite`: stale notification ahead of the write), green after the drain move; `cargo test --lib transport` 55 ok.
- Pre-flight §10.4: rfkill unblocked, `control.enabled = false`.
- Hardware `dump living-room --anonymise` (BLUETTF, session established, disconnected each time; raw in gitignored `captures/dump-elite-*.txt`), non-zero rows:
  - 100–200: 100=3949 101=98 102=77 103=2 104=249 105=249 107=1 108=1 110–115 `lE ti e 02 0 2V`, 116–119 masked, 120–122=1 123=8 124=10 125=404 130=4304 131=4 142=389 148=485 150=1 152=1959 156=2238 158=404 161=1 162/164/165=404 167=3635 169=230 170=50 171=32; **180–200 exception 0x02**.
  - 1300–1520: **1300–1419 and 1460–1499 exception 0x02** (40-reg chunks); 1420=389 1422=1959 1429=1 1430=389 1431=2300 1432=16 1500=499 1501=1664 1508=1 1509=1 1510=483 1511=2287 1512=21; 6 chunks, 4 unread.
  - 6000–6020: 6000–6002=1 6003=3950 6004=98 6005=77 6006=100 6007=71 6008=34 6009=2 6010=426 6011=108 6012=864 6013=3 6016=1 6017=249 6018=249 6019=5681.
  - Probes: 1300–1314 ok (1303=2238 1305=404 1312=1, 1300/1314=0, grid off); 1315 ok =0; 1470 ok =499; 1440–1459 all 0.
- Cross-check: 389 W / 39.49 V = 9.85 A ≈ 101; 77 % × 2073.6 Wh / 389 W ≈ 246 min ≈ 104/105; PR #49's 152 = battery V/100 contradicted (19.59 V).
- §14 Q5: EB3A BLUETTF V1 ARM 2197.02/DSP 2196.01; Elite BLUETTF V2 BMS 1068.11 (step 11 `detect`).

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM.
- `1315–1419` range overstated (1315 reads alone) → deferred to A24.
- §4.5 bullet lacks firmware → deferred to A25.
- drain covers only already-queued late replies; comment overstates → deferred to A26.

**Decision:** Elite poll block (1300, 15) stays — a 40-register read from 1300 fails, 15 succeeds. **Carry-forward:** none. **Next:** step 15 — replay fixtures.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T10:00:34Z

```text
test recorded_handshakes_have_the_documented_structure ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/hardware.rs (target/llvm-cov-target/debug/deps/hardware-e834033cf2500684)

running 1 test
test connects_subscribes_and_disconnects ... ignored, needs a station in BLE range and BLUETTI_TEST_MAC

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 15 ✓ 2026-10-13 — replay fixtures for EB3A + Elite 200 V2; `tests/replay.rs`; A26 settled

**Changed:**
- `tests/frames/{eb3a,elite200v2}.json`: plaintext `read` sessions (10 frames each), masked by `anonymise`.
- `tests/replay.rs`: `station::detect` + `station::poll` over `ReplayGatt` per station, as `read` runs them; asserts identity, every decoded value, no problems, fixture fully consumed.
- A26: `transport/read.rs` `drain` doc narrowed; `cli/dump.rs` flags the chunk read right after a timed-out one.

**Check:**
- Pre-flight §10.4: rfkill unblocked, adapter powered, `control.enabled = false`.
- Hardware: `read router --anonymise --record captures/eb3a.json` → exit 0 (soc 90, ac_output_on 1, rest 0; ARM 2197.02 / DSP 2196.01). `read living-room --anonymise --record captures/elite200v2.json` → exit 0 (39.31 V, 9.9 A, soc 69, AC out 408 W; BMS 1068.11).
- Hand-read: EB3A 43=0x5a, 48=1; Elite 100=0x0f5b, 101=0x63, 102=0x45, 142=0x198; 6175–6176=0xa13b/1. Serial regs 17–20 / 116–119 zero.
- Green on arrival (tests over existing code); teeth: one mutated fixture request byte + one wrong expected value → both tests FAILED; restored.
- Grep of real MAC (4 separator forms), SN (digits, first/last 8, hex both word orders, non-zero words), keys.toml values over `tests/frames/` → 0 matches (count only).

**reviewer:** INTENT satisfied (15, A26); 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM: a timeout recovered on retry inside `read_block` never sets `dump`'s flag; doc overclaims → deferred to A27.

**Decision:** none. **Carry-forward:** none. **Next:** step 16 — acceptance run.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T10:24:16Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 16 ✓ 2026-10-13 — acceptance run: EB3A 20/20, Elite 200 V2 20/20

**Changed:**
- No source change. `make build` (release, `--locked`); the run script and its output live in gitignored `captures/` (`acceptance.sh`, `acceptance-<station>.log`, one `.out` per poll).

**Check:**
- Pre-flight §10.4: rfkill unblocked, adapter powered, `control.enabled = false`.
- Each poll: `timeout -s INT -k 5 55 target/release/bluetti-controller --config config.toml --anonymise read <station>`, then sleep to the end of the 60 s slot. One process at a time, router then living-room, so no two connections overlap (golden rule #5). SIGINT on timeout lets the CLI disconnect (A20); no timeout fired.
- `router` (EB3A, ARM 2197.02 / DSP 2196.01): 13:25:37–13:44:36 +03:00, **20/20 ok**, exit 0 each, 20–21 s per poll. soc 89→88 %, ac_output_on 1, powers 0 W. `ac_output_power` 0 W is known wrong: the outlet fed the router (~9 W) — CF-4 (owner, 468178c), open; exit 0 does not clear it.
- `living-room` (Elite 200 V2, BMS 1068.11): 13:44:40–14:03:56 +03:00, **20/20 ok**, exit 0 each, 19–20 s (poll 1: 4 s). Polls 1–5: ~444 W AC out at 11.1–11.3 A; poll 6: 190 W / 7.2 A; polls 7–20: 0 W / 0.2 A (load switched off on site). soc 60→58 %, AC input 0 V throughout.
- ok = exit 0, so every profile field decoded and passed `problems` (range plausibility; CF-4 is outside what a range check sees). Every `.out` ends in `disconnected`.
- Gate (RULES.md v0.3, loop part): ≥ 19/20 per station → met for both. App comparison stays H1/H2 in `HUMAN-CHECKLIST.md`.

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM: entry claimed EB3A decoding plausible without naming CF-4 → fixed in this entry (own uncommitted text).

**Decision:** none. **Carry-forward:** CF-4 still open (discharge before close). Owner 774df8f puts the AC180 (`mobile`) in scope: needs a PLAN amendment (scan/detect/read + 20-poll run) before the close. **Next:** that amendment, then step 17.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:06:00Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Plan amendment ✓ 2026-10-13 — AC180 + CF-4 added as steps 18–22

**Changed:**
- `PLAN.md` `## Amendments`: dated note + checkbox steps 18–22 (AC180 identity/read, fixtures, acceptance run, CF-4 dump, AC180 docs close-out), per the owner decision of 14:05 (774df8f) and CF-4 (468178c). Frozen steps 1–17 untouched; step 17 runs first, then 18–22.

**Check:**
- `where.sh --json` → step 17 of 22, title unchanged; `make amendments` counts only `- A<n>` items, so the new checkboxes are not findings.

**reviewer:** batched — one pass over `@{u}..HEAD` (468178c, 774df8f, 274a955 + this commit) before the push.

**Decision:** none new (owner's). **Carry-forward:** CF-4 now has a home: step 21. **Next:** step 17.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:08:04Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Batch review ✓ 2026-10-13 — 468178c, 774df8f, 274a955, 3bcd58a

**reviewer:** INTENT satisfied (468178c, 774df8f, 274a955); shortfall (3bcd58a). 0 CRITICAL / 1 HIGH / 2 MEDIUM.
- HIGH: step 17's check cannot pass with AC180 boxes annotated `→ step <n>` (ROADMAP.md:968, :985) → fixed: dated note in PLAN `## Amendments` extends step 17's exclude regex; step 22 keeps the strict one.
- MEDIUM: §12 preamble (ROADMAP.md:933) and BRIEF **Out** still exclude the AC180 → deferred to A28.
- MEDIUM: step 19 has no plaintext-AC180 branch → deferred to A29.

**Next:** step 17.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:10:03Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 17 ✓ 2026-10-13 — docs close-out (EB3A + Elite)

**Changed:**
- `ROADMAP.md` §M1/§M2: boxes ticked on steps 1–16 evidence; AC180 items split out as `→ step 18/19/20`; app comparisons and `verified` flips → H1–H3; sidecar fallback n/a. Headings drop `v0.2-ble`/Phase 3 (A9).
- `ROADMAP.md` M5 + §10.2/§10.3: user-level unit on the dev laptop; homelab install after the MVP (A9). §4.3 Elite row 1300–1419 (A24). §4.5 firmware (A25). §4.9 crypto row: locked versions (match `Cargo.lock`: p256 0.14.0, aes 0.9.3, cbc 0.2.1, md-5 0.11.0).
- `HUMAN-CHECKLIST.md` H1–H3: the fields a pass flips (SoC + four powers per profile), a fail opens a fix task (A10, old step 15).
- `README.md`: status line, `make tools`, `make test` meaning (A4); credits already complete (MIT sources + crypto port).
- `AGENTS.md` §4/§11, `loop/README.md`, `.claude/commands/loop-step.md`: human-only list points at RULES.md; dev-laptop user unit is not a production deploy (A11).
- `src/transport/read.rs` `drain` doc narrowed to "after a failed chunk" (A27).
- `loop/STATE.md`: CF-2 discharged. PLAN dispositions: A4 A7 A9 A10 A11 A24 A25 A27 fixed.

**Check:**
- step check (amended regex) → `0`; open boxes left: AC180 → steps 18/19/20 only.
- `make amendments` → 2 open: A28 (home step 22), A29 (home step 19) — later steps by design; every step-17 home settled.
- `make loop-hygiene` → OK.
- Done-when bluer grep: `grep -rn bluer src/ | grep -v '^src/transport/bluer/'` → only `src/transport/mod.rs` (doc line, `pub mod bluer;`, two re-exports) and a doc comment in `src/transport/station.rs`; no `use bluer` outside the adapter.

**reviewer:** INTENT satisfied (except the step's own `make amendments` check, see A30). 0 CRITICAL / 0 HIGH / 3 MEDIUM.
- MEDIUM: step 17's "none open" check not met (A28/A29 homed later) → deferred to A30.
- MEDIUM: EB3A "met, 20/20" hides CF-4 plausibility gap → deferred to A31 (home step 21).
- MEDIUM: §10.2 lost its system unit → deferred to A32.

**Decision:** none. **Carry-forward:** CF-2 discharged. **Next:** step 18 (AC180 identity + first read).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:14:43Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18 ✓ 2026-10-13 — AC180 `BLUETTF`, V2 layout, BMS 1052.03; `read` exit 0

**Changed:** none in `src/` — `read` reported no problem, so no `dump` and no `AC180` profile fix (step text's conditional branch not taken).

**Check (hardware, read-only 0x03, one station, pre-flight spec §10.4):**
- Pre-flight: `bluetoothctl show` Powered yes; `rfkill` not blocked; `control.enabled = false` (config.toml:36); app not connected (handshake completed). App firmware not noted — owner-side, H3.
- `make build` → release binary OK.
- `scan --anonymise` → `AC180… -61 dBm AC180 encrypted=yes` (EB3A, Elite also seen).
- `detect mobile --anonymise` → `protocol=V2 flag=BLUETTF`, `connected, session established`, `device type=AC180 layout=V2 firmware: bms=1052.03`, `disconnected`.
- `read mobile --anonymise` → EXIT=0 (second run, exit captured; first run identical values):
  - pack_voltage 33.09 V · pack_current 0.90–1.00 A · battery_soc 93 %
  - dc_out 0 W · ac_out 0 W · dc_in 0 W · ac_in 0 W
  - dc_input_voltage 0.10 V · ac_input_frequency 0 Hz · ac_input_voltage 2.10 V
  - ac_output_on 1 · dc_output_on 0 · every field `verified=false` (Q4)
- Notes for H3 (not gate items): ~1 A pack draw at 0 W out fits an idle inverter with AC on; 2.1 V AC in with 0 Hz is residual noise, no grid input.
- A29: `mobile` is encrypted, so step 19 runs as written (wire fixture + real-key replay).

**reviewer:** batched (LOG-only, no code; reviewed with the next batch before the push)

**Decision:** none. **Carry-forward:** none. **Next:** step 19 (AC180 fixtures).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:16:52Z

```text

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 2 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 19 ✓ 2026-10-13 — AC180 replay + handshake fixtures; real-key replay of 6 captures

**Changed:**
- `tests/frames/ac180.json`: plaintext `read` session (14 frames), masked by `anonymise`.
- `tests/frames/handshake-ac180.json`: wire capture of `dump mobile --from 100 --to 110` (9 frames), masked by `anonymise --wire`; 100–110 holds no serial register (V2 serial = 116–119).
- `tests/replay.rs`: `ac180_read_session_replays` — identity + all 12 values + no problems + fixture consumed.
- `tests/handshake_vectors.rs`: `FIXTURES` gains `("handshake-ac180", 11)`.

**Check:**
- Pre-flight §10.4: rfkill unblocked, adapter powered, `control.enabled = false` (config.toml:36).
- Hardware (`--anonymise`, MACs sed-masked): `read mobile --record captures/ac180.json` → EXIT=0, 14 frames (33.09 V, 1.00 A, soc 93, AC out 0 W, ac_output_on 1; BMS 1052.03). `dump mobile --from 100 --to 110 --record-wire captures/hs-ac180-{1,2}.json` → 1 chunk, 0 unread, EXIT=0 each.
- Hand-read: 100=0x0ced, 101=0x0a, 102=0x5d; 110–112 "AC180"; 6175–6176=0x9af3/1; 140–147 = 0; 1213=1; 1300=0, 1314=0x15; 2011=1, 2012=0. Serial regs 116–119 zero in the fixture.
- Green on arrival (tests over existing code); teeth: expected AcInputVoltage 2.1→2.2 → `ac180_read_session_replays` FAILED; raw `hs-ac180-1` as the fixture → structure test panicked ("ciphertext kept"). Both restored.
- Ignored test `cargo test --test handshake_vectors -- --ignored --nocapture`: 6 captures replayed byte for byte, 2 session frames each decrypted; AC180 station point across 2 sessions: different (ephemeral, as EB3A/Elite).
- Grep of real MACs (4 separator forms + reversed), SN (digits, first/last 8), raw serial words, ephemerals, keys.toml values over `tests/frames/` → 28 patterns, 0 matches (count only).
- A29: `mobile` is `BLUETTF`, so the plaintext branch does not apply → fixed (step ran as written).

**reviewer:** batch (581ca45 step 18 + step 19): INTENT satisfied for both; 0 CRITICAL / 0 HIGH / 0 MEDIUM. It re-checked every hand-read value against the fixture frames and re-ran the count-only secret grep (0 hits).

**Decision:** none. **Carry-forward:** none. **Next:** step 20 (AC180 acceptance run).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T11:29:09Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
