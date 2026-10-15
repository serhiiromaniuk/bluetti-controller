# Outcome

> Written when the task closes — done **or** blocked.

## Summary
Milestone M3 is done. `bluetti-controller serve` polls every configured station
in sequence over local BLE and stores samples, `poll_failure` rows, hourly
register snapshots and 5-minute rollups in SQLite through one writer task. It
derives the grid and house outage log and the rollup energy from that data, and
serves `/healthz`. The runtime forecast and the daily totals are tested pure
functions (`metrics::runtime`, `energy::totals`, `house::totals`) with no
production caller yet; the M4 views wire them up. A restart keeps open events and rollup
progress. The v0.4 gate passes on its objective part: `make check` is green
(`src/protocol/` 99.82 %, `src/metrics/` 99.3 %), and the 12 h run 4 has no
collector miss outside the owner's manual switch-off. The DB grows 0.341 MB per
station-day (max 10). The unplug test is deferred to `HUMAN-CHECKLIST.md` H4.

**Gate evidence scope (owner decision 2026-10-15 13:35, `loop/STATE.md`).** Run 4
counts with the owner's 02:18–02:31 switch-off excluded. Full window: EB3A
712/720, AC180 710/720, every failure inside the switch-off. Outside it (LOG step
18d, scheduled polls, capped at 100 %): EB3A 152/151 before + 555/555 after,
AC180 151/151 + 556/555, Elite 200 V2 151/151 before. The Elite was off after the switch-off, so its
ratio covers 2 h 31 min, not 12 h. The 95 % threshold is unchanged; the owner
accepted this evidence instead of a new 12 h run.

## Deliverables
- Commits (code and key records, oldest first; full list `git log 2a8fc29^..v0.4-collector`):
  - `2a8fc29`, `81e012e` — task open, PLAN (19 steps)
  - `7daabfa` — M3 design record in ROADMAP (step 1), CF-5 discharged on the Elite
  - `04e6ba7`, `cc2ea0b` — SQLite schema migration 001, store writes and station health (steps 2–3)
  - `e738c3b`, `652b791`, `e8fe9f2`, `d4886f4`, `9eb8226` — metrics: grid debounce, house outage log, runtime forecast, energy (steps 4–7)
  - `4613a40`, `b97d1f6` — writer task with restart-safe events; rollup, retention, compaction (steps 8–9)
  - `f4838f8`, `d0483f3`, `696ad94` — poll loop, hourly snapshot, replay end-to-end test (steps 10–12)
  - `b516090` — `feat(web): serve subcommand with /healthz and signal shutdown` (step 13)
  - `d83f562`, `886d22b` — `find` ends the scan once heard (A15); hardware smoke 2 (step 14)
  - `65f8c3f`, `e616dfd` — 12 h run start, acceptance script (steps 15–16)
  - `32e1653`, `848dae2`, `87dc057`, `dee0f64`, `53774c9` — docs close-out and amendment fixes (step 17a–17e)
  - `823d402`, `0cdaa3f`, `7b4cbf4` — run FAIL, BLE reliability research, discover-miss cause (step 18, 18a)
  - `795a34e`, `15dfa92`, `df66955` — LE discovery restart, key-exchange retry, A30 (18b, 18b2)
  - `9415894` — v0.4 run is 12 h (owner); `7799fe7`, `c057c9f` — gate-miss rule, ~100 % goal (owner)
  - `4d2d992`, `2f39c31`, `742761d`, `6cc1dbf`, `65c7f56`, `8add7e9` — smokes, runs 3–4, owner decision, measurement (18c–18d)
  - `cdf4116` — phase gate (step 19)
- Files / artifacts: `src/store/`, `src/collector/`, `src/metrics/`, `src/web/`,
  `src/cli/serve.rs`, `migrations/001_*.sql`, `tests/collector.rs`,
  `scripts/acceptance_v04.py`, `tasks/phase-4_collector/research-ble-reliability.md`,
  ROADMAP.md §5.2, §5.4, §6, §7, §12 M3, `HUMAN-CHECKLIST.md` H4–H8.
- Milestone tag: `v0.4-collector`

## Pending / Follow-up
- [ ] Owner: H4 — a candidate real grid outage is on record (close-review M2):
      `grid_event` 7 in `data/acceptance-24h.db`, house outage from 1792055214
      (2026-10-15 12:06:54 EEST), after the 12 h window but inside the run. EB3A
      221.0 V → 0.0 V, AC180 228.7 → 2.3 V; the event is written at 1792055274,
      120 s after the last grid sample (RULES.md: ≤ 2 min). `grid_event` 3
      (02:03:54 EEST) overlaps the owner's switch-off. The owner judges; the loop
      does not mark H4.
- [ ] Owner: H5 (AC180 grid-input scale), H6 (optional USB adapter), H7 (AC180
      stays on and in range), H8 (laptop charger on a station outlet).
- [ ] CF-6 → phase 5: `loginctl enable-linger` before the 7-day soak.
- [ ] The Elite 200 V2 poll ratio rests on 2 h 31 min of run 4; the phase-5
      7-day soak measures it over the long run.
- [ ] Phase 5 (M4): wire `metrics::runtime`, `energy::totals` and
      `house::totals` into the views (close-review M1).
- [ ] Open phase 5 (web UI + deploy + 7-day soak — M4 + M5, `v1.0-mvp`).

## Amendments
- A1–A17, A20–A31 — fixed during the task; each disposition is in its PLAN entry.
- A18 — re-targeted → step 18; settled there: run 4 has no `discover` miss
  outside the owner's switch-off (LOG step 18d).
- A19 — re-targeted → CF-6 (phase 5, M5 user unit).
- `make amendments` → 31 finding(s), all disposed.

## Lessons / Notes
- BlueZ ended LE discovery 0.26–1.08 s after `StartDiscovery` inside `serve`, so
  a station not heard in that window missed at 15 s. Timing time-to-heard per
  station (18a) found the cause; a longer timeout would have hidden it.
- A 12 h run on a laptop is exposed to the host: a flat battery killed run 3
  and the owner switched the stations off in run 4. Power the host from a
  station (H8) and agree a quiet window with the owner before a long run.
- A late second reviewer report after the commit (A14) still needed an
  amendment; wait for the reviewer's final report before the commit.
- No grader verdict was rejected as false this task.

<!-- Calibration proposals for the owner (AGENTS.md §8b): -->
- reviewer · missed: a completed pinned future awaited again after a `select!`
  arm already finished it (`serve.rs`, A14) — absent from the first step-13
  report, found only in a late second one.
