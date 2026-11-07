# Outcome — BLE reliability fixes

**Status: done (2026-11-07).** v1.3.4 (with the 1.3.3 fixes) runs on the homelab host.

## Summary
The service no longer swap-thrashes: one shared `bluer::Session` per process
stopped the RSS growth, unreachable stations back off to 10 min, and a clean
poll sends one Disconnect. On the host over 2026-11-01 – 11-06: RSS 24 MB and
flat, polls 99.0 % (router), 96.6 % (mobile), 98.6 % (living-room, the owner's
switch-off windows excluded).

## Deliverables
- 650fa1e `fix(transport)`: one shared bluer session per process, uncancellable discovery start.
- 9471aa0 `fix(collector)`: back off unreachable stations to 10 min, one Disconnect per clean poll.
- `v1.3.3` (c01a148 changelog, f97d777 release): the fixes.
- `v1.3.4` (b1c940e, c188e4b, 34769bc): adds `feat(web)` grid uptime badge, outside this BRIEF, at the owner's request.
- Both deployed to the homelab host on the owner's go.
- 246d60e, 99d313f, e8331a9, and this close: host checks in LOG.md.

## Pending / Follow-up
- [ ] `tasks/2026-11-07_elite-read-stalls/`: the Elite's ~5.5 "dropped while connected" a day (33 in 6 days; 1 more on the EB3A)
      (the station stops answering reads, then the link drops). Re-targeted there by the owner.
- [ ] `BUG-station-ble-invisible.md` (problem #3) stays owner-handled.

## Amendments
None were deferred: PLAN `## Amendments` holds two plan notes and no A-items.

## Lessons / Notes
- systemd's `Memory:` line counts page cache (36 MB here, mostly the SQLite file).
  A memory criterion names process RSS or cgroup `anon`, never that line.
- A done-when that counts station-side events ("zero dropped") cannot be met by
  host code alone. Split it from host-side criteria when the task opens.
- `/api/series` raw samples per day give the poll rate without reading the
  production DB.
