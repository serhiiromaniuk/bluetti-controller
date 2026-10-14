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

## 2026-10-13 — Task opened
- Folder opened from `tasks/_template/`; BRIEF written from ROADMAP.md §12 M3, RULES.md v0.4 gate, STATE CF-5.
- No PLAN yet (`needs_plan`): the `planner` writes it next iteration, then the `plan-reviewer`.
- reviewer: batched (doc-only BRIEF; reviewed with the PLAN batch before the push).
- Next: PLAN.md via planner + plan-reviewer.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:05:43Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Plan ✓ 2026-10-13 — PLAN.md written (19 steps, Q1–Q5 delegated) and audited

**Changed:** `tasks/phase-4_collector/PLAN.md` — planner wrote 19 steps; group A = steps 5–7 (disjoint stub files from step 4).

**Check:** `plan-reviewer` audit — 0 CRITICAL · 2 HIGH · 6 MEDIUM · 3 LOW. Plan not started, so all fixed in the steps (no amendments):
- P1 HIGH per-round budget (absent station ~80 s > 60 s interval) → step 1 (j) deadline + arithmetic, step 10 timeout + "hanging station never delays others" test, Q1 cites it. fixed
- P2 HIGH `systemd-run --user` cwd is `$HOME`; inhibit vs polkit → `--working-directory="$PWD"` in Q2/14/15, inhibit fallback verified in step 14. fixed
- P3 smoke foreground 16 min > 10 min cap → step 14 runs as transient unit `bluetti-smoke`, rehearses step 15's launch. fixed
- P4 CF-5 placed after outage logic → step 1 runs `read living-room`/`read mobile` grid-on; step 14 re-confirms. fixed
- P5 `make lock` re-resolves all deps → unlocked `cargo build --all-targets` (steps 2, 13, Risks). fixed
- P6 in-step wait in step 18 → BLOCKED with ETA always. fixed
- P7 size budget per table → steps 3, 11 state bytes/day; step 16 prints per-table bytes. fixed
- P8 bluer grep matches doc comments → `grep -rnE 'bluer::|use bluer' src/`. fixed
- P9 `--min-pct`/`--max-mb` args; P10 step 18 ticks M3 24 h item; P11 step 10 reuses `src/cli/session.rs` core. fixed
- Reviewer verified: §9 config sections already in `src/config.rs`; `make test` gates `src/protocol/` only; `python3` present; H4 row exists; Q1–Q5 cross nothing human-only.

**Review:** plan-reviewer is this increment's grader (PROMPT §1 needs_plan); it also covered the unpushed BRIEF commit 2a8fc29 → push.

**Next:** step 1 — M3 design record (+ CF-5 reads).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:17:45Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-10-13 — M3 design record + CF-5 grid-on reads

**Changed:**
- `ROADMAP.md`: "M3 decisions" blocks — §5.4 stages + Q4 decode rule (b), per-round budget (j); §5.5 derived `unreachable` (c); §6 migrations (a), sample split (b), rollup/retention (d), restart (f), snapshots (h); §7.2 pure windowed functions, Q3 (g); §7.4 segment → later sample's bucket (e); §8.2 `/healthz` JSON (i — the API table is its home).
- `HUMAN-CHECKLIST.md` H5 (AC180 grid-on read, Q5) · `loop/STATE.md` CF-5 discharged.

**CF-5 (read-only `read`, `target/debug` binary, MACs filtered from output):**
- EB3A reg 77 = 219.0 V → grid on.
- Elite `living-room`: reg 1314 = 223.3 V, reg 1300 = 49.90 Hz → scale ×0.1 correct.
- AC180 `mobile`: 2.1 V / 0.00 Hz → not on grid → Q5 → H5.

**Budget arithmetic (j):** healthy `read` 4.97 s (EB3A) / 3.99 s (Elite). Silent station: cached 3 × (15 + 5 disconnect) + 2 + 4 = 66 s; uncached + advertising 20 + 66 = 86 s (uncached + silent = 20 s scan, then `discover` fails). Deadline 60 / 3 = 20 s; disconnect by address after every poll; unreachable → 1 attempt, no scan.

**Check:** items a–j each end in a recommendation; `make loop-hygiene` OK (STATE 35/40).

**Review (reviewer, one pass):** INTENT shortfall → fixed.
- H1 (j) sums two exclusive `find` paths, omits cleanup disconnect → rewritten as three paths, 66–86 s. fixed
- H2 deadline cancelling `connect`/`open_channel` orphans the link → disconnect by address after every poll. fixed
- M1 20 s deadline < 15 + 5 + 2 s, so a timed-out connect never retries → deferred to amendment A1 (step 10).
- M2 (b)/(h)/(i) lacked a recommendation → added (the step's own check). fixed

**Next:** step 2 — store schema (rusqlite, migration 001).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:22:29Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-91ff6e0c3af51e62)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-10-13 — store schema: rusqlite, migration 001, writer/reader open

**Changed:**
- `Cargo.toml`: `rusqlite = { version = "0.40.2", features = ["bundled"] }` (§4.9); `Cargo.lock` via unlocked `cargo build --all-targets`.
- `migrations/001_init.sql`: §6 SQL block verbatim (extracted by `awk` from ROADMAP.md).
- `src/store/{mod,schema}.rs`: `open_writer` (WAL checked, `synchronous=NORMAL`, `busy_timeout` 5000 ms, migrate by `user_version`, one transaction per migration, refuses a newer schema); `open_reader` (`SQLITE_OPEN_READ_ONLY`). `src/lib.rs`: `pub mod store`.
- `src/store/schema/tests.rs`: 4 temp-dir tests (7 tables + v1 + WAL/NORMAL; reopen no re-migration; reader refuses INSERT; newer schema refused).

**Check:** `cargo test --lib store::` — failed first with `.skip(0)` (re-applies 001): `reopening_runs_no_migration_twice` "SQL logic error"; then 4/4 pass. `make check` green.
- `Cargo.lock`: 117 insertions, 1 deletion — `"hashbrown"` → `"hashbrown 0.17.1"` (cargo disambiguation, second hashbrown via hashlink); no existing package version changed.

**Review (reviewer, one pass):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 0 MEDIUM.

**Next:** step 3 — store writes + extras.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:26:25Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-10-13 — store writes + extras

**Changed:**
- `src/store/write.rs`: `Stage`, `Sample`/`Failure`, `Sample::from_poll` (any problem → `decode` failure listing `<key> missing` / `<key> out of range: v`, Q4); `sync_stations` (upsert by name, `capacity_wh` from the profile, returns `name → id`, no delete); `insert_sample` (6 core columns to `sample`, the rest to `sample_extra`, one transaction); `insert_failure`, `insert_snapshot` (big-endian u16), `open_outage`/`close_outage` (closing a closed/unknown event errors); `station_health` → `Ok | Unreachable { since }` = failures after the newest sample ≥ 3, `since` = the first (§5.5, step 1 c).
- `Model::name()` in `src/protocol/device.rs`; `src/cli/scan.rs` reuses it (duplicate match removed).
- Test `TempDb` moved to `src/store/temp_db.rs` (cfg(test)), shared by schema and write tests.
- `src/store/write/tests.rs`: 7 tests — sync, EB3A 6 core + 3 extras, decode failure only, health 3 failures/since/cleared, snapshot bytes, outage open/close once, size budget.

**Size budget (§6, v0.4 gate):** extras per poll — EB3A 3, AC180 6, Elite 3 (the PLAN's "≈ 40" was an over-estimate). AC180 worst case: 1440 × 6 × ~40 B ≈ 0.35 MB + `sample` 1440 × ~60 B ≈ 0.09 MB. Measured by test: one AC180 station-day = 315 392 B (0.32 MB) after `wal_checkpoint(TRUNCATE)` → ~31× under 10 MB. Snapshots are step 11's to add.

**Check:** `cargo test --lib store::` failed first with the threshold mutated to 30 (`three_failures_…` left: Ok, right: Unreachable { since: 60 }); restored → 11/11 pass. `make check` green.

**Review (reviewer, one pass):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- M1 `station_health` cutoff falls back to -1 once retention purges a silent station's last sample → deferred to amendment A2 (step 9, retention).

**Next:** step 4 — grid presence + `src/metrics/` coverage gate.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:30:39Z

```text
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-10-13 — grid presence + `src/metrics/` coverage gate

**Changed:**
- `src/lib.rs`: `pub mod metrics`. `src/metrics/mod.rs` declares `energy`, `grid`, `house`, `runtime`; `house.rs`/`runtime.rs`/`energy.rs` hold only their module doc (group A fills them).
- `src/metrics/grid.rs` (§7.1): `reading` (voltage ≥ min, else `ac_in_w > 0`, else `None` = not a sample); `Grid` starts `Unknown`; `observe` accepts a change after `debounce` consecutive samples, backdated to the run's first ts; a sample matching the accepted state drops the pending run (flapping never flips); `tick(now)` → `Unknown` (since = last sample) when `now − last > 3 × interval`; `observe` runs the gap check first.
- `src/metrics/grid/tests.rs`: 6 tests — reading + fallback, debounce 2 backdates (0, 120), flapping on/off/on, gap 241 s → Unknown then recovers at 1000, gap drops a pending run, debounce 1 + skipped field-less sample.
- `Makefile`: `COV_DIRS := src/protocol/ src/metrics/`; `COV_FILTER` takes `$dir`; `make test` runs it per directory, one line each.

**Check:** failure-first by mutation (backdate to `ts` instead of the run's first): 4/6 failed, e.g. `left: Some(Change { state: Present, since: 60 }) right: … since: 0`; restored → 6/6 pass.
- `make test COV_MIN=101` → `jq: error … src/protocol/ line coverage 99.82% < 101%`, exit non-zero.
- `make test COV_DIRS=src/nope/` → `jq: error … no src/nope/ files in coverage`.
- `make check` green: protocol 99.82%, metrics 98.21%.

**Review (reviewer, one pass):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM. Reviewer re-ran the gate with a faked 10% protocol summary → fails naming `src/protocol/`.
- M1 debounce 1 + gap hides the `Unknown` change inside `observe` → deferred to amendment A3 (steps 5/8).

**Next:** parallel group A — steps 5–7 (house, runtime, energy).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:34:43Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.21% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 5 ✓ 2026-10-13 — house outage log (group A, worktree branch)

**Changed:** `src/metrics/house.rs` + `house/tests.rs` (commit 652b791).
- `vote(&[GridState])`: any `Present` → `Power`, else any `Absent` → `Outage`, else `Unknown`; an unreachable sensor is passed as `Unknown` (no vote).
- `House::new(precision_s, open)` (open = restored open `grid_event` start); `update(&[(GridState, since)])` → opened (`end: None`) or closed `Event`; start = latest `since` of non-`Present` sensors, end = earliest `since` of `Present` sensors, never before start; an open outage stays open through `Unknown` (§6 Restart); unknown with no open outage → no event.
- `totals(events, from, to)` → `{count, seconds_without_power, longest_s}` over events overlapping `[from, to)`, clipped; open event ends at `to` (Q3).
- A3: `update` takes each sensor's `Grid::state()`, never `observe`'s return; contract documented on `update` (tick or observe every sensor each round first).

**Check:** failing first on stubs, 0/7 — `house/tests.rs:73 left: None, right: Some(Event { start: 120, end: None, precision_s: 60 })`; then 7/7 pass (one sensor unreachable + other no grid → outage 120–360; both unreachable → unknown, no event; restart resume; flapping via real `Grid` debounce 2 → no event; totals clipping).

**Next:** steps 6, 7 (same group).

## Step 6 ✓ 2026-10-13 — runtime forecast with low-load floor (group A)

**Changed:** `src/metrics/runtime.rs` + `runtime/tests.rs` (commit e8fe9f2). `forecast(samples, capacity_wh, usable_factor, GridState) -> Forecast {Unknown, OnBattery{load_w, remaining_wh, runtime_h, until_ts}, OnGrid{…}}`; samples `&[(i64, &BTreeMap<Metric, f64>)]`, oldest first.
- EWMA `α = 1 − e^(−Δt/600)`; `until_ts` counted from the newest SOC sample (no `now` param); grid `Unknown` → `Unknown`.
- Outputs read 0 W = newest `ac_out + dc_out ≤ 0`; then load = SOC slope over the trailing non-rising SOC run with drop ≥ 2 points (span to the first sample at the newest level), else `Unknown`. The 1 W floor applies only to a nonzero newest load.

**Check:** failing first 5/8 — `runtime/tests.rs:107 expected OnBattery, got Unknown`; then 8/8 pass, incl. 268.8 Wh 89 → 87 % over 2280 s = 8.488421 W, 23.4175 h; EWMA 100/200/50 W uneven Δt = 118.6666 W; charging 90 → 85 → 87 → `Unknown`.

**Next:** step 7 (same group).

## Step 7 ✓ 2026-10-13 — energy totals and rollup values (group A)

**Changed:** `src/metrics/energy.rs` + `energy/tests.rs` (commit d4886f4). `BUCKET_S = 300`; `totals(samples, interval_s, capacity_wh, from, to) -> Totals{energy_in_wh, energy_out_wh, stored_delta_wh, losses_estimate_wh, missing_s}`; `rollup(samples, interval_s, bucket) -> Option<Rollup>` (§6 `rollup_5m` columns); both on one `integrate` helper.
- Segment counts when its later sample is in the window (step 1 e); caller passes the sample just before it. Gap `dt > 3 × interval` adds no energy/SOC, full `dt` → `missing_s`. Δsoc = Σ over counted segments. Missing power = 0 W; `soc_*` over samples with SOC.

**Check:** failing first on `todo!()` stubs (`energy.rs:39 not yet implemented`); then 10/10 pass: 100 W × 10 min = 16.667 Wh; ramp 0→120 W = 2 Wh; 240 s gap missing, 180 s counted; bucket-crossing segment → later bucket; losses 100 − 50 − 20 = 30.

**Group gate:** `loop/merge-gate.sh` on the 3 branches → green together: protocol 99.82 %, metrics 98.93 %. Branches cherry-picked onto main. Shell needed `PATH=$HOME/.cargo/bin`; `.claude/worktrees/` added to `.git/info/exclude` (local only).

**Group review (reviewer, one batched pass over 652b791..d4886f4):** INTENT satisfied ×3; no creep, no cross-file contradiction, metrics pure; 0 CRITICAL / 1 HIGH / 1 MEDIUM.
- HIGH H1 `runtime.rs:127` — Δt started at the first sample of the run's top plateau, so an idle on-grid plateau before the fall diluted the slope (10 h flat + 89 → 87 in 38 min → ~0.5 W, runtime ~17× long). Verified in source → fixed: Δt now runs between observed downward transitions (top plateau skipped). New test `idle_on_grid_plateau_does_not_dilute_the_soc_slope` failed on the old code, passes now; the 8.5 W and 60 W tests gained the preceding higher sample (the old windows began at an edge plateau); 9/9 pass. Fix re-reviewed (second pass, fix only).
- MEDIUM M1 `energy.rs:95` SOC-less sample drops Δsoc → deferred to amendment A4.
- Fix re-review: 0 CRITICAL / 0 HIGH / 1 MEDIUM (three SOC levels now needed; untested edge) → deferred to amendment A5.
- A3 honoured by step 5's `update` contract (reviewer confirmed `house.rs:77-80`).

**Next:** step 8 — store writer task.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T12:44:57Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 8 ✓ 2026-10-13 — store writer task with restart-safe grid and house events

**Changed:** `src/store/writer.rs` + `writer/tests.rs`; `src/store/mod.rs` exports `Record`, `Station`, `Writer`.
- `Writer::restore(conn, stations, Params)` rebuilds each `Grid` from its last `debounce` samples with a grid field, and keeps open `grid_event` rows (station + `house`) as the state (spec §6 Restart). A station named `house` is refused (scope collision).
- `Writer::spawn(rx)` → `spawn_blocking` + `blocking_recv`; each record is committed before the next, so channel close needs no flush. Failed records → `tracing::error!`, skipped.
- `handle`: Sample → insert + `observe`; Failure → insert; Snapshot → insert only. Unknown station id → error, nothing stored. Then every grid `tick(ts)` before the house vote (A3).
- Station events level-based: `Absent` + none open → open at `since`; `Present` + open → close at `max(since, start)`; `Unknown` → nothing. House events through `metrics::house::House`.

**Check:** `cargo test --lib store::writer` 6/6; failure-first by mutant: `House::new(.., None)` (no house restore) → `house_outage_survives_a_restart_as_one_event` failed with a second `("house", 120, Some(300))` row; restored code passes.
- Restart: 2 sensors off from 120, writer dropped at 180, new writer, `a` on from 300 → one house row 120→300, `a` 120→300, `b` open.
- Flapping (debounce 2) → 0 rows; 6-round gap of failures → Unknown, 0 rows; outage held through Unknown (non-sensor `router` on), ends backdated at 660.

**reviewer:** INTENT shortfall (by mutation) · 0 CRITICAL / 2 HIGH / 1 MEDIUM
- HIGH H1 `writer/tests.rs:149` — no test failed with the `tick(ts)` deleted (A3 untested) → fixed: the gap test asserts both grids `Unknown` since 60, then `Present` since 480; new `an_unreachable_sensor_stops_voting_after_the_tick` (debounce 1 and 2): sensor `a` silent, `b` OFF → house stays `Power` until `a` turns `Unknown` at 300, then opens at 120.
- HIGH H2 `writer/tests.rs:99` — the restart test was debounce-aligned, so dropping the replay passed → fixed: new `restore_replays_the_last_samples_in_order` (ON 0, ON 60, OFF 120 | restart | OFF 180 → `a` + house open at 120).
- Re-checked by mutants: no tick → 2 tests fail; no replay → 1 fails; no `reverse()` → 1 fails; real code 8/8. The fix is test-only, so there is no second review pass.
- MEDIUM M1 `writer.rs:179`: the house event can be lost on a failed `open_outage` → deferred to amendment A6.

**Decision:** none · **Carry-forward:** none
**Next:** step 9 — rollup + retention.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T13:39:26Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 9 ✓ 2026-10-13 — rollup, retention and compaction driven by the writer

**Changed:** `src/store/rollup.rs` + `rollup/tests.rs`; `writer.rs` runs the jobs; `mod.rs` exports `rollup`, `retention`, `compact`.
- `rollup(conn, station, interval, now)`: cursor `max(rollup_5m.ts)`; every bucket with samples after it and before `now`'s open bucket → one row via `metrics::energy::rollup`, fed the sample before each bucket (crossing segment). Plain `INSERT`: the PK refuses a double count.
- `retention(conn, days, now)`: raw `sample`/`sample_extra` older than `days` AND inside a rolled-up bucket AND not the station's newest sample (A2); `register_snapshot`/`poll_failure` older than 90 days; `grid_event` untouched.
- `compact`: `optimize` then `wal_checkpoint(TRUNCATE)` (other order leaves 12 KB of WAL: test caught it).
- Writer: jobs timed by record ts, once per period (5-min bucket / UTC day / UTC week) on the first record entering it, plus once at start. Relies on records in ts order (sequential collector, step 10). A failed job logs and retries next period. `Writer::restore` takes `raw_retention_days`.

**Check:** `cargo test --lib store::` 27/27. Failure-first by mutants, each caught: no preceding sample (4 fail), open bucket rolled (4), no rolled-up check (1), newest sample deleted (1), cursor ignored (4), writer never maintains (1).
- Hand values: 100 W in / 40 W out every 60 s → bucket 0 = 6.667/2.667 Wh n 5; bucket 300 (incl. 240→300) = 8.333/3.333 Wh n 5; a 1140 s gap adds 0 Wh.

**Amendments:** A2 fixed here (newest sample kept; test `retention_keeps_each_station_newest_sample`). A4 stays open: rollup rows carry no Δsoc, only `totals` does.

**reviewer:** INTENT shortfall · 0 CRITICAL / 1 HIGH / 2 MEDIUM
- HIGH H1 `rollup/tests.rs:120` — no test kept a rolled-up sample younger than the cutoff; mutant dropping `ts < ?1` passed → fixed: new `retention_keeps_rolled_up_raw_rows_younger_than_the_cutoff` (95 d rows rolled up, retention at 100 d keeps 3/3); same mutant now fails it; real code 28/28. Test-only fix, no second pass.
- MEDIUM M1 `writer.rs:172` retention/compaction triggers untested → amendment A7.
- MEDIUM M2 `writer.rs:163` out-of-order sample silently skipped by rollup → amendment A8.

**Decision:** none · **Carry-forward:** none
**Next:** step 10 — collector loop (keep record ts nondecreasing, A8).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T13:45:14Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 10 ✓ 2026-10-13 — collector loop: sequential polls under a deadline, always released

**Changed:** `src/collector/{mod,bluez,tests}.rs` (new); `lib.rs` `pub mod collector`.
- `Connect` trait: `open(station, retry, &mut stage)` → `(Gatt, encrypted)`; `release(station)` = disconnect by address. `stage` starts `discover` and moves as the open gets further, so a deadline names where it stopped.
- `BluezConnect`: `find` (or `find_cached` when unreachable, 1 connect attempt) → advertised-model check → `transport::open`; a `HandshakeFailed` source → `handshake`. Error text MAC-masked.
- `Collector::run`: ticks at `start + k × interval` (own `next_tick`; tokio `Skip` fired one late round at once — test caught it), stations in turn, poll under `timeout(interval / n)`, shutdown via `select!`; `release` after every poll on every path; 3rd consecutive failure warns `unreachable` once; `Record` to the writer; writer gone → stop.
- Record ts: `Clock` = max(last, wall, start + monotonic): nondecreasing (A8, collector half).
- Transport lift: `transport::with_disconnect` (body, then always disconnect) now used by `cli/session.rs` and the collector; `transport::{release, find_cached}` in `bluer/gatt.rs`; `link::HandshakeFailed` wraps the key-exchange error.
- ROADMAP §5.4: the deadline bounds a timed-out connect; `connect_attempts` retries fast failures (A1 fixed).

**Check:** `cargo test --lib collector::` 12/12 under `start_paused`. Failure-first: `a_late_round_skips_the_ticks_it_missed` failed with tokio `MissedTickBehavior::Skip` (opens `[0, 130]`, want `[0, 180]`); `a_deadline_during_the_read…` added after mutant "stage not moved to read" survived.
- Mutants, each caught: no release (5 fail), deadline = interval (1), always retry (1), pause ignored (1), no shutdown branch in the poll (1), stage not moved to read (1).
- `grep -rn bluer src/collector` → no match (exit 1).

**reviewer:** INTENT satisfied · 0 CRITICAL / 1 HIGH / 2 MEDIUM
- HIGH H1 `bluer/gatt.rs` `release` skipped the disconnect when `is_connected()` was false, so a connect cancelled mid-attempt by the deadline could finish after the poll → fixed: unconditional bounded `disconnect()` (cancels a pending Connect, as `connect` already relies on); an error counts only while still connected. Hardware path, no unit test; second review pass on the fix: no findings.
- MEDIUM M1 `collector/mod.rs:211` clock floor can freeze `ts` after a wall-clock step back → amendment A9.
- MEDIUM M2 `collector/bluez.rs:59` `HandshakeFailed` → `handshake` mapping untested → amendment A10.

**Decision:** none · **Carry-forward:** none
**Next:** step 11 — hourly `register_snapshot`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:14:30Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 11 ✓ 2026-10-13 — hourly `register_snapshot` from the collector

**Changed:** `src/collector/{mod,tests}.rs`.
- `poll` reads with `read_profile`, builds the `Poll` via `device::decode`/`problems`, and returns the sample with every profile block as read `(start, values)`; `station::poll` stays for the CLI.
- Per-station `new_hour(last, ts)` (UTC hour = `ts.div_euclid(3600)`): the first successful poll of an hour sends one `Record::Snapshot` per block, right after its `Sample`. Failures (any stage, decode included) send none and leave the hour open.
- Hour state is in memory: a restart may add one set in the same hour (different ts, so no PK clash).
- Size per day (24 sets; ~2 B/register + ~15 B/row overhead): EB3A 61 regs, 2 rows → 24 × 152 B ≈ 3.6 KB; AC180 43 regs, 3 rows → ≈ 3.1 KB; Elite 46 regs, 5 rows → ≈ 4.0 KB. Under 0.05% of 10 MB per station-day; 90-day retention ≈ 0.36 MB per station.

**Check:** `cargo test --lib collector::` 15/15. Failure-first: compile error `cannot find function new_hour`; then the hour test failed (60 polls of 62) — the 64-slot test channel filled with snapshots, raised to 256.
- Tests: 62 polls over 3660 s → one set per UTC hour seen (≥ 2) at that hour's first poll, EB3A blocks `(10, 40)` and `(70, 21)`; connect/read/decode failures over 3 rounds → 9 failures, 0 snapshots; `new_hour` on 3599/3600/3660/7199/7200/7200 → T T F F T F.
- Mutants, each caught: always due (2 fail), never send (1 fail).

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM
- MEDIUM M1 `collector/tests.rs` 436 lines, past the ~400 soft limit → amendment A11.

**Decision:** none · **Carry-forward:** none
**Next:** step 12 — replay end-to-end test in `tests/collector.rs`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:17:47Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 12 ✓ 2026-10-13 — replay end-to-end test: collector + writer on a temp DB

**Changed:** `tests/collector.rs` (new, 312 lines; no `src/` change).
- `Replay` implements `Connect`: a fresh `ReplayGatt` per poll from `tests/frames/{eb3a,ac180,elite200v2}.json`; `station::detect` consumes the fixture's leading identity reads, so the collector's `read_profile` gets the profile frames. A fourth station `absent` fails at stage `connect`.
- `start_paused`: 3 rounds (ticks 0/60/120 s), `Shutdown` at 150 s. The writer (`Writer::restore` + `spawn`) drains the channel after the collector stops, so virtual time never waits on its blocking thread.
- Asserts: each replay station has 3 samples whose core columns + `sample_extra` equal `tests/replay.rs`'s hand-read values, 0 failures, `Health::Ok`; `absent` has 3 `poll_failure` rows, all `connect`, 0 samples, `Unreachable { since: first failure }`.

**Check:** `cargo test --locked --test collector` 1/1, green on arrival. Mutants, each caught (src restored by `git checkout`): extras not inserted, failure stage forced to `discover`, unreachable threshold 4, core `soc` not stored.
- Hardware: none (fixture MAC placeholder).

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM. Noted, not a finding: the collector and the writer do not run concurrently (no backpressure coverage); the step does not ask for it.

**Decision:** none · **Carry-forward:** none
**Next:** step 13 — `serve` + `/healthz`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:21:28Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ff3da3329bfc40c0)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13 ✓ 2026-10-13 — `serve` + `/healthz`

**Changed:**
- `Cargo.toml`/`Cargo.lock`: `axum = "0.8.9"`, tokio `net`; `cargo build --all-targets`, lock diff insertions only (+229).
- `src/web/{mod,tests}.rs` (new): `router(reader)` with `/healthz` → 200 `{status:"ok", last_poll_age_s}` (now − `max(sample.ts)`, `null` before the first sample; a DB error logs and still answers 200, §8.2), read in `spawn_blocking` over `Arc<Mutex<Connection>>` from `open_reader`; `serve(listener, router, shutdown)` with graceful shutdown.
- `src/cli/serve.rs` (new): signals installed first; `open_writer` (migrates) → `sync_stations` → `Writer::restore` → `open_reader` → bind `web.listen` (all fail fast, before any poll) → writer, server, collector (`BluezConnect`, `ReadPolicy::from_ble`). SIGINT/SIGTERM → `Control::Shutdown`; collector returns after its poll's release, its sender drops, the writer drains, then the server stops. An early collector/server exit is an error.
- `src/main.rs`: keeps the whole `Config`; `Serve` dispatched there with owned config + keys (`StaticKeys` is not `Clone`); `cli::run`'s `Serve` arm is unreachable (`bail!`).
- `Makefile`: `make run` (`CONFIG ?= config.toml`). `src/store/mod.rs`: `temp_db` `pub(crate)` for the web test. No new config value (`config.rs` unchanged).

**Check:** `cargo test --lib web::` 1/1 — raw `TcpStream` GET on `127.0.0.1:0`, temp DB: `null` before a sample, age 30–35 s after one at now − 30, 404 on `/nope`, server returns `Ok` on stop. Mutant `last_poll_age_s` = 0 → fails (`age 0`).
- Smoke, zero-station scratch config under gitignored `data/` (deleted after): `/healthz` → `{"status":"ok","last_poll_age_s":null}` HTTP 200; SIGTERM → exit 0 (log `SIGTERM, shutting down` → `stopped`); SIGINT → exit 0.
- Fail fast: no `--config` → `serve needs --config`; port taken → `Address already in use`; missing `keys_file` → `No such file or directory` before serving.

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 2 MEDIUM
- MEDIUM writer `JoinError` skips the server stop → amendment A12.
- MEDIUM `/healthz` `max(ts)` full scan → amendment A13.

**Decision:** none · **Carry-forward:** none
**Next:** step 14 — hardware smoke + CF-5 (read-only).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:26:16Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 13 addendum 2026-10-13 — late reviewer finding recorded

**Changed:** `PLAN.md` amendment A14 only (no code).
- The step-13 reviewer delivered a second report after the commit: MEDIUM M1 is new and confirmed against source. `serve.rs` awaits `polling` after the select arm may already have completed it, so it panics on the writer-gone path. This becomes A14. Its M2 is the same finding as A13.
- reviewer · missed: the first report did not include the re-await of a completed pinned future (§8b candidate for the close).

**reviewer:** batched (doc-only) · **Next:** step 14.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:27:06Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 14a ✓ 2026-10-13 — smoke 1 failed on the EB3A (13/16); uncached `find` fixed (A15)

**Smoke 1** (`bluetti-smoke`, 1791901716 + 16 min, release build, `data/smoke.toml`):
- §10.4 pre-flight: adapter powered, rfkill unblocked, `control.enabled = false`; app force-quit not verifiable by the loop (phones are the owner's).
- `systemd-run --user … systemd-inhibit --what=sleep:idle` accepted (listed by `systemd-inhibit --list`): no polkit prompt, so no Q2 H-row; step 15 uses this form.
- `/healthz` 200 at start (`null`), +2 min (`51`), +16 min (`48`). Stop: SIGTERM → `stopped`, unit `Stopped` (not failed), 6.3 MB peak.
- Samples / failures: living-room 16 / 0, mobile 16 / 0, router 13 / 3 (`read`, "no result within the 20s poll deadline", at +120, +180, +900 s). Router **fails** the ≥ 14 check.
- `bluetoothctl info`: `Connected: no` on all 3. `register_snapshot` 10 rows, `grid_event` 0.
- CF-5 (EB3A `ac_in_v` 218–220 V, grid on): Elite `ac_in_v` 221.5–224.6 V, AC180 218.3–221.4 V; `ac_input_frequency` 49.9–50.0 Hz on both (16 samples each). Scales right; CF-5 AC180 half recorded in `loop/STATE.md`; H5 stays the owner's.

**Cause:** every router poll took ~20 s (next station's `ts` = router `ts` + 20 in all 13 rounds). Debug `read router` 60 s apart: "station not in the BlueZ cache, scanning", then the full 15 s `discover` sleep; 20.0 s per read. BlueZ drops the unpaired EB3A ~30 s after the disconnect (`TemporaryTimeout`).

**Changed (14a, amendment A15):**
- `src/transport/bluer/scan.rs`: `scan_for(adapter, address, timeout)` — discovery until the station is heard (manufacturer data + RSSI, checked every 250 ms), ≤ timeout + D-Bus margin; `le_adapter` shared with `discover`.
- `src/transport/bluer/gatt.rs`: `find` uses `scan_for` on a cache miss; `find_cached` removed (`transport/mod.rs`).
- `src/collector/{bluez,mod}.rs`: always `find`; `unreachable` keeps 1 connect attempt but scans, else a station dropped from the cache never recovers.
- `ROADMAP.md` §5.4: the finding and the recommendation.

**Check:** uncached `read router` (60 s apart, "scanning" logged): 20.5 / 20.0 / 20.0 s before → 6.0 / 5.0 s after; cached 5.0–6.0 s both ways. `make check` green. No unit test: bluer code has no fake (adapter boundary); 14b's re-smoke is the proof.

**reviewer:** INTENT partial by design (14b re-smokes), no creep · 0 CRITICAL / 0 HIGH / 1 MEDIUM; batch also covers 10e523d (A14, doc-only).
- MEDIUM `scan_for` swallows `advert_of` errors → amendment A16.

**Decision:** ROADMAP §5.4 — `find` scans until heard; `unreachable` stations scan too · **Carry-forward:** CF-5 AC180 half observed (STATE)
**Next:** step 14b — re-run the smoke as written (A15).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T14:55:33Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 14b ✓ 2026-10-13 — smoke 2 passes the count check; CF-5 re-confirmed

**Smoke 2** (`bluetti-smoke`, start 1791903375 + 960 s, release build of d83f562, `data/smoke.toml`; smoke 1's DB moved to `data/old/`):
- §10.4 pre-flight: adapter powered, rfkill unblocked, `control.enabled = false`; app force-quit not verifiable by the loop.
- `make build` failed first: `cargo` not on this shell's `PATH` (`~/.cargo/bin`). A unit started on the already-fresh binary was stopped after ~10 s and its DB deleted; rebuilt (`Finished … 0.09s`, nothing to do) and started clean.
- `systemd-inhibit --what=sleep:idle` accepted again (`systemd-inhibit --list`: `v0.4 smoke`, `block`).
- `/healthz` 200 at start (`null`), +120 s (`50`), +955 s (`46`). Stop: journal `Stopped`, not failed; transient unit gone.
- Samples / failures: router 14 / 2, living-room 15 / 1, mobile 16 / 0. **Check (≥ 14 each) passes.**
- All 3 failures are `discover` ("station not found within 15s") on a cache miss: router +480 and +720 s, living-room +545 s. No `read` deadline misses (smoke 1's cause is fixed).
- `bluetoothctl info`: all 3 stations "not available" (dropped from the BlueZ cache, so not connected). The only connected device is the owner's JBL speaker.
- `register_snapshot` 20 rows (start + the 15:00 UTC hour), `grid_event` 0.
- CF-5 re-confirmed (already discharged in STATE): EB3A `ac_in_v` 213–222 V (grid on); Elite 217.1–226.0 V, AC180 213.8–222.3 V; `ac_input_frequency` 49.9–50.0 Hz on both (15 / 16 samples).

**Risk for step 15:** router 2 misses in 16 = 87.5 %, below the v0.4 gate's 95 % if that rate holds over the 12 h run (A17: ~90 misses of 720 vs ≤ 36 allowed). Sample is small; cause unproven (BR/EDR speaker coexistence on the same adapter, or advert interval) → amendment A18. A15 → fixed.

**Next:** step 15 — start the 12 h run (A17).

**reviewer:** INTENT satisfied · 0 CRITICAL / 1 HIGH / 1 MEDIUM.
- HIGH duplicate id A17 (owner's 12 h decision, 9415894, landed mid-run) → fixed: scan finding renumbered A18.
- MEDIUM 24 h wording → fixed now (one-line text edit): 12 h, 720 polls, ≤ 36 misses.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:14:53Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 15 ✓ 2026-10-13 — 12 h acceptance run started (A17)

**Started:** unit `bluetti-acceptance-24h` (transient, `systemd-run --user`), start **1791904529** (2026-10-13T15:15:29Z).
- **Relaunch / ETA (step 18):** `now ≥ 1791947729` (2026-10-14T03:15:29Z); window `[1791904529, 1791947729)`, ≥ 684/720 polls per station (A17).
- Binary: `make build` on 886d22b (no `src/`/`Cargo.*` change since d83f562, the smoke-2 build) copied to `data/run/bluetti-controller`.
- Config: `data/acceptance.toml` = `config.toml` with only `storage.path = "data/acceptance-24h.db"` (fresh file; `diff` shows one line). All under gitignored `data/`.
- Command as step 14 verified: `systemd-inhibit --what=sleep:idle --why="v0.4 acceptance"` wrapper; `systemd-inhibit --list` shows `v0.4 acceptance` `sleep:idle` `block`.
- Journal: `serving listen=0.0.0.0:8080 stations=3 interval_s=60`.

**Check (+203 s, 1791904732):** `is-active` → `active`; samples router 4, living-room 3, mobile 3 (each ≥ 1); failures living-room `discover` 1; `/healthz` 200 `{"status":"ok","last_poll_age_s":22}`. **Passes.**

**Risk noted:** owner's JBL Charge 4 still connected on the same adapter (A18 suspect). Not disconnected: it is the owner's device in use, not a station. Step 18's per-station miss rate decides A18.

**From now until step 18:** no command touches a station; `data/run/` binary is not rebuilt (golden rule #5).

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM `Linger=no`: logging out of GNOME before the ETA ends the user manager and kills the run → amendment A19.
**Next:** step 16 — acceptance measurement script (station-free).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:20:14Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 16 ✓ 2026-10-13 — acceptance measurement script

**Changed:** `scripts/acceptance_v04.py` — new; stdlib `sqlite3`, `file:…?mode=ro`, run with `python3 -I`. Per-station polls vs `(to − from) // interval` (capped at 100 % when an unaligned window holds one extra tick), failures by stage, DB+WAL size per station-day, bytes per table (`dbstat`, else rows × avg row length), `grid_event` rows overlapping the window, exit 1 on any FAIL.

**Check (smoke 2, `[1791903375, 1791904335)`, 60 s, 16 scheduled):**
- Reproduces step 14: router 14 samples / `discover` 2, living-room 15 / `discover` 1, mobile 16 / none. Exit 1 (router 87.50 %, living-room 93.75 % FAIL at 95 %).
- `--min-pct 101` → mobile 100 % FAIL too, exit 1.
- Size line: 551848 B / 3 / 0.0111 days = 16.6 MB per station-day FAIL — expected on 16 min: the WAL (506 KB, not checkpointed) is fixed overhead that a 12 h window divides 45× smaller. `dbstat` available (SQLite 3.46.1): `sample_extra` 12288 B largest; 183 rows cross-checked by count.
- Live run, read-only, +359 s: router 6, living-room 5 (+1 `discover`), mobile 6 — PASS lines; no station touched, unit untouched.

**reviewer:** INTENT satisfied · 0 CRITICAL / 0 HIGH / 1 MEDIUM.
- MEDIUM WAL bytes not named in size breakdown → amendment A20.
- Reviewer's own check: `--min-pct 85 --max-mb 20` exit 0, `--min-pct 101 --max-mb 20` exit 1 (the flag flips the result).

**Next:** step 17 — docs close-out (station-free).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:23:08Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 17a ✓ 2026-10-13 — docs close-out, done-when greps (A21 split)

**Changed (no station touched; run `active`, `data/run/` untouched):**
- ROADMAP.md §12 M3: six code items ticked (evidence: LOG steps 2–13); the Human 12 h item stays for step 18. §5.2 milestone column: M1/M2/M3 modules ✓, `web` = M3 `/healthz` stub ✓ / M4.
- README.md: status → phase 4, new `## Run` (`make run`, `CONFIG=`, what `serve` does), `make test` line names both coverage dirs.
- `config.example.toml`: comments on `storage.path` (serve creates + migrates; directory must exist — no `create_dir` in `src/`) and `web.listen` (`/healthz`, pages in M4).

**Done-when greps:**
- `grep -rnE 'bluer::|use bluer' src/ | grep -v ^src/transport/bluer/` → only `src/transport/mod.rs:23-24` (`pub use self::bluer::…` re-exports).
- `grep -rnE 'std::env::var|env::var' src/` → no hit (config is file-only).
- `grep -c '^| H4 ' HUMAN-CHECKLIST.md` → 1.

**Amendments:** A3 fixed (step 8), A17 fixed (12 h window applied), A18 re-targeted → step 18, A19 re-targeted → CF-6 (new, `loop/STATE.md`). A21 splits step 17: 17b–17e fix A4–A14, A16, A20 during the run; 17e ticks step 17. `make amendments` → 14 open (A21 + those 13).

**reviewer:** batched (low-risk docs).
**Next:** step 17b — A4 + A5 (metrics).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:25:09Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.94% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 17b ✓ 2026-10-13 — A4 + A5 (metrics)

**Changed (no station touched; run `active`, `data/run/` untouched):**
- `src/metrics/energy.rs` `integrate`: carries the last SOC forward within a gap-free run; a gap clears it. Module doc + `stored_delta_wh` doc say so. `rollup` does not use Δsoc, so production rollups are unchanged; `totals` has no production caller yet.
- ROADMAP.md §7.4 M3: one sentence on the SOC carry.
- Tests: `soc_carries_over_a_sample_without_soc` (89/None/87 → −20 Wh; gap before None → 0 Wh, missing 600 s); `a_fall_seen_from_the_window_edge_needs_three_soc_levels` (89→88→87 from the edge and 90→88 jump → `Unknown`).

**Check:**
- Failure first: `cargo test --lib metrics` → `soc_carries_over_a_sample_without_soc` FAILED, "got 0, want -20". A5 test is a pinning test: green on the unchanged code, as intended.
- After the fix: 35/35 metrics tests; `make check` green, `src/metrics/` 98.96 %.

**Amendments:** A4 fixed, A5 fixed.

**reviewer:** one pass over `@{u}..HEAD` (17a, batched) + the 17b diff: INTENT satisfied for both · 0 CRITICAL / 0 HIGH / 0 MEDIUM.

**Next:** step 17c — A6 + A7 + A8 (writer).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:29:06Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 98.96% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 17c ✓ 2026-10-13 — A6 + A7 + A8 (writer)

**Changed (no station touched; run `active`, `data/run/` untouched):**
- `src/store/writer.rs`: A6 — `house_open` holds `(id, started_at)`; a failed house `open_outage` resets `House` (next round re-opens), a failed `close_outage` restores `House` open and the id (next round re-closes, backdated). A8 — `rolled_past` (sample ts < rollup cursor + 300) → `tracing::warn!` in `handle`; the sample is still stored. Collector record ts stay nondecreasing (step 10 `Clock`; A9 covers its edge).
- `src/store/writer/tests.rs`: `a_failed_house_write_is_retried_next_round` (temp trigger `RAISE(ABORT)` on house INSERT, then UPDATE).
- `src/store/writer/tests/jobs.rs` (new; keeps `tests.rs` ≤ 400 lines): `retention_runs_daily_and_compaction_weekly` (retention 1 day; day 1 deletes rolled rows < 600, no second run same day, week 1 deletes again + WAL 0 B), `a_sample_behind_the_rollup_is_stored_and_flagged`.

**Check:**
- Failure first (`rolled_past` stubbed `Ok(None)`): A6 test FAILED — house row missing (`[("a",120,None,60)]`); A8 test FAILED `None` vs `Some(300)`. A7 test green on unchanged code (pins existing triggers).
- After the fix: `cargo test --lib store::` 31/31; `make check` green.

- A7 mutation (retention period → 5 min, compaction → 1 day): `retention_runs_daily_and_compaction_weekly` FAILED (`[87000, 87600]` vs `[600, 87000, 87600]`); restored.

**Amendments:** A6 fixed, A7 fixed, A8 fixed.

**reviewer:** risky, before commit: INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM. Noted, no action: a persistent house-write error blocks the §6 jobs for later records, as per-station event errors already do; only an outside DB edit can cause it (one-writer rule).

**Next:** step 17d — A9 + A10 + A11 + A16 (collector, transport).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:33:42Z

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

## Step 17d ✓ 2026-10-13 — A9 + A10 + A11 + A16 (collector, transport)

**Changed (no station touched; run `active`, `data/run/` untouched):**
- `src/collector/mod.rs` A9: `Clock::now(station)` also floors at that station's last ts + 1, so a wall clock stepped back never repeats a `(station_id, ts)` key; global ts stay nondecreasing (A8). Anchor floor kept: it moves ts under paused tests; the startup-ahead trade-off is in the `Clock` doc.
- `src/collector/bluez.rs` A10: pure `open_stage(&anyhow::Error)` → `Handshake` on a typed `HandshakeFailed` source (also under `.context`), else `Connect`.
- `src/collector/tests/snapshot.rs` (new) A11: `snapshots()` + the three snapshot tests; `tests.rs` 436 → 382 lines.
- `src/transport/bluer/scan.rs` A16: `listen_for` keeps the masked last `advert_of` error, `debug!`s it, and appends it to the not-found error, now built in `scan_for` (`gatt.rs` `find` just returns it). No unit test: needs BlueZ, and no station access during the run.

**Check:**
- Failure first (Clock bump removed): `a_clock_stepped_back_still_gives_each_station_a_new_ts` FAILED `[0, 0, 0, 0, 0]` vs `[0, 0, 1, 1, 2]`; `record_timestamps_never_go_back` FAILED. Mutation (`open_stage` always `Connect`): `a_typed_handshake_error_names_the_handshake_stage` FAILED. Both restored.
- After: `cargo test --lib collector` 17/17 (snapshot tests run under `collector::tests::snapshot::`); `make check` green.

**Amendments:** A9 fixed, A10 fixed, A11 fixed, A16 fixed.

**reviewer:** risky, before commit: INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM.

**Next:** step 17e — A12 + A13 + A14 (serve, web) + A20 (script); its check ticks step 17.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:37:44Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 17e ✓ 2026-10-13 — A12 + A13 + A14 (serve, web) + A20 (script); step 17 done

**Changed (no station touched; run `active`, `data/run/` untouched):**
- `src/cli/serve.rs` A14: `collector_done` set when the `polling` arm wins; the shutdown path skips the second `polling.await` (a finished `async fn` panics when polled again). A12: `written = writer.await` is kept, the server is stopped and awaited, then the writer `JoinError` propagates.
- `src/web/mod.rs` A13: `/healthz` takes `max((SELECT max(ts) FROM sample WHERE station_id = s.id)) FROM station s`. EXPLAIN QUERY PLAN: `SEARCH sample USING PRIMARY KEY (station_id=?)`; the amendment's `GROUP BY station_id` form still does `SCAN sample`, so not used.
- `scripts/acceptance_v04.py` A20: size line prints `db <n> B + wal <n> B = <n> B`; comment that per-table bytes are logical pages.

**Check:**
- A13 timing, in-memory DB, 3 × 43200 rows, 100 runs: old 4.73 ms, new 0.004 ms per query; both return 2591940. `healthz_reports_the_newest_sample_age` pins the result (null, then age 30–35 s).
- A20 on `data/smoke.db`: `db 45056 B + wal 506792 B = 551848 B` (matches the A20 numbers).
- A12/A14: no unit test — `serve` glue over `BluezConnect`; reachable only by a writer panic.
- `make check` green; `make amendments` → 21 findings, all disposed.

**Amendments:** A12, A13, A14, A20 fixed; A21 fixed (17a–17e done).

**reviewer:** risky, before commit: INTENT satisfied · 0 CRITICAL / 0 HIGH / 0 MEDIUM; EXPLAIN QUERY PLAN re-run by the reviewer.

**Next:** step 18 — blocked until the 12 h ETA (start 1791904529 + 43200 = 1791947729).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:40:11Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18 ⏸ 2026-10-13 — 12 h window still open; interim check flags the router

**State (no station touched; `data/run/` untouched):**
- now 1791906031 < ETA 1791947729 (start 1791904529 + 43200 = 2026-10-14 03:15:29 UTC). Unit `bluetti-acceptance-24h` active; `/healthz` ok, `last_poll_age_s` 7.
- Interim, read-only: `scripts/acceptance_v04.py data/acceptance-24h.db 1791904529 1791906000 60` (24 scheduled polls):
  - router (EB3A) 19/24 = 79.17 % FAIL — 6 `discover` failures ("station not found within 15s") at +361, +601, +661, +961, +1021, +1441 s, spread over the window, not a startup burst.
  - living-room (Elite200V2) 24/24, 1 `discover` failure retried; mobile (AC180) 25/25, none.
  - size FAIL is window-length noise: 783 KB is mostly a 738 KB WAL over 0.017 days; WAL is capped by autocheckpoint (~4 MB), so the 12 h figure is the one that counts.
- The JBL Charge 4 is still connected on the same adapter (`bluetoothctl devices Connected`) — A18's suspected cause. Router at ~21 % misses would give ~150 of 720 vs 36 allowed.

**Decision:** keep the run (the step forbids an in-step wait or an early verdict on 24 polls; changing the adapter mid-run would void the window). Owner may stop it early: disconnect the JBL, or move it off the dev laptop's adapter, and re-run 15 → 18 by amendment. If the run ends FAIL, A18's path applies: measure `scan_for` time-to-heard per station before the re-run.

**reviewer:** batched (LOG-only).

**Next:** step 18 at the ETA.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T15:41:20Z

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

## Owner action 2026-10-13 18:47 — 12 h run restarted after JBL disconnect
- Owner decision: the JBL Charge 4 was disconnected from the laptop adapter (18:43:13); the run restarted at once, no observation window.
- Run 1 stopped (SIGTERM); its DB kept as `data/acceptance-24h-run1.db*` (not measured for the gate).
- Run 2: same unit `bluetti-acceptance-24h`, same command and binary, fresh `data/acceptance-24h.db`.
- **New window for step 18: start 1791906447, ETA 1791949647** (2026-10-14 03:47:27 UTC = 06:47 EEST). Supersedes start 1791904529.
- Keep audio devices off the laptop's Bluetooth during runs (A18).

## Step 18 ✗ 2026-10-14 — 12 h run FAIL: router 86.81 %, living-room 91.81 % (min 95 %)

**State:** now 1791949750 ≥ ETA 1791949647. `/healthz` ok (`last_poll_age_s` 40). `systemctl --user stop bluetti-acceptance-24h`: SIGTERM, "stopped", clean exit; 12h 2min wall, 29 s CPU, 19.3 MB peak memory.

**Check:** `python3 -I scripts/acceptance_v04.py data/acceptance-24h.db 1791906447 1791949647 60` → `RESULT: FAIL`, EXIT=1. 720 scheduled polls per station (A17).
- router (EB3A): 625/720 = 86.81 % FAIL — 95 misses vs 36 allowed; all 95 `discover` ("station not found within 15s").
- living-room (Elite200V2): 661/720 = 91.81 % FAIL — 48 `discover`, 6 `handshake` ("station signature does not verify"), 5 `read` (20 s poll deadline).
- mobile (AC180): 718/720 = 99.72 % PASS — 2 `handshake` ("ff01: notification stream closed").
- size: db 409600 B + wal 4144752 B = 4554352 B over 3 stations × 0.5 days = 3.036 MB per station-day PASS (max 10).

**Miss shape (`discover` per hour from start, read-only `sqlite3`):**
- router 7, 6, 12, 7, 7, 8, 8, 9, 6, 9, 9, 7: steady ~13 % for 12 h. Not a startup burst, not tied to the outage.
- living-room 7, 7, 6, 4, 6, 7, 2, 4, 2, 1, 1, 1: falls with time.
- The JBL was off the adapter for the whole run (owner, 18:43), so A18's coexistence cause is falsified for router. Next suspect: router's advert interval vs the 15 s `scan_for` window.

**Real house outage (for the owner's H4; the loop does not mark it):** `grid_event` id 3 `house` 1791911072 → 1791922527 (3 h 11 min). Station rows: router id 1 from 1791911067, living-room id 2 from 1791911072, mobile id 4 from 1791911076; precision 60 s. The true grid-loss time is unknown, so the detection delay cannot be computed here; the owner compares it with the house clock for H4.

**Decision:** gate failed, so no tick and no ROADMAP M3 tick. Fix the work, never the threshold. A22 splits the fix path: 18a measure, 18b fix, 18c re-run, 18d measure. The DB stays under `data/` for 18a.

**reviewer:** risky, with the batch 67733a7 + 6d2e56c. INTENT satisfied; numbers re-run and matched; 0 CRITICAL / 0 HIGH. M1 (18b must fit the 20 s per-poll budget) and M2 (escape hatch is the PLAN 3-try hatch, not AGENTS §6) fixed in A22's own wording before its first commit.

**Next:** step 18a — measure `scan_for` time-to-heard per station (read-only).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T03:52:30Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18a ✓ 2026-10-14 — discover misses come from LE discovery stopping < 1 s after start inside `serve`

**What:** measured time-to-heard read-only (A22 18a): three probes with no 15 s cap, then two `serve` runs at HEAD with debug logging. Probe scripts were throwaway, under `/tmp`. Scout page `research-ble-reliability.md` covers community tools and BlueZ internals (owner goal 2026-10-14).

**Time-to-heard, 30 samples per station each (ms, p50/p95/max, misses):**
- A, `bluetoothctl` LE scan, no connect: router 291/857/859, living-room 566/1405/1405, mobile 289/574/849; 0 misses.
- B, as A plus a GATT connect for 3 s and a disconnect: router 268/807/809, living-room 269/809/1075, mobile 269/806/809; 0 misses. One router connect was not up within 15 s.
- C, the real `read` path (run-2 binary, scan cap 90 s, 60 s rounds): 90/90 exit 0, every scan heard < 0.3 s, `read` 3.9–6.0 s.
- Result: advert interval and the 15 s window are not the cause. The A18 coexistence cause was already out.

**Reproduced only under `serve`** (HEAD 53774c9, user unit, 40 min): router 31/40 (9 `discover`), living-room 35/40 (5), mobile 40/40. The A16 error text ends "last BlueZ error: the target object was either not present or removed", so the device object never appeared.

**Mechanism** (`gdbus monitor --system --dest org.bluez` + `serve`, 20 min, 62 `Discovering` events):
- Every poll's `StartDiscovery` gives `Discovering: true`, then `false` 0.26–0.80 s later. BlueZ does not restart it, and the session stays open 14–15 s with no adverts.
- A station heard inside that first ≤ 0.8 s window succeeds. Otherwise it misses at 15 s, e.g. router 17:18:01.352 true, 02.113 false, miss 17:18:16.
- The next session's `StartDiscovery` works again: living-room was heard 0.5 s after the router miss.
- Probes A/B logged no true→false before heard. The early stop is seen only in `serve`, where the poll waits after it.
- Who stops the scan (kernel or bluetoothd) is unproven: the bluetoothd journal shows only "No matching connection for device". The fix does not depend on it.
- Run 2's misses are independent per poll: router P(miss | previous miss) 0.14 vs P(miss) 0.132; living-room 0.06 vs 0.067. No silent stretches.

**Living-room `handshake` "station signature does not verify"** (run-2 journal): 6 of ~672 Elite handshakes (0.9 %), isolated, the next poll succeeds. None in 625 EB3A or 720 AC180 handshakes, and none in the 60 Elite polls today.
- Hypothesis, unproven: an Elite firmware encoding edge, e.g. a leading-zero r/s/X, expected rate 2/256 = 0.78 %.
- Also seen: 5 Elite `read` misses (block 100 gets no notification twice, then "dropped while connected").

**Next, 18b inputs:**
- (1) Keep the station scan alive: restart discovery whenever `Discovering` goes false inside `scan_for`, or hold one session across polls (scout page recommendation 1).
- (2) Retry one failed handshake or read inside the 20 s budget, or log message-4's shape (lengths, leading bytes, never key bytes).

**reviewer:** batched with the 2 unpushed commits (low-risk: doc and measurement only, no `src/` change).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T14:36:33Z

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

**reviewer dispositions (batch `@{u}..7b4cbf4`, 2026-10-14):**
- The batch is 6 commits, not 2. A concurrent session committed 0cdaa3f, 8624469 and dddbf4e during 18a (A23, H6, the second-adapter page). Its page content is theirs; this step's `git add` of the page was a no-op.
- H1 fixed: A24 supersedes A23's rule #5 clause and requires an explicit owner line. It also carries the restart-on-`Discovering=false` steer (M1, A23 part).
- M1 (page), M2 (marker docs), M3 (precision of this entry): deferred to A25, A26, A27.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T14:42:07Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18b ✓ 2026-10-14 — LE discovery restart in `listen_for`: 123/123 polls over 41 min (A28)

**Changed:** `src/transport/bluer/scan.rs` — `listen_for` checks the adapter's `Discovering` every 250 ms; on `false` it drops the session and starts a new one (A24 part 1). Debug log counts `restarts` per scan. `PLAN.md` A28 (split 18b/18b2), A29.

**Check:** `make check` green. Read-only `serve` (user unit `bluetti-18b2`, `/tmp/b18b2`, fresh DB, binary built from this tree 19:18:47), window `[1791994727, +2460)`, no suspend in the journal:
- `scripts/acceptance_v04.py`: router 41/41, living-room 41/41, mobile 41/41, 0 `poll_failure` rows; 0.719 MB per station-day.
- 18a baseline under `serve` (40 min): router 31/40, living-room 35/40, mobile 40/40.
- 83 scans: 69 heard with 0 restarts, 14 heard only after 1 restart (each a likely miss before the fix); 0 not heard, 0 restart failures.

**Interrupted first run** (`bluetti-18b`, 17:45:09, binary built 17:44 with the restart logic, tree edited 17:47):
- Lid closed, laptop suspended 18:14:53 to ~19:17, so it covers only 29 polls: router 29/29, living-room 29/29, mobile 21/29.
- Mobile failures cluster 18:04–18:13: connect `le-connection-abort-by-local` 2, handshake "out of order: expected message 6, got 4" 2, discover 4 (2 with discovery up, 0 restarts: station silent for 15 s), deadline 1. Cause unproven (station side or phone app); 18b2's per-failure log targets it.
- Stopped and replaced by the clean run above; its DB is kept at `/tmp/b18b/run-1.db`.

**reviewer:** risky, before commit. INTENT satisfied, 0 CRITICAL / 0 HIGH. bluer 0.17.4 source checked: drop-then-start has no race (new start waits for the old `StopDiscovery`), still bounded by `scan_for`'s timeout. M1 (a restart error overwritten by the same pass's `advert_of` error in the not-found text): deferred to A29.

**Next:** 18b2 — A23 (2) handshake retry once on a fresh connection, (3) per-failure log (stage, warm/cold cache, time since last success); A29 fits there.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T17:00:50Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18b2 ✓ 2026-10-14 — key-exchange retry once on a fresh connection; per-failure log fields (A23 parts 2–3, A29)

**Changed:**
- `src/collector/mod.rs` — `Collector::poll`: an open failing at `handshake` on a reachable station is released and opened once more, inside the same §5.4 deadline. Unreachable stations get no retry (one attempt, §5.4).
- `Connect::open` takes `&mut Progress { stage, cached }` instead of `&mut Stage`. The `poll failed` warn adds `cache` (warm / cold / -) and `since_success` (seconds since the station's last good poll, or `never`).
- `transport::find` returns `(Advert, bool)`, true = from BlueZ's cache. Callers updated: `src/collector/bluez.rs`, `src/cli/session.rs`, `tests/hardware.rs`.
- `src/transport/bluer/scan.rs` — A29: restart error kept in its own variable; pure `not_found` names both errors.
- Tests: `src/collector/tests/retry.rs` (new; keeps `tests.rs` at 392 lines), `FailFirst` fake behaviour, `each_failure_names_its_stage` releases 4 → 5.

**Check:** `make check` green. Failed first as intended: `a_handshake_failure_is_retried_once_on_a_fresh_connection` (`["handshake"]` vs `["ok"]`), `an_unreachable_station_gets_no_handshake_retry` (4 opens vs 7), `each_failure_names_its_stage` (releases 4 vs 5). `other_open_failures_are_not_retried_in_the_poll` passed before and after (guard). No station touched; hardware effect is measured by 18c's run.

**reviewer:** risky, before commit. INTENT shortfall (part 3 untested), 0 CRITICAL / 2 HIGH / 0 MEDIUM.
- H1 fixed: the retry reset `progress`, so a deadline during the retry stored `discover`/`connect` and dropped the first error. `Progress.retrying` keeps it, and both the deadline path and a failed retry now store `handshake` with both errors. Tests `a_deadline_during_the_retry_still_names_the_handshake` and `a_failed_retry_keeps_both_errors_under_the_handshake` failed first. Mutation check: when the deadline branch ignores `retrying`, the test fails with `Connect`.
- H2 fixed: new pure `failure_fields`, tested warm / cold / `-` / `never` / 75 s; the Fake now sets `progress.cached`.
- Second pass on the fixes only: INTENT satisfied, H1 and H2 confirmed fixed, 0 CRITICAL / 0 HIGH. M1 (a deadline during the in-poll `release` loses the first error): deferred to A30.

**Next:** 18c — rebuild `data/run/`, step 14's smoke, then step 15's 12 h run (A22).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T17:09:21Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18c1 ✓ 2026-10-14 — smoke on 15dfa92 passes; 12 h start held, AC180 stopped advertising (A31)

**Changed:** no code. `make build` on 15dfa92 (`target/release/`; `data/run/` not yet replaced). Run-2 DB moved to `data/old/run2/`, smoke-2 DB to `data/old/smoke2/`. `PLAN.md` A31 (split 18c). `HUMAN-CHECKLIST.md` H7.

**Smoke** (`bluetti-smoke`, start 1791997817 + 950 s, `data/smoke.toml`, same as `config.toml` except `storage.path`):
- §10.4 pre-flight: adapter powered, rfkill unblocked, `control.enabled = false`, no device connected; app force-quit not verifiable by the loop.
- `systemd-inhibit --what=sleep:idle` accepted (`v0.4 smoke`, `block`). `/healthz` 200 at +120 s (`10`) and +950 s (`45`). Stop: SIGTERM, `stopped`, unit gone; 6.4 MB memory peak.
- Samples / failures: router 16 / 0, living-room 16 / 0, mobile 14 / 2. **Check (≥ 14 each) passes.**
- Mobile failures: `discover` at 17:24:44 and 17:25:41, `cache="-"`, `since_success` 72 s and 129 s (18b2's fields work). Last mobile sample 1791998608.
- First live 18b2 handshake retry: router 17:14:19, "notification stream closed", recovered on the fresh connection (router 0 failures). 1 `read` retry (start 1300, attempt 1, no notification in 5 s), recovered. `register_snapshot` 10 rows; 0 devices connected after stop.
- Grid outage during the whole window: `ac_in_v` 0 V router and Elite, ≤ 2.3 V AC180; 4 open `grid_event` rows (house + each station, from the first poll). CF-5 not re-checkable now; it stays discharged (STATE).

**AC180 absent:** read-only `scan` at 17:27 (15 s, 30 s) and 5× at ~110 s to 17:36: EB3A and Elite heard every time, AC180 never. Likely switched off or moved during the outage (cause not visible to the loop).

**Decision:** the 12 h run does not start with a station known to be absent (A31); H7 asks the owner. A30 (code) lands first.

**reviewer:** risky, before commit. INTENT satisfied, 0 CRITICAL / 0 HIGH / 3 MEDIUM, all one-line text fixes to this uncommitted entry, fixed now:
- M1: handshake retry added above; H7 now says 7 scans, silent between 17:23:28 and 17:24:29 UTC (DB).
- M2: A31 now re-runs step 14's smoke on the A30 commit before the run.
- M3: A31 now says why H7 blocks despite the deferred-checks rule.
**Next:** A30, then 18c2 — start the 12 h run once `scan` hears `mobile`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T17:39:07Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## A30 ✓ 2026-10-14 — a deadline during the pre-retry release keeps the first key-exchange error

**Changed:**
- `src/collector/mod.rs` `poll` — `progress.retrying = Some(first)` now runs before `release`, so the deadline path names both errors under `handshake`.
- `src/collector/tests/retry.rs` — `a_deadline_during_the_retry_release_keeps_the_first_error`: `Fail(Handshake)` with a 70 s release (> 60 s deadline).
- `PLAN.md` A30 disposition: fixed.

**Check:** `make check` green. Failed first as intended: the stored error was only "no result within the 60s poll deadline" (no "handshake failed"). Other retry tests unchanged and green. No station touched.

**reviewer:** risky, before commit. INTENT satisfied, 0 CRITICAL / 0 HIGH / 0 MEDIUM (timing of the new test traced: record sent at 130 s < 140 s).

**Next:** 18c2 — one read-only `scan` for `mobile` (A31); heard → step 14's smoke on this commit, then step 15's 12 h run; silent → `<<LOOP:BLOCKED>>` on H7.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T17:42:59Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18c2 ✓ 2026-10-14 — AC180 back; smoke on df66955 passes; 12 h run 3 started (A31)

**Changed:** no code. `PLAN.md` A31 disposition: fixed. Smoke-3 DB (18c1) moved to `data/old/smoke3/`. `make build` on df66955, copied to `data/run/bluetti-controller` (`cmp` equal).

**Scan** (read-only, 30 s, 17:44 UTC): `AC180`, `Elite200V2` and `EB3A` all heard (-65 / -66 / -70 dBm). H7's condition met; H7 stays for the owner to mark.

**Smoke** (`bluetti-smoke`, invocation 809db8a7, start 1791999868, stop 1792000841, `data/smoke.toml`):
- §10.4 pre-flight: adapter powered, rfkill unblocked, `control.enabled = false` (both configs); `config.toml` vs `data/smoke.toml` / `data/acceptance.toml` differ only in `storage.path`. App force-quit not verifiable by the loop.
- `systemd-inhibit --list`: `v0.4 smoke` `sleep:idle` `block`. `/healthz` 200 at +120 s (`54`), +731 s (`6`), +973 s (`5`).
- Samples / failures: router 17 / 0, living-room 17 / 0, mobile 16 / 0. **Check (≥ 14 each) passes.**
- Journal (this invocation): 0 retries; one WARN, "dropped while connected" at SIGTERM. `stopped`, unit inactive, 6.4 MB memory peak. 0 devices connected after stop. `register_snapshot` 15 rows.
- Grid still out: `ac_in_v` 0 V router and Elite, ≤ 2.3 V AC180; 4 open `grid_event` rows from the first poll (house + each station).

**12 h run 3 started:** unit `bluetti-acceptance-24h` (invocation 4fe36d01), start **1792000851** (2026-10-14T18:00:51Z), `data/run/bluetti-controller serve --config data/acceptance.toml`, fresh `data/acceptance-24h.db`, inhibit wrapper as step 15.
- **ETA (18d):** `now ≥ 1792044051` (2026-10-15T06:00:51Z); window `[1792000851, 1792044051)`, ≥ 684/720 polls per station (A17). Attempt 2 of step 18 (A22).
- `Linger=no` still (A19 → CF-6): the owner must stay logged in until the ETA.
- From now to 18d: no command touches a station; `data/run/` is not rebuilt.
- **Check (+212 s, 1792001063):** `is-active` → `active`; samples router 4, living-room 4, mobile 4, 0 failures; `/healthz` 200 `{"last_poll_age_s":23}`; `systemd-inhibit --list` shows `v0.4 acceptance`; journal `serving … stations=3 interval_s=60`, no WARN. **Passes.**

**reviewer:** risky, before commit. INTENT satisfied, 0 CRITICAL / 0 HIGH / 0 MEDIUM (binary `cmp`, smoke and run invocations, DB counts, ETA arithmetic re-checked read-only).
**Next:** 18d — step 18 as written once `now ≥ 1792044051`; before then `<<LOOP:BLOCKED>>` with the ETA.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T18:06:10Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-ceece4274e26e53a)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.3% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 18d ⏸ 2026-10-14 — 12 h run 3 still open; A25–A28 settled meanwhile

**Window:** now 1792001194 (18:06:34Z) < ETA 1792044051 (2026-10-15T06:00:51Z). `bluetti-acceptance-24h` `active`. Unit not stopped, no station touched, `data/run/` not rebuilt.

**Changed (doc only, no `src/`):**
- A25: `research-ble-reliability.md` second-adapter section — dated superseding note; the ~17 s per-poll window claim struck.
- A26: `WORKFLOW.md` edge case "Gate fails" now matches PROMPT §7 (attempts 1–2 continue, third → `<<LOOP:GATE_FAILED>>`, which wins over the 3-attempt BLOCKED hatch on a gate step). `loop/STATE.md` decision line for the owner's 2026-10-14 rule.
- A27: corrections to step 18a (that entry stays as written):
  - (a) Mode C's "every scan heard < 0.3 s" covers 44 scans; 46 of 90 polls were BlueZ cache hits (mobile 28/30, living-room 16/30). The page's "every poll uncached" line is corrected; ROADMAP §5.4 already says it for the EB3A only.
  - (b) The early stop ranged 0.26–1.08 s (17:34:01.352 → 02.429), not 0.26–0.80 s. Two sessions not started by a poll (17:24:19, 17:25:10) stayed on 42 s and 51 s.
  - (c) The fresh start that worked after a miss is 17:19, not 17:18.
- A28: fixed (18b + 18b2 done).
- Open: A22 (closes at 18d's verdict), A23 (its persistent-link clause depends on run 3 < 99 %).

**Check:** `make amendments` → 2 open (A22, A23), both settled by 18d. `make check` via step-done.

**reviewer:** before commit. INTENT satisfied, 0 CRITICAL / 0 HIGH / 0 MEDIUM; A27(c) taken from the amendment text (raw capture not re-read).

**Next:** 18d once `now ≥ 1792044051`: stop the unit, `scripts/acceptance_v04.py data/acceptance-24h.db 1792000851 1792044051 60` (≥ 684/720 per station), CF-5 from the DB if needed, H4 note on any house outage.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-14T18:08:59Z

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
