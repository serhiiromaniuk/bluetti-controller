# Outcome

> Written when the task closes — done **or** blocked.

## Summary
R3 slice 1 is in code: each station's AC and DC output can be switched from the
web control panel, the CLI `set` and a daily `on`/`off` schedule run by `serve`.
Every path goes through one gate (`control::Command::new`: `control.enabled`
plus the per-model allow-list in `switches()`); the default stays off, and no
write frame was sent to a station. Hardware validation is H14–H17 in
`HUMAN-CHECKLIST.md`; every switch stays `verified: false` until then.

## Deliverables
- Commits:
  - `0c7ef3b` — docs(docs): record the R3 slice 1 control + schedule spec (ADR-010)
  - `20e8d1d` — feat(protocol): add the AC/DC output write allow-list
  - `5e3a960` — feat(protocol): add the 0x06 write frame, control token rule and schedule table
  - `a9b12d2` — feat(transport): add the output write path and the control gate
  - `99f8271` — feat(collector): run output switch requests between poll rounds
  - `55b8742` — feat(cli): add set and the web output control route behind the gate
  - `eec2e89` — feat(web): add the schedule routes behind the token
  - `a9f2abc` — feat(web): add the per-station output control panel
  - `3ec4329` — feat(control): add the pure scheduler due-slot arithmetic
  - `af7f3dc` — feat(control): run the daily schedule in serve when control is enabled
  - `24533c2` — docs(docs): document R3 output control and add hardware checks H14-H17
  - close commit — docs(docs): close R3 output control + schedule task (A3/A4/A7/A14/A15 fixes)
- Files: `src/protocol/control.rs`, `src/protocol/modbus.rs` (0x06),
  `src/transport/write.rs`, `src/control/{mod,schedule}.rs`,
  `src/collector/control.rs`, `src/cli/set.rs`, `src/web/{control,schedule}.rs`,
  `src/web/pages.rs` + `templates/status.html` (panel),
  `migrations/002_schedule.sql`, README "Control", ROADMAP ADR-010 / §R3.
- Milestone tag: none (ad-hoc task; no release cut, nothing deployed).
- Done-when → tests: LOG.md step 14 entry. Coverage: `src/protocol/` 98.82%.

## Pending / Follow-up
- [ ] `2026-11-10_r3-control-hardening` (todo): A8–A13, before the owner enables control.
- [ ] H14–H17 (owner, `HUMAN-CHECKLIST.md`): first real write per station, UI toggle, a schedule firing.
- [ ] Elite 200 V2 joins `switches()` only after H16 confirms 2011/2012 (A2).
- [ ] Close-review M1: README "Missed runs" and ROADMAP §R3 say a new entry applies its latest due
      state at once; after a delete + re-add in the same service run it fires only from the next
      slot until A13 lands (`src/control/schedule.rs:161-169`). A13's fix makes the docs true.
- [ ] No release / deploy yet: `make release` + `make deploy` on the owner's go.

## Amendments
- A1 — fixed in a9f2abc (bearer sent only when the token field exists and is set).
- A2 — fixed in 20e8d1d (`switches(Elite200V2)` is empty until H16).
- A3 — fixed in the close commit (BUG file: the abort count is unknown, the journal was not read).
- A4 — fixed in the close commit (BUG file: each run length names its start point; ~46 h since the 10-23 self-recovery).
- A5 — fixed in 0c7ef3b (ROADMAP §R3) and 24533c2 (README "off 23:00 saved at 23:30" example).
- A6 — fixed in a9f2abc (rows per `switches(model)`).
- A7 — fixed in the close commit (doc says the gate holds by convention; steps 8, 9, 13b reviews checked every call site).
- A8, A9, A10, A11, A12, A13 — re-targeted → `2026-11-10_r3-control-hardening`.
- A14 — fixed in the close commit (README example `set router dc on`).
- A15 — fixed in the close commit (README: latest slot only; cap per service run).

## Lessons / Notes
- A typed gate (`Command` with private fields) proves less than its doc claimed
  while `set_output` and `switches()` stay public (A7). A `set_output(&Command)`
  signature would make it hold by type; left for a later slice.
- Q3's "catch up" has a sharp edge the owner should know: a single past-due
  time saved in the UI fires within a minute (A5, README).
- Calibration proposal: driver-side, not a grader — the A4 close fix relabelled all three run
  lengths "not a power-on", which was false for the 76 h run; the batch reviewer caught it.
  No grader miss or false finding this task.
