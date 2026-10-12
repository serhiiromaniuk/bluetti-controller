# loop/STATE.md — the non-derivable pointer

> **`loop/where.sh --json` for position; this file for judgement.** Keep only what
> no script can derive. **Budget ≤40 lines** (`make loop-hygiene`). Update on a
> gate change, a decision, or a carry-forward — **not** every iteration.

| Field | Value |
|-------|-------|
| **Gate status** | 🟢 Phase 1: RULES.md + ROADMAP.md approved, `make check` green — close + tag `v0.1-foundation` |
| **Blocked?** | no (owner questions pending: tasks/phase-1_foundation/PLAN.md) |

## Carry-forward — obligations that outlive the task that found them

- CF-1 → phase 2: install `libdbus-1-dev` + `pkg-config` on the dev host before adding `bluer`. Recorded in: ROADMAP.md §4.9. Discharge: `cargo build` with bluer succeeds.
- CF-2 → phase 3: Elite 200 V2 map and encryption state are unverified. Recorded in: ROADMAP.md §4.3, §4.5. Discharge: M2 dump + acceptance run.

## Decisions — do not relitigate
- 2026-10-13: read all stations over local BLE, porting the community handshake to Rust — beat the official cloud and the licensed crypto module (ROADMAP.md ADR-002).
- 2026-10-13: no APK reverse-engineering; register sweeps and HCI snoop first (ADR-003).
- 2026-10-13: stack tokio + bluer + axum + askama + rusqlite + uPlot (ADR-004).
- 2026-10-13: web UI only; no notifications, no Prometheus, no cloud in the MVP (ROADMAP.md §2).
- 2026-10-13: no CI/CD; `make check` locally is the gate (owner).
- 2026-10-13: RULES.md approved by the owner — golden rules, phase table, acceptance thresholds.
- 2026-10-13: ROADMAP.md approved by the owner — scope, decisions ADR-001…008, milestones.
