# AGENTS.md — operating manual for agentic work in `bluetti-controller`

How an AI agent (Claude Code, or any coding agent) must work in this repo.
[`RULES.md`](./RULES.md) is the *governance layer* (what is true about the
project: golden rules, architecture, conventions). **This file is the *operating
layer*** (how you actually execute work across long-running, multi-session,
phase-gated tasks). [`WORKFLOW.md`](./WORKFLOW.md) is the worked example.

Read order at the start of any session: **run
[`loop/where.sh`](loop/where.sh)` --json` first**, then exactly the files it
lists in `.read` (RULES.md and AGENTS.md only when `CLAUDE.md` does not already
import them — see `.loaded`), then `where.sh --context` once: the current step's
full text, the tail of the active task's `LOG`, and the governing-spec sections
the step cites.
The oracle computes position, gate and tree state from disk, so
[`tasks/INDEX.md`](tasks/INDEX.md) is **not** read for orientation and closed
tasks' folders are archive.

> Do not improvise, skip steps, or deviate from the workflow below. When the
> workflow and a clever shortcut disagree, the workflow wins.

---

## 0. The one rule that overrides this whole file

**The golden rules in [`RULES.md`](./RULES.md) are non-negotiable.** Nothing in
this operating manual permits you to violate them. In particular:

- The runtime invariants (RULES.md golden rules #1/#4/#5) constrain the *product
  you build*, not you the dev agent. The loop, subagents, and this manual are
  **dev-time tooling only** — they must leave **zero** forbidden runtime
  dependency in `src/`. If you find yourself about to introduce one, stop: that
  is the line.
- **Golden rule #6 gates everything.** No later-phase production logic until the
  foundation phase is complete and reviewed. The loop must refuse to write
  later-phase code while the foundation gate is open. See §6 (Gates).

---

## Quick reference — setup, commands & structure

> The build/test/run commands and the repo map in one place, so any agent (or
> tool) has them up front. Deeper conventions live in [`RULES.md`](./RULES.md);
> the *how-to-execute* workflow is §1+ below. Safety (§0 above) overrides all.

### Prerequisites
- Rust stable ≥ the `rust-version` in `Cargo.toml`, with `rustfmt` and `clippy`
  (rustup, or Debian/Ubuntu packages `rustfmt` + `rust-clippy`), GNU make, git,
  jq. From M1: `libdbus-1-dev` + `pkg-config` (for `bluer`), BlueZ ≥ 5.63 and a
  BLE adapter. Device config and keys live in a local `config.toml`, never in
  the repo (RULES.md golden rule #3).

### Commands — the `Makefile` is the only sanctioned entrypoint

```bash
make install   # set up the isolated environment + install deps
make check     # lint + typecheck + tests, in parallel — RUN BEFORE EVERY COMMIT
make step-done MSG="…" [PUSH=1]   # gate → LOG evidence → tick → hygiene → scan → commit
make test      # tests with coverage (≥80% target)
make lint | format | typecheck   # the individual gates
make lock      # freeze exact dep versions
make help      # full target list
```

Project-specific targets arrive with their milestones (ROADMAP.md §12): `make
build` (release binary, M1), `make run` (serve with a local config, M3). Commands
that touch real stations (`bluetti-controller scan|read|dump`) are human-run
(RULES.md "human-only boundary").

### Project structure

```
src/                  the crate (lib + binary)
  protocol/             pure: Modbus framing, register maps, decoding (M0); crypto (M2)
  transport/            HARD boundary — `bluer` (BlueZ) lives ONLY here (M1)
  collector/ store/ metrics/ web/   poll loop, SQLite, derived metrics, UI (M3-M4)
  config.rs             every config value read + validated in ONE place (M1)
tests/                integration tests + anonymised frame fixtures (tests/frames/)
ROADMAP.md            spec + research + decisions + milestones (the governing spec)
loop/                 agent loop: PROMPT.md (invariant prompt) · loop.sh ·
                        where.sh (position oracle) · step-done.sh (close a step) ·
                        entry-size-guard.sh · amendments-guard.sh · merge-gate.sh ·
                        STATE.md
tasks/                per-phase BRIEF/PLAN/LOG/OUTCOME folders + INDEX.md ledger
.claude/agents/       specialist subagents (planner, plan-reviewer, reviewer, close-reviewer, adjudicator, scout; verifier optional)
.claude/commands/     slash-command shortcuts (/loop-step, /where)
CLAUDE.md             @-imports RULES.md and AGENTS.md, so every session and subagent has them
RULES.md              governance: golden rules, architecture, conventions, git
AGENTS.md             this file — how an agent executes work here
WORKFLOW.md           worked example
Makefile              the single sanctioned dev entrypoint
```

---

## 1. Core philosophy

- **One session = one focused outcome.** A session advances exactly one task
  (one phase, or one slice of a phase). Resist scope creep; spin a new task.
- **Structure is memory, not the context window.** The task folder
  (`BRIEF / PLAN / LOG / OUTCOME`) plus git history *is* the agent's memory. A
  fresh agent with zero conversation history must be able to resume from the
  files alone. Progress lives on disk, not in tokens.
- **Iteration over perfection.** Small, verified increments beat big unverified
  leaps. Commit + push each one.
- **Failures are data.** A failed step is a `LOG.md` entry and a plan
  adjustment, not a silent retry. Document, adapt, continue.
- **Every claim needs evidence.** "Tests pass" means the `make check` output is
  in the log. No asserting success you didn't observe.
- **Simplicity first.** Reach for a subagent or a parallel fan-out only when a
  single straight-line pass genuinely can't do it. Don't add machinery you don't
  need.

---

## 2. The unit of work: phases and tasks

Work is organized by the **phase roadmap in [`RULES.md`](./RULES.md)**. Each
phase (or a meaningful slice of one) becomes a **task folder** under
[`tasks/`](tasks/):

```
tasks/
├── INDEX.md          # the ledger — every task, its phase, its status
├── _template/        # copy this to start a task
│   ├── BRIEF.md   PLAN.md   LOG.md   OUTCOME.md
└── phase-1_<short-kebab>/   # naming: phase-N_short-kebab  (or  YYYY-MM-DD_short-kebab for off-roadmap work)
    ├── BRIEF.md   PLAN.md   LOG.md   (OUTCOME.md added when done)
```

**Folder naming**
- On-roadmap: `phase-<N>_<short-kebab>`.
- Off-roadmap / ad-hoc: `YYYY-MM-DD_<short-kebab>`.

**The four documents** (one purpose each — never merge them):

| File | Lifecycle | Holds |
|------|-----------|-------|
| `BRIEF.md` | **Static** after first write | what, why, scope (in/out), **done-when** (verifiable), refs |
| `PLAN.md`  | Steps frozen once work starts; **append amendments** at bottom | numbered steps, risks/deps, **escape hatches** |
| `LOG.md`   | **Append-only**, newest at bottom, timestamped | what happened, commands run, decisions, findings |
| `OUTCOME.md` | Written at close (done **or** blocked) | summary, deliverables (commits/tags), follow-ups, lessons |

Discipline: never edit a past `LOG.md` entry; never rewrite `PLAN.md` steps
mid-flight (amend instead); never silently overwrite — append a dated note.
Deferred reviewer findings live in `PLAN.md`'s `## Amendments` as
`- A<n> … · disposition: open` items; each is settled in place (`fixed`,
`re-targeted`, `declined — <reason>`) and accounted for in `OUTCOME.md` at the
close (`make amendments` counts them). A deferred finding is never dropped.

The ledger [`tasks/INDEX.md`](tasks/INDEX.md) gets one row per task with
status `todo | in-progress | blocked | done`.

---

## 3. Session rituals

### 3a. Startup (before touching anything)
1. Run [`loop/where.sh`](loop/where.sh)` --json` — phase · task · step N/M ·
   step title · governing spec (and whether it is still a stub) · gate · tree state
   · **the read list**.
2. Read **exactly** the files in `.read`, and nothing else: `STATE.md` for the
   gate/decisions, the active task's `BRIEF.md` and `PLAN.md`, the governing spec
   when the step cites no section of it, and [`RULES.md`](./RULES.md) / this file
   only when `CLAUDE.md` does not already load them (`.loaded`).
3. Run `where.sh --context` once — the step's full text, the newest two `LOG.md`
   entries, the cited spec sections. Never open a whole `LOG.md`, and never a
   closed task's.
4. Dispatch on the oracle's flags in loop/PROMPT.md §1's order, first true one
   wins: `.error` → open the next task · `.tree_clean == false` → **reconcile the
   interrupted increment first** · `.needs_open` / `.needs_plan` → that IS this
   iteration · `.unasked_questions > 0` → ask them all in one batch ·
   `.waiting_on` → blocked on an owner answer · `.spec_stub` → write the spec ·
   `.all_steps_done` → close the task (`make amendments` first) ·
   `.parallel_steps` (more than one) → the whole group is this increment ·
   otherwise → step `.step`.

### 3b. Shutdown (end of session / loop iteration)
1. Append a `LOG.md` entry in `_template/LOG.md`'s shape — **≤40 lines, bullets**.
   This is the ONE place per-step detail is written.
2. `loop/step-done.sh --commit "<msg>" [--push]` (§7): the gate tail into the LOG,
   the step's `PLAN.md` box, `make loop-hygiene`, the secret scan, the commit — one
   command. Push only reviewed commits (§8).
3. Touch [`loop/STATE.md`](loop/STATE.md) or
   [`tasks/INDEX.md`](tasks/INDEX.md) **only** when the gate verdict changes,
   a decision is made, a carry-forward is raised/discharged, or a task
   opens/closes — never to record a step. `where.sh` computes step position from
   the PLAN checkboxes and the last result from the LOG tail.
4. On a `make loop-hygiene` warning, shorten prose; never raise a budget.
5. The remote lags the work only by low-risk commits awaiting their batched
   review (≤3, §8) — never by reviewed ones.
6. If the task is finished or blocked, write `OUTCOME.md`.

---

## 4. The agent loop (long-running tasks)

The technique: **re-feed the same prompt to a fresh agent over and over; the
filesystem and git history carry progress between iterations.** The invariant
prompt is [`loop/PROMPT.md`](loop/PROMPT.md); the harness is
[`loop/loop.sh`](loop/loop.sh); the position oracle is
[`loop/where.sh`](loop/where.sh) and the non-derivable pointer is
[`loop/STATE.md`](loop/STATE.md). See [`loop/README.md`](loop/README.md).

**One iteration does exactly this:**
1. Load context (§3a) — `where.sh --json` first, then only what it names.
2. Do step `.step` of `.steps` — the next single unchecked `PLAN.md` step. One.
3. **Verify** it (run the relevant check — §5), then the `reviewer` by risk tier
   (§8): risky steps before the commit, low-risk steps in a batch before the push;
   started in the background while you write the LOG. **One** pass, CRITICAL/HIGH
   fixed now, MEDIUM appended to `PLAN.md`'s `## Amendments` as an `A<n>` item
   with its `file:line`.
4. Record what changed and the dispositions in `LOG.md` — the template's shape,
   ≤40 lines.
5. `loop/step-done.sh --commit "<msg>" [--push]`: gate evidence, tick, hygiene,
   scan, commit (and push when nothing unreviewed remains).
6. Emit a **completion marker** (below) and stop. The loop re-invokes a fresh
   agent for the next step.

**Completion markers** (exact strings — the harness greps for them):
- `<<LOOP:CONTINUE>>` — increment done, more steps remain in this phase.
- `<<LOOP:PHASE_COMPLETE>>` — every step done **and** the phase gate passed (§6).
  The agent writes `OUTCOME.md`, **tags the milestone** (`git tag … && git push
  --tags`), opens the next phase's task, and the loop **re-invokes** to roll into
  it. Tag only when the gate objectively passed — never weaken a threshold to tag.
- `<<LOOP:DONE>>` — the whole project goal is reached: all phases done, final
  milestone tagged, everything verified. The loop **stops**.
- `<<LOOP:BLOCKED>>` — stuck (escape hatch tripped) or a step needs a human (e.g.
  a human-only boundary). The loop stops; `LOG.md`/`OUTCOME.md` explain why.
- `<<LOOP:GATE_FAILED>>` — a hard gate (acceptance criteria, coverage, golden
  rule) failed. The loop stops; **never** lower the threshold to get past it.

**Safety (mandatory):**
- `--max-iterations` is the **primary** safety mechanism. The loop is always
  bounded; an unbounded loop is forbidden. With autonomous tagging, a single
  bounded run can carry the project across several phases — the iteration cap is
  what keeps it reviewable.
- **Escape hatch:** if a single step fails **3 times**, stop, write the blocker
  to `LOG.md`, set the task `blocked`, emit `<<LOOP:BLOCKED>>`. Do not thrash.
- **What the loop MAY do autonomously:** tag a milestone and roll into the next
  phase — but **only after that phase's gate has objectively passed** (§5/§6).
- **The human-only boundary the loop must NEVER cross autonomously:** secrets /
  production credentials, production deploys, destructive infrastructure, and
  irreversible external actions. At these, stop (`<<LOOP:DONE>>` at the final
  milestone, else `<<LOOP:BLOCKED>>`) and hand back to a human.
- **Credentials stay out of reach.** Anything in the loop's environment or home
  can be read by every command the agent runs, so run it unattended where no
  credentials are set; `loop.sh` names any it sees ([`loop/README.md`](loop/README.md)
  "Credentials").

---

## 5. Verification — give yourself a check that returns pass/fail

A step isn't done until a check confirms it. Pick the cheapest check that
actually proves the step.

| What changed | Check (record output in LOG.md) |
|--------------|---------------------------------|
| Any code in `src/` or `tests/` | `make check` (lint + typecheck + tests) |
| New/changed logic | a **failing test first**, then green (TDD) |
| Correctness-critical logic | unit test with hand-computed expected values |
| Measured results (a gate) | the acceptance criteria vs thresholds (RULES.md) |
| Runtime/packaging change | a build / config-validation command |
| Research / design | the relevant doc section filled with a stated **recommendation** |

Coverage target **≥ 80%** (adjust to the project). If you cannot verify a
change, you cannot call it done — say so in the log and leave the box unchecked.

---

## 6. Gates — where the loop must stop

Gates are **hard**. You cross one only on observed evidence that it passed — the
milestone gate below is the one the loop crosses on its own — and you may
**never** weaken one to pass it ("fix the work, never lower the thresholds").
A gate that needs a human sign-off (golden rule #6's review, the human-only
gate) is crossed only once that sign-off is on record.

- **Foundation gate (golden rule #6):** the design/research/scaffold phase must
  be complete + reviewed before *any* later-phase production code. While it's
  open, the loop works only inside that phase's scope.
- **Acceptance gate:** the authoritative criteria live in [`RULES.md`](./RULES.md)
  — do not restate the numbers here (avoids drift). The agent evaluates
  objectively against RULES.md: **all pass → tag the milestone and continue; any
  miss → `<<LOOP:GATE_FAILED>>`** (fix the work, never lower a threshold).
- **Milestone gate (autonomous):** at the end of a phase, once the gate has
  **objectively passed** (`make check` green for code phases; the documented
  acceptance criteria otherwise), the agent itself tags the milestone and rolls
  into the next phase. A tag asserts the gate passed — so it is applied **only**
  on observed-green evidence, never to "make progress".
- **When the gate is the wrong metric — the falsified-metric path.** Rare, and
  never a shortcut. "Fix the work, never lower the threshold" assumes the
  threshold measures what it claims to. Occasionally it does not, and then
  *neither* legal move exists: the work is correct, and the number still cannot
  be met. Do **not** grind the step, and do **not** touch the constant. All
  three tests must hold before you may even propose a change:
  1. **Replicated** — the same overage *shape* is already on record at **≥3
     independent checkpoints** (prior `LOG.md` / decision entries), not just
     today's step. One data point is a slow increment; three is a metric
     measuring the wrong thing.
  2. **Arithmetically unreachable** — show the sum. The threshold cannot be met
     while also satisfying the project's *other* mandated requirements (classic
     case: a volume cap that binds against per-feature tests the rules
     themselves require). Compute it; asserting it does not count.
  3. **Re-scope, never delete** — the proposal splits *what is counted* and
     keeps a **hard gate on the portion moved out**. Any proposal whose net
     effect is "this quantity is no longer measured" is a weakened check —
     forbidden however it is framed.
  Then **the increment is the decision record, not the edit**: write it up (an
  ADR under `decisions/` if the repo has one, else a dated `LOG.md` entry) with
  the measured numbers, **≥2 options**, and a recommendation; leave the gate and
  the work exactly as verified; emit `<<LOOP:GATE_FAILED>>` naming the decision
  needed. **A human edits the constant.** Why this exists: a gate no correct
  work can pass stops the loop *every* iteration, and the third time you write
  "the overage is entirely mandated test volume, the production code is lean" is
  the loop telling you the metric is wrong — record that instead of re-deriving
  it a fourth time.
- **Human-only gate — permanent:** secrets / production credentials, production
  deploys, destructive infra, and irreversible external actions are **never**
  agent actions (golden rules #2/#3/#5). The agent reaches the boundary and
  **stops** (`<<LOOP:DONE>>` or `<<LOOP:BLOCKED>>`).
- **Harmless local setup is NOT that boundary — don't over-block.** Full
  permission means you MAY act without prompting. A **missing local tool** is not
  a `<<LOOP:BLOCKED>>` reason: install it (via the project's sanctioned tool
  setup) or run it via its **official Docker image** (`docker run …`) instead.
  The gate is about **harm / irreversibility / external reach** — not "a binary
  is absent" or "spinning a local container." Rule of thumb: **local + reversible
  + harmless → just do it**; global / irreversible / external / secret → stop.

---

## 7. Execution rules

- **Read before you write.** Open the file (and its neighbours) before editing.
- **Respect the hard boundaries** from RULES.md: vendor SDK only inside its
  adapter; all config through the one config module; state persists where
  correctness needs it; files small (≤ ~400 lines).
- **Atomic commits straight to the main branch.** One logical change per commit,
  Conventional Commits with a repo scope — `feat(<component>): …`,
  `test(<component>): …`, `docs(<component>): …`. No feature branches or PRs by
  default. Commit every completed increment with `loop/step-done.sh`, and push
  once it is reviewed (§8) — risky increments at once, low-risk ones with their
  batch. Task-folder docs
  may ride along with the code change they describe, or be their own `docs(...)`
  commit.
- **Parallel work → its own worktree branch (§8a).** Sequential steps commit
  straight to the main branch. When steps run concurrently, read-only discovery
  and steps that only create their own new files may share the working tree;
  every other concurrent edit gets its own short-lived branch in its own git
  worktree, so edits can't collide. Those branches merge back only after
  `loop/merge-gate.sh <branches>` is green on the **merged** tree.
- **Secrets hygiene.** Never write a key/token/secret into any file or commit.
  Secrets live only in the environment (gitignored). Before committing, scan the
  staged diff for anything secret-like.
- **Never commit** env files, keys, anything under data/output/log directories,
  or generated artifacts. `.gitignore` enforces it; if `git status` shows one,
  stop and fix it.

---

## 8. Subagents & delegation

Use a subagent when a side task would flood the main context with file dumps or
search output you won't reuse, or when you want an **independent** opinion. They
live in [`.claude/agents/`](.claude/agents/):

| Subagent | Use it to… | Phase |
|----------|------------|-------|
| `planner` | turn a `BRIEF.md` into a numbered `PLAN.md` (steps, risks, escape hatches) without touching code | any |
| `plan-reviewer` | audit a fresh `PLAN.md` before step 1 — missing work, step sizing, order, checks | task open |
| `reviewer` | audit a diff (or a batch of commits) in a **fresh context** against the step text, the golden rules + correctness, report gaps only | any |
| `verifier` | *optional* — run a check that needs judgment (smoke run, thresholds) and report pass/fail **with evidence**, no edits | any |
| `adjudicator` | rule whether a failing hard gate is itself wrong | on a gate failure |
| `scout` | discover one area read-only and write one sourced page (findings, gaps, open questions) | research / design tasks |
| `close-reviewer` | audit a task close before done-when is ticked — re-run the checks, exercise the deliverable, hunt stubs, test each declined or re-targeted amendment | task close |

Patterns: **parallelize independent reads**; **evaluator-optimizer** =
`reviewer`/`verifier` checking the builder's output in a fresh context so the
writer isn't its own grader — per step, and `close-reviewer` once for the whole
task, because steps can each pass while the BRIEF stays unmet. Keep the toolset small — more agents ≠ better.

**Mandatory, by risk (PROMPT.md §4b):** every commit is reviewed by the `reviewer`
in a fresh context before it is **pushed**.

- **Risky** — code with logic, scripts, infra/deploy config, host- or
  environment-changing steps, decision records, anything near a golden rule or a
  secret: reviewed **before the commit**.
- **Low-risk** — doc-only prose, mechanical edits: committed unpushed, then ONE
  batched pass over `@{u}..HEAD` when 3 are waiting (`where.sh` `.unreviewed`),
  before a risky step, or at the wave/task end. It sees the batch as one change,
  which catches contradictions between documents a per-step pass cannot.
- **Pipelined** — start the reviewer in the background as soon as the gate is
  green; write the LOG meanwhile; wait for the verdict before the commit (risky)
  or the push (batch). CRITICAL/HIGH are always fixed before the push.

The deterministic gate needs no subagent: `loop/step-done.sh` runs it and writes
the evidence. Skipping the reviewer, or pushing an unreviewed commit, is a loop
violation. Why tiers and not every step: per-step reviews rarely find a CRITICAL,
and their HIGH findings land mostly on scripts, config and decision records — the
classes that stay per-step. `planner` runs earlier — at an
objective's start — to write `PLAN.md`; it is not part of the per-commit gate.

### 8a. Parallelize independent work (the default — with guardrails)

When a unit of work decomposes into **genuinely independent** sub-tasks — ones
that don't consume each other's output and don't write the same files — run them
**concurrently as parallel subagents**, not serially. Good fits: independent
research/reads across subsystems, several review lenses over one diff at once,
independent file edits **via separate git worktrees** (§7) so concurrent writes
can't collide.

**How a `[parallel: X]` group runs** (the planner tags it; `where.sh` reports it
as `.parallel_steps`; the group is ONE increment):

1. Fan out one subagent per step, launched together. A read-only discovery step
   goes to a `scout` (one area, one sourced page, nothing else written). Steps
   that only create their own new files run in the same tree; anything else gets
   its own branch in its own git worktree (§7).
2. Subagents deliver their step's files only — never `LOG.md`, `PLAN.md` or a
   shared index row. Branches go through `loop/merge-gate.sh <branches>` (the
   gate on the MERGED tree), then merge.
3. ONE batched `reviewer` pass with every step's text: it sees the group as one
   change, which is where two pages contradicting each other get caught.
4. One LOG entry per step, shared index rows as their own edit, then
   `loop/step-done.sh --steps "<N M …>"` ticks the group together.

**Research-heavy tasks** open with such a group: one read-only discovery area per
`scout`, each writing its own page, then the decision records drafted side by
side. Done serially, that phase can dominate the whole run.

This does **not** override §1's simplicity rule: map the dependencies first and
only fan out work that is actually independent and non-trivial. If task B needs
task A's result, they are a **wave boundary** (sequential), not parallel. When
unsure, sequence it.

**Hard guardrails — parallelism never bends the safety model:**
- It speeds up *gathering and analysis within a step*. It does **not** let the
  loop abandon the one-verified-step-at-a-time discipline (§4) or cross a phase
  gate (§6) concurrently. Each parallel result is still independently
  **verified** (§5) before it's accepted.
- **Never run secret-touching, production-affecting, or shared-state mutations
  in parallel** — these stay strictly sequential and gated (golden rules
  #2/#3/#5). Concurrency must not create a race on state that decides whether an
  irreversible action happens.

The reviewer will always find *something*; act only on gaps that affect
correctness or a stated requirement, not on style or speculative hardening.

### 8b. Calibrate the graders — from their misses, by the owner

A separate grader is far easier to make skeptical than a writer is to make
self-critical — but only if its verdicts are read against what happened. Graders
are tuned from the cases where they were wrong:

- A CRITICAL/HIGH finding the driver rejects after reading the cited source is
  logged as `rejected — <why it is false>`; a defect found after a grader passed
  it (by a later step, the `close-reviewer`, or the owner) is logged naming that
  grader.
- At the close, the driver proposes one line per clear case in `OUTCOME.md`
  `## Lessons / Notes`: `<grader> · missed|false: <the kind of defect> — <why>`.
- The **owner** adds the lines they agree with to that grader's `## Calibration`
  section (ten at most per grader). The agent being graded never edits its
  grader — a writer that tunes its own reviewer has made itself the judge again.
- A grader that repeats a calibrated mistake needs its prompt changed or its
  model sized up (§10), not an eleventh line.

---

## 9. Multi-perspective review (before any significant or risky change)

Run the change past four lenses and note conclusions in `LOG.md`:

| Lens | Ask |
|------|-----|
| **Engineer** | Correct? Simplest approach? Inside the architecture boundaries? Rollback path? |
| **Risk** | Could this cause harm beyond intended limits? Are the safety guards still in place? Does the safety mode/flag still gate the dangerous path? Do safety limits still override normal logic? |
| **Security** | Any secret exposed/logged? Inputs validated at the boundary? Errors not leaking sensitive data? |
| **Future** | What does this make harder later? New tech debt? Does it bend a golden rule "just this once"? |

---

## 10. Throughput — what actually costs time

Measured on real projects: the model's thinking is rarely the bottleneck. Wall
time goes to round-trips (every tool call is a model turn), to serial work that
was independent, and to fixed per-step ceremony paid once per step.

- **Batch independent tool calls** into one turn: reads, searches, independent
  subagents. Three sequential calls cost three turns.
- **Fan out independent work** (§8a) and **ask owner questions in one batch**
  at task open (PLAN `## Questions for the owner`), not one per stalled step.
- **Let scripts do ceremony:** `where.sh --context` for the resume slice,
  `step-done.sh` to close a step, a parallel `make check`.
- **Right-size steps:** a batch step for trivial items one check proves; review
  by risk (§8), not by ritual.
- **Model and effort are sized per role**, in each `.claude/agents/*.md`
  frontmatter (`model:` + `effort:`). Both are set explicitly: a subagent without
  them inherits the session's, so a session at low effort would grade at low
  effort.

  | Who | Model | Effort | Runs | Why this size |
  |-----|-------|--------|------|---------------|
  | loop driver | `loop.sh --model opus` | config default | every iteration | writes the increment; the graders below check it |
  | `planner` | `opus` | `high` | once per task | decomposition; `plan-reviewer` audits it next |
  | `plan-reviewer` | `fable` | `high` | once per task | plan defects cost the most and surface only when executed; one call per task |
  | `reviewer` | `opus` | `high` | every risky step, every batch | the most frequent strong call; each false finding costs a fix-and-re-review round |
  | `close-reviewer` | `opus` | `high` | once per close | checks evidence and runs the deliverable; a stronger model adds little to that |
  | `adjudicator` | `fable` | `xhigh` | rarely — a gate failed at ≥3 checkpoints | its ruling can move a threshold, so the per-row arithmetic must hold |
  | `verifier` (optional) | `haiku` | `high` | only checks that need judgment | reads output against written thresholds; the smallest model, thinking hard, still costs little |
  | `scout` | `sonnet` | `medium` | each read-only discovery step (§8a) | breadth reading with sources; the batched `reviewer` checks every page together |
  | other parallel-group workers | inherited | inherited | steps that write files | the same work the driver would do, so the driver's model |

  Move a role up or down on evidence, not by habit: `verifier` → `sonnet` when its
  output needs real interpretation; `reviewer` → `fable` when its findings keep
  turning out false. The aliases (`opus`, `sonnet`, `haiku`, `fable`) follow each
  new release; pin a full model ID (e.g. `claude-opus-5-5`) only for a run that must
  be reproducible. `CLAUDE_CODE_EFFORT_LEVEL` overrides every `effort:` line, so
  keep it unset where the loop runs. Never give a grader `isolation: worktree` (the
  worktree branches from the default branch, not `HEAD`, so it grades the wrong
  tree) or `memory:` (it adds Write and Edit to a read-only role), and leave
  `omitClaudeMd` off: the rules reach every subagent through `CLAUDE.md`.
- In Claude Code, `/fast` speeds up the main session's output on the same model —
  useful for long mechanical stretches.

---

## 11. Anti-patterns (don't)

- Discovering owner questions one step at a time, or running independent
  research areas one after another.
- Holding progress only in the conversation — if context were lost, work would
  vanish. Write it to the task folder.
- Editing past `LOG.md` entries or rewriting `PLAN.md` steps mid-task.
- Marking a step done without recorded verification evidence.
- Pushing a commit no reviewer has seen, or retyping gate output by hand when
  `loop/step-done.sh` would have written it.
- Lowering a gate threshold, or skipping the foundation phase, to "make progress".
- Editing a grader's prompt or `## Calibration` from inside the loop (§8b).
- Writing an owner's sign-off — a `human-verified` LOG line or an owner decision
  in `STATE.md` — from inside the loop. Only the owner records their own check.
- Running an unbounded loop, or thrashing on a failing step past the 3-try hatch.
- Batching unrelated changes into one commit, or letting the remote lag.
- Importing a vendor SDK outside its adapter, or reading config outside the one
  config module.
- Introducing a forbidden runtime dependency (golden rule #1).
- Crossing a human-only boundary (secrets, production deploys, destructive infra,
  irreversible external actions) autonomously. (Milestone tagging and code-phase
  gates ARE autonomous once the gate objectively passes — §4/§6.)
