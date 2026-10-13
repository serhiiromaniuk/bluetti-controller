# loop/STATE.md — the non-derivable pointer

> **`loop/where.sh --json` for position; this file for judgement.** Keep only what
> no script can derive. **Budget ≤40 lines** (`make loop-hygiene`). Update on a
> gate change, a decision, or a carry-forward — **not** every iteration.

| Field | Value |
|-------|-------|
| **Gate status** | 🟢 Phase 1 closed, tagged `v0.1-foundation`. Phase 2 in progress: `tasks/phase-2_ble-transport/` (opened 2026-10-13) |
| **Blocked?** | no (Q2 partly answered, M5 open; Q6 answered; Q5 comes from M1 `scan`; Q3/Q4/Q7/Q8 block M3+ only) |

## Carry-forward — obligations that outlive the task that found them

- ~~CF-1 → phase 2: install `libdbus-1-dev` + `pkg-config` on the dev host before adding `bluer`. Recorded in: ROADMAP.md §4.9. Discharge: `cargo build` with bluer succeeds.~~ Discharged 2026-10-13: `cargo build --locked` compiles bluer 0.17.4 (phase-2 step 1).
- CF-2 → phase 3: Elite 200 V2 map and encryption state are unverified. Recorded in: ROADMAP.md §4.3, §4.5. Discharge: M2 dump + acceptance run.
- ~~CF-3 → phase 2: transport reassembly must stop at 5 bytes when the function byte has 0x80 set (Modbus exception). Recorded in: PLAN A6, `src/protocol/modbus.rs` module doc. Discharge: pure `expected_frame_len` helper + exception-vector test in M1.~~ Discharged 2026-10-13: `expected_frame_len` + `frame_len_stops_at_five_on_exception` (phase-2 step 3).

## Decisions — do not relitigate
- 2026-10-13: read all stations over local BLE, porting the community handshake to Rust — beat the official cloud and the licensed crypto module (ROADMAP.md ADR-002).
- 2026-10-13: no APK reverse-engineering; register sweeps and HCI snoop first (ADR-003).
- 2026-10-13: stack tokio + bluer + axum + askama + rusqlite + uPlot (ADR-004).
- 2026-10-13: web UI only; no notifications, no Prometheus, no cloud in the MVP (ROADMAP.md §2).
- 2026-10-13: no CI/CD; `make check` locally is the gate (owner).
- 2026-10-13: RULES.md approved by the owner — golden rules, phase table, acceptance thresholds.
- 2026-10-13: ROADMAP.md approved by the owner — scope, decisions ADR-001…008, milestones.
- 2026-10-13: owner resumed development; Phase 2 may open (owner).
- 2026-10-13: M1 runs on the owner's dev laptop (internal BT adapter); the homelab host arrives after MVP (owner, ROADMAP.md §14 Q2).
- 2026-10-13: AC180 stays off for now; M1 validates on the EB3A, the Elite 200 V2 is in BLE range (owner).
