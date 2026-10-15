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

## Step 6 ✓ 2026-10-15 — status view: banner, station cards, forecast, paused_until (A1 applied)

**Changed:** `src/web/status.rs` — `status(conn, now, &AppState) -> Status { house: Banner, stations: Vec<Card>, paused_until }`, all `Serialize`; `src/web/status/tests.rs` (new, 12 tests).
- A1: staleness first (> 3 intervals or no sample → `unknown`), then open station event → `absent`; banner: open house event → `outage` since its start, else `house::vote`; `power` since the newest closed house `ended_at`.
- Forecast: last 2 h strictly; "lasts until HH:MM" / "if the power goes now: N.N h" / "—"; `unverified` unless every SOC/power field is verified.
- Health: `station_health` + 24 h sample/failure counts + newest failure stage, inline SQL.

**Check:** red first — `status()` stub returned an error, 0/12 passed. Green 12/12. Hand-computed: EB3A 268.8 Wh × 50 % × 0.9 / 100 W = 4355 s from 09:00Z → "lasts until 13:12" (Kyiv).
**Parallel:** branch `p5-step6`, `loop/merge-gate.sh p5-step6 p5-step7 p5-step8` → MERGE GATE OK (make check green on the merged tree), cherry-picked as 0e37f2a.

**reviewer (batched, steps 6–8):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM.
- MEDIUM M2: "lasts until HH:MM" drops the date for > 24 h runtimes → A5 (step 10).

**Decision:** banner `outage` with `since: null` when the vote says outage but no house event is open.
**Carry-forward:** none.

## Step 7 ✓ 2026-10-15 — history view: 7-day energy table and outage log

**Changed:** `src/web/history.rs` — `energy(conn, days, now, interval_s, tz) -> Vec<EnergyDay>` (newest day first, today partial), `outages(conn, n, now)`, `outages_between(conn, from, to, now)` → `Vec<Outage>`; `src/web/history/tests.rs` (new, 6 tests).
- Extra args beyond the step text: `interval_s` (gap rule of `energy::totals`), `now` (ongoing duration).
- House hours without power returned in seconds; template converts.

**Check:** red first against `todo!()` stubs, 0/6; green 6/6. Hand-computed (Kyiv, D0 = 1_782_853_200 = 2026-07-01 00:00 local): 1 h at 100 W in / 50 W out, SOC 50→55 on 500 Wh → 100 / 50 / Δ25 / losses 25 Wh; local-midnight split 2 Wh + 1 Wh (UTC would give 3 Wh on one day); gap → missing 540 s; open outage clipped → 3600 s; 21 events → newest 20.
**Parallel:** cherry-picked as b369238 after the merge gate.

**reviewer:** batched (above); `missing_s` counting only inter-sample gaps is the §7.4 integrate rule, not a defect.
**Carry-forward:** none.

## Step 8 ✓ 2026-10-15 — assets: uPlot 1.6.32 vendored, app.js/app.css, /assets route

**Changed:** `assets/{uPlot.iife.min.js,uPlot.min.css,uPlot.LICENSE,app.js,app.css}`; `src/web/assets.rs` static table + `Cache-Control: max-age=86400`, unknown → 404; `src/web/tests.rs` (`raw_request` helper, asset test, `/assets/app.js` out of the 501 list); `README.md` Credits line.
- Source: `raw.githubusercontent.com/leeoniya/uPlot/1.6.32/…`; tag `1.6.32` = commit `e995b061`; LICENSE MIT (Leon Sorokin).
- `wc -c assets/*`: app.css 2094 · app.js 4203 · uPlot.iife.min.js 51081 · uPlot.LICENSE 1078 · uPlot.min.css 1857 (spec §8.4 i sizes match, budgets held).
- app.js DOM contract (step 10 follows it): `[data-chart-station]`, `a[data-range]`, `form#pause` with `minutes` [+ `token`], `output#pause-reply`.

**Check:** red first — `uPlot.iife.min.js` answered 501. Green: `cargo test --lib web::tests::every_asset` → `every_asset_is_served_with_its_type_and_bytes ... ok` (status, type, cache header, exact bytes, 404). `cargo test --lib web::` 21 passed. `grep -nE 'https?://' assets/app.*` → none. `grep -c uPlot README.md` → 2.

**reviewer:** batched (above).
- MEDIUM M1: `cargo test web::assets` runs 0 tests (test lives in `web::tests`) → addressed here: the real filter and its pass line are the evidence above.
- MEDIUM M3: nothing parses `Metric::key()` names yet; stub test uses `metric=soc` → A6 (step 9).

**Decision:** none.
**Carry-forward:** none.
**Next:** parallel group B, steps 9–12 (JSON API, status page, SSE, pause).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:33:08Z

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

## Step 9 ✓ 2026-10-15 — JSON API: status, series, outages, energy

**Changed:** `src/web/api.rs` (180 lines): four handlers, JSON error type, `spawn_blocking` read helper, hand-parsed query (every bad input → JSON 400); `src/web/api/tests.rs` (new, 9 tests); `src/protocol/device.rs` `Metric::ALL` + `Metric::from_key` (A6) with round-trip test.
- Read failure → 500 `{"error":"database read failed"}`; `/api/energy` default `min(7, cap)` → spec line deferred to A8.
- Unknown station detected inside the read, still 400.

**Check:** red first — `0 passed; 9 failed` (stubs 501). Green: `cargo test --lib web::api` 9 passed (raw ≤ 48 h incl. exactly 48 h, `rollup_5m` > 48 h, `metric=battery_soc` + `step=120` average, 12 bad-input cases, outages window/defaults, energy default/cap/30-day cap).
**Parallel:** branch `grpB-api`, cherry-picked as 0a84c78 after the merge gate.

**reviewer:** batched (group B, below this group's last entry).
**Carry-forward:** none.

## Step 10 ✓ 2026-10-15 — status page: banner, cards, outage log, energy tables, charts

**Changed:** `src/web/pages.rs` (248): `GET /` with `?range=24h|7d`, `render(conn, now, state, week)` over `status::status`, `history::energy(7)`, `history::outages(20)`; `templates/{base,status}.html`; `src/web/pages/tests.rs` (new, 3 tests); `src/web/status.rs` A5 fix.
- A5 format "lasts until Sat 17 Oct 10:39" (weekday alone repeats after 7 days; low-load Elite can exceed it).
- Banner `power/outage` → CSS classes `present/absent` (what `app.css` defines); `app.{js,css}` untouched.

**Check:** red first — `/` 501; A5 test `left: "lasts until 10:39"`. Green: `cargo test --lib web::` 25 passed on branch. Hand-computed at 2026-10-15 12:00 Kyiv: banner "Grid power since Thu 15 Oct 10:00", SOC "50 %", "if the power goes now: 1.2 h", ongoing outage 10 min above closed 30 min, router energy row, unverified badge, token field only with a token (value never in HTML).
- `grep -nE 'TODO|placeholder|lorem|https?://' templates/*` → none. Fixture `/` = 4 454 B (budget 25 000 B, asserted).
**Parallel:** branch `grpB-pages`, cherry-picked as 77c262c.

**reviewer:** batched (below). **Carry-forward:** none.

## Step 11 ✓ 2026-10-15 — SSE /events: named events, lag skip, ends on shutdown

**Changed:** `src/web/sse.rs` (77): `futures_util::stream::unfold` over `broadcast::Receiver` + `watch` shutdown, `Lagged` → skip, `Closed`/shutdown → end, keep-alive 15 s; payload via `serde_json::json!` with column keys (`Update` not `Serialize`, store untouched); `src/web/sse/tests.rs` (new, 3 tests, own `start` keeping the `Arc<AppState>`).

**Check:** red first — `0 passed; 3 failed` (501). Green ×3: `3 passed`. Stop with an open client after shutdown: ~0.2 ms (2 s bound); mutation: shutdown signal disabled → test fails at the 2 s timeout. Keep-alive not tested (15 s wait).
**Parallel:** branch `grpB-sse`, cherry-picked as ca63187.

**reviewer:** batched (below). **Carry-forward:** none.

## Step 12 ✓ 2026-10-15 — pause POST: 0..=60 min, bearer token, never replaces Shutdown

**Changed:** `src/web/pause.rs` (79): hand-parsed `minutes`, XOR-fold constant-time token compare (length first), `send_if_modified` keeps `Shutdown` → 503 `{"error":"shutting down"}` (A2), reply `{"paused_until"}`; order token 401 → minutes 400 → shutdown 503; `src/web/pause/tests.rs` (new, 5 tests); `src/collector/tests.rs` +14 (`a_pause_sent_while_running_skips_polls_until_it_ends`: 90 s pause at t=70 → opens [0, 60, 180, 240]); `status::paused_until` → `pub(crate)`.

**Check:** red first — 5 pause tests `501`. Green: `cargo test --lib web::` 26, `collector::` 25 passed on branch. `grep -rnE '0x06|0x10|write_single|write_multiple' src/web src/collector` → none.
**Parallel:** branch `grpB-pause`, cherry-picked as ec58752.

**Group B:** prep ddd4a8a/ad98405 (shared `pub(super)` test helpers, 501 test → `/telemetry` only, rustfmt). `loop/merge-gate.sh grpB-api grpB-pages grpB-sse grpB-pause` → MERGE GATE OK (protocol 99.82 %, metrics 99.4 %).
**reviewer:** batched over `@{u}..HEAD` — INTENT satisfied ×4, no creep; 0 CRITICAL / 0 HIGH / 2 MEDIUM. app.js ↔ markup, `/api/series` body and pause form verified consistent.
- MEDIUM M1: 500 on read failure + energy default `min(7, cap)` not in §8.4 f → A8 (step 15).
- MEDIUM M2: after owner A7 (1 h run) ROADMAP §10.3/§12 M5 and INDEX still say 7-day soak → A9 (step 19).
- A2, A5, A6 → fixed.
**Decision:** none. **Carry-forward:** none.
**Next:** step 13, telemetry page (A4).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:45:24Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13 ✓ 2026-10-15 — /telemetry: every stored metric, verified flag, 24 h SVG sparkline, no JS

**Changed:** `src/web/telemetry.rs` (stub → ~145): per station the newest sample's core columns + every recent `sample_extra` key, value `{:.1} unit`, local time, flag from `Field::verified` (unknown model/key → unverified); `window(now)` bucket-aligned start → ≤ 48 × 30-min buckets via `read::series`; pure `sparkline(t, v, from, to)` → `polyline` points in a 120×30 box (flat/single → mid-height); `templates/telemetry.html` (overrides the scripts block: no JS); `assets/app.css` +2 `.spark` rules (2094 → 2261 B).
**Shared:** `pages::local` and new `status::model_named` → `pub(crate)` (status `verified` reuses it).
**A4:** `store::read::latest_extras` → one `GROUP BY metric` with `max(ts)` bare column, bounded to `ts ≥ newest extra − 86400`; new test `latest_extras_skip_keys_older_than_a_day_before_the_newest`; the old "key missing from the newest poll" test still passes. Same 3 keys as the old query on a copy of `data/acceptance-24h.db`.
**Tests:** `web/telemetry/tests.rs` (4): hand-computed 3-point sparkline `0.0,30.0 60.0,0.0 120.0,15.0`; flat + single + empty; a 30 h minute series at the worst alignment → exactly 48 points; `/telemetry` lists `battery_soc 50.0 %`, `ac_input_voltage 230.0 V`, extra `dc_input_voltage 12.5 V` with the `unverified` badge, a `<polyline`, no NULL column, no `<script`. `every_unbuilt_route_answers_501` → `healthz_is_null_before_the_first_poll` (no stub route left).

**Check:** red first — telemetry `0 passed; 4 failed`, `latest_extras_skip…` FAILED. Green: `cargo test --lib web::` 46 passed, `store::read` 10 passed; `make check` green after `make format`.

**reviewer:** risky, before commit — INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM M1: flag assertion not tied to the extra's row; verified branch never runs → A10 (step 15).
- A4 → fixed.
**Decision:** none. **Carry-forward:** none.
**Next:** step 14, UI smoke + page weight + LAN latency.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T11:51:20Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 14 ✓ 2026-10-15 — UI smoke: page weight 65.7 KB, max page 5.9 ms on the LAN, SSE + pause live

**Pre-flight (§10.4):** `bluetoothctl show` Powered: yes; `rfkill` hci0 not blocked; `control.enabled = false`; app force-quit not observable from here (H9 covers the app). `make build` → release in 18.7 s.
**Run:** scratch `data/ui-smoke.toml` (gitignored, absolute `storage.path`) on `data/ui-smoke.db` = `sqlite3 .backup` of `data/acceptance-24h.db`; `systemd-run --user --unit=bluetti-ui-smoke … serve`, 11:52:21–11:57:44 UTC (5.4 min), stopped by `systemctl --user stop` — "SIGTERM, shutting down" → "stopped" in 1.8 s (open `/events` closed).
**Routes** (`curl` to 192.168.0.179:8080, status bytes): `/` 200 6324 · `/?range=7d` 200 6324 · `/telemetry` 200 14132 · `/healthz` 200 35 · `/api/status` 200 931 · `/api/series` router soc 24h 200 13640, living-room 7d 200 2605 · `/api/outages` 200 838 · `/api/energy` 200 3663 · `/assets/nope.js` 404.
**Assets:** app.css 2261 · app.js 4203 · uPlot.iife.min.js 51081 · uPlot.min.css 1857.
- **Page weight = `/` + 4 assets = 65 726 B < 100 KB** ✓.
**Latency:** 20 GETs of `/` to the LAN IP: min 3.2 ms, median 3.7 ms, **max 5.9 ms ≤ 1 s** ✓. One each: `/telemetry` 14.8 ms, `/api/*` ≤ 3.0 ms.
**SSE:** `/events` delivered `poll_failure` (station 2, discover) and `sample` (stations 1, 3) with `:` keep-alives.
**Pause:** POST `minutes=2` at 1792065249 → 200 `{"paused_until":1792065369}`, journal `web::pause: polling control set from the web control=Pause {…}`; DB has no `sample`/`poll_failure` row in (1792065221, 1792065381) — two cycles skipped. POST `minutes=0` → 200 `null`, journal `control=Run`, next sample ts 1792065381. `minutes=61` → 400.
- No write frame: journal `grep -ciE 'write|0x06|0x10'` → 0.
**Assets offline:** `grep -rnE 'https?://' templates assets/app.*` → none (exit 1).
**Stations:** living-room (Elite) failed discover every cycle (out of range or off; not this step's gate); mobile one connect failure after resume ("GATT services have not been resolved"), router fine.
**H9** added to `HUMAN-CHECKLIST.md` (phone layout, charts, pause with the app, page time); `grep -c '^| H9 '` → 1.
**Gate flake:** first `step-done.sh` gate RED, `279 passed; 1 failed` (lib), name not kept; not reproduced in 4 `make check` + 160 test-binary runs → A11 (step 15).
**reviewer:** batched (low-risk; `.unreviewed` 1 after this commit).
**Decision:** none. **Carry-forward:** none.
**Next:** step 15, M4 close-out + boundary greps (A8, A10).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:02:04Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

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

## Step 15 ✓ 2026-10-15 — M4 close-out: ROADMAP/README/config docs, boundary greps clean, A8/A10/A11 fixed

**Docs:** ROADMAP §12 M4 ticked (page weight 65.7 KB; phone look → H9), §5.2 `web` row M4 ✓, §8.4 f (A8: bad parameter 400, failed read 500, energy default `min(7, cap)`). README: status line, "Web UI and API" section (pages, route table). `config.example.toml`: `web.listen` (LAN only, GET unauthenticated) and `web.token` (unset = POST open; bearer when set) comments.
**Greps (done-when):**
- `bluer::|use bluer` → `src/transport/bluer/gatt.rs` 5, `scan.rs` 6, `src/transport/mod.rs:23-24` (`pub use self::bluer::…` re-exports) ✓.
- `env::var|from_default_env|from_env` → only `src/main.rs:38` (`EnvFilter::try_from_default_env`, the documented `RUST_LOG` exception) ✓.
- `0x06|0x10|write_single|write_multiple` in `src/web src/collector` → none (exit 1) ✓.
- `reqwest|hyper::client|https?://` in `src/web templates` → one hit, `src/web/pages/tests.rs:108`, the test asserting the page has no `http(s)://`; no client, no URL in production code ✓.
**A10:** pure `telemetry::is_verified(fields, metric)`; new `the_flag_follows_the_matching_field` (a verified field → true; unverified, absent, `None` → false); page test asserts the badge inside the `dc_input_voltage` row. Mutation `&& !f.verified` → both tests FAILED; restored → 5 passed.
**A11:** stress, lib test binary 400 runs × 12-way: 1 failure, `collector::tests::stations_are_polled_in_turn_and_every_poll_disconnects`, `left: [0, 1, 60, 60, 120, 120]`. Cause: `Clock::now` takes `max(unix_now(), anchor + virtual elapsed)`; under paused time the real second can tick between two stations of one round (or between `Clock::new` and the first poll → `[0, 0, 59, …]`). Fix: the collector's `unix_now()` returns a frozen constant under `cfg(test)` (integration `tests/collector.rs` keeps the real clock). `collector::` × 3000: old binary 2 failed (both shapes), new 0.
**Amendments:** A8/A10/A11 → fixed. A3 (step 16/18), A7 (steps 17–22), A9 (step 19) stay open: they target later steps, so `make amendments` → 3 open (A3 A7 A9), none due by step 15.
**Check:** `make check` green, coverage 99.82 % / 99.4 %.
**reviewer:** risky, before commit; batch `@{u}..HEAD` (step 14) + step 15 — INTENT satisfied both; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM M1: `api.rs:152` doc comment "default 7" vs `min(7, cap)` → A12 (step 16).
- Deviations (A3/A7/A9 open, test-only net-grep hit, test code for A10/A11) judged sound.
**Decision:** none. **Carry-forward:** none.
**Next:** parallel group C (steps 16–17; A7 declines 17).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:08:31Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

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

## Step 16 ✓ 2026-10-15 — deploy unit, INSTALL.md, `make dist` tarball; A3/A12 fixed

**Changed:** `deploy/bluetti-controller.service` (§10.3 + `systemd-inhibit --what=sleep:idle:handle-lid-switch --mode=block` wrapper, `Restart=on-failure`, `RestartSec=10`). `deploy/INSTALL.md`: build/unpack, binary/config/data dirs, absolute paths (A3), unit + `systemctl --user enable --now`, `loginctl enable-linger`, logs, upgrade, uninstall, §10.2 pointer; no hourly probe timer (A7). `Makefile` `dist: build` → `dist/bluetti-controller-$(git describe --tags --always --dirty)-x86_64-linux.tar.gz`. `.gitignore` `/dist/`. `src/web/api.rs:152` comment → default `min(7, cap)` (A12).
**Check:**
- `systemd-analyze --user verify deploy/bluetti-controller.service` → exit 0 (only an unrelated `/usr/lib/systemd/user/spice-vdagent.service` warning).
- `make build` → release in 11.9 s; `make dist` first run put `config.example.toml` at the tarball root (transform `^.*/` needs a slash) → transform `^\(.*/\)\?` → `tar tzf`: 4 entries, all under `bluetti-controller-v0.4-collector-22-g3ab9066-dirty-x86_64-linux/` (binary, `.service`, `INSTALL.md`, `config.example.toml`).
- `make check` green.
**reviewer:** risky, before commit — INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM. It ran `systemd-inhibit … sh -c 'exit 3'` under `systemd-run --user --wait` → lock granted, exit=3 propagates (`Restart=on-failure` holds); `--config` is global.
- MEDIUM M1: INSTALL.md §2–§3 bare names fail from the repo root → A13 (step 19).
**Decision:** none. **Carry-forward:** none (CF-6 stays for step 18).
**Next:** step 18 — install the user unit (1 h run, A7).

## Step 17 ✓ 2026-10-15 — (declined — A7): no soak, no `scripts/soak_probe.sh`; step 21 measures the 1 h run directly

**reviewer:** n/a (no change). **Next:** step 18.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:12:10Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

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

## Step 18 ✓ 2026-10-15 — user unit installed and running; Linger=yes (CF-6 discharged); 1 h run started 12:13:12 UTC

**Scope (A7):** install + start a 1 h run, not a 7-day soak; no probe timer, no `soak.log` (step 17 declined), ETA = start + 3600.
**Changed (host, user-level only):**
- `loginctl enable-linger` → exit 0, `loginctl show-user "$USER" -p Linger` → `Linger=yes`.
- From `dist/bluetti-controller-v0.4-collector-23-gd900065-x86_64-linux.tar.gz` (unpacked in a temp dir, INSTALL.md §2–§3): binary `~/.local/bin/bluetti-controller`; unit `~/.config/systemd/user/bluetti-controller.service`; `~/.config/bluetti-controller/config.toml` (600) = local `config.toml` with absolute `storage.path = /home/serhiiromaniuk/.local/share/bluetti-controller/data.db` (fresh) and `keys_file` → `~/.config/bluetti-controller/keys.toml` (600); `web.listen = 0.0.0.0:8080`, no token, `control.enabled = false`.
- `systemctl --user daemon-reload && systemctl --user enable --now bluetti-controller`.
**Repo:** `HUMAN-CHECKLIST.md` H10 (1 h run: lid open, AC from the Elite, no restart, note interventions); `loop/STATE.md` CF-6 struck.
**Pre-flight (§10.4):** `bluetoothctl show` Powered: yes; `rfkill` hci0 not blocked; `control.enabled = false`; no other bluetti unit or process; app force-quit and firmware are the owner's (H9/H10).
**Check:**
- `is-active` → `active`; `is-enabled` → `enabled`; `NRestarts=0`, `InvocationID=71d0b93f…`, `ActiveEnterTimestamp` 15:13:12 EEST = **start 1792066392**, **ETA 1792069992** (13:13:12 UTC).
- `systemd-inhibit --list` → `bluetti-controller … 1387543 systemd-inhibit sleep:idle:handle-lid-switch monitoring block` (all three `what`s, no polkit refusal under linger).
- Journal: `serving listen=0.0.0.0:8080 stations=3 interval_s=60`.
- At start + 180 s (read-only `sqlite3 mode=ro`): router 3, living-room 3, mobile 3 samples; 0 `poll_failure` rows.
- `curl http://192.168.0.179:8080/healthz` (LAN) → `200`, 1.2 ms.
- Service PID = child of `MainPID` (`pgrep -P`), `readlink /proc/<pid>/exe` → `~/.local/bin/bluetti-controller`; `VmHWM` 11472 kB, `VmRSS` 11072 kB.
- Probe timer: n/a (A7). From here to step 21: no station command, no reinstall.
**reviewer:** risky, before commit — INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM. It confirmed read-only: installed unit byte-identical to `deploy/`, binary sha256 = tarball's, DB fresh, 4 samples/station at ~+190 s, perms 600/700, nothing forbidden staged.
- MEDIUM M1: H10 "measured from 13:13:12" reads as a later start → A14 (step 19).
**Next:** step 19 (docs, A9/A13) during the run; step 21 measures at ≥ ETA.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:17:55Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

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

## Step 19 ✓ 2026-10-15 — M5 docs: ROADMAP §10.3/§10.5/§12 M5 match the shipped unit and the 1 h run; README Deploy; A9/A13/A14 fixed

**Scope (A7):** no soak runbook, no `soak_probe.sh` lines; the 1 h run replaces them (A9). No station access, no reinstall; the unit kept running.
**Changed (docs only):**
- ROADMAP §10.3: unit block = `deploy/bluetti-controller.service` minus comments (inhibit-wrapped `ExecStart`); polkit granted the lock under linger (step 18); "Soak" bullet → 1 h run (polls, RSS, LAN latency, gate per RULES.md v1.0).
- ROADMAP §10.5: runbook rows for the user unit (status/logs, linger, inhibit lock).
- ROADMAP §12 header, M5 (unit, build/dist, install ticked with step 16/18 pointers; "Human:" labels → done by the loop under the standing delegation; run + OUTCOME open; acceptance = RULES.md v1.0), §14 Q2.
- README: `## Deploy` → `deploy/INSTALL.md`, §10.2 human-only.
- `deploy/INSTALL.md` §1: `make dist`, §2–§3 run inside the unpacked directory (A13).
- `HUMAN-CHECKLIST.md` H10: "runs 12:13:12–13:13:12 UTC (measured at step 21 after 13:13:12)" (A14).
- `tasks/INDEX.md` phase-5 label: "1 h run" (A9). BRIEF stays static.
**Check:** `diff` of the ROADMAP unit block vs `deploy/` (comments stripped) → identical; `grep -i soak` leaves only the "replaces the 7-day soak" notes; `make loop-hygiene` OK; `git diff --stat` → docs only (6 files).
**reviewer:** batched (low-risk), before the push. **Next:** step 20 (declined — A7), then step 21 at ≥ 13:13:12 UTC.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:19:31Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

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

## Step 20 ✓ 2026-10-15 — (declined — A7): no day-1 checkpoint; the 1 h run is measured once at step 21

**Changed:** none in code or docs; PLAN step 20 ticked as declined (A7: no 7-day soak, so no day-1 RSS projection).
**Check (read-only, 12:20:03 UTC):** `systemctl --user show bluetti-controller` → `ActiveState=active`, `NRestarts=0`, `InvocationID=71d0b93f…` (same as step 18).
**reviewer:** batched (low-risk), before the push.
**Next:** step 21 — measure the 1 h run at ≥ 13:13:12 UTC (ETA 1792069992).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:20:24Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 21 · interim 2026-10-15 12:21 UTC — 1 h run not over; waiting on ETA 1792069992 (13:13:12 UTC)

**Scope:** step 21's "now < ETA" branch (A7: ETA = start 1792066392 + 3600). Not an escape-hatch attempt; step not ticked; INDEX stays `in-progress`; no OUTCOME.
**Interim probe (read-only, 12:21:01 UTC, start + 469 s):**
- `systemctl --user show` → `ActiveState=active`, `NRestarts=0`, `InvocationID=71d0b93f…` (unchanged since step 18), `MainPID=1387543`.
- Service PID (`pgrep -P` MainPID) exe → `~/.local/bin/bluetti-controller`; `VmHWM` 11624 kB, `VmRSS` 11624 kB.
- 20 LAN GETs of `http://192.168.0.179:8080/` → 20× `200`, max 2.98 ms, median 2.22 ms; `/healthz` → `200`, 0.75 ms.
**Repo:** `loop/STATE.md` `Blocked?` → waiting on ETA 1792069992.
**reviewer:** batched (low-risk), with steps 19–20, before the push.
**Next:** relaunch after 13:13:12 UTC; step 21 measures the full hour (interventions, max `VmHWM`, page and API times, `acceptance_v04.py`, grid events).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T12:21:28Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
