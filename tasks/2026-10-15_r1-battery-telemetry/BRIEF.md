# Brief — R1 battery health + R1a telemetry extras

## What
Two post-MVP items from ROADMAP.md §2.2, built side by side and shipped in one
release (1.2.0) with one deploy:

- **R1a telemetry extras** (V2 stations: AC180, Elite 200 V2): the station's own
  charging status and time to full / to empty from the block already polled;
  lifetime energy counters, charging mode and pack temperature from slow blocks
  read every 10 min.
- **R1 battery health:** usable capacity per station estimated from observed
  discharges, shown on `/telemetry` with the suggested `usable_capacity_factor`.

## Why
Owner, 2026-10-15: "do R1 and R1a fast, in parallel … first make a plan of it",
then "single deployment … make sure everything will work and no downtime will
be on production" (recorded at the owner's request; this approves R1a, which
§2.2 had postponed).

## Scope
In: ROADMAP §2.2 decision record; protocol extras + u32 fields; slow-block
reads in the collector that never fail the core poll; capacity estimator from
`rollup_5m`; `/telemetry` section; `read` CLI prints extras; CHANGELOG 1.2.0;
one deploy to the homelab host after a live read-only check.
Out: SOH register (none known; a register sweep stays an R1 discovery item),
EB3A sweep, a separate trend page, the station's time to empty on the cards,
any write, any schema migration.

## Done when
- [x] `make check` green with tests for u32 decode, lenient extras, slow-block
      failure isolation, and the segment finder (hand-computed values).
- [x] (amended: post-deploy `/telemetry` instead) A live read-only `read` of the AC180 and the Elite prints the extras
      (production polling paused around it).
- [x] v1.2.0 deployed once; `bluetti-update status` shows it active, `/healthz`
      reports `v1.2.0` with samples, no rollback.

## Refs
ROADMAP.md §2.2, §4.3 (register maps and dumps), §6 (`sample_extra`,
`rollup_5m`), §10.6 (release + deploy).
