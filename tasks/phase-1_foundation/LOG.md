# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns).

## Steps 1–4 ✓ 2026-10-13 — Foundation drafted in the planning session

**Changed:** ROADMAP.md (spec + research + milestones), RULES.md, AGENTS.md,
CLAUDE.md, loop/, tasks/, .claude/agents + commands, Makefile, .gitignore,
src/protocol/{modbus,device}.rs.

**Check:** `cargo test` — 11 passed, 0 failed (modbus: CRC vectors 0xCDC5 /
0x0A84, request layout, parse, bad CRC, exception, truncated reassembly;
device: BLE names, encryption flag, block coverage, EB3A decode, SOC plausibility).
`make check` not run: `rustfmt` and `clippy` are not installed on the dev host (Q2).

**reviewer:** not run — initial scaffold commit made in the planning session.

**Decision:** ROADMAP.md §3 ADR-001…008 (local BLE in Rust, community handshake
port, no APK reverse-engineering, stack, storage, UI, keys outside repo, host power).
No CI/CD (owner, 2026-10-13).
**Carry-forward:** see loop/STATE.md.
**Next:** step 5 — `make check` green (needs Q2).

## Step 5 ✓ 2026-10-13 — toolchain pinned, `make check` green

**Changed:** `rust-toolchain.toml` (1.99.0, rustfmt, clippy, minimal profile);
`src/protocol/*.rs` rustfmt only; RULES.md conventions, AGENTS.md prerequisites,
Makefile header, ROADMAP.md §4.9 / §12 M0 / §13 R12 now point at the toolchain file.

**Check:** `make check` — first run red on `fmt-check` (formatting only; clippy
and 11 tests green), `make format`, then green. Evidence below.

**reviewer:** batched (low-risk: toolchain pin + mechanical formatting).

**Decision:** toolchain via rustup, exact version in `rust-toolchain.toml`;
`rust-version` 1.85 stays as the MSRV (owner answer Q2).
**Carry-forward:** none.
**Next:** step 6 — owner review of RULES.md + ROADMAP.md (needs Q1).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T22:11:55Z

```text
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::device::tests::drops_implausible_soc ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::read_request_layout ... ok
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

## Owner sign-off 2026-10-13 — RULES.md approved

human-verified 2026-10-13: RULES.md (golden rules, phase table, acceptance thresholds) → approved as written.
Recorded at the owner's request in an interactive session ("i approve RULES.md").
Step 6 stays open: ROADMAP.md review and the ROADMAP.md §14 answers are pending.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T22:15:29Z

```text
test protocol::device::tests::decodes_eb3a_block ... ok
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::device::tests::drops_implausible_soc ... ok
test protocol::device::tests::detects_model_from_ble_name ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

>> make check: all gates green
```

<!-- gate:end -->

## Step 6 ✓ 2026-10-13 — Owner sign-off: ROADMAP.md approved

human-verified 2026-10-13: ROADMAP.md (scope, decisions, milestones) → approved as written.
Recorded at the owner's request in an interactive session ("i approve ROADMAP.md too").
With RULES.md approved earlier, step 6 is complete. ROADMAP.md §14 Q2–Q8 stay open; none blocks Phase 1.
**Next:** task close — OUTCOME.md, close-reviewer, tag `v0.1-foundation`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-12T22:18:58Z

```text
test protocol::device::tests::reads_encryption_flag ... ok
test protocol::device::tests::every_field_is_covered_by_a_block ... ok
test protocol::modbus::tests::crc_matches_reference_vectors ... ok
test protocol::modbus::tests::parses_registers ... ok
test protocol::modbus::tests::read_request_layout ... ok
test protocol::modbus::tests::rejects_bad_crc ... ok
test protocol::modbus::tests::rejects_truncated_reassembly ... ok
test protocol::modbus::tests::reports_exception ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
>> make check: all gates green
```

<!-- gate:end -->
