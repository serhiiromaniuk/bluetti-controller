# Plan

> Steps are frozen once work starts; append amendments instead of rewriting.

## Questions for the owner
- Q1 — Does the AC180 normally remain plugged into a house-mains socket? Were its 20 Nov input losses outages or disconnections? · blocks: 4 · answer: asked 2026-11-20
- Q2 — Is the Elite powered on, with Bluetooth enabled and in range? Is the missing item the entire card, its chart line, or another view? · blocks: 2, 4 · answer: asked 2026-11-20

## Steps
- [x] 1. Record read-only, redacted evidence: current sensor roles, station
  latest samples and discovery failures; chart AC180/EB3A input and event
  timestamps across the false event; trace the vote, persisted open event,
  station list and UI paths. Distinguish an input loss from a proven house
  outage. Cite `spec §7.1` and `spec §7.2` — check: documented timestamps and
  code paths reproduce the reported banner and identify what remains unknown.
- [ ] 2. (needs Q2) Reproduce the reported missing station in the indicated UI
  with read-only diagnostics, and distinguish software omission from missing
  BLE advertisements. For a confirmed implementation defect, add a failing
  regression test, fix it and rerun the test plus `make check`; otherwise
  record evidence and a physical recovery checklist without inventing a fix.
  Cite `spec §8.4` and `spec §5.5` — check: the indicated view preserves
  configured stations and reports stale/unreachable data correctly, or the
  physical limitation is documented and handed to the owner; reviewer.
- [x] 3. Validate whether any outage-state implementation defect exists
  independently of sensor roles; implement only evidenced corrections with a
  failing-then-passing regression covering absent, present, unknown, debounce,
  restart and event timing as applicable. Do not silently make a non-sensor
  authoritative or fabricate a historical event. Cite `spec §7.1`, `spec §7.2`
  and `spec §6` — check: relevant regression and `make check` pass; reviewer.
- [ ] 4. (needs Q1, Q2) Prepare a reviewable production handoff: role changes
  only for reliable mains-connected sensors; safe service validation/restart;
  separately propose event-history reconciliation using a consistent SQLite
  backup and the evidenced intervals, preserving uncertainty and specifying
  rollback. If code changed, prepare a reviewed release and the documented
  `make release` / human-operated `make deploy` procedure with version and
  health checks. Cite `spec §10.6` and `spec §7.2` — check: commands are reviewed
  and dry-run against a disposable database, never the production host; reviewer.
- [ ] 5. **HUMAN** performs approved host changes and, separately, any history
  correction; records results. Verify read-only current status, samples and
  intended displayed output; leave unproven live/physical checks open. Cite
  `spec §10.6` and `spec §8.4` — check: owner evidence and service health.

## Risks / Dependencies
- A disconnected station and a mains outage produce the same voltage reading;
  no algorithm can infer which happened without independent context.
- The persisted false open event survives restart and spans separate AC180
  input-loss intervals; changing a single start timestamp would falsify history.
- An unreachable Elite may be off, out of range or unable to advertise; a
  missing chart line is not proof that a card disappeared.
- Commands on the live service may affect a co-tenant or shared BLE adapter;
  only read-only diagnostics are agent actions.

## Escape hatches
- Three failures of one step: log and mark blocked; do not thrash.
- Production deployment, database edits, sudo/system-wide changes, pairing,
  device writes and credentials are human-only: prepare and hand off.
- If UI or physical-state evidence cannot be obtained, record the gap and do
  not invent a fix or tick verification.

## Amendments
- 2026-11-20 — While Q2 blocks step 2, execute independent step 3 first, using `--steps 3`. Correct the current banner when every sensor is unknown without closing the historical event; ROADMAP §7.2 requires unknown while §6 keeps the event open. This does not fix the misconfigured sensor roles or rewrite history.
- A1 · 2026-11-20 · MEDIUM `src/web/chart.rs` — uptime counts intervals with no reachable grid sensor as monitored; calculate sensor coverage before reporting availability · disposition: open
- 2026-11-20 — While Q2 remains unanswered, mitigate A1 independently: label event totals as recorded estimates and suppress the unsupported uptime percentage. Full historical coverage remains open because sensor-role history and old raw readings are unavailable; this increment does not tick an unrelated step.
