# R3 control hardening  |  2026-11-10_r3-control-hardening

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
Close the six MEDIUM review findings R3 slice 1 left open (A8–A13 of
`2026-11-07_r3-control-schedule/PLAN.md`): two untested collector command
paths, a missing handshake retry, a wrong status code, a stale-UI race and a
scheduler slot leak.

## Why
Re-targeted at the R3 close (2026-11-10). None affects the default
`control.enabled = false`, but each matters once the owner enables control for
H14–H17 (golden rules #2 and #5). Fix them before that, not after.

## Scope
**In:** each finding as written in the R3 PLAN `## Amendments`:
- A8 — collector command tests: a station past `UNREACHABLE_AFTER` (no retry,
  no write) and `retry == true` in the between-rounds test.
- A9 — commands retry a failed key exchange once on a fresh connection like
  polls, or ROADMAP §R3 states they skip it.
- A10 — test the release-by-address branch of a command (deadline mid-write).
- A11 — a collector "shutting down" reply maps to `503`, not `502`.
- A12 — the control panel survives an auto-refresh during a request (pending
  set, re-find the row before writing the reply).
- A13 — the scheduler's `applied` map is pruned, so a deleted and re-added
  entry does not inherit the old slot.
**Out:** any real write frame, enabling control on a host, deploy (human-only);
new control features (later R3 slices).

## Done when
- [ ] A8, A10, A13: the new tests named in the findings red then green (`make check`)
- [ ] A9: decided in ROADMAP §R3 and, if code, a test (`make check`)
- [ ] A11: a test with a fake answering "shutting down" gets `503` (`make check`)
- [ ] A12: a page test or a documented manual check in `HUMAN-CHECKLIST.md`
- [ ] `make check` green; `src/protocol/` coverage ≥ 80%

## References
- Governing spec: `ROADMAP.md`
- RULES.md golden rules #2, #5 · ROADMAP.md §R3, §8.2 · `tasks/2026-11-07_r3-control-schedule/PLAN.md` A8–A13
- Related files: `src/collector/control.rs`, `src/collector/tests/control.rs`,
  `src/web/control.rs`, `templates/status.html`, `src/control/schedule.rs`
