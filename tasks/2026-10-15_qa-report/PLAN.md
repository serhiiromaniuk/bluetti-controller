# Plan

## Questions for the owner
None — the live UI and requested evaluation criteria are available.

## Steps
- [x] 1. Inspect the running status and telemetry pages, then compare observations with the UI implementation and project requirements — check: evidence-backed report written and reviewed for observed/inferred distinction.

## Risks / Dependencies
- Live station readings are a time-specific snapshot and are not independent validation against the Bluetti app.
- No wattmeter or browser energy profiler is available in this review; avoid quantitative power claims.

## Escape hatches
- If the local app becomes unavailable, report the concrete blocker and use source review only.
- If a destructive / secret-touching / irreversible external action is required: stop and hand to a human.

## Amendments
None.

- 2026-10-15 — amendment: rank QA outcomes as P1 data trust/stability, P1 outage usability, then P2 UI self-power; preserve freshness while reducing unnecessary work.
