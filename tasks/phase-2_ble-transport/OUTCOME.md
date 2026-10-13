# Outcome

> Written when the task closes — done **or** blocked.

## Summary
Closed as **superseded** by an owner scope change on 2026-10-13, after step 8 of
16. Step 8's `scan` showed the EB3A advertises encrypted (`BLUETTF`), like the
Elite 200 V2, so M1 cannot validate on a plaintext EB3A. The owner merged phases
2 and 3 (RULES.md phase table). Steps 9–16 and the encryption handshake continue
in `tasks/phase-2_ble-encryption/`.

## Deliverables
- `8655534` — `build(deps): add M1 dependencies at ROADMAP §4.9 versions`
- `5a36ee5` — `feat(config): load and validate config; add exception-aware frame length (CF-3)`
- `acc568f` — `build(loop): gate src/protocol/ line coverage with cargo llvm-cov in make check`
- `7b01906` — `feat(transport): add Gatt trait, fixture model and ReplayGatt`
- `602ab17` — `feat(transport): add read_block and read_profile with bounded retry`
- `c338b49` — `feat(transport): add MAC/SN anonymiser, RecordingGatt and MAC-shape secret scan`
- `1f2809b` — `feat(transport): add bluer scan adapter, scan CLI and make build`
- Milestone tag: none (`v0.2-ble` dropped; the merged phase tags `v0.3-encryption`).

## Pending / Follow-up
- [ ] Steps 9–16 of this PLAN → `tasks/phase-2_ble-encryption/`.

## Amendments
- A1–A8 — re-targeted → `phase-2_ble-encryption` (owner scope change 2026-10-13).

## Lessons / Notes
- The planner assumed a plaintext EB3A from the community docs; the real
  advertisement said otherwise. Read the advertisement before planning around a
  station's encryption state.
- Owner questions surfaced one phase at a time and stopped the loop for ~7 h.
  Fixed by RULES.md "Standing owner delegation" and `HUMAN-CHECKLIST.md`.
