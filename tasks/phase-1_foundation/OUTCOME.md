# Outcome

> Written when the task closes — done **or** blocked.

## Summary
Phase 1 (M0 Foundation) is done. Research on the Bluetti BLE protocol, encryption,
access options, Rust stack, homelab host and outage schedules is consolidated in
ROADMAP.md with sources; decisions ADR-001…008 are recorded; the repo is
agent-ready (RULES/AGENTS, loop, ledger, subagents, Makefile); the pure protocol
layer is coded with 11 unit tests. The owner approved RULES.md and ROADMAP.md.

## Deliverables
- Commits:
  - `1d6412f` — `chore(docs): scaffold repo with roadmap, rules and protocol layer`
  - `7ffd121` — `build(config): pin Rust 1.99.0 via rust-toolchain.toml`
  - `323a5ec` — `docs(docs): record owner approval of RULES.md`
  - `2c68f4d` — `docs(docs): record owner approval of ROADMAP.md`
  - `7519af3` — `docs(docs): owner sign-off — RULES.md and ROADMAP.md approved` (owner-authored)
  - `baac5be` — `fix(protocol): close-hold 1 findings for phase 1`
- Files / artifacts: `ROADMAP.md`, `RULES.md`, `AGENTS.md`, `CLAUDE.md`,
  `rust-toolchain.toml`, `Makefile`, `loop/`, `tasks/`, `.claude/`,
  `src/protocol/{modbus,device}.rs`, `.agent-readiness/` (grade A, 91/100).
- Milestone tag: `v0.1-foundation`

## Pending / Follow-up
- [ ] ROADMAP.md §14 Q2–Q8 (host, region, router DC/AC, firmware, names, poll
      interval, VPN) — needed before Phase 2 hardware runs.
- [ ] Open the Phase 2 task (M1, ROADMAP.md §12) when development resumes;
      discharge loop/STATE.md CF-1 (libdbus-1-dev) and CF-3 (exception-aware
      reassembly) there.

## Amendments
- A1 — fixed: batched reviewer pass over every commit of the task plus the fixes (close-hold 1)
- A2 — fixed in baac5be (doc references point at ROADMAP.md)
- A3 — fixed in baac5be (all fields `verified: false` until hardware runs)
- A4 — fixed in `7519af3` (owner-authored sign-off commit)
- A5 — fixed in baac5be (`keys.toml` ignored)
- A6 — fixed (doc) in baac5be; length helper re-targeted → CF-3
- A7 — fixed in baac5be (AGENTS.md gate and coverage wording)

## Lessons / Notes
- The Debian/Ubuntu `cargo` ships without rustfmt and clippy; rustup with a
  pinned `rust-toolchain.toml` fixed it and makes the toolchain reproducible.
- Steps 1–4 were committed in the planning session without a per-step reviewer;
  the close-reviewer caught it and one batched reviewer pass covered them. Run
  the reviewer per risk tier from the first commit next time.
- Owner sign-offs must be owner-authored commits; a driver relaying "I approve"
  does not count under RULES.md.

## Close review (second pass): VERDICT close
- MEDIUM — `loop/PROMPT.md:130` still said `lint + typecheck + test` → fixed in the close commit.
- MEDIUM — fix commit cited without a hash → `baac5be` written into OUTCOME and PLAN.
- MEDIUM — owner sign-off lives in an owner-authored commit (`7519af3`), not a LOG
  line; optional for the owner: add a `human-verified` LOG line in their own commit.
