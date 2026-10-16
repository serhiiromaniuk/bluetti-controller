# Outcome — R1b dashboard panels

**Status: done (2026-10-16).** v1.3.1 runs on the homelab host.

## Deliverables
- 2782de0 cards (pack °C, mode, station time left) · 88ed026 temperature chart · b350f80 counter energy + lifetime line.
- Merge + the fix for readings below 0 °C, CHANGELOG 1.3.0, `v1.3.0`.
- Asset cache-busting, `v1.3.1`. Both deployed, health checks passed, no rollback.

## Follow-ups
- AC180 counters 150–159 read 0: on this firmware they are not populated, or they live elsewhere. Add to H12, or hide the AC180's counter columns once confirmed.
- The AC180's temperature line has gaps (09–11 h): check the journal for slow-block warnings if it repeats.

## Lessons / Notes
- reviewer · missed: a stale browser cache after a deploy. The tests and reviews check the server, not what a browser runs. A real-browser check after the deploy caught it.
- The workflow (3 worktree builders + reviewers) took ~8 min wall time for three features; only one merge conflict, in a test file.
