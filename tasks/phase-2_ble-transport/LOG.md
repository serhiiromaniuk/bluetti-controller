# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers. Budget: ≤40 lines per entry. Shape:
> `tasks/_template/LOG.md`.

## 2026-10-13 — Task opened
- Opened from `tasks/_template/` after `where.sh` reported no in-progress task;
  STATE.md gate: owner resumed phase 2 on 2026-10-13.
- BRIEF from ROADMAP.md §12 M1 + RULES.md v0.2 acceptance; carry-forwards
  CF-1 and CF-3 name phase 2 and are in scope.
- PLAN.md is the template: `planner` + `plan-reviewer` write it next iteration.

**reviewer:** batched (task-open docs, no code)
**Next:** `needs_plan` — planner writes PLAN.md, plan-reviewer audits it.

## 2026-10-13 — PLAN.md written and audited
**Changed:** `PLAN.md` — 16 steps, 4 owner questions (Q1 addresses, Q2 app
firmware, Q3 AC180-off scope of §14 Q5, Q4 hardware window), one parallel group A
(steps 2, 3; step 2 in its own worktree).

**Check:** `loop/where.sh --json` parses it — step 1/16, `unasked_questions` 4.

**plan-reviewer** (0 CRITICAL · 2 HIGH · 3 MEDIUM · 2 LOW), all fixed in the plan
before step 1:
- HIGH F1 step 8 too big, no hardware evidence until step 11 → fixed: split into
  8 (adapter + `scan` CLI, proven by `scan`) and 9 (`BluerGatt` + `detect`, proven
  by `detect`); old step 9 folded in.
- HIGH F2 a real MAC could reach LOG.md, secret scan has no MAC shape → fixed:
  step 7 extends `builtin_scan` with a non-placeholder MAC pattern; steps 11–13
  name stations by config name only.
- MEDIUM F3 fixture lacks advert, cores not generic → fixed: fixture header
  `{name, manufacturer_data}`; `detect`/`read` take `&mut impl Gatt` + `Advert`.
- MEDIUM F4 step 11 needs Q1 → fixed: Q1 blocks 9, 11, 12, 13; Q4 adds 8, 9.
- MEDIUM F5 `.tools/` missing in merge-gate worktrees → fixed: resolved from the
  main worktree; check adds a throwaway-worktree `make check`.
- LOW F6/F7 → fixed: `timeout 55` per poll slot; PROMPT §1 `human-verified` shape.

**reviewer:** n/a — plan increment, its grader is the plan-reviewer (PROMPT §1).
**Next:** `unasked_questions` 4 — ask Q1–Q4 in one batch.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:22:04Z

```text
test protocol::device::tests::decodes_eb3a_block ... ok
test protocol::device::tests::detects_model_from_ble_name ... ok
test protocol::device::tests::drops_implausible_soc ... ok
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

>> make check: all gates green
```

<!-- gate:end -->

## 2026-10-13 — Batched review of 3 unpushed docs commits
**Scope:** `cb3e584..9f40a49` (owner resume record, task open, plan). Intent:
d571b0d and 9f40a49 satisfied; cb3e584 shortfall.
- HIGH H1 `ROADMAP.md` §14 Q6 struck as "defaults approved" with no owner record
  of a names approval (the STATE decision covers station availability only) →
  fixed: Q6 row re-opened; asked again as PLAN Q5 (blocks 16).
- MEDIUM M2 `loop/STATE.md` "Q2, Q6 answered" vs ROADMAP "Q2 partly" → fixed in
  the same line as H1 (shared edit).
- MEDIUM M1 BRIEF "scan near all stations" vs AC180 off → deferred to A1.
- No second pass: the H1 fix is doc wording, no logic.

**reviewer:** batched pass done; push after this commit.
**Next:** `unasked_questions` 5 — ask Q1–Q5 in one batch.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:23:36Z

```text
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
>> make check: all gates green
```

<!-- gate:end -->

## 2026-10-13 — Owner questions Q1–Q5 asked in one batch
Owner: please answer each in `PLAN.md` `## Questions for the owner` (replace
`answer: asked 2026-10-13` with your answer), or tell the next session.
- **Q1** (blocks 9, 11, 12, 13) — station MACs: you write a gitignored local
  `config.toml` (human-only), or the agent passes the `scan` MAC on the command
  line only, never into a file?
- **Q2** (blocks 11) — firmware versions of EB3A, Elite 200 V2, AC180 as the
  official app shows them (§14 Q5, cross-check of EB3A regs 23/25).
- **Q3** (blocks 11) — AC180 off: accept "scan near all stations" as met by EB3A +
  Elite, AC180 carried as CF-4 → phase 3 (also settles A1)? Or switch it on for
  one read-only scan?
- **Q4** (blocks 8, 9, 11, 12, 13) — app force-quit on every phone, laptop in BLE
  range, EB3A on: always true, or only in a window you name?
- **Q5** (blocks 16) — station names (§14 Q6): approve `router` (EB3A), `mobile`
  (AC180), `living-room` (Elite 200 V2), or give names.

**Changed:** `PLAN.md` — Q1–Q5 `answer: pending` → `asked 2026-10-13`.
**Check:** `loop/where.sh --json` → `unasked_questions` 0, `waiting_on` [] for step 1.
**reviewer:** batched (doc-only, low-risk).
**Next:** step 1 — add the M1 dependencies (waits on no question).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:24:16Z

```text
test protocol::device::tests::detects_model_from_ble_name ... ok
test protocol::device::tests::drops_implausible_soc ... ok
test protocol::device::tests::decodes_eb3a_block ... ok
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-10-13 — M1 dependencies added at the §4.9 versions

**Changed:**
- `Cargo.toml` — anyhow, bluer (`bluetoothd`), clap (`derive`), serde (`derive`),
  serde_json, thiserror, tokio (rt-multi-thread, macros, signal, time, sync), toml,
  tracing, tracing-subscriber (`env-filter`). No code uses them yet.
- `Cargo.lock` — `make lock`: 117 packages; every direct dep resolves to its §4.9
  version (toml `1.1.8+spec-1.1.0`). uuid held at 1.26.1 (1.28.0 needs Rust 1.89;
  `rust-version` = 1.85).
- `loop/STATE.md` — CF-1 struck, discharged.

**Check:** `cargo build --locked` compiled `dbus-tokio`/bluer, finished OK;
`make check` green. Green on arrival: a deps-only step has no assertion to fail
first.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM — step 1 (risky: build config)
plus batched 12ccbb5 (Q1–Q5 asked); INTENT satisfied for both.

**Decision:** none
**Carry-forward:** CF-1 discharged in `loop/STATE.md`.
**Next:** steps 2+3 — parallel group A (`config.rs`; CF-3 `expected_frame_len`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:26:08Z

```text
test protocol::device::tests::drops_implausible_soc ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo clippy --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-10-13 — config loading with fail-fast validation (group A)

**Changed:**
- `src/config.rs` — spec §9 schema, `deny_unknown_fields` on every table;
  `load(path)` / `parse(text)` via anyhow with file-path context.
  Zero timeouts/attempts/poll interval rejected by `NonZeroU64/U32` types;
  model mapped by a custom deserializer (keeps `src/protocol/` serde-free);
  `validate()` checks MAC shape and unique names (error names the station, never
  the address). `[ble]`, `[control]`, `[[station]]` optional with defaults;
  `[crypto]` optional (phase 3); `BleConfig::default()` = §9 values.
- `config.example.toml` — §9 verbatim, `AA:BB:CC:DD:EE:0x` placeholders, Q5
  default names.
- `src/lib.rs` — `pub mod config;`, crate doc no longer says "M0 only".
- Group A run serially in the main tree: disjoint files, no concurrency, so no
  worktree (AGENTS.md §8a applies to concurrent edits).

**Check:** `cargo test --lib config::` — with `validate()` stubbed to `Ok(())`,
`rejects_duplicate_name` and `rejects_bad_mac` FAILED (7 passed, 2 failed);
restored, 9/9 pass. The other rejections are serde-enforced and assert on the
error text. `control_defaults_to_disabled` asserts its edit happened.

**reviewer:** see step 3 entry (one batched pass over the group).

**Next:** step 4 — `cargo llvm-cov` in `make check`.

## Step 3 ✓ 2026-10-13 — CF-3 exception-aware frame length (group A)

**Changed:**
- `src/protocol/modbus.rs` — `expected_frame_len(prefix, count)`: `None` below
  2 bytes, 5 when function byte & 0x80, else `read_response_len(count)`
  (spec §4.2). Module doc points the reassembler at it.
- `loop/STATE.md` — CF-3 struck, discharged.

**Check:** `cargo test --lib modbus::` — failed first (E0425:
`expected_frame_len` not found), then 9/9 pass incl.
`frame_len_stops_at_five_on_exception` (vector `01 83 02 <crc>`, prefix and
full frame) and `frame_len_of_normal_reply` (count 40 → 85).

**Carry-forward:** CF-3 discharged.

**reviewer (group A, before commit):** INTENT satisfied for steps 2 and 3, no
creep; 0 CRITICAL / 0 HIGH / 2 MEDIUM.
- MEDIUM derived `Debug` would log MAC/token unmasked → deferred to A2.
- MEDIUM `[metrics]` floats unvalidated → deferred to A3.

**Next:** step 4 — `cargo llvm-cov` in `make check`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:29:00Z

```text
test protocol::modbus::tests::frame_len_stops_at_five_on_exception ... ok
test config::tests::rejects_bad_mac ... ok
test protocol::modbus::tests::frame_len_needs_two_bytes ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::reports_exception ... ok
test config::tests::rejects_zero_timeout_and_attempts ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo clippy --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.06s
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-10-13 — `cargo llvm-cov` coverage gate in `make check`

**Changed:**
- `Makefile` — `make tools` installs `cargo-llvm-cov` 0.9.1 (`--locked`) into
  `.tools/` of the main worktree (path from `git rev-parse --git-common-dir`).
  `test` = one `cargo llvm-cov --all-targets --locked` run, `tests.rs` files
  ignored; jq (`COV_FILTER`) sums `/src/protocol/` lines, fails below `COV_MIN`
  (80) or when no protocol file is found. Missing tool: "run: make tools".
- Protocol unit tests moved verbatim to `src/protocol/{modbus,device}/tests.rs`
  (excluded from coverage, so test code never covers itself) + 6 new tests:
  short frame, unexpected function, `FrameError` texts, metric keys/units,
  `field`, output-flag plausibility.
- `rust-toolchain.toml` `llvm-tools-preview`; `.gitignore` `/.tools/`;
  `AGENTS.md` command list.

**Check:**
- inline tests counted: 81.7% (inflated, see reviewer H1).
- tests excluded, before new tests: `src/protocol/ line coverage 69.38% < 80%`
  — red; fixed the work, not the threshold.
- after: `make check` green, `src/protocol/ line coverage 100% (min 80%)`
  (device.rs 79/79, modbus.rs 68/68).
- forced `make test COV_MIN=101` → `… 100% < 101%`, exit 2 — the gate bites.
- throwaway `git worktree add --detach` + this diff: `make check` green via the
  main tree's `.tools/`.

**reviewer (before commit):** INTENT satisfied; 0 CRITICAL / 1 HIGH / 1 MEDIUM.
- HIGH H1 inline test lines inflate protocol coverage (prod ~68.5%) → fixed
  (above); second pass on the fix: 0 CRITICAL / 0 HIGH, tests byte-identical.
- MEDIUM README lacks `make tools` → deferred to A4.
- Side note (not a finding): is `PackCurrent` signed? Check at step 12 decode.

**Next:** step 5 — `Gatt` trait, `ReplayGatt`, fixture model.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:35:20Z

```text
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test config::tests::rejects_unknown_model ... ok
test protocol::modbus::tests::rejects_short_frame ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::rejects_unexpected_function ... ok
test protocol::modbus::tests::reports_exception ... ok
test config::tests::rejects_zero_timeout_and_attempts ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 5 ✓ 2026-10-13 — `Gatt` trait, fixture model, `ReplayGatt`

**Changed:**
- `src/transport/mod.rs` — `Gatt` trait (`write` → ff02, `next_notification(timeout)`
  ← ff01, `disconnect`; `impl Future + Send`), bluer-free `Advert`, `TransportError`.
- `src/transport/fixture.rs` — `Fixture {advert: {name, manufacturer_data}, frames:
  [{t_ms, dir, uuid, hex}]}` (spec §11), `deny_unknown_fields`, `load`/`from_json`,
  `advert()` under placeholder `AA:BB:CC:00:00:00`, strict hex encode/decode.
- `src/transport/replay.rs` — `ReplayGatt`: decodes + uuid-checks every frame in
  `new`; a write must equal the next recorded write (rejected write consumes
  nothing); notifications served in order; none due → `Timeout` at once;
  after `disconnect` → `NotConnected`.
- `src/lib.rs` — `pub mod transport;`.

**Check:** `cargo test transport::` 10 tests.
- Failed first: `hex_round_trips_and_rejects_garbage` — `decode_hex("+1")`
  accepted (`from_str_radix` takes a sign) → fixed with an all-hexdigit check.
- Mutation: write matching any bytes → `unexpected_write_is_error` FAILED.
- `make check` green, 39 tests, protocol coverage 100%.

**reviewer (before commit):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM.

**Decision:** replay time does not pass: a missing notification times out
immediately, so retry tests stay fast.
**Carry-forward:** none
**Next:** step 6 — `read_block` / `read_profile` over `Gatt`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T23:40:31Z

```text
test transport::fixture::tests::parses_spec_shape ... ok
test transport::fixture::tests::rejects_unknown_key_and_dir ... ok
test transport::replay::tests::new_rejects_bad_frames ... ok
test transport::replay::tests::serves_fragments_in_order ... ok
test transport::replay::tests::disconnect_ends_the_session ... ok
test transport::replay::tests::unexpected_write_is_error ... ok
test transport::replay::tests::exhausted_fixture_times_out ... ok
test config::tests::rejects_zero_timeout_and_attempts ... ok

test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
