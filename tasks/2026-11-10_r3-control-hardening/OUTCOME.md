# Outcome

> Written when the task closes — done **or** blocked.

## Summary
The six MEDIUM findings R3 slice 1 left open (R3 PLAN A8–A13) are closed.
Commands now retry a failed key exchange once on a fresh connection, like polls,
and never send a write before the second handshake succeeds. The untested
collector command paths have tests, a shutdown reply maps to `503`, the
scheduler's `applied` map is pruned, and the control panel survives an
auto-refresh. The default `control.enabled = false` is unchanged; nothing was
deployed and no write frame was sent.

## Deliverables
- Commits:
  - `e5b6fa0` — `docs(docs): define the command handshake retry in ROADMAP §R3 (A9)`
  - `ab21b88` — `test(collector): cover the unreachable no-retry and deadline release paths of a command` (A8, A10)
  - `ee79981` — `fix(collector): retry a command's failed key exchange once on a fresh connection (A9)`
  - `312d8ed` — `fix(web): map a collector error during shutdown to 503, not 502 (A11)`
  - `88a1158` — `fix(control): prune the scheduler's applied map so a re-added entry starts fresh (A13)`
  - `b3968a3` — `docs(docs): log parallel group A (A11, A13)`
  - `bbe4b0c` — `fix(web): keep the control panel row and typed times through an auto-refresh (A12)`
  - `43182bb` — `fix(web): always swap #live and carry busy or typed control rows (A1, A2, A3)` —
    A1 test, A2 carry, A3 input-only carry
  - `20cca40` — `docs(docs): record close-hold 1 of the R3 control hardening task (A4-A6)`
  - `98ba912` — `test(web): tolerate an early server close in the raw-request helper; settle close-hold 1 (A4-A6)` —
    A4 `BrokenPipe`-tolerant test helper, A5 this list, A6 H20 checks
- Files: `src/collector/open.rs` (new, shared handshake retry), `src/collector/mod.rs`,
  `src/collector/control.rs`, `src/collector/tests.rs`, `src/collector/tests/control.rs`,
  `src/web/control.rs`, `src/web/control/tests.rs`, `src/web/pages/tests.rs`,
  `src/web/tests.rs` (A4), `src/control/schedule.rs`, `src/control/schedule/tests.rs`,
  `assets/app.js`, `assets/control.js`, ROADMAP.md §R3, `HUMAN-CHECKLIST.md` H20.
- Milestone tag: none (ad hoc task).

## Pending / Follow-up
- [ ] H20 (owner): the live refresh race, manual check in `HUMAN-CHECKLIST.md`.
- [ ] Not deployed: the homelab runs v1.4.0. These fixes ship with the next
      `make release` + `make deploy`, on the owner's go.
- [ ] Next task: `2026-11-07_elite-read-stalls` (todo).

## Amendments
- A1 — fixed in `43182bb`: `a_success_newer_than_the_stored_last_due_is_not_resent`
  (`src/control/schedule/tests.rs`); red under the `last_due.is_none()` mutant (3 sends, not 2).
- A2 — fixed in `43182bb` by the finding's first option: `#live` always swaps,
  and a busy (disabled) or focused `.ctl` row is moved into the fresh tree and keeps
  its focus. No freeze, no false STALE, typed times kept. A first try (a hold bounded
  to `stale_s / 2`) was rejected by the reviewer: at an auto-refresh of 5m or more it
  disabled the A12 guard.
- A3 — raised by the second review pass on the A2 fix (a focused button carried its row on
  every refresh); fixed in `43182bb`: only a focused input carries a row.
- A4 — close-hold 1 (CRITICAL, predates the task, `b3fa70e`): the 3 MiB login test flaked on
  `BrokenPipe` when the server sent 413 and closed mid-body. Fixed: the raw-request helper in
  `src/web/tests.rs` accepts `BrokenPipe`/`ConnectionReset` and keeps the reply that arrived.
  Red at HEAD under 16-way load (4 of 160), 0 of 160 after; the lib suite 50 runs, 0 failures.
- A5 — close-hold 1: Deliverables and Files above completed.
- A6 — close-hold 1: H20 now also checks that the banner and cards update while a time input
  holds focus (A2), and that a toggle left focused does not freeze its row (A3).

## Lessons / Notes
- Page-weight budgets (`app.js` 12000 B, `control.js` 2000 B) bind on small JS fixes:
  shorten comments, never raise the budget.
- The repo has no JS runtime, so JS guards are pinned by string tests plus a manual
  H-row. The reviewer reading the JS logic found the one real defect (A2).

<!-- Calibration proposals for the owner (AGENTS.md §8b): -->
- plan-reviewer · missed: an unbounded UI hold written into a step's design (step 6
  specified "skip the `#live` swap while a time input has focus" with no time limit) —
  the step reviewer found it as A2 only after it was built.
