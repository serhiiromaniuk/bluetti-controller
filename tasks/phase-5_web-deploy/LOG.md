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

## 2026-10-15 — Task opened
- Folder opened from `tasks/_template/`; BRIEF written from ROADMAP.md §12 M4/M5, RULES.md v1.0 gate, STATE CF-6, phase-4 OUTCOME follow-ups.
- Unit install + soak start are inside the standing delegation (user-level `systemctl --user`); the homelab system unit stays out (§10.2, human-only).
- No PLAN yet (`needs_plan`): the `planner` writes it next iteration, then the `plan-reviewer`.
- reviewer: batched (doc-only BRIEF; reviewed with the PLAN batch before the push).
- Next: PLAN.md via planner + plan-reviewer.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T10:43:07Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## 2026-10-15 — PLAN written + plan-review
- `planner` wrote PLAN.md: 22 steps (M4 1–15, M5 16–22), groups A (6–8), B (9–12), C (16–17); Q1–Q7 all `delegated 2026-10-15`, none asked.
- `plan-reviewer`: 0 CRITICAL, 1 HIGH, 5 MEDIUM, 4 LOW. All fixed in the PLAN before step 1 (steps not yet frozen):
  - HIGH P1 fixed — inhibit wrapper is `sleep:idle:handle-lid-switch` (logind `LidSwitchIgnoreInhibited=yes` ignores `sleep` locks on lid close); step 18 checks the listed lock; H10 says lid open, on AC.
  - P2 fixed — hourly transient user timer appends probe lines to `soak.log` (Q5, step 18); steps 20/21 read it.
  - P3 fixed — soak waits commit `--no-tick`, INDEX stays in-progress, no OUTCOME, not an escape-hatch attempt.
  - P4 fixed — energy table adds Δstored and losses (estimate, spec §7.4); step 7 asserts losses.
  - P5 fixed — tz helpers in `src/metrics/localtime.rs` at step 2; 6/7 only call them.
  - P6 fixed — `app.{js,css}` moved to step 8; step 10 keeps templates + `pages.rs`; groups A/B file-disjoint.
  - LOW P7–P10 fixed — uPlot release and sizes named in step 1 i; env grep covers `from_default_env` (main.rs `RUST_LOG` exception); `paused_until` wall-clock conversion in 1 h; step 16 worktree reuses `target/`.
- reviewer: batched (doc-only plan; reviewed with the BRIEF commit before the push).
- Next: step 1, M4/M5 design record (risky).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T10:57:12Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-10-15 — M4/M5 design record in ROADMAP.md

**Changed:**
- ROADMAP.md §3 — ADR-009 `jiff` + host tzdb (Q3); §4.9 jiff 0.2.38 row.
- ROADMAP.md §8.4 — items a–k (module layout, grid state + poll health, forecast, energy table, outage log, API shapes, SSE, pause, assets, front end, telemetry).
- ROADMAP.md §9 — phase-5 note: token unset = open POST (Q1), tz fail-fast, no `~` expansion.
- ROADMAP.md §10.3 — M5 design (l): inhibit-wrapped unit, install paths, `make build`/`make dist`, soak + intervention/RSS/page-time definitions (Q5, Q6).

**Check:** §8/§9/§10.3 hold a–l, each a recommendation; §3 has ADR-009; `make loop-hygiene` OK.
- Facts checked: uPlot tag `1.6.32` = `e995b061`; unpkg `uPlot.iife.min.js` 51 081 B, `uPlot.min.css` 1 857 B, MIT; page weight 87 938 B < 100 000 B; `cargo info jiff` 0.2.38 Unlicense OR MIT; `man systemd-inhibit`: returns the program's exit status.

**reviewer:** 0 CRITICAL / 2 HIGH / 3 MEDIUM (risky step + the BRIEF/PLAN batch `@{u}..HEAD`)
- HIGH H1: `/api/energy` `1..=31` passes raw retention and contradicts step 9 → fixed: `1..=min(30, raw_retention_days)`, beyond → 400 (§8.4 f); step 9 text matches.
- HIGH H2: step 17/Q6 read `MainPID` = `systemd-inhibit`, so RSS gate measured the wrapper → fixed in PLAN before the freeze: Q6 + step 17 read the `bluetti-controller` child; step 17 check asserts `/proc/<pid>/exe`.
- MEDIUM M1: open station event before staleness (writer keeps it through `Unknown`) → A1 (step 6).
- MEDIUM M2: pause POST can overwrite `Control::Shutdown` → A2 (step 12).
- MEDIUM M3: `~` path in §10.3 install → A3 (steps 16/18).

**Decision:** M4/M5 design = ROADMAP.md §8.4, §10.3 "M5 design", ADR-009.
**Carry-forward:** none.
**Next:** step 2 — config (tz, token comment) + `src/metrics/localtime.rs`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:03:34Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-10-15 — config tz + token semantics, `metrics::localtime`

**Changed:**
- `Cargo.toml`/`Cargo.lock` — `jiff = "0.2.38"` via unlocked `cargo build --all-targets`. Lock: +109; the 2 `-` lines are `"bitflags"` → `"bitflags 2.13.2"` renames (bitflags 1.3.2 now also listed, via non-Linux `defmt`); on Linux jiff builds only `jiff-core`.
- `src/config.rs` — `general.timezone: jiff::tz::TimeZone` via `timezone_from_str` (host tzdb, ADR-009); empty/blank `web.token` rejected; doc: unset = POST open (Q1). 376 lines, tests stay inline.
- `src/metrics/localtime.rs` (new, pure) — `local_day_bounds(ts, tz) -> Result<(start, end)>`, `local_hhmm(ts, tz) -> Result<String>`; out-of-range ts is an error.

**Check:** `cargo test metrics::localtime` failed first (5/5 `todo!()` panics), then 5/5 ok; `cargo test config::` failed first (E0599 `iana_name` on `String`), then 19/19 ok.
- Hand-computed (zdump confirms Kyiv 2026 DST Mar 29 / Oct 25): Mar 29 day `[1774735200, 1774818000)` = 23 h; Oct 25 day `[1792875600, 1792965600)` = 25 h; 1767225600 → `02:00`, 1782909240 → `15:34`.
- Fail-fast smoke: `Mars/Base` → `general.timezone: unknown time zone \`Mars/Base\` (failed to find time zone …)` at line 10.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM — INTENT satisfied; lock renames accepted as insertions-only.

**Decision:** none.
**Carry-forward:** none.
**Next:** step 3 — store read API (`src/store/read.rs`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:07:39Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-cdfeb076974a97ea)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-10-15 — store read API (`src/store/read.rs`)

**Changed:**
- `src/store/read.rs` (new, 273 lines) — `stations`, `latest_sample`, `samples` (`[from,to)` + one before), `last_samples`, `latest_extras`, `events_between`, `last_events`, `open_event` (full `GridEventRow`), `series` (§8.4 f: raw ≤ `RAW_MAX_S` = 48 h, else `rollup_5m` for soc + 4 powers, extras/`ac_in_v` stay raw; `step` ≥ 1 buckets). `CORE` maps the 6 columns to `Metric`.
- `src/store/writer.rs` — `last_samples`/`open_event` moved out; writer maps `GridEventRow` to `(id, started_at)`.
- `src/store/rollup.rs` — private `load` replaced by `read::samples` (same SQL; now also carries `ac_in_v`, which `energy::rollup` ignores).
- `src/store/schema.rs` — `migrate` is `pub(crate)`; `store::migrate` re-export is `#[cfg(test)]` (Q4).
- `stations()` added beyond the step list: the status view needs `capacity_wh` from the DB (§8.4 c), and groups A/B must not edit `read.rs`.

**Check:** `cargo test --lib store::read` 9/9 ok; `cargo test --lib store::` 40/40 ok (writer/rollup tests unchanged but for imports).
- Tests were written with the code (not red first). Mutation: `>` → `>=` on the 48 h switch → `series_switches_to_the_rollup_past_48_hours` FAILED, restored.
- Hand-computed: step 120 over soc 40/50/70/80 → buckets `[0,120)` 45, `[120,240)` 75; an event ending at `from` does not overlap.

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM. `stations()` not counted as creep.
- MEDIUM M1: `latest_extras` does a full scan through a correlated subquery → deferred to A4 (step 13, where `/telemetry` uses it).

**Decision:** none.
**Carry-forward:** none.
**Next:** step 4 — the writer publishes `Update`s on a `broadcast` channel.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:13:17Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-cdfeb076974a97ea)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-10-15 — the writer publishes `Update`s on a `broadcast` channel

**Changed:**
- `src/store/writer.rs` — `pub enum Update { Sample { station, ts, core }, GridEvent { scope, started_at, ended_at }, PollFailure { station, ts, stage } }`; `Writer::restore` takes `broadcast::Sender<Update>`. Each is sent after its row's insert/update returns (§8.4 g); `core` = the sample's `read::CORE` metrics only; a snapshot publishes nothing; `send` errors (no receiver) ignored.
- `src/store/mod.rs` — re-exports `Update`.
- `src/cli/serve.rs` — creates `broadcast::channel(UPDATES = 64)` and hands it to the writer (step 5 gives `AppState` its clone).
- Call sites (`writer/tests.rs`, `writer/tests/jobs.rs`, `rollup/tests.rs`, `tests/collector.rs`) pass a sender with a dropped receiver (`quiet()`), so every existing writer test also proves "no receiver never fails a record".
- `src/store/writer/tests/updates.rs` (new) — the two step tests, split out to keep `tests.rs` at 381 lines.

**Check:** red first — 18 compile errors (`Update`/`broadcast` missing, restore arity). Then `cargo test --lib store::` 42/42 ok.
- `publishes_one_update_per_committed_row`: hand-computed sequence, debounce 2: ON 0/60, OFF 120/180 → at 180 `Sample` + `GridEvent a [120,–)` + `house [120,–)`; ON 240/300 → at 300 closes both at 240; failure 360 → one `PollFailure`; snapshot → none; extra `pack_voltage` not in `core`; `rx` dropped → record 420 stored (7 samples).
- `a_failed_insert_publishes_nothing`: temp triggers abort `sample` and `poll_failure` inserts → `handle` errs, `try_recv` empty.
- First run of the new test failed on my own count (expected 8 samples, real 7: 360 is a failure) — test arithmetic fixed, not code.

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM.
- Note (not a finding): `Update` omits `encrypted` and the failure `error` text, per the step text; §8.4 g says "the row as JSON" — step 11 settles the SSE payload.

**Decision:** none.
**Carry-forward:** none.
**Next:** step 5 — web state and skeleton (`AppState`, route stubs, askama).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:17:53Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-cdfeb076974a97ea)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 5 ✓ 2026-10-15 — web state, every route registered as a 501 stub, askama + futures-util locked

**Changed:**
- `src/web/state.rs` (new) — `AppState { reader, control, updates, interval_s, usable_factor, raw_retention_days, tz, token, shutdown: watch::Sender<bool> }` behind one `Arc` (a `watch::Sender` is not `Clone`); no `Debug` (token). `AppState::new(reader, &Config, control, updates)`; `#[cfg(test)] for_tests(reader)` returns a plain `AppState` so tests can set fields.
- `raw_retention_days` added beyond the step's field list: §8.4 f caps `/api/energy?days=` with it, and group B must not edit `state.rs`.
- `src/web/mod.rs` — `router(Arc<AppState>)` registers `/`, `/telemetry`, `/assets/{name}`, the four `/api/*` GETs, `POST /api/polling/pause`, `/events`, `/healthz`; `healthz` reads `state.reader`.
- Stubs: `status.rs`, `history.rs` (doc only, view models); `assets.rs`, `api.rs`, `pages.rs`, `sse.rs`, `pause.rs` and `telemetry.rs` (handler → `501`). `telemetry.rs` is beyond the list: step 13's route needs a home, and creating it now keeps step 13 out of `mod.rs`.
- `src/cli/serve.rs` — builds the `Control` channel before the router, `AppState::new(open_reader, config, control, updates)`, the writer gets `updates.clone()`. On exit: `state.shutdown.send_replace(true)` before `Control::Shutdown` (§8.4 g). `send_replace` because `send` does not store with no receiver.
- `Cargo.toml` — `askama = "0.16.1"`, `futures-util = "0.3.34"` (already in the tree through axum; step 11 needs `Stream` combinators for `Sse`). `Cargo.lock` +79/−0.
- `src/web/tests.rs` — `request(addr, method, path)` + `start(state)` helpers for groups A/B; `healthz` test moved to `start`.

**Check:** `every_unbuilt_route_answers_501`: 9 routes → `501`, `/healthz` → `200` with `last_poll_age_s: null`. Red first not possible (the old `router` signature does not compile against it). Mutation: `/events` route removed → test FAILED at the 501 assert (404), restored. `cargo test --lib web::` 2/2 ok.
- New crates' licences (`cargo metadata`): askama*, basic-toml, glob, rustc-hash are all MIT OR Apache-2.0.

**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM. It accepted `telemetry.rs` and `raw_retention_days` as in scope, not creep.
- MEDIUM M1: a pause POST during shutdown can overwrite `Control::Shutdown` and hang `polling.await`. The control sender now lives in `AppState` (`src/cli/serve.rs:116`). This is A2 (step 12: `send_if_modified` never replaces `Shutdown`, with a test), so no new amendment. No live bug while `pause.rs` is a stub.

**Decision:** `AppState` is shared as `Arc<AppState>`; shutdown is a separate `watch::Sender<bool>` that `/events` subscribes to.
**Carry-forward:** none.
**Next:** parallel group A, steps 6–8 (status view, history view, assets).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:22:35Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
