# Plan

> Written before work starts. **Do not edit the steps once work begins** —
> append changes under `## Amendments`. One step = one loop increment.

## Questions for the owner

<!-- Every unknown the repo cannot answer (a fact only a human knows, a product
     call, access, a credential) — collected HERE at plan time and asked in ONE
     batch when the task opens, not discovered one per step. One line each:
       - Q1 — <the question> · blocks: 4, 7 · answer: pending
     This section stays live after step 1 (the steps do not). `answer:` moves `pending` → `asked <date>` (all asked in one batch) → the
     answer and its date. A step that cannot start without one says
     `(needs Q1)` in its text; `loop/where.sh` reports `.unasked_questions`,
     `.open_questions` and `.waiting_on`. None? Say so in one line. -->

## Steps
- [ ] 1. <first step — small, verifiable; cite `spec §<key>`> — check: <the command or test that proves it>
- [ ] 2. [parallel: A] <independent step> — check: …
- [ ] 3. [parallel: A] <independent step, disjoint files> — check: …
- [ ] 4. <batch step: trivial mechanical items sharing ONE check> (needs Q1) — check: …

<!-- `[parallel: A]` right after the number marks steps that write disjoint
     files and do not consume each other's output: the session may fan the
     group out (AGENTS.md §8a) and close it together. Never tag host-, secret-
     or shared-state-changing steps. A batch step groups trivial items (a row
     per table, a rename per file) that one check proves; items needing
     unrelated checks are separate steps. -->

## Risks / Dependencies
- <risk, dependency, or unknown — and how it's mitigated>
- <which steps are independent (the `[parallel: …]` groups) and why>

## Escape hatches
- If a step fails 3 times: stop, log the blocker, set status: blocked, emit `<<LOOP:BLOCKED>>`.
- If a destructive / secret-touching / irreversible external action is required: stop and hand to a human.
- (gate phases) if the gate fails: `<<LOOP:GATE_FAILED>>` — fix the work, never the threshold.

## Amendments
<!-- Append here during work; never edit the steps above. Two kinds of entry:
       - YYYY-MM-DD — plan: <a change to the plan, e.g. step 4 split into 4a/4b>
       - A1 · YYYY-MM-DD · MEDIUM `<file:line>` — <deferred reviewer finding> · disposition: open
     A1, A2, … are deferred reviewer findings (PROMPT §4b), one list item each.
     The increment that settles one rewrites its field: `disposition: fixed in <sha>`
     · `re-targeted → <CF-n or task>` · `declined — <reason>`. `make amendments`
     counts the ones still open; at the close every open one gets a disposition
     here and in OUTCOME.md. Only `- A<n>` items are counted. -->
