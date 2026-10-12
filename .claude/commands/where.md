---
description: Report where the project stands — active phase, task, next step, gate status, and what's blocking the next milestone.
allowed-tools: Read, Bash(loop/where.sh*), Bash(loop/amendments-guard.sh*), Bash(git status*), Bash(git tag --list*), Bash(git log*), Grep, Glob
---

<!-- Named /where, after the loop/where.sh oracle it runs — /status is a Claude Code
     built-in and would shadow it. -->

Report the current state of the `bluetti-controller` agentic workflow. Be concise — a
status readout, not an essay. Do not change anything.

1. Run `loop/where.sh --human` — phase, task, step N/M, governing spec (and
   whether it is still a stub), gate, tree state, last result. This replaces
   reading `loop/STATE.md` + `tasks/INDEX.md` for position.
2. Read the active task's `BRIEF.md` (done-when boxes) and run
   `loop/where.sh --context` for the step text and the LOG tail. Closed tasks'
   folders are archive — do not open them.
3. Read `loop/STATE.md`'s carry-forward + decision sections only if the question
   needs them. Run `loop/amendments-guard.sh` when the question is about what is
   still open.
4. Run `git tag --list` (which milestones are tagged) and `git log --oneline -5`.
5. Cross-check the gate: which phase gate is the next blocker (see RULES.md phase
   table + AGENTS.md §6)?

Output:
- **Phase / task / next step** (one line each)
- **Gate status** — open/closed, and what must pass to lift the next gate
- **Done-when progress** — checked vs unchecked from the BRIEF
- **Repo** — clean/dirty, latest tag, last commit
- **Next action** — the single next step the loop (or a human) should take

If `$ARGUMENTS` names a phase, focus the report on that phase instead.
