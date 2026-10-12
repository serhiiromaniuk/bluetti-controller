---
description: Perform exactly ONE loop increment interactively (do one step → verify → log → commit → emit a marker), following loop/PROMPT.md.
---

Execute **one** iteration of the agent loop by hand, exactly as `loop/PROMPT.md`
specifies (read it if it is not already in context). This is the interactive
equivalent of one `loop/loop.sh` cycle — use it to step through carefully or to
test the loop before running it unattended.

Do this now:
1. Run `loop/where.sh --json` FIRST, then read exactly the files it lists in
   `.read` — nothing else — and run `loop/where.sh --context` once for the step
   text, the LOG tail and the cited spec sections. Never open a closed task's
   `LOG.md`. Dispatch on its flags in PROMPT §1's order, first true one wins:
   `.error` · `.tree_clean == false` · `.needs_open` · `.needs_plan` ·
   `.unasked_questions > 0` · `.waiting_on` · `.spec_stub` · `.all_steps_done` ·
   `.parallel_steps` (more than one) · otherwise step `.step`.
2. Respect the gates (AGENTS.md §6) — especially: if the foundation phase is
   open, do not write later-phase `src/` code. Never cross a human-only boundary
   (secrets, production deploys, destructive infra, irreversible external
   actions) autonomously.
3. Do step `.step` of `.steps`, titled `.step_title` — the single next unchecked
   `PLAN.md` step. Just one.
4. Verify it (AGENTS.md §5), then the `reviewer` by risk tier (PROMPT §4b —
   risky before the commit, low-risk batched before the push; started in the
   background while you write the LOG; ONE pass; MEDIUM go to `## Amendments`,
   there is no LOW; a MEDIUM becomes a `- A<n> … · disposition: open` entry).
   Record what changed and the dispositions in `LOG.md` in the template's shape,
   ≤40 lines.
5. Touch `loop/STATE.md` / `tasks/INDEX.md` **only** if the gate verdict changed,
   a decision was made, a carry-forward moved, or the task opened/closed — never
   to record a step.
6. `git add` the step's files, then `loop/step-done.sh --commit
   "<type>(<scope>): <subject>"`, plus `--push` when nothing unreviewed remains.
7. End with exactly one marker on the last line: `<<LOOP:CONTINUE>>`,
   `<<LOOP:PHASE_COMPLETE>>`, `<<LOOP:DONE>>`, `<<LOOP:GATE_FAILED>>`, or
   `<<LOOP:BLOCKED>>`.

Then STOP. Do not start the next step — that's the next iteration.

If `$ARGUMENTS` is given, treat it as a focus/override for which step to do, but
still do only one.
