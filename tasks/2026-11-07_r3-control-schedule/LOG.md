# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes. `loop/where.sh`
> computes step position from `PLAN.md` and the last result from this file's newest
> `## ` heading — so restating an entry in a pointer file is prose every later
> iteration pays to re-read.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns). Bullets, not
> narrative. Cite a spec/doc section, never re-quote it. Record the decision and
> the evidence, not the reasoning that produced them. The shape below is the shape.

## 2026-11-10 — Task open
Context loaded per `loop/where.sh --json` (no in-progress row; R3 first per STATE.md).

**Changed:** `tasks/INDEX.md` — R3 row `todo` → `in-progress`; this LOG created.

**Check:** BRIEF + PLAN already committed in 171edda, audited by the plan-reviewer
(findings applied) and the batch reviewer (H1 applied, A1/A2 open). No new plan.

**reviewer:** batch pass over a650183 + this commit — 0 CRITICAL / 0 HIGH / 2 MEDIUM
- MEDIUM M1/M2 (a650183, BUG-station-ble-invisible.md:97,100) → deferred to PLAN A3/A4.
**Decision:** none · **Carry-forward:** none
**Next:** step 1 — spec: record the R3 slice in ROADMAP.md.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:32:41Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-11-10 — R3 slice 1 spec recorded in ROADMAP.md (ADR-010)

**Changed:** `ROADMAP.md` — ADR-010 in §3; R3 slice 1 in §2.2; 0x06 write frame, encrypted-write
assumption and no-write-retry rule in §4.2; write allow-list table in §4.3 (Elite pair documented,
out of `switches()` until H16, per A2); `schedule` table (migration 002, SCHEMA_VERSION 2, rollback
via §10.6 DB backup) in §6; four routes + error codes + control panel in §8.2 (schedule edits need
`web.token` set, per the 2026-11-07 plan amendment; bearer sent only when the field exists, per A1);
§9 control/token rule; §12 R3 slice boxes + runtime rules (gate, serialisation, CLI, scheduler,
missed runs, retry cap, editing). Each section ends with a recommendation.

**Check:** `loop/where.sh --context` reads back all 7 cited sections (2.2, 4.2, 4.3, 6, 8.2, 9, R3)
with the new text (9 matches of ADR-010 / slice markers). Doc-only: no code changed.

**reviewer:** 0 CRITICAL / 1 HIGH / 2 MEDIUM · INTENT: satisfied (Elite exclusion and the added
`502` confirmed against code)
- HIGH H1: missed-run bullet ("never drops a run") contradicted the 10-failure cap; "polling paused"
  would count toward it → fixed in §R3: the cap applies within one service run (a restart resets it),
  a `"polling paused"` answer is not a failure and waits for the pause end.
- MEDIUM M1: a new entry's past-due single time fires at once → deferred to PLAN A5.
- MEDIUM M2: step 12's "both rows for every station" vs no Elite rows → deferred to PLAN A6.

**Decision:** control route answers `502` with the error when the write or read-back fails;
"polling paused" never counts toward the scheduler's retry cap.
**Carry-forward:** none
**Next:** step 2 — protocol allow-list, pure (`src/protocol/control.rs`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:39:52Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
