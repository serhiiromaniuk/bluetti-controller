# Plan

> Written before work starts. **Do not edit the steps once work begins** —
> append changes under `## Amendments`. One step = one loop increment.

## Questions for the owner

- Q1 — Confirm or edit golden rules, phase table and thresholds (RULES.md) · blocks: 6 · answer: approved as written, 2026-10-13
- Q2 — Install `rustfmt` + `clippy` on the dev host (apt `rustfmt rust-clippy`, or rustup)? · blocks: 5 · answer: rustup installed by the owner, 2026-10-13
- Q3–Q8 — ROADMAP.md §14 (host, region, router DC/AC, firmware, names, poll interval, VPN) · blocks: none in this phase · answer: asked 2026-10-13

## Steps
- [x] 1. Research consolidated with sources (spec §4, spec §15) — check: every §4 fact has a source id
- [x] 2. ROADMAP written: decisions, architecture, data model, metrics, UI, config, deploy, tests, milestones, risks (spec §3, spec §12) — check: owner review (step 6)
- [x] 3. Agent-readiness scaffold: RULES/AGENTS/WORKFLOW/CLAUDE.md, loop/, tasks/, .claude/, Makefile, .gitignore — check: `loop/where.sh --json` runs; agent-readiness re-audit
- [x] 4. Protocol layer M0 (spec §4): framing, CRC, model detection, encryption flag, maps, decoding — check: `cargo test` 11/11
- [x] 5. `make check` green locally (spec §11) (needs Q2) — check: `make check`
- [x] 6. Owner review and sign-off of RULES.md + ROADMAP.md (spec §3, spec §12, spec §14) (needs Q1) — check: owner's `human-verified` line in LOG.md

## Risks / Dependencies
- `rustfmt`/`clippy` are missing from the Debian cargo on the dev host; `make check` cannot pass until installed (Q2).
- Golden rules were drafted by the agent from the owner's answers; only the owner can confirm them.

## Escape hatches
- If a step fails 3 times: stop, log the blocker, set status: blocked, emit `<<LOOP:BLOCKED>>`.
- If a destructive / secret-touching / irreversible external action is required: stop and hand to a human.
- (gate phases) if the gate fails: `<<LOOP:GATE_FAILED>>` — fix the work, never the threshold.

## Amendments
- 2026-10-13 — plan: steps 1–4 were done in the planning session before this plan existed (initial commit).
- A1 · 2026-10-13 · HIGH `tasks/phase-1_foundation/LOG.md:23` — no reviewer pass on any commit of this task, src/protocol included · close-hold 1 · disposition: fixed in the close-hold 1 fix commit (LOG 2026-10-13)
- A2 · 2026-10-13 · MEDIUM `src/lib.rs:5` — references to missing docs/ROADMAP.md and docs/RESEARCH.md · close-hold 1 · disposition: fixed in the close-hold 1 fix commit (LOG 2026-10-13)
- A3 · 2026-10-13 · MEDIUM `src/protocol/device.rs:157` — fields marked `verified: true` without a run on our hardware, contradicts RULES.md · close-hold 1 · disposition: fixed in the close-hold 1 fix commit (LOG 2026-10-13)
- A4 · 2026-10-13 · UNVERIFIED `tasks/phase-1_foundation/LOG.md:72` — owner sign-offs were written by the driver; an owner-authored record is needed · close-hold 1 · disposition: fixed in 7519af3 (owner-authored commit)
- A5 · 2026-10-13 · HIGH `.gitignore:4` — `keys.toml` (ROADMAP.md §9) was not ignored · close-hold 1 · disposition: fixed in the close-hold 1 fix commit (LOG 2026-10-13)
- A6 · 2026-10-13 · MEDIUM `src/protocol/modbus.rs:3` — reassembly doc ignored 5-byte exception replies · close-hold 1 · disposition: fixed (doc) in the close-hold 1 fix commit; length helper re-targeted → CF-3
- A7 · 2026-10-13 · MEDIUM `AGENTS.md:56` — docs named a `typecheck` target and coverage the Makefile does not have · close-hold 1 · disposition: fixed in the close-hold 1 fix commit (LOG 2026-10-13)
