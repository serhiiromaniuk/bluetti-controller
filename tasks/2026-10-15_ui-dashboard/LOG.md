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

## Step 2 ✓ 2026-10-15 — Grafana-style dashboard: time/refresh pickers, dark theme, cross-station panels

**Changed:**
- `templates/base.html` — dark by default, "Theme" toggle (localStorage), `.js` class gates JS-only controls, empty inline favicon.
- `templates/status.html` — tools (range: 1 h…30 d + custom, Refresh, Auto off/1m…1h), `#age`, compact cards without charts, 3 panels (Battery %, Output W, Input W), `#live`/`#tables` swap targets.
- `assets/app.js` — one `/api/chart` per load/refresh; URL state `from/to/refresh`; drag-zoom → absolute range; house-outage bands; synced cursors; gap breaks > 3 steps; age ticker on the server clock (STALE after 3 intervals); hidden tab skips ticks, catches up once. SSE no longer used by the page.
- `assets/app.css` — CSS variables, dark + light.
- `src/web/pages.rs` — `newest_ts/at`, `now`, `stale_s`; a day with no station data is one row, house "no data".

**Check:** `make check` green. Telemetry "no JS" test now means no script file (inline theme only).

**reviewer:** 0 CRITICAL / 1 HIGH / 3 MEDIUM — all fixed:
- HIGH H1 drag-zoom fired 3 requests (synced charts replay mouseup) → only the chart with `cursor.event` zooms; CDP run: 1 request.
- MEDIUM M1 refresh error overwritten by `age()` → `broken` flag.
- MEDIUM M2 `?refresh=30d` overflows setInterval → listed values only.
- MEDIUM M3 stale chart responses → `seq` guard.
- Re-check: fixes hold; new MEDIUM — a recorded house outage on a day without samples read "no data" → blank only when no outage; test `a_recorded_outage_shows_on_a_day_without_samples` (fails on the old rule).

**Decision:** auto-refresh minimum 1 m (= poll interval); default off.
**Next:** step 3 — deploy + Chrome evidence.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T15:30:54Z

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

## Step 3 ✓ 2026-10-15 — deployed to the user unit, verified in headless Chrome

**Changed:** `~/.local/bin/bluetti-controller` ← release build of 66c597e; `systemctl --user restart bluetti-controller` → active. v1.0 binary kept in the session scratchpad for rollback.

**Check:** Chrome 155 headless over raw CDP (Claude-in-Chrome extension not connected), live server, network log per action:
- load: `/` + 4 assets + ONE `/api/chart` (12 h); dark, "Last 12 hours", auto off; 3 charts × 3 stations; cards end at y=388 px (all 3 above the fold at 1280×900).
- idle 70 s, refresh off: 0 requests. Refresh click: `/` + `/api/chart`.
- theme toggle → light, persists across reload; telemetry page themed.
- range 7 d: 1 request, URL `?from=now-7d&to=now`. Drag-zoom: 1 request, select → Custom, absolute ms in URL. Custom apply: 1 request.
- auto 1m: 2 requests per minute visible; 0 in 65 s hidden; 1 catch-up refresh on visible.
- 390 px: scrollWidth 390 (no horizontal scroll). No-JS: tools/charts hidden, cards + tables render.
- console errors: none (favicon 404 gone).

**Evidence:** server side, curl on the live unit: `/` 7078 B in 1.4 ms; `/api/chart` 12 h 16 354 B in 1.9 ms, 7 d 1831 B, 30 d 1118 B (< 1 ms); RSS 12.1 MB.

**reviewer:** n/a (no code change; verification only).
**Next:** close the task.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T15:35:19Z

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

## Close ✓ 2026-10-15 — real-Chrome check, close review fixed, budget rebalanced by the owner

**Chrome (owner's browser, Claude-in-Chrome, live unit):** load = `/` + 4 assets + ONE `/api/chart`; 0 localhost requests in 30 s idle; Refresh = `/` + `/api/chart`; hover legend synced across 3 panels; Theme toggles dark/light with no request; Last 3 hours → 1 request, URL `?from=now-3h&to=now`; drag-zoom 4:00–4:40 → 1 request, picker shows Custom with both times; Auto 1m → exactly 1 refresh in 63 s; forced old `data-ts` → red "STALE — no new sample for 11 min" within one 30 s tick; telemetry themed; 420 px window: no horizontal scroll; console errors: none. Pause not pressed (it would stop polling).

**close-reviewer:** hold — 1 HIGH / 2 MEDIUM, fixed:
- HIGH spec/README described the old UI; app.js 10 914 B > §8.4 i's 6 000 B. Owner chose to rebalance (same 87 938 B sum, 100 KB limit unchanged): JS 12 000 / CSS 5 000 / HTML 18 000, now enforced by `web::tests::assets_stay_within_the_page_weight_budget` and the HTML test (25 000 → 18 000). §8.1, §8.2, §8.4 i–j and README updated, `/api/chart` in both route tables.
- MEDIUM H9 described the 24 h / 7 d toggle → rewritten for the new UI and the look check; OUTCOME points to it.
- MEDIUM OUTCOME's "45 requests/min" stated as fact → marked as an estimate with its basis.

**Decision:** page-weight split JS 12 000 / CSS 5 000 / HTML 18 000 (owner, 2026-10-15, ROADMAP §8.4 i).
