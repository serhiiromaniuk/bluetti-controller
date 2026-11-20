# Outage and missing station investigation  |  2026-11-20_outage-station-debug

> Static once written. Changes in scope require a new task.

## What
Investigate and correct the false house-outage indication and the reported
missing living-room power station, with evidence and a safe production handoff.

## Why
The house event opened on 19 Nov at 20:49 when the router EB3A was unplugged;
the AC180 simultaneously measured mains but was excluded from the vote. The
Elite has not supplied a sample since 17 Nov. Unreliable status and missing
station information make the outage history and dashboard misleading.

## Scope
**In:** reproduce the reported display issue and isolate the Elite discovery
failure with read-only diagnostics; examine sensor selection, state and event
history; fix confirmed repository defects with regression tests; prepare a
reviewable host configuration and history-reconciliation procedure.

**Out:** device writes, pairing, secrets, production host changes and event
history mutations by the agent; these are human-only. Inferring a house outage
solely from a station that may have been unplugged is not permitted.

## Done when
- [ ] Evidence distinguishes an absent card or series from an unreachable
  Elite, and any confirmed software defect has a failing-then-passing test.
- [ ] The outage's cause and trustworthy observed intervals are recorded;
  required sensor roles are established against actual device usage.
- [ ] Confirmed code fixes pass `make check` and independent risk-tier review.
- [ ] Production change and history reconciliation are documented with checks
  and rollback, and the owner executes and verifies any human-only operations.
- [ ] Task outcome states remaining unknowns and physical checks, without
  claiming outage or recovery that was not observed.

## References
- Governing spec: `ROADMAP.md` — §5.3–5.5, §6, §7.1–7.2, §8.1, §8.4 and §10.6.
- RULES.md (golden rules and human-only boundary) · `deploy/HOMELAB.md`.
- Related: `src/metrics/house.rs`, `src/store/writer.rs`, `src/web/`,
  `templates/status.html`, `src/transport/bluer/`.
