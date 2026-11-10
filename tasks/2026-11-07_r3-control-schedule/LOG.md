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

## 2026-11-10 — Task open
Context loaded per `loop/where.sh --json` (no in-progress row; R3 first per STATE.md).

**Changed:** `tasks/INDEX.md` — R3 row `todo` → `in-progress`; this LOG created.

**Check:** BRIEF + PLAN already committed in 171edda, audited by the plan-reviewer
(findings applied) and the batch reviewer (H1 applied, A1/A2 open). No new plan.

**reviewer:** batch pass over a650183 + this commit — 0 CRITICAL / 0 HIGH / 2 MEDIUM
- MEDIUM M1/M2 (a650183, BUG-station-ble-invisible.md:97,100) → deferred to PLAN A3/A4.
**Decision:** none · **Carry-forward:** none
**Next:** step 1 — spec: record the R3 slice in ROADMAP.md.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:32:41Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-11-10 — R3 slice 1 spec recorded in ROADMAP.md (ADR-010)

**Changed:** `ROADMAP.md` — ADR-010 in §3; R3 slice 1 in §2.2; 0x06 write frame, encrypted-write
assumption and no-write-retry rule in §4.2; write allow-list table in §4.3 (Elite pair documented,
out of `switches()` until H16, per A2); `schedule` table (migration 002, SCHEMA_VERSION 2, rollback
via §10.6 DB backup) in §6; four routes + error codes + control panel in §8.2 (schedule edits need
`web.token` set, per the 2026-11-07 plan amendment; bearer sent only when the field exists, per A1);
§9 control/token rule; §12 R3 slice boxes + runtime rules (gate, serialisation, CLI, scheduler,
missed runs, retry cap, editing). Each section ends with a recommendation.

**Check:** `loop/where.sh --context` reads back all 7 cited sections (2.2, 4.2, 4.3, 6, 8.2, 9, R3)
with the new text (9 matches of ADR-010 / slice markers). Doc-only: no code changed.

**reviewer:** 0 CRITICAL / 1 HIGH / 2 MEDIUM · INTENT: satisfied (Elite exclusion and the added
`502` confirmed against code)
- HIGH H1: missed-run bullet ("never drops a run") contradicted the 10-failure cap; "polling paused"
  would count toward it → fixed in §R3: the cap applies within one service run (a restart resets it),
  a `"polling paused"` answer is not a failure and waits for the pause end.
- MEDIUM M1: a new entry's past-due single time fires at once → deferred to PLAN A5.
- MEDIUM M2: step 12's "both rows for every station" vs no Elite rows → deferred to PLAN A6.

**Decision:** control route answers `502` with the error when the write or read-back fails;
"polling paused" never counts toward the scheduler's retry cap.
**Carry-forward:** none
**Next:** step 2 — protocol allow-list, pure (`src/protocol/control.rs`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:39:52Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-11-10 — protocol write allow-list (`src/protocol/control.rs`)

**Changed:** new `src/protocol/control.rs` + `control/tests.rs`; `pub mod control` in
`src/protocol/mod.rs`. `Output { Ac, Dc }` with `key()`/`from_key`; `OutputSwitch` with private
fields and getters only; `switches(model)` returns EB3A 3007→48 / 3008→49, AC180 2011 / 2012,
Elite empty (spec §4.3 "Write allow-list", A2). All `verified: false`.
**Check:** failing first — `switches` stubbed empty for EB3A/AC180: `cargo test --lib
protocol::control` 2 failed (the two per-model register tests), 4 passed; then 6/6 green.
`make check` green, `src/protocol/` coverage 98.91%.
**Reviewer:** batched (low-risk per PLAN Risks "Parallel groups": step 2 not in the risky list).
**Decision:** none — the Elite's empty list follows spec §4.3 (step 1), which supersedes
step 2's "four register pairs per model" wording; A2 is satisfied for `switches()`.
**Carry-forward:** none
**Next:** parallel group A — steps 3 (write frame), 4 (config token rule), 5 (schedule table).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:42:04Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.91% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-11-10 — write frame, pure (`src/protocol/modbus.rs`)

**Changed:** `write_request(register, value) -> [u8; 8]` (0x06), `WRITE_RESPONSE_LEN = 8`,
`expected_write_len(prefix)` (5 on the exception flag, else 8), `parse_write_response(frame,
register, value)` and `FrameError::EchoMismatch`. The length/CRC/function check of
`parse_read_response` moved into a private `checked_body`, shared by both parsers.
**Check:** failing first — the 6 new tests in `modbus/tests.rs` did not compile (`cannot find
function write_request`). Hand-computed vectors (python CRC-16/MODBUS): EB3A AC on
`01 06 0B BF 00 01 7B CA`, off `… 00 00 BA 0A`, exception `01 86 02 C3 A1`. Then
`cargo test --lib protocol::modbus` 18/18 green, existing read tests unchanged.

## Step 4 ✓ 2026-11-10 — config: control needs a token (`src/config.rs`)

**Changed:** `Config::validate` fails `control.enabled = true requires web.token to be set`;
`config.example.toml` `[web]`/`[control]` comments state the rule and that control and the
schedule live in the web UI. No new keys.
**Check:** failing first — `config::tests::control_needs_token` FAILED (example with `enabled =
true` parsed); then 14/14 green (example parses; enabled + no token rejected naming both keys;
enabled + token parses).

## Step 5 ✓ 2026-11-10 — schedule table in the store

**Changed:** `migrations/002_schedule.sql` (spec §6 DDL verbatim), `SCHEMA_VERSION` = 2;
`Record::ScheduleSet/ScheduleDelete/ScheduleRun` handled in `Writer::handle` (station id checked,
no event update or §6 job: no sample time); `store::write::{set,delete,run}_schedule`;
`store::read::{ScheduleEntry, schedule_entries}` (an unknown output key fails the read).
`src/collector/tests.rs:252` helper arm became `_ => None` (forced by the new variants).
**Check:** failing first — `cargo test --lib store` did not compile (9 errors: no `ScheduleSet`,
`schedule_entries`, `ScheduleEntry`). Then 48/48 green: `writer/tests/schedule.rs` (set keeps
`last_due`, run sets it, edit keeps it, delete, run after delete no-op, per-output rows, CHECK
refuses no time, unknown station refused); `read/tests.rs` order + unknown key;
`schema/tests.rs` fresh db v2 with `schedule`, a v1 file migrates to v2 keeping its rows.

**Reviewer (group A + step 2, batched, `@{u}..HEAD` + working diff):** 0 CRITICAL / 0 HIGH / 0
MEDIUM · INTENT satisfied for steps 2, 3, 4, 5; CRCs re-derived independently. Note: `read.rs` is
408 lines (soft ~400); the 0..1439 minute range is validated at the web boundary (step 11).
**Decision:** none. **Carry-forward:** none.
**Next:** parallel group B — steps 6 (transport write path) and 7 (control gate), both risky.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:48:36Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 6 ✓ 2026-11-10 — transport write path (`src/transport/write.rs`)

**Changed:** `transport::write::{set_output, WriteError}` (+ `pub mod write`): one 0x06 frame, echo
checked with `parse_write_response`, then `read_block(read_back, 1)` under the normal retry policy;
`ReadBackMismatch { wanted, got }` when the register is not `u16::from(on)`.
`read.rs`: drain + write + reassemble extracted into `pub(super) exchange` (`read_once` calls it) so
the write path reuses it instead of a copy — the one file outside the step's list.
**Check:** failing first — 8/8 `transport::write` tests panicked (`unimplemented!`). Then 8/8 green:
EB3A AC on (echo split over two notifications, read-back 48 = 1; `eb3a_ac_on_fixture()` is
`pub(crate)` for step 9), AC180 DC off (2012 = 0), exception `01 86 02` and echo of another value
fail with no read-back, read-back 0 = mismatch, a lost read-back retried, a timed-out write not
resent (fixture exhausted), a stale notification drained. Existing `transport::read` tests green.

## Step 7 ✓ 2026-11-10 — the control gate (`src/control/mod.rs`)

**Changed:** `control::{Command, Refused, Reply, Request}` (+ `pub mod control` in `lib.rs`).
Private fields, one constructor: `Refused::Disabled` unless `control.enabled`, then
`Refused::NotAllowed { model, output }` when `switches(model)` lacks the output. No I/O.
**Check:** failing first — 3/3 `control::tests` panicked (`unimplemented!`). Then green: disabled
refuses all 3 models × AC/DC × on/off; enabled EB3A DC → 3008/49, AC180 AC → 2011; enabled Elite
refuses both outputs (A2: no switches until H16).

**Reviewer (group B, before the commit):** 0 CRITICAL / 0 HIGH / 1 MEDIUM · INTENT satisfied
for 6 and 7; the `read.rs` `exchange` extraction judged no-behaviour-change, not creep. M1 (`set_output`
and `switches()` are `pub`, so the gate holds by convention only) → deferred to amendment A7.
**Decision:** none. **Carry-forward:** none.
**Next:** step 8 — the collector executes commands between rounds.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T15:54:20Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 8 ✓ 2026-11-10 — the collector executes commands between rounds (`src/collector/control.rs`)

**Changed:** `Collector.commands: mpsc::Receiver<Request>`; `run(mut self)` loops its tick `select!`
over sleep / shutdown / `commands.recv()`, so a request runs only between rounds and one sent
during a round waits in the channel. `control.rs`: `execute` drops a closed-reply request before
any link, answers `"polling paused"` / `"shutting down"` without `connect.open`, else opens with
`retry` from the run loop's `failures` (passed as `&[u32]`, index resolved by name), runs
`with_disconnect { set_output(command.switch(), …) }` under the poll deadline and a shutdown
`select!`, releases when not disconnected, logs station/output/state/read-back (no MAC).
`serve` creates the channel (`COMMANDS = 8`), keeps the sender unused until step 10.
`tests/collector.rs` and the unit fake gained the field (dropped sender: no requests).
**Check:** failing first — 5/5 `collector::tests::control` panicked (`unimplemented!`). Then green:
between rounds one exact `01 06 0B BF 00 01 7B CA` + read-back true; during a 30 s stalled round
the request opens at t=30 after the round (fake panics on two links); unreachable answers
`connect failed`, no frame; paused answers `polling paused`, no open; closed reply, no open.
All 36 collector tests green.

**Reviewer (risky, before the commit):** 0 CRITICAL / 0 HIGH / 3 MEDIUM · INTENT satisfied; the
`failures: &[u32]`, "shutting down" answers, `run(mut self)` and `tests/collector.rs` field judged
justified, not creep. A7 holds at this call site (`command.switch()`). M1 (no unreachable-station
`retry == false` test) → A8; M2 (no handshake retry on the command path) → A9; M3 (release by address
on a cut command untested) → A10.
**Decision:** none. **Carry-forward:** none.
**Next:** group C — steps 9 (CLI `set`) and 10 (web control route).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:00:05Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 9 ✓ 2026-11-10 — CLI `set <station> ac|dc on|off` behind the gate (`src/cli/set.rs`)

**Changed:** `cli::Command::Set { station, output, state }` (clap parsers `parse_output` /
`parse_state`); `main.rs` dispatches it with `config.control` after `ctx` (`set needs --config`);
`set.rs`: `command()` = config station name only, then `control::Command::new` — both before
`session::run`; `apply` calls `set_output(command.switch(), …)` (A7 holds), prints the read-back.
`main.rs` module doc and `cli/mod.rs` doc name the gate instead of "only 0x03".
Deviation: `eb3a_ac_on_fixture()` is `cfg(test)` in the lib, unreachable from bin tests, so
`set/tests.rs` rebuilds the same 4 frames from public items (`write_request`, `read_request`, `crc16`).
**Check:** tests written with the code; `parses_station_output_and_state` failed first on a real bug
(clap: "Argument 'state' is positional … action is SetTrue" — a `bool` positional), fixed with
`action = Set`. Then 4/4 green: parse; MAC + unknown name refused; disabled → `Refused::Disabled`;
`apply` over `ReplayGatt` → `true`. Smoke (example config, no keys): `set router ac on` → `Error:
control is disabled` exit 1; MAC → `set takes a station name from the config`; no `--config` → exit 1.

## Step 10 ✓ 2026-11-10 — `POST /api/control/output` + `ac_output_on`/`dc_output_on` on the cards

**Changed:** `AppState` + `commands`, `control_config` (name: `control` is the pause watch sender),
`stations: Vec<Target>`; `serve` creates the command channel before the state and passes
`targets.clone()`. `web/control.rs`: 401 (also with no `web.token`) → 400 station/output/state →
gate (`Disabled` 403, `NotAllowed` 400) → 503 on `Shutdown` → send + reply under `timeout(interval/n
+ interval)`: 504 / 503 (channel gone) / 502 collector error / 200 read-back (§8.2). `pause.rs`:
`same`, `error` `pub(super)`; its test `post` helper moved to `web/tests.rs`. `status.rs`: one
`latest_extras` read per card; output states only from extras at the newest sample's ts.
**Check:** 7 route tests + 1 status test green; mutation (bearer check off, 403→200, output-state
ts filter off) → 3 FAILED (`…401…`, `…403…`, `output_states_come_from_the_newest_sample_only`),
restored. Steps 9+10 ran serially in one tree (disjoint files), no worktree.

**Reviewer (group C, risky, before the commit):** 0 CRITICAL / 0 HIGH / 1 MEDIUM · INTENT satisfied for 9
and 10; deviations (a)–(f) judged acceptable. A7 holds at both `set_output` callers (`cli/set.rs`,
`collector/control.rs`). M1 (late shutdown answers 502, not 503) → deferred to amendment A11.
Reviewer note for step 12: after a 504 the switch may still have happened; show the card's next state.
**Decision:** none. **Carry-forward:** none.
**Next:** step 11 — web schedule routes.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:09:30Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 11 ✓ 2026-11-10 — `GET/POST/DELETE /api/schedule` (`src/web/schedule.rs`)

**Changed:** `web/schedule.rs`: `list` (open, `api::read` → `schedule_entries`, ids → names via
`state.stations`, rows of unconfigured stations skipped, times `HH:MM`); `set` / `delete`: 403 with no
`web.token` (amendment H1) → 401 bearer → 400 station (config name only) / output (`switches(model)`) /
`on`·`off` `HH:MM` 00:00–23:59 (empty = unset) / at least one / `on ≠ off` → `Record::ScheduleSet` /
`ScheduleDelete` with the target id → `202` with the entry. Accepted while control is disabled.
`AppState.records`; `serve` creates the record channel before the state; route in `web/mod.rs`;
`api::read` `pub(super)`; `web/tests.rs` `post` → generic `send(method, auth)`.
Deviation: `records` is `mpsc::WeakSender<Record>`, not `Sender`: the sample counter task keeps the
`Arc<AppState>` alive for the process, so a strong sender would keep the writer from draining on
shutdown (`serve` awaits it after the collector drops its sender). Upgrade fails → `503`.
**Check:** 6 tests in `web/schedule/tests.rs` (403, 401, 12 POST + 2 DELETE 400 cases naming the
field, 202 with the exact `Record`s while disabled, GET from a temp db, `HH:MM` parse/format).
First run failed on the GET body's field order (struct order, not alphabetical) — expectation fixed.
Mutation (bearer check off, `on == off` check off) → 2 FAILED (`…401`, `…400_naming_it`), restored.
**Reviewer (risky, before the commit):** INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM; the
`WeakSender` deviation verified against `spawn_sample_counter` + `serve`'s shutdown order.
**Decision:** none. **Carry-forward:** none.
**Next:** step 12 — the per-station control panel on `/` (A1, A6 apply).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:16:19Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 12 ✓ 2026-11-10 — per-station control panel on `/`

**Changed:** `templates/status.html`: per card one `<fieldset class="ctl">` per output in
`switches(model)` (A6: none for the Elite) — legend, toggle (`data-state` on/off/—, `disabled`
when `control.enabled = false`), `on`/`off` `<input type="time">`, Save, Clear, `<output>`; one
note "control disabled — the schedule will not run" per card when disabled. `pages.rs`:
`CardView.outputs` (`OutputRow`), `Page.control`; rows from `state.stations` by name (config
station only), state from the card's `ac_output_on`/`dc_output_on`, times from
`schedule_entries`. `schedule::hh_mm` `pub(super)`. `app.css`: `.ctl` (3 lines).
Inline `<script>` (delegated click): toggle POSTs `/api/control/output` (opposite state; from
"—" it sends `on`), Save POSTs `/api/schedule`, Clear DELETEs; fieldset disabled while in
flight; reply inline; read-back sets the label; 502/504 set "—" (step-10 reviewer note);
bearer only when the pause form's token field exists and is non-empty (A1).
Deviations: (a) times rendered server-side from `schedule_entries` (the `GET /api/schedule`
source), not a JS fetch: keeps app.js' one-request load and survives the `#live` swap.
(b) JS inline, not in app.js: app.js was 11 985 / 12 000 B and the first try failed
`assets_stay_within_the_page_weight_budget` (14 265 B); §8.4 i says trim, never the cap. The
HTML share (≤ 18 000 B) holds it; base.html already inlines scripts. `pages.rs` 387 lines.
**Check:** 2 new tests (`control_panel_has_one_row_per_allowed_output`,
`disabled_control_disables_the_toggles_only`) red first (0 toggles rendered), then green.
Mutation (toggle `disabled` dropped; rows for EB3A switches on every station) → 1 and 2
FAILED, restored. JS: `node --check` only, no DOM run (no jsdom) — H17 (step 14) covers it.
**Reviewer (risky, before the commit):** INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM;
deviations (a)–(c) accepted. M1 (auto-refresh mid-request re-enables the row, loses the reply
and typed times) → deferred to amendment A12.
**Decision:** none. **Carry-forward:** none.
**Next:** step 13a — scheduler arithmetic, pure.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:24:01Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13a ✓ 2026-11-10 — scheduler arithmetic, pure

**Changed:** new `src/control/schedule.rs` (69 lines): `Due { station_id, output, on, due_ts }`,
`pub fn due(entries, now: &Zoned) -> Result<Vec<Due>, jiff::Error>` — per entry the latest
`on`/`off` occurrence ≤ now (today's, else yesterday's, compatible DST disambiguation), kept when
newer than `last_due` (`None` = never run). `src/control/mod.rs`: `pub mod schedule`.
Deviations: (a) named `Due` struct, not a tuple; `Result` because a minute ≥ 1440 errs
(`Time::checked_add`) instead of wrapping into a valid time; (b) `now` by reference.
**Check:** 9 tests in `src/control/schedule/tests.rs`, Kyiv instants hand-computed from
2026-01-01T00:00Z = 1767225600: due exactly at `off` (and not 1 s before); restart 30 min after a
missed `off` due once, not with `last_due` = that slot; restart 5 min after a recorded run not due;
both missed → only the latest; `off` moved earlier due at once; never run → yesterday's latest,
second entry already current skipped; DST gap 03:30 on 2026-03-29 → 04:30 EEST = 01:30Z (and
yesterday's slot at 01:00Z); minute 1440 errs; local-midnight constant cross-checked by jiff.
Red first against a stub returning empty: 6 FAILED. Mutations `due_ts > last` → `>=` (5 FAILED),
`today <= now` → `<` (1 FAILED), restored. `make format` then `make check` green.
**Reviewer (risky, before the commit):** INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM;
deviations (a)–(b) accepted, instants re-derived. Note for 13b: one `Err` aborts the whole
`due` call (migration 002 has no minute-range CHECK) — 13b logs it, never panics.
**Decision:** none. **Carry-forward:** none.
**Next:** step 13b — scheduler task + `serve` wiring.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:29:29Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13b ✓ 2026-11-10 — scheduler task + `serve` wiring

**Changed:** `src/control/schedule.rs` (282 lines): `Scheduler::new(reader, stations, control,
commands, records)` + `tick(&now) -> bool` + `pub async fn run(scheduler, tz, shutdown)` (60 s
`interval`, first tick immediate, `MissedTickBehavior::Delay`, stops on the shutdown watch or a gone
collector/writer). Per tick: `schedule_entries` in `spawn_blocking` on its own `open_reader`
connection, `last_due` overlaid with in-memory successes, `due`, then per slot `Command::new` →
request → on `Ok` `Record::ScheduleRun` + remember; on `Err` count per slot, cap `MAX_FAILURES = 10`
(one `warn!` at the cap); a `PAUSED` answer does not count. `src/control/mod.rs`: `pub const PAUSED`,
used by `src/collector/control.rs`. `src/cli/serve.rs`: built only when `control.enabled`, spawned
after the writer, awaited after the shutdown flag and before the collector/writer.
Deviations: (a) `Scheduler` struct + 3-arg `run`, not one 7-arg fn; tests drive `tick(&now)` with
Kyiv instants, since `now` is wall clock and paused tokio time does not move it; (b) `records` is a
`WeakSender` (writer drain unchanged); (c) `PAUSED` const; (d) a `Refused` logs every tick it occurs.
**Check:** 5 tests in `src/control/schedule/tests.rs` `mod task` (temp db, fake collector task):
due entry → 1 request + 1 `ScheduleRun`, no repeat next tick though the writer never stored it;
failed run retried next tick, newer `on` slot supersedes it; 3 pauses + 10 failures → 13 sends over
15 ticks; disabled → nothing; `run` sends at once and stops on shutdown. Red against a no-op `tick`:
4 FAILED (disabled passes trivially). Mutations (overlay removed; cap `>=` → `>`) → 2 FAILED,
restored. `make format`, `make check` green.
**Reviewer (risky, before the commit):** INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM;
deviations (a)–(d) accepted; A7 holds (scheduler only via `Command::new`). M1 (`applied` never
pruned: delete + re-add inherits the old slot) → deferred to amendment A13.
**Decision:** none. **Carry-forward:** none.
**Next:** step 14 — docs batch + close evidence.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:37:29Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
