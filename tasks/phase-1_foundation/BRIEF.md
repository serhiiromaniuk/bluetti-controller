# Phase 1 — Foundation  |  phase-1_foundation

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
Research, spec, agent-ready scaffold and the pure protocol layer (milestone M0),
reviewed by the owner.

## Why
Golden rule #6: no hardware or later-phase production code before the foundation
is complete and reviewed. Everything after this depends on the decisions in
ROADMAP.md §3.

## Scope
**In:** research findings with sources, decisions, ROADMAP, RULES/AGENTS/loop
scaffold, Makefile, `.gitignore`, `src/protocol/` (framing, CRC, model detection,
encryption flag, register maps, decoding) with unit tests.
**Out:** BLE transport, CLI, encryption handshake, storage, UI (phases 2–5).
CI/CD (owner decided against it).

## Done when
- [ ] ROADMAP.md covers goal, scope, decisions, research, architecture, data
      model, metrics, UI/API, config, deployment, testing, milestones, risks,
      questions — check: owner reads it and signs off in LOG.md
- [ ] `make check` green locally — check: `make check` output in LOG.md
- [ ] Owner confirms golden rules, phase table and thresholds in RULES.md —
      check: owner's `human-verified` line in LOG.md
- [ ] the phase gate passes — see RULES.md / AGENTS.md §6

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules, phase table) · ROADMAP.md §3, §12 (M0), §14
- Related files: `src/protocol/modbus.rs`, `src/protocol/device.rs`
