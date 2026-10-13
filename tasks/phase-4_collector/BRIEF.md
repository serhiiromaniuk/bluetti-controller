# Phase 4 — Collector, storage, metrics  |  phase-4_collector

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
A `serve` subcommand that polls every configured station on a schedule, stores
the readings in SQLite and derives the outage log, runtime forecast and energy
accounting (milestone M3).

## Why
Phase 2 closed with `v0.3-encryption`: all three stations read over local BLE.
Phase 4 turns one-shot reads into a reliable, restart-safe collector — the core
of the MVP (golden rules #1, #4, #5; RULES.md phase table, v0.4 gate).

## Scope
**In:** ROADMAP.md §12 M3 — `store` (migrations per §6, single writer task,
read-only connection, retention + 5-minute rollups), `collector` (sequential
poll loop per §5.3–§5.5, pause/shutdown via `watch`, `poll_failure` rows,
`unreachable` state), `metrics` (§7.1 grid presence + debounce, §7.2 house
outage log, §7.3 runtime forecast incl. the low-load floor, §7.4 energy
integration, §7.5 extras into `sample_extra`), hourly `register_snapshot`,
`serve` with a `/healthz` web stub, the config sections these need (§9:
`[storage]`, `[metrics]`, `[web] listen`) through `src/config.rs`; CF-5
(grid-on `read` of Elite/AC180 regs 1314 and 1300) before outage logic relies
on them; the 12 h acceptance run on the dev laptop (owner, 2026-10-13; was 24 h).
**Out:** web pages, JSON API, SSE (phase 5, M4); systemd unit and 7-day soak
(phase 5, M5); any write frame (0x06/0x10) or device control (golden rule
#2); notifications, Prometheus, cloud (§2); battery-health analytics (R1).

## Done when
- [ ] `make check` green with `src/protocol/` and `src/metrics/` ≥ 80% line
      coverage — check: `make check` output in LOG.md
- [ ] Migrations create the §6 schema; a restart keeps open outage events and
      rollup progress (no lost or double-counted event) — check: store tests
- [ ] Unit tests for every metric with hand-computed values: flapping input,
      missing polls / gaps, unknown state, low-load floor — check: tests pass
- [ ] Collector driven end-to-end from replay fixtures (no hardware): samples,
      `poll_failure` rows and `unreachable` state land in the DB — check: test
- [ ] `bluer` still only under `src/transport/bluer/`; config read only in
      `src/config.rs` — check: grep in LOG.md
- [ ] CF-5 discharged: grid-on `read` of Elite 200 V2 / AC180 regs 1314 and
      1300, values in LOG.md
- [ ] 12 h `serve` run: ≥ 95% successful polls per configured station
      (≤ 36 misses of 720), DB < 10 MB per station-day — check: SQL counts
      and file size in LOG.md
- [ ] Unplug test is a row in `HUMAN-CHECKLIST.md` (H4, deferred; a real grid
      outage recorded during the run satisfies it) — check: the row exists
- [ ] the phase gate passes (`v0.4-collector`) — see RULES.md / AGENTS.md §6

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #1/#2/#4/#5, architecture rules, v0.4 gate, deferred
  physical checks) · ROADMAP.md §5.3–5.5, §6, §7, §9, §11, §12 M3, §4.3
  (low-load floor) · loop/STATE.md CF-5 · HUMAN-CHECKLIST.md H4
- Previous task: `tasks/phase-2_ble-encryption/OUTCOME.md`
- Related files: `src/transport/`, `src/protocol/`, `src/cli/`, `src/config.rs`,
  new `src/collector/`, `src/store/`, `src/metrics/`, `src/web/`
