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

## 2026-10-13 — Task opened
- Folder opened from `tasks/_template/`; BRIEF written from ROADMAP.md §12 M3, RULES.md v0.4 gate, STATE CF-5.
- No PLAN yet (`needs_plan`): the `planner` writes it next iteration, then the `plan-reviewer`.
- reviewer: batched (doc-only BRIEF; reviewed with the PLAN batch before the push).
- Next: PLAN.md via planner + plan-reviewer.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:05:43Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Plan ✓ 2026-10-13 — PLAN.md written (19 steps, Q1–Q5 delegated) and audited

**Changed:** `tasks/phase-4_collector/PLAN.md` — planner wrote 19 steps; group A = steps 5–7 (disjoint stub files from step 4).

**Check:** `plan-reviewer` audit — 0 CRITICAL · 2 HIGH · 6 MEDIUM · 3 LOW. Plan not started, so all fixed in the steps (no amendments):
- P1 HIGH per-round budget (absent station ~80 s > 60 s interval) → step 1 (j) deadline + arithmetic, step 10 timeout + "hanging station never delays others" test, Q1 cites it. fixed
- P2 HIGH `systemd-run --user` cwd is `$HOME`; inhibit vs polkit → `--working-directory="$PWD"` in Q2/14/15, inhibit fallback verified in step 14. fixed
- P3 smoke foreground 16 min > 10 min cap → step 14 runs as transient unit `bluetti-smoke`, rehearses step 15's launch. fixed
- P4 CF-5 placed after outage logic → step 1 runs `read living-room`/`read mobile` grid-on; step 14 re-confirms. fixed
- P5 `make lock` re-resolves all deps → unlocked `cargo build --all-targets` (steps 2, 13, Risks). fixed
- P6 in-step wait in step 18 → BLOCKED with ETA always. fixed
- P7 size budget per table → steps 3, 11 state bytes/day; step 16 prints per-table bytes. fixed
- P8 bluer grep matches doc comments → `grep -rnE 'bluer::|use bluer' src/`. fixed
- P9 `--min-pct`/`--max-mb` args; P10 step 18 ticks M3 24 h item; P11 step 10 reuses `src/cli/session.rs` core. fixed
- Reviewer verified: §9 config sections already in `src/config.rs`; `make test` gates `src/protocol/` only; `python3` present; H4 row exists; Q1–Q5 cross nothing human-only.

**Review:** plan-reviewer is this increment's grader (PROMPT §1 needs_plan); it also covered the unpushed BRIEF commit 2a8fc29 → push.

**Next:** step 1 — M3 design record (+ CF-5 reads).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:17:45Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
