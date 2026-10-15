# loop/STATE.md — the non-derivable pointer

> **`loop/where.sh --json` for position; this file for judgement.** Keep only what
> no script can derive. **Budget ≤40 lines** (`make loop-hygiene`). Update on a
> gate change, a decision, or a carry-forward — **not** every iteration.

| Field | Value |
|-------|-------|
| **Gate status** | 🟢 `v0.4-collector` tagged 2026-10-15 (phase 4 closed; unplug deferred to H4). Next: open `tasks/phase-5_web-deploy/` (M4+M5, `v1.0-mvp`) |
| **Blocked?** | no — owner answered everything up front 2026-10-13; physical checks go to `HUMAN-CHECKLIST.md` |

## Carry-forward — obligations that outlive the task that found them

- ~~CF-1 → phase 2: install `libdbus-1-dev` + `pkg-config` on the dev host before adding `bluer`. Recorded in: ROADMAP.md §4.9. Discharge: `cargo build` with bluer succeeds.~~ Discharged 2026-10-13: `cargo build --locked` compiles bluer 0.17.4 (phase-2 step 1).
- ~~CF-2 → phase 2 (merged): Elite 200 V2 map and encryption state are unverified. Recorded in: ROADMAP.md §4.3, §4.5. Discharge: M2 dump + acceptance run.~~ Discharged 2026-10-13: `BLUETTF` + handshake, dump vs PR #49 in §4.3 (step 14), 20/20 acceptance (step 16).
- ~~CF-4 → phase 2 (before the close): EB3A `ac_output_power` read 0 W on 20/20 acceptance polls while its AC outlet powered the router (~9 W) in a grid outage (owner observation, `HUMAN-CHECKLIST.md`). Plausible-range checks cannot catch it. Discharge: find the right register or a documented low-load floor (community maps, `dump`), recorded in LOG.~~ Discharged 2026-10-13: low-load floor documented in ROADMAP.md §4.3 (reg 38 stays; 0 W at ~9 W; H1 bounds it), phase-2 step 21.
- ~~CF-3 → phase 2: transport reassembly must stop at 5 bytes when the function byte has 0x80 set (Modbus exception). Recorded in: PLAN A6, `src/protocol/modbus.rs` module doc. Discharge: pure `expected_frame_len` helper + exception-vector test in M1.~~ Discharged 2026-10-13: `expected_frame_len` + `frame_len_stops_at_five_on_exception` (phase-2 step 3).
- ~~CF-5 → phase 4 (before outage logic): Elite 200 V2 / AC180 `ac_input_voltage` (1314, ×0.1) and `ac_input_frequency` (1300) never read with the grid on. Recorded in: phase-2 OUTCOME (close-review M2). Discharge: grid-on `read` of both, values in LOG.~~ Discharged 2026-10-13 on the Elite: 223.3 V / 49.90 Hz with the grid on (phase-4 step 1); the AC180 half read in the step-14 smoke: 218.3–221.4 V / 49.9–50.0 Hz, 16 polls (H5's own check stays the owner's).
- ~~CF-6 → phase 5 (M5 unit): `Linger=no` on the dev laptop, so a `systemd-run --user` unit dies at logout. Recorded in: phase-4 PLAN A19. Discharge: `loginctl enable-linger` (user-level; polkit may refuse → H-row "stay logged in") before the 7-day soak, `loginctl show-user` in LOG.~~ Discharged 2026-10-15: `loginctl enable-linger` → `Linger=yes` (phase-5 step 18).

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
- 2026-10-13/15: v0.4 run is 12 h (A17); run 4 was the evidence, the owner's 02:18–02:31 switch-off excluded (owner, phase-4 LOG).
- 2026-10-15: no 7-day soak — v1.0 = a 1 h functional run on the dev laptop (RULES.md v1.0, phase-5 PLAN A7); the homelab install follows v1.0 interactively (owner).
- 2026-10-14: a failed hard gate stops the loop only on its third attempt (`GATE_FAILED`); attempts 1–2 log, amend and continue (owner; loop/PROMPT.md §7).
- 2026-10-14: goal is ~100% poll success, not just the 95% gate; grid outages happen ~2×/day. Fix the approach, not only the timeout: study how community tools (bluetti_mqtt, bluetti-bt-lib, HA) keep BLE reliable. Rework freely, decide yourself, never block on the owner (owner).
- 2026-10-13: Q4 router on EB3A AC · Q7 poll 60 s · Q8 LAN only · M5 soak on the dev laptop, user-level systemd (owner).
