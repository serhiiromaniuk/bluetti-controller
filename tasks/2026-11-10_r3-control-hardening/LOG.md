# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes. `loop/where.sh`
> computes step position from `PLAN.md` and the last result from this file's newest
> `## ` heading — so restating an entry in a pointer file is prose every later
> iteration pays to re-read.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns). Bullets, not
> narrative. Cite a spec/doc section, never re-quote it. Record the decision and
> the evidence, not the reasoning that produced them. The shape below is the shape.

## Plan ✓ 2026-11-11 — PLAN.md written (6 steps) and audited

**Changed:** `PLAN.md` — 6 steps: A9 spec (1), A8+A10 tests (2), A9 code (3), group A = A11 (4) + A13 (5), A12 (6).

**Check:** `planner` wrote it; `plan-reviewer` audited it before step 1.

**plan-reviewer:** 0 CRITICAL / 2 HIGH / 6 MEDIUM / 4 LOW — all fixed in the plan (step 1 not started):
- HIGH P1: `Behave::Stall` stalls polls and the echo too → step 2 adds a read-back-only stall variant in `src/collector/tests.rs`.
- HIGH P2: `refresh()` replaces `#live` whole, so pending set alone cannot keep the row disabled or typed times → step 6 also guards the swap in `assets/app.js`; H20 matches what is built.
- MEDIUM P3/P7: step 3 test uses a pause so the command is the first open; two failure-path tests added.
- MEDIUM P4/P5: step 5 sequence gets the pruning tick; residual window restated.
- MEDIUM P6: step 4 names the test-helper change (`send_replace`).
- MEDIUM P8: void (P2 fixed by building; step 6 stays sequential).
- LOW P9–P12: Q1 delegated answer recorded, §4.2 cost sentence, `spec §R3` cite, paths fixed.

**Decision:** A9 — commands retry a failed key exchange once like polls (PLAN Q1, delegated 2026-11-11).
**Carry-forward:** none
**Next:** step 1 — A9 spec bullet in ROADMAP §R3.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:33:30Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-11-11 — A9 spec: commands retry a failed key exchange once, like polls

**Changed:** `ROADMAP.md` §R3 "Slice 1 runtime rules", *Serialisation* bullet — defines "the station's normal retry": one retry on a fresh connection after a failed key exchange (release by address, reopen; phase-4 A23 part 2), none past `UNREACHABLE_AFTER` (3), one poll deadline bounds both, no 0x06 frame before a handshake succeeds; cost sentence (§4.2 "No write retry").

**Check:** spec-only step — the bullet states the rule and why it is safe (golden rules #2, #5); `make check` green (below). Code follows in step 3.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM — INTENT satisfied; checked against `src/collector/mod.rs:31,268-287`, `control.rs:74-110`, `schedule.rs:255-259`.
- Cosmetic: one over-long line → reflowed before the commit.

**Decision:** none new (PLAN Q1).
**Carry-forward:** none
**Next:** step 2 — A8 + A10 collector command tests.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:37:12Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-11-11 — A8 + A10: collector command tests

**Changed:** tests only, production code untouched.
- `src/collector/tests.rs` — `Behave::StallReadBack`: reads 0, echoes the 0x06, then a read on the same link never returns (`FakeGatt::wrote`, so the next round reads normally).
- `src/collector/tests/control.rs` — (a) `a_station_past_unreachable_after_gets_one_open_without_retry`: misses at 0/60/120 s, command at 130 s opens with `retry == false`, no write. (b) `a_request_between_rounds_writes_once_and_reads_back` asserts `retry == true`. (c) `a_deadline_mid_write_releases_by_address`: `Err` names the 60 s poll deadline, one write, `releases == 1`, `live == None`, opens at 0/10/70 s (the overdue round runs at once).

**Check (red via temporary mutant, reverted; `git diff` shows no `control.rs`):**
- `control.rs:74` `<` → `>=`: (a) and (b) FAILED, 5 passed.
- `control.rs:80-82` release commented out: (c) FAILED, 6 passed.
- Restored: 7/7 `collector::tests::control` green; `make check` green (below, after `make format`).

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM; re-ran both mutants (red, file restored). Note: the release mutant trips the fake's "two links open at once" panic before `releases == 1` — still red.

**Decision:** none · **Carry-forward:** none
**Next:** step 3 — A9 code: command handshake retry via a shared `Collector` method in `src/collector/open.rs`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:43:02Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-11-11 — A9 code: commands retry a failed key exchange once

**Changed:**
- `src/collector/open.rs` (new) — `Collector::open`: the retry block moved out of `poll` unchanged (release by address, `cached` reset, `retrying` kept for A30); a failed retry sets `progress.stage = Handshake` and names both errors.
- `src/collector/mod.rs` — `poll` calls `self.open` and maps to its `Failure` (423 lines, was 438).
- `src/collector/control.rs` — `write` calls `self.open` in place of `connect.open`.
- `src/collector/tests/control.rs` — helper `command_after_pause` (round 0 paused, command at 10 s) + 3 tests: retry then one write (opens `[(a,true,10),(a,true,10)]`, 1 release); both handshakes fail (both errors, 2 opens, no write); deadline cuts the retry (`Err` names the 60 s deadline, 2 releases, no write).

**Check:**
- Red first: all 3 new tests `Err("handshake failed")` before the refactor (7 passed, 3 failed).
- After: `collector::` 41/41 green; `src/collector/tests/retry.rs` unchanged (`git diff --quiet`); `make check` green (below).

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM; confirmed poll stage/error text, A30 `retrying` order, `cached` reset, no 0x06 before a handshake succeeds, 41/41 re-run.
- Note: a command cut by the deadline mid-retry names only the deadline, not the first handshake error (as the step asks).

**Decision:** none · **Carry-forward:** none
**Next:** step 4 — group A (A11 web 503, A13 scheduler prune), two worktrees.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:46:47Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-11-11 — A11: a collector error during shutdown is 503

**Changed** (`312d8ed`, worktree branch `ga-a11`):
- `src/web/control.rs` — guarded arm before `Ok(Some(Err(_)))`: when `*state.control.borrow() == Control::Shutdown`, answer `503 "shutting down"`; other errors stay `502`.
- `src/web/control/tests.rs` — `run()` clones `state.control` into the fake; `Fake::ShutDown` calls `send_replace(Control::Shutdown)`, then answers `Err("shutting down")`; test `a_shutdown_while_waiting_is_503`.

**Check:**
- Red first: `a_shutdown_while_waiting_is_503` FAILED with `HTTP/1.1 502 Bad Gateway` (6 passed, 1 failed).
- After: `web::control::tests` 7/7 (incl. `a_collector_error_is_502`); `make check` green in the worktree.

**reviewer:** batched with step 5 · INTENT satisfied · 0 CRITICAL / 0 HIGH; the shutdown race can report a real write failure as 503 — accepted, the collector logs the real error (no finding).

**Decision:** none · **Carry-forward:** none

## Step 5 ✓ 2026-11-11 — A13: the scheduler prunes its applied map

**Changed** (`88a1158`, worktree branch `ga-a13`, landed by cherry-pick):
- `src/control/schedule.rs` — `tick` retains an `applied` key only while its row is in the read and the row's `last_due` is `None` or older than the key's slot.
- `src/control/schedule/tests.rs` — `a_deleted_and_re_added_entry_does_not_inherit_the_old_slot`: `off 23:00` sent, `last_due` stored, a tick prunes, re-added as `on 08:00`, next tick sends `on`.

**Check:**
- Red first: `left: [("router", Ac, false)] right: [("router", Ac, false), ("router", Ac, true)]`.
- After: `control::schedule` 15/15 (writer-lag test stays green); `make check` green in the worktree.
- `loop/merge-gate.sh ga-a11 ga-a13`: MERGE GATE OK (green together).

**reviewer:** batched with step 4 · INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM; no double-send path; accepted re-add window can only under-send. MEDIUM (untested `last_due < slot` keep branch) deferred to amendment A1.

**Decision:** none · **Carry-forward:** none
**Next:** step 6 — A12 control panel refresh race (`assets/control.js`, `assets/app.js`, H20).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T13:56:39Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 6 ✓ 2026-11-11 — A12: the control panel survives an auto-refresh

**Changed:**
- `assets/app.js` — `refresh()` keeps `#live` while `.ctl:disabled, .ctl input:focus` matches (a row mid-request or a focused time); `#tables` still swaps; `rendered` is reset only with `#live` (`#age` lives in it).
- `assets/control.js` — `pending` set keyed `station/output`, a click on a pending row returns; the reply, error, label and re-enable go to `row()`, the fieldset re-found in `#live` by `data-station`/`data-output` (falls back to the clicked one); the key is deleted in the final `.then`.
- `src/web/pages/tests.rs` — `control_panel_has_one_row_per_allowed_output` asserts the pending guard, the key clear, the re-find selector and the `refresh()` skip guard.
- `HUMAN-CHECKLIST.md` — row H20 (the live race, manual).

**Check:**
- Red first: the extended test against the old assets panicked at `src/web/pages/tests.rs:360` (1 failed).
- First `make check` red: `app.js: 12199 B > 12000 B`, then `control.js: 2177 B > 2000 B` (page-weight budget). Fixed the work, not the budget: comments shortened, `row()`/`say()` folded; now 12000 B and 2000 B.
- `node --check` passes on both files (node is on the host; the repo still has no JS test runtime).
- After: `make check` green.

**reviewer:** risky, before the commit · INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM; the race windows (click during an in-flight refresh, double sends, stuck rows on a non-JSON reply) are clean. MEDIUM (an unbounded focus hold freezes `#live` and shows a false STALE) deferred to amendment A2.

**Decision:** none · **Carry-forward:** none
**Next:** close the task (`make amendments`: A1, A2 open).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T14:05:08Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Close fixes ✓ 2026-11-11 — A1 test, A2 `#live` always swaps

**Changed:**
- `src/control/schedule/tests.rs` — `a_success_newer_than_the_stored_last_due_is_not_resent` (A1).
- `assets/app.js` — `refresh()` always swaps `#live`. A disabled or focused `.ctl` row is moved into the fresh tree (matched by `data-station`/`data-output`), and focus is restored. `rendered` resets on every swap. Header comment and section dashes shortened: 11992 B, budget 12000 B.
- `src/web/pages/tests.rs` — pins the carry, the unconditional swap and the focus restore.
- PLAN A1, A2 disposed; `OUTCOME.md` drafted.

**Check:**
- A1: green, then red under the mutant `e.last_due.is_none()` (`left: [on, off, off]`, `right: [on, off]`); reverted.
- A2: the extended page test is red against HEAD's `app.js` (`(f.disabled || f.contains(a))` missing), green after. `node --check` passes. `make check` EXIT=0.

**reviewer:** pass 1 on the first A2 try (a hold bounded to `stale_s / 2` from the last swap) · INTENT satisfied · 0 CRITICAL / 1 HIGH / 1 MEDIUM.
- HIGH H1: at an auto-refresh of 5m or more, `now - rendered` is always above the bound, so the A12 guard never fires and typed times are wiped → fixed by replacing the hold with the carry (the finding's first option).
- MEDIUM M1: a bounded hold can still show a false STALE → fixed by the same change: `#live` is never held.
- Pass 2 (logic changed, carry fix only) · INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM: a focused button also carried its row, freezing it → PLAN A3, fixed in this commit (`a.tagName === "INPUT"`, pinned); `app.js` 12000 B.

**Decision:** none · **Carry-forward:** none
**Next:** close-reviewer on the task close.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T14:15:43Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Close ✗ 2026-11-11 — close-reviewer: hold (close-hold 1)

**close-reviewer:** VERDICT hold · 1 CRITICAL / 0 HIGH / 2 MEDIUM / 0 UNVERIFIED · TREE-CHANGED none.
- CRITICAL: `an_oversized_password_or_body_stays_bounded` flakes on `BrokenPipe` at `src/web/tests.rs:77` (3 of 25 lib runs); it predates the task, but done-when 5 needs a deterministic gate → PLAN A4.
- MEDIUM: OUTCOME Deliverables are stale (bounded hold, no A3, no `b3968a3`, Files incomplete) → PLAN A5.
- MEDIUM: H20 checks nothing for A2/A3 → PLAN A6.
- Passed: done-when 1–3 by mutants in scratch copies; A1 and A3 by mutants; A12/A2/A3 exercised in headless Chrome against a mocked `fetch` (all scenarios green); no stubs; no carry-forwards.

**Done-when:** nothing ticked.
**Next:** the next close fixes A4–A6 (each reviewed, `--no-tick`), then asks the close-reviewer again.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T14:25:57Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Close-hold 1 fixes ✓ 2026-11-11 — A4 flaky login test, A5 OUTCOME, A6 H20

**Changed:**
- `src/web/tests.rs` — `exchange` passes the body write and the read through `closed_early`, which accepts `BrokenPipe`/`ConnectionReset` and panics on any other error; the reply bytes read before a reset are kept (A4).
- `tasks/…/OUTCOME.md` — Deliverables name `b3968a3` and `43182bb` (A1 test, A2 carry, A3 input-only carry); Files completed; A4–A6 accounted (A5).
- `HUMAN-CHECKLIST.md` H20 — adds (a) banner and SOC cards update while a time input holds focus, (b) a toggle left focused does not freeze its row (A6).
- PLAN A4–A6 disposed; `make amendments`: 6 findings, all disposed.

**Check (A4):**
- `an_oversized_password_or_body_stays_bounded` alone, 60 runs: 0 failures with or without the fix (no load, no flake).
- 16 concurrent copies × 10 rounds: 4 of 160 red at HEAD (fix stashed), 0 of 160 with the fix.
- Full lib suite (`cargo test --lib` binary), 50 runs with the fix: 0 failures (470 s).
- `make amendments`: 6 findings, all disposed.

**reviewer:** batched pass over `@{u}..HEAD` + this diff · INTENT shortfall · 0 CRITICAL / 1 HIGH / 0 MEDIUM.
- HIGH: OUTCOME A4 claimed the 50-run result before the loop finished → fixed: committed only after the loop ended (50 runs, 0 failures, above), so the sentence is now observed.
- Passed: `closed_early` masks no real failure (status-checking callers still fail on an empty reply); `read_to_end` keeps partial bytes; OUTCOME matches `git log`; H20 has (a) and (b).

**Decision:** none · **Carry-forward:** none
**Next:** close-reviewer on the task close (close-hold 1 settled).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-11T14:37:34Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->
