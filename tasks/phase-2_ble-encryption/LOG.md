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

## YYYY-MM-DD HH:MM — Session start
Context loaded per `loop/where.sh --json`. Starting at step <N>.

<!-- Entry template — copy, fill, keep it under 40 lines:

## Step <N> <✓|blocked> YYYY-MM-DD — <the headline result, one line>

**Changed:** <file/module — what, one line each>

**Check:** <the check that proves this step> — failed first for <the intended
reason>, then passed. <or, when it is green-on-arrival: the mutation that proves
the assertion has teeth, in one line>

**Evidence:** written below this entry by `loop/step-done.sh` (the gate tail,
fenced in `gate:begin/gate:end`). Add by hand only what no script captures: a
smoke result, measured acceptance numbers — the decisive lines.

**reviewer:** <n> CRITICAL / <n> HIGH / <n> MEDIUM · or `batched` (low-risk,
reviewed with the next batch before the push)
- CRITICAL|HIGH <id>: <finding> → fixed in <where>.
- MEDIUM <id>: <finding> → deferred to PLAN `## Amendments` A<n>.

**Decision:** <a choice the next iteration must not relitigate · none>
**Carry-forward:** <raised/discharged in loop/STATE.md · none>
**Next:** step <N+1> — <title>.

Commit: via `loop/step-done.sh --commit "<type>(<scope>): <subject>"`   ·   Marker: <<LOOP:CONTINUE>>
-->

## Plan ✓ 2026-10-13 — PLAN.md written (17 steps) and audited

**Changed:** `PLAN.md` — 17 steps, parallel group A (1–4), Q1–Q5 all `delegated 2026-10-13`; old steps 9–16 and A1–A8 carried (mapping in Risks).

**Check:** `plan-reviewer` audit — 0 CRITICAL / 2 HIGH / 4 MEDIUM / 1 LOW; all acted on before the commit:
- HIGH F1 (step 13 committed a session secret + station key, against golden rule #3 and BRIEF done-when) → fixed: step 13 commits key-free envelope structure only; step 1 decides the committable-artefact rule and whether the station key is ephemeral.
- HIGH F2 (key scan armed only at step 8, after steps 6–7 port key-bearing sources) → fixed: step 5 writes the local `keys.toml` and proves the scan refuses its values; step 8 only loads keys.
- MEDIUM F3 (replay `detect` vs `read` recording) → fixed: step 12 `read` runs `detect` + `poll` in one session.
- MEDIUM F4 (no owner for an EB3A V2 re-map) → fixed in step 11.
- MEDIUM F5 (step 12 oversized) → `--record-wire` moved to step 13.
- MEDIUM F6 (step 1 check lacks the fixture decision) → fixed in step 1's check. LOW F7 → step 9 check wording.

**reviewer (batch `@{u}..HEAD` = 001de51, 1f2809b, 63ca4e8):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM → deferred to PLAN `## Amendments` A9 (ROADMAP §12 M1/M2/M5 bodies stale), A10 (H-rows lack the `verified` follow-through), A11 ("production deploys" undefined vs user-level units); all homed at step 17.

**Decision:** none beyond the delegated Q1–Q5 in PLAN.
**Carry-forward:** none.
**Next:** parallel group A — steps 1–4.

Commit: `docs(loop): plan phase-2 encryption task` · Marker: <<LOOP:CONTINUE>>

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:30:48Z

```text

test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-f1d778b563e65a98)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
