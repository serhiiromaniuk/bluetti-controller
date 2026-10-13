# loop/STATE.md — the non-derivable pointer

> **`loop/where.sh --json` for position; this file for judgement.** Keep only what
> no script can derive. **Budget ≤40 lines** (`make loop-hygiene`). Update on a
> gate change, a decision, or a carry-forward — **not** every iteration.

| Field | Value |
|-------|-------|
| **Gate status** | 🟢 Phase 1 closed, tagged `v0.1-foundation`. Phase 2 (merged with 3) in progress: `tasks/phase-2_ble-encryption/` (planned, 17 steps) |
| **Blocked?** | no — owner answered everything up front 2026-10-13; physical checks go to `HUMAN-CHECKLIST.md` |

## Carry-forward — obligations that outlive the task that found them

- ~~CF-1 → phase 2: install `libdbus-1-dev` + `pkg-config` on the dev host before adding `bluer`. Recorded in: ROADMAP.md §4.9. Discharge: `cargo build` with bluer succeeds.~~ Discharged 2026-10-13: `cargo build --locked` compiles bluer 0.17.4 (phase-2 step 1).
- ~~CF-2 → phase 2 (merged): Elite 200 V2 map and encryption state are unverified. Recorded in: ROADMAP.md §4.3, §4.5. Discharge: M2 dump + acceptance run.~~ Discharged 2026-10-13: `BLUETTF` + handshake, dump vs PR #49 in §4.3 (step 14), 20/20 acceptance (step 16).
- CF-4 → phase 2 (before the close): EB3A `ac_output_power` read 0 W on 20/20 acceptance polls while its AC outlet powered the router (~9 W) in a grid outage (owner observation, `HUMAN-CHECKLIST.md`). Plausible-range checks cannot catch it. Discharge: find the right register or a documented low-load floor (community maps, `dump`), recorded in LOG.
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
- 2026-10-13: phases 2 and 3 merged — the EB3A advertises `BLUETTF`; handshake built and validated on the EB3A + Elite, AC180 deferred (owner).
- 2026-10-13: standing delegation + deferred physical checks (RULES.md); the loop never stops to ask what it can answer (owner).
- 2026-10-13: static keys from the MIT sources may go into a gitignored local keys file (owner).
- 2026-10-13 14:05: AC180 (`mobile`) is on and in range; its MAC is in `config.toml`. Read-only access is allowed like the others: scan, detect, read, dump, acceptance run. Add it to phase 2 by a PLAN amendment before the close; only the app comparison stays in `HUMAN-CHECKLIST.md` H3 (owner).
- 2026-10-13: Q4 router on EB3A AC · Q7 poll 60 s · Q8 LAN only · M5 soak on the dev laptop, user-level systemd (owner).
