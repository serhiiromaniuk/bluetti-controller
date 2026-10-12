# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns).

## Steps 1–4 ✓ 2026-10-13 — Foundation drafted in the planning session

**Changed:** ROADMAP.md (spec + research + milestones), RULES.md, AGENTS.md,
CLAUDE.md, loop/, tasks/, .claude/agents + commands, Makefile, .gitignore,
src/protocol/{modbus,device}.rs.

**Check:** `cargo test` — 11 passed, 0 failed (modbus: CRC vectors 0xCDC5 /
0x0A84, request layout, parse, bad CRC, exception, truncated reassembly;
device: BLE names, encryption flag, block coverage, EB3A decode, SOC plausibility).
`make check` not run: `rustfmt` and `clippy` are not installed on the dev host (Q2).

**reviewer:** not run — initial scaffold commit made in the planning session.

**Decision:** ROADMAP.md §3 ADR-001…008 (local BLE in Rust, community handshake
port, no APK reverse-engineering, stack, storage, UI, keys outside repo, host power).
No CI/CD (owner, 2026-10-13).
**Carry-forward:** see loop/STATE.md.
**Next:** step 5 — `make check` green (needs Q2).
