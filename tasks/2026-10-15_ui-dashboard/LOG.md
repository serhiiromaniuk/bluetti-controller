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

## 2026-10-15 18:10 — Session start
Context loaded per `loop/where.sh --json`. Starting at step 1.

<!-- Entry template — copy, fill, keep it under 40 lines:

## Step <N> <✓|blocked> YYYY-MM-DD — <the headline result, one line>

**Changed:** <file/module — what, one line each>

**Check:** <the check that proves this step> — failed first for <the intended
reason>, then passed. <or, when it is green-on-arrival: the mutation that proves
the assertion has teeth, in one line>

**Evidence:** written below this entry by `loop/step-done.sh` (the gate tail,
fenced in `gate:begin/gate:end`). Add by hand only what no script captures: a
smoke result, measured acceptance numbers — the decisive lines.

**reviewer:** <n> CRITICAL / <n> HIGH / <n> MEDIUM · or `batched` (low-risk,
reviewed with the next batch before the push)
- CRITICAL|HIGH <id>: <finding> → fixed in <where>.
- MEDIUM <id>: <finding> → deferred to PLAN `## Amendments` A<n>.

**Decision:** <a choice the next iteration must not relitigate · none>
**Carry-forward:** <raised/discharged in loop/STATE.md · none>
**Next:** step <N+1> — <title>.

Commit: via `loop/step-done.sh --commit "<type>(<scope>): <subject>"`   ·   Marker: <<LOOP:CONTINUE>>
-->

## Step 1 ✓ 2026-10-15 — `/api/chart` batched endpoint, no-data energy days, meter-floor forecast text

**Changed:**
- `src/web/chart.rs` (new) — per-station SOC / in W / out W on one axis + window outages; step raised to ≤ 1000 buckets, multiple of 300 s over 48 h; values rounded to 0.1.
- `src/web/api.rs` — `GET /api/chart`; 400 on `from < 0`, span > 366 d, step outside `1..=span`.
- `src/web/history.rs` — `StationEnergy.no_data`; a day without samples is all missing.
- `src/web/pages.rs` + `templates/status.html` — `?range` dropped; no-data rows show "—"/"no data"; fresh 0 W output explains the missing forecast.

**Check:** `make check` — new tests `chart::tests::*`, `api::tests::chart_returns_every_station_and_rejects_bad_input`, `history::tests::a_day_without_samples_is_no_data_all_missing`, `pages::tests::zero_output_says_why_there_is_no_forecast`. First run red on the 24 h step (87 s, uneven buckets) → rounded to a multiple of the poll interval.

**reviewer:** 0 CRITICAL / 0 HIGH / 3 MEDIUM — all fixed in this step:
- M1 step 180/240 s over rollup rows (48–67 h windows) → base rounded to 300 s; 50 h test.
- M2 unchecked i64 on raw query values → bounded in the handler; 3 overflow cases tested.
- M3 floor text on a stale 0 W → only with a known grid state; stale case tested.

**Decision:** charts take one request per refresh; `/api/series` and `/events` stay as API.
**Carry-forward:** none
**Next:** step 2 — frontend.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T15:17:00Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
