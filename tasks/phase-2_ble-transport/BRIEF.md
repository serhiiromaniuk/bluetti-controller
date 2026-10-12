# Phase 2 — BLE transport + CLI, EB3A  |  phase-2_ble-transport

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
A BLE transport behind our own `Gatt` trait, a replay transport with fixture
recording, config loading, and a read-only CLI (`scan`, `detect`, `read`,
`dump`) validated on the EB3A (milestone M1).

## Why
Phase 2 of the RULES.md phase table. Phase 1 closed (`v0.1-foundation`) and the
owner resumed development on 2026-10-13. Every later phase reads stations
through this transport, so its timeouts, retries and connect-read-disconnect
discipline (golden rule #5) are set here.

## Scope
**In:** dependencies (ROADMAP.md §12 M1); `src/config.rs` with validation and
`config.example.toml` (§9); `transport::Gatt` trait, `BluerGatt` adapter (the
only `bluer` import), `ReplayGatt`, `--record` and the MAC/SN anonymiser;
exception-aware reassembly length (CF-3); CLI `scan`, `detect`, `read`, `dump`
with function 0x03 only; an anonymised EB3A fixture and a replay test of the
full read path; `cargo llvm-cov` in `make check` (≥ 80% on `src/protocol/`);
the read-only `scan` near all stations to answer §14 Q5; the EB3A acceptance
run (20 polls at 60 s).
**Out:** encryption handshake and V2 profiles (phase 3, CF-2); collector,
storage, metrics (phase 4); web UI and deploy (phase 5); any write frame
(0x06/0x10), pairing, or device control (golden rule #2, human-only).

## Done when
- [ ] `make check` green with `cargo llvm-cov` wired in and `src/protocol/` ≥ 80%
      line coverage — check: `make check` output in LOG.md
- [ ] `bluer` is imported only under `src/transport/` — check: `grep -rn bluer src/`
- [ ] CF-3: reassembly length stops at 5 bytes on a Modbus exception — check:
      exception-vector unit test
- [ ] Replay test drives `detect` + `read` from an anonymised EB3A fixture —
      check: the test passes and the fixture has no real MAC/SN (scan in LOG.md)
- [ ] `scan` run near all stations; encryption flag and firmware of each
      recorded in ROADMAP.md §14 Q5 — check: scan output in LOG.md
- [ ] EB3A acceptance run: ≥ 19/20 polls at 60 s succeed — check: run output in
      LOG.md
- [ ] **Human:** battery % equals the official app, power within ±5 W; confirmed
      fields marked `verified: true` — check: owner's `human-verified` line in
      LOG.md
- [ ] the phase gate passes — see RULES.md / AGENTS.md §6

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #2/#3/#5, phase table) · ROADMAP.md §4.1–4.3, §4.6,
  §4.9, §5, §9, §11, §12 (M1), §14 Q5 · loop/STATE.md CF-1, CF-3
- Related files: `src/protocol/modbus.rs`, `src/protocol/device.rs`
