# Plan

Direct work in an interactive session (owner: no loop needed). One release.

## Questions for the owner
None: the owner approved R1 + R1a and one deploy on 2026-10-15.

## Steps
- [ ] 1. ROADMAP §2.2 decision record for R1 + R1a (fields, ranges, slow-block
      rules, estimator rule) — check: section reads complete; reviewer batch.
- [ ] 2. [parallel: A] R1a: `Field` u32 support, `Profile.extras` (never a
      problem, out-of-range dropped), `Profile.slow_blocks`; collector reads slow
      blocks every 600 s after the core blocks, one try per block, bounded by
      half the poll deadline, failures logged only; `read` CLI prints extras —
      check: `make check` with new tests.
- [ ] 3. [parallel: A] R1: `metrics::capacity` segment finder over `rollup_5m`
      rows, `store::read::rollups`, "Battery health" section on `/telemetry` —
      check: `make check` with hand-computed tests.
- [ ] 4. Merge, reviewer pass on the whole diff, fix CRITICAL/HIGH — check:
      `make check` on the merged tree.
- [ ] 5. Live read-only check: pause production polling, `read` the AC180 and
      the Elite from the dev laptop, resume — check: extras printed, no problem
      lines.
- [ ] 6. CHANGELOG 1.2.0, `make release VERSION=1.2.0`, `make deploy
      HOST=debian@192.168.0.192` — check: `bluetti-update status`, `/healthz`.

## Risks / Dependencies
- A slow block (6007 on the AC180 is unconfirmed) answers with a Modbus
  exception: one try, logged, the sample is stored anyway.
- Slow reads stretch a poll past its deadline: bounded to half the deadline
  from the poll start, so the core sample always lands first.
- An extra decodes to a sentinel (0xFFFF): dropped by its range, never a failure.
- No migration: extras go to the existing `sample_extra`, so a rollback to
  1.1.1 needs no database restore.
- Deploy downtime: `bluetti-update` restarts the unit (a few seconds of web
  gap, at most one missed poll); health check + automatic rollback guard it.
- Steps 2 and 3 write disjoint files (protocol/collector/cli vs metrics/store
  read/telemetry): run in parallel, step 3 in its own worktree.

## Escape hatches
- If a step fails 3 times: stop, log the blocker, hand back to the owner.
- Live reads denied or a station unreachable: deploy anyway only if `make
  check` is green and the failure-isolation tests pass; note it in the LOG.

## Amendments
