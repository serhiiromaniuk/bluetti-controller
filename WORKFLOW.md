# WORKFLOW.md — a worked example, start to finish

This is the concrete companion to [`AGENTS.md`](./AGENTS.md). It walks one task
through the whole loop so the abstract rules have a shape. The example is a
generic **Phase 1 — Foundation** task, then a sketch of a later coding phase so
you see the pattern repeat.

Quick map of the moving parts:

| Layer | File(s) | Role |
|-------|---------|------|
| Governance | [`RULES.md`](./RULES.md) | golden rules, architecture, git |
| Operating manual | [`AGENTS.md`](./AGENTS.md) | how to execute |
| This walkthrough | `WORKFLOW.md` | the example |
| Loop engine | [`loop/`](loop/) | `PROMPT.md`, `loop.sh`, `where.sh` (position oracle), `step-done.sh` (close an increment), `entry-size-guard.sh`, `amendments-guard.sh`, `merge-gate.sh`, `STATE.md` |
| Memory | [`tasks/`](tasks/) | `BRIEF/PLAN/LOG/OUTCOME` per task + `INDEX.md` |
| Specialists | [`.claude/agents/`](.claude/agents/) | planner, plan-reviewer, reviewer, close-reviewer, adjudicator, scout, verifier (optional) |
| Shortcuts | [`.claude/commands/`](.claude/commands/) | `/where`, `/loop-step` |

---

## Phase A — Triage (human + agent, a few minutes)

A task starts from a human prompt. Three flavours:

- **Clear:** *"Do Phase 1 — design the `<component>` interface and record the
  decision."*
- **Vague:** *"start the foundation work."* → the agent asks 3–5 scoping
  questions first (which parts? any option already preferred? deadline?).
- **Exploratory:** *"is `<approach>` even feasible here?"* → the first plan step
  is literally "investigate", with a time-boxed escape hatch.

Output of triage: agreement on **what one task** this session advances and its
**done-when** condition. Everything else the agent will need from the owner is
collected by the planner into `PLAN.md`'s `## Questions for the owner` and asked
in **one** batch right after the plan is written — not discovered step by step.

---

## Phase B — Create the task structure (agent)

Copy the template:

```bash
cp -r tasks/_template tasks/phase-1_<short-kebab>
```

Then fill the three live docs (OUTCOME comes later):

**BRIEF.md** — frozen once written:
```markdown
# Phase 1 — Foundation

## What
Complete the foundation document: the key decisions (approach, components,
constraints, dependencies) this project rests on.

## Why
Golden rule #6 — no later-phase production code may start until this is done and
reviewed. Every later decision must trace to a conclusion here.

## Scope
**In:** the foundation document's sections + a feasibility note.
**Out:** any production code in src/ (forbidden until this task closes + review).

## Done when
- [ ] Every section has findings AND an explicit recommendation
- [ ] Key parameters recorded with rationale/sources
- [ ] Dependency list ready to pin
- [ ] Human review complete → milestone tag v0.1-<name>

## References
- RULES.md (golden rules, phase table) · <spec doc §...>
```

**PLAN.md** — steps frozen once work begins; amend at the bottom:
```markdown
# Plan
> Written before work. Do not edit steps once work starts — append amendments.

## Questions for the owner
- Q1 — Is <option> already ruled out by a contract? · blocks: 3 · answer: pending

## Steps
- [ ] 1. [parallel: A] Section A discovery → own page + recommendation
- [ ] 2. [parallel: A] Section B discovery → own page + recommendation
- [ ] 3. [parallel: A] Section C discovery → decision (needs Q1)
- [ ] 4. Parameters table + dependency list → sourced values, pinned versions
      (batch step: one check — every row has a source)
- [ ] 5. Feasibility note; flag if a constraint is at risk
- [ ] 6. Self-review (multi-lens) + request human review

## Risks / Dependencies
- <external constraint that may rule out an option — verify early>.
- Steps 1–3 write separate pages and read nothing from each other → group A.

## Escape hatches
- If a section has no defensible recommendation after 3 attempts: log the blocker,
  set status: blocked, emit <<LOOP:BLOCKED>>.
- No access to a needed source: record what's missing, continue other steps.

## Amendments
<!-- dated plan notes, and deferred reviewer findings as
     `- A<n> · <date> · MEDIUM <file:line> — <finding> · disposition: open` -->
```

**LOG.md** — first entry:
```markdown
# Log
> Append-only. Newest at bottom.

## YYYY-MM-DD HH:MM — Session start
Context loaded per `loop/where.sh --json`. Starting at step 1.
```

Then flip this task's row in [`tasks/INDEX.md`](tasks/INDEX.md) to
`in-progress` — that is all the pointer work an open needs. Step position comes
from the `PLAN.md` checkboxes, so nothing restates "step 1" anywhere.
[`loop/STATE.md`](loop/STATE.md) is touched only for the gate verdict, a
decision, or a carry-forward. Confirm with `make where`.

---

## Phase C — Execute the loop (agent, possibly unattended)

Each iteration = **one step → verify → review → log → `step-done.sh` → marker**
(AGENTS.md §4). A `[parallel: A]` group is one iteration (AGENTS.md §8a):

```text
where.sh: .parallel_steps = [1,2,3] → spawn three `scout` subagents in ONE
turn, one page each → one batched reviewer pass over the three pages →
one LOG entry per step → loop/step-done.sh --steps "1 2 3" --commit "docs(…): …"
```

Good `LOG.md` entries are factual and carry evidence:
```markdown
## YYYY-MM-DD HH:MM — Step 1 done: <component> choice
Compared options A / B / C on <criteria>. Recommendation: <choice> — <one-line
justification>. Written to the foundation doc, Section A.
Verify: Section A now ends with a stated recommendation. ✅
reviewer: batched (doc-only; reviewed with Sections B–C before the push)
Commit: loop/step-done.sh --commit "docs(<component>): foundation Section A recommendation"
<<LOOP:CONTINUE>>
```

To run it unattended, a human starts the harness from a terminal:
```bash
loop/loop.sh --max-iterations 8        # bounded; stops on a marker or the cap
```
Or do one increment interactively with the `/loop-step` command, and check
position any time with `/where`.

The loop **stops itself** when it emits `<<LOOP:DONE>>`, `<<LOOP:BLOCKED>>`, or
`<<LOOP:GATE_FAILED>>` — or when it hits the iteration cap. It rolls
phase→phase autonomously via `<<LOOP:PHASE_COMPLETE>>` only once a gate is
observed-green.

---

## Phase D — Close (agent writes, human gates the boundary)

When every step is checked and the gate is satisfied, draft **OUTCOME.md**, then
spawn the `close-reviewer` before ticking any done-when box. It re-runs the
checks, opens the foundation doc as its reader would, and tests each declined or
re-targeted amendment; a `hold` with findings turns them into open amendments
for the next iteration. The draft:
```markdown
# Outcome
## Summary
Foundation doc complete: <approach>, <components>, parameters table, pinned
deps. <feasibility verdict>.

## Deliverables
- Commits: <hashes> (docs(<component>): foundation …)
- Foundation doc, all sections + feasibility note
- Pinned dependency list ready for the next phase

## Pending / Follow-up
- [ ] Human review of the foundation doc
- [ ] On approval: tag v0.1-<name>; open Phase 2 task

## Amendments
- A1 — fixed in <sha>. A2 — re-targeted → CF-1 (phase 2). (`make amendments`: 0 open.)

## Lessons / Notes
- <anything worth remembering>
```

This phase's done-when includes a **human** review (golden rule #6). The
`close-reviewer` reports that box UNVERIFIED until the owner's approval is on
record, so nothing is ticked or tagged yet: set `loop/STATE.md` **Blocked?** to
`yes — awaiting review of the foundation doc` and emit `<<LOOP:BLOCKED>>` naming
the review. Once the owner records the approval (a `STATE.md` decision line), the
next iteration re-runs the close; the `close-reviewer` accepts the record, and on
`VERDICT: close` the iteration finishes **in this order**: tag the milestone
(`git tag v0.1-<name> && git push --tags`), set `tasks/INDEX.md` → `done`, open
the Phase 2 task from `_template/`, and only then emit `<<LOOP:PHASE_COMPLETE>>`.
A phase whose gate is fully objective (`make check` green, acceptance criteria
met) skips the hand-off — but keeps the order: tag and open first, marker last.

---

## The pattern repeats (later coding phase sketch)

A coding phase is the same loop with code-shaped steps and checks:

```text
tasks/phase-2_<component>/
  BRIEF  done-when: <component> behaves correctly on known inputs,
                    ≥80% coverage on src/<component>/, make check green
  PLAN   1. failing test: known input → expected output  (TDD red)
         2. implement src/<component>/base.<ext> interface
         3. implement src/<component>/<impl>.<ext>  (green)
         4. edge cases + tests
  each step → make check → reviewer (risky: before the commit, in the
  background while the LOG is written) → loop/step-done.sh --commit … --push
  → <<LOOP:CONTINUE>>
```

Same discipline, same markers, same gates. The only thing that changes between
phases is the content of the steps and which check proves them.

---

## Edge cases

- **Blocked mid-step:** 3 failed attempts → log blocker, `status: blocked`,
  write OUTCOME, `<<LOOP:BLOCKED>>`. Don't thrash.
- **Task too big:** split into `phase-N_part-a`, `phase-N_part-b`; each gets its
  own folder, plan, and commits.
- **Plan was wrong:** never edit the original steps — append an `## Amendments`
  note explaining the change, then proceed.
- **Gate fails:** `<<LOOP:GATE_FAILED>>`, fix the *work*, re-run. Never edit the
  thresholds.
