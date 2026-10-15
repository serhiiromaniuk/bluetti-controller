# Plan

## Questions for the owner
None — the owner set the direction on 2026-10-15 (manual refresh by default,
Grafana-style time picker with last 12 h default, dark theme with toggle,
lowest overhead first).

## Steps
- [x] 1. Backend: `GET /api/chart?from=&to=&step=` (per station SOC, in W, out W on one time axis, plus outages in the window), `/` drops `?range`, empty energy days flagged as no data, forecast text for a load below the meter floor — check: `make check` with new tests for the endpoint and the no-data day.
- [x] 2. Frontend: templates, `app.css`, `app.js` — dark default + toggle, time and refresh pickers, overview cards, three cross-station panels with outage bands and synced cursors, drag-zoom, data-age ticker — check: `make check` (page tests) + reviewer.
- [ ] 3. Deploy to `~/.local/bin`, restart the user unit, verify in Chrome: default 12 h, zero background requests with refresh off, one chart request per refresh, theme toggle, custom range, zoom — check: Chrome observations + network request list in LOG.

## Risks / Dependencies
- Restarting the user unit interrupts polling for a few seconds (allowed: user-level unit on the dev laptop).
- A second server instance must not poll BLE (golden rule #5): verify only on the restarted unit.

## Escape hatches
- A step fails 3 times → stop and log the blocker.
- Anything needing sudo → stop and hand to the owner.

## Amendments
None.
