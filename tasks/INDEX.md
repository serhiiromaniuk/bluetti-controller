# Task ledger

Every task, its phase, and its status. **One line per task — a ledger row, not a
history.** Update on create, on status change, and on close; **never** to record a
step. Status: `todo · in-progress · blocked · done`.

> **Budget: ≤200 B per row** (`make loop-hygiene` warns). Detail belongs in that
> task's `OUTCOME.md` — link it, don't inline it. Exactly one row carries
> `in-progress`: that is what `loop/where.sh` reads to find the active task, and its
> first markdown link must be the task folder.

See [`../AGENTS.md`](../AGENTS.md) for how tasks work,
`loop/where.sh --human` for the live position, and
[`../loop/STATE.md`](../loop/STATE.md) for the gate + carry-forwards.

| Started | Phase | Task | Folder | Status | Milestone tag |
|---------|-------|------|--------|--------|---------------|
| 2026-10-13 | 1 | Foundation (M0) | [OUTCOME](./phase-1_foundation/OUTCOME.md) | done | `v0.1-foundation` |
| 2026-10-13 | 2 | BLE transport + CLI (superseded) | [OUTCOME](./phase-2_ble-transport/OUTCOME.md) | done | — |
| 2026-10-13 | 2 | BLE + encryption, EB3A + Elite (M1+M2) | [OUTCOME](./phase-2_ble-encryption/OUTCOME.md) | done | `v0.3-encryption` |
| 2026-10-13 | 4 | Collector, storage, metrics (M3) | [phase-4_collector](./phase-4_collector/) | in-progress | `v0.4-collector` |

<!--
Roadmap (RULES.md phase table, details in ROADMAP.md §12) — open a task per phase as you reach it:
  1  Foundation (M0)                         → v0.1-foundation
  2  BLE + encryption, EB3A + Elite (M1+M2)  → v0.3-encryption
  4  Collector, storage, metrics (M3)        → v0.4-collector
  5  Web UI + deploy + 7-day soak (M4 + M5)  → v1.0-mvp
-->
