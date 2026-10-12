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
