# Phase 5 — Web UI, deploy, 7-day soak  |  phase-5_web-deploy

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
The read-only LAN web UI and JSON/SSE API over the collector's database
(milestone M4), a user-level systemd deployment on the dev laptop, and a
7-day unattended soak that earns `v1.0-mvp` (milestone M5).

## Why
Phase 4 closed with `v0.4-collector`: the stations are polled, stored and turned
into an outage log, runtime forecast and energy totals, but nothing shows them.
Phase 5 makes that data usable on the LAN and proves the service runs
unattended — the last MVP gate (RULES.md phase table, v1.0 gate; golden rules
#1, #4, #5).

## Scope
**In:** ROADMAP.md §12 M4 — axum router, askama templates, embedded assets
(uPlot vendored, credited in `README.md`), status page and telemetry page
(§8.1), JSON API + SSE (§8.2), pause-polling POST wired to the collector's
`Control` channel (max 60 min; optional bearer `web.token`, §9), page weight
< 100 KB, phone-friendly layout, handler tests against an in-memory DB; wiring
`metrics::runtime`, `energy::totals` and `house::totals` into the views
(phase-4 close-review M1). ROADMAP.md §12 M5 — `deploy/bluetti-controller.service`
(user-level, §10.3) with install notes, `make build` + `make dist`, installing
and starting the user unit on the dev laptop (standing owner delegation:
`systemctl --user`), CF-6 (`loginctl enable-linger`, or an H-row if polkit
refuses), the 7-day soak measured against the v1.0 gate.
**Out:** the hardened system unit and homelab install (§10.2, human-only, after
the MVP); any write frame or device control (golden rule #2 — the pause button
stops polling, it never writes to a station); notifications, Prometheus, cloud
(§2); outage schedules (R2), battery-health analytics (R1); auth beyond the
optional bearer token.

## Done when
- [x] `make check` green with `src/protocol/` and `src/metrics/` ≥ 80% line
      coverage — check: `make check` output in LOG.md
- [x] Every §8.2 endpoint and both §8.1 pages answer from an in-memory DB with
      the expected shape; the POST honours `web.token` and the 60-min cap, and
      pausing stops polls — check: handler + collector tests
- [x] Status page shows the house banner, station cards with runtime forecast,
      outage log and 7-day energy table from `metrics` (no stub values) —
      check: handler test asserting rendered values
- [x] Page weight < 100 KB, assets embedded, no JS build step — check: byte
      count of `/` plus its assets in LOG.md
- [x] `bluer` still only under `src/transport/bluer/`; config read only in
      `src/config.rs`; no route sends a write frame — check: grep in LOG.md
- [x] `deploy/bluetti-controller.service` + install notes, `make build` and
      `make dist` produce the binary and tarball — check: commands in LOG.md
- [x] User unit running on the dev laptop; CF-6 discharged (`loginctl
      show-user` shows `Linger=yes`) or an H-row records the refusal
- [x] 1 h functional run (owner 2026-10-15, replaces the 7-day soak): ≥ 95% polls
      per powered station, RSS ≤ 50 MB, status page and API ≤ 1 s on
      the LAN — check: measured numbers in LOG.md; the phone-browser look is a
      `HUMAN-CHECKLIST.md` row
- [x] the phase gate passes (`v1.0-mvp`) — see RULES.md / AGENTS.md §6

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #1/#2/#4/#5, v1.0 gate, human-only boundary, standing
  delegation) · ROADMAP.md §2.1, §8, §9, §10.3, §12 M4/M5 · loop/STATE.md CF-6
- Previous task: `tasks/phase-4_collector/OUTCOME.md` (Pending / Follow-up)
- Related files: `src/web/`, `src/collector/`, `src/store/`, `src/metrics/`,
  `src/config.rs`, `src/cli/`, `Makefile`, new `deploy/`
