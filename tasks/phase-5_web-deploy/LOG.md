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

## 2026-10-15 — Task opened
- Folder opened from `tasks/_template/`; BRIEF written from ROADMAP.md §12 M4/M5, RULES.md v1.0 gate, STATE CF-6, phase-4 OUTCOME follow-ups.
- Unit install + soak start are inside the standing delegation (user-level `systemctl --user`); the homelab system unit stays out (§10.2, human-only).
- No PLAN yet (`needs_plan`): the `planner` writes it next iteration, then the `plan-reviewer`.
- reviewer: batched (doc-only BRIEF; reviewed with the PLAN batch before the push).
- Next: PLAN.md via planner + plan-reviewer.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T10:43:07Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## 2026-10-15 — PLAN written + plan-review
- `planner` wrote PLAN.md: 22 steps (M4 1–15, M5 16–22), groups A (6–8), B (9–12), C (16–17); Q1–Q7 all `delegated 2026-10-15`, none asked.
- `plan-reviewer`: 0 CRITICAL, 1 HIGH, 5 MEDIUM, 4 LOW. All fixed in the PLAN before step 1 (steps not yet frozen):
  - HIGH P1 fixed — inhibit wrapper is `sleep:idle:handle-lid-switch` (logind `LidSwitchIgnoreInhibited=yes` ignores `sleep` locks on lid close); step 18 checks the listed lock; H10 says lid open, on AC.
  - P2 fixed — hourly transient user timer appends probe lines to `soak.log` (Q5, step 18); steps 20/21 read it.
  - P3 fixed — soak waits commit `--no-tick`, INDEX stays in-progress, no OUTCOME, not an escape-hatch attempt.
  - P4 fixed — energy table adds Δstored and losses (estimate, spec §7.4); step 7 asserts losses.
  - P5 fixed — tz helpers in `src/metrics/localtime.rs` at step 2; 6/7 only call them.
  - P6 fixed — `app.{js,css}` moved to step 8; step 10 keeps templates + `pages.rs`; groups A/B file-disjoint.
  - LOW P7–P10 fixed — uPlot release and sizes named in step 1 i; env grep covers `from_default_env` (main.rs `RUST_LOG` exception); `paused_until` wall-clock conversion in 1 h; step 16 worktree reuses `target/`.
- reviewer: batched (doc-only plan; reviewed with the BRIEF commit before the push).
- Next: step 1, M4/M5 design record (risky).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T10:57:12Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
