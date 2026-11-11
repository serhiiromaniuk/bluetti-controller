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
