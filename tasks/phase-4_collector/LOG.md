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
