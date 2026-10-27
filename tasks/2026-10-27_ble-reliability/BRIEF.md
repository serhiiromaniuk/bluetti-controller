# BLE reliability fixes  |  2026-10-27_ble-reliability

> Static once written. The contract for this task.

## What
Stop the fleet-wide poll failures on the homelab host (owner problems #1 and #2)
with the smallest set of changes the investigation supports.

## Why
Since 2026-10-27 all three stations fail polls at connect/handshake/read, the
AC180 next to the host included. INVESTIGATION.md: the service swap-thrashes
against `MemoryMax=128M` (RSS 139 MB after 11 days, ~20 MB expected), a leak
most likely from a new `bluer::Session` per BlueZ call. Golden rule #5 and the
v1.0 gate (RSS ≤ 50 MB, ≥ 95 % polls) are no longer met in production.

## Scope
**In:** INVESTIGATION.md §3 steps 1–4 (one session per process, poll backoff
for unreachable stations, one Disconnect per poll, release 1.3.3 + deploy on
the owner's go).
**Out:** problem #3 (station invisible to the app and us) — owner-handled,
`BUG-station-ble-invisible.md`. Everything in INVESTIGATION.md "Not doing".

## Done when
- [ ] Session churn no longer grows RSS: the `#[ignore]` churn test passes, and was red before the fix (`cargo test -- --ignored`)
- [ ] Unreachable stations are polled with capped backoff: collector test green (`make check`)
- [ ] A clean poll sends one Disconnect: collector test green (`make check`)
- [ ] 1.3.3 released and deployed; on the host after ≥ 1 h: `Memory:` < 50 MB and flat, ≥ 95 % polls per powered station, zero "dropped while connected" (journal + `/api/status`)

## References
- Governing spec: `ROADMAP.md`
- `tasks/2026-10-27_ble-reliability/INVESTIGATION.md` (evidence, plan, not-doing list)
- RULES.md golden rule #5 · deploy/HOMELAB.md §6
