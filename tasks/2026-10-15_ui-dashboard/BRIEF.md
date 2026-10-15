# UI dashboard rework  |  2026-10-15_ui-dashboard

## What
Rework the status page into a Grafana-style dashboard: a time-range picker
(default last 12 h), a refresh picker (manual by default), a dark theme by
default with a toggle, cross-station chart panels with outage shading, and an
always-visible data-age indicator. The lowest possible client and server
overhead is the top priority.

## Why
Owner request after the live UI QA report (`reports/qa-2026-10-15.md`): no
background traffic unless the owner asks for it, time selection like Grafana,
dark theme, better charts and logic. The router's 0 W is a real reading below
the meter floor (the station's own screen shows 0 too, ROADMAP §4.3), not a
decoding bug.

## Scope
**In:** one batched chart endpoint, the status page template/CSS/JS, the
telemetry page theme, no-data energy days, forecast text for a load below the
meter floor, deploy to the dev-laptop user unit, Chrome verification.
**Out:** new device registers, SSE endpoint removal (kept as API), notifications,
gzip/compression middleware, incremental chart fetches.

## Done when
- [x] `/` makes no request after load unless the owner refreshes or picks an
      auto-refresh interval; auto-refresh pauses while the tab is hidden.
- [x] Charts load with ONE request per refresh (all stations, SOC/in/out + outages).
- [x] Time picker: quick ranges, custom from/to, drag-to-zoom; default last 12 h; state in the URL.
- [x] Dark theme by default, toggle persists; telemetry page themed too.
- [x] A day without samples reads "no data", not 0 Wh.
- [x] `make check` green; Chrome check of the deployed page recorded in LOG.

## References
- `reports/qa-2026-10-15.md` · ROADMAP §4.3, §8 · RULES.md golden rules #1, #5
