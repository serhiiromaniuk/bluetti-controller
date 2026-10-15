# Outcome

> Written when the task closes — done **or** blocked.

## Summary
Milestones M4 and M5 are done. `bluetti-controller serve` now serves a read-only
LAN web UI from the collector's database: a status page (house banner, station
cards with the runtime forecast, outage log, 7-day energy table, 24 h / 7 d
charts), a telemetry page with every stored metric, the JSON API, SSE `/events`
and a pause-polling POST (0–60 min, optional bearer token, never a write frame).
The service ships as a user-level systemd unit with `deploy/INSTALL.md` and a
`make dist` tarball; it runs on the dev laptop with `Linger=yes` (CF-6). The
v1.0 gate passes: `make check` green (`src/protocol/` 99.82 %, `src/metrics/`
99.4 %), and the 1 h run 12:13:12–13:13:12 UTC measured EB3A 60/60, Elite 200 V2
60/60, AC180 59/60 (98.3 %), `VmHWM` 12.3 MB, every page and API route ≤ 2.1 ms
on the LAN (pause POST ≤ 0.36 ms), 0 manual interventions.

**Scope change (owner, 2026-10-15, PLAN A7).** The 7-day soak became a 1 h
functional run (RULES.md v1.0). Steps 17 and 20 were declined; the homelab
install follows v1.0 in an interactive session.

## Deliverables
- Commits (oldest first; full list `git log v0.4-collector..v1.0-mvp`):
  - `74ab2f3`, `a4b43ee` — task open, PLAN (22 steps, plan-review fixes)
  - `a297bae` — M4/M5 design record: ROADMAP §8.4, §10.3 M5, ADR-009 jiff (step 1)
  - `7aa8187` — config timezone (jiff), token semantics, local-time helpers (step 2)
  - `ccb9341`, `bada35c` — store read API; writer publishes `Update`s (steps 3–4)
  - `20ddb29` — `AppState`, routes as 501 stubs, askama + futures-util (step 5)
  - `0e37f2a`, `b369238`, `05f2dc7`, `c6a193b` — group A: status view, history view, assets with uPlot 1.6.32 (steps 6–8)
  - `ddd4a8a`, `ad98405`, `0a84c78`, `77c262c`, `ca63187`, `ec58752`, `f1d9be9` — group B: JSON API, status page, SSE, pause POST (steps 9–12)
  - `ba7a549` — telemetry page (step 13)
  - `4ff329d` — UI smoke: page weight 65.7 KB, LAN latency, H9 (step 14)
  - `3ab9066` — M4 close-out, boundary greps, A10/A11 (step 15)
  - `d900065` — user unit, INSTALL.md, `make dist`; step 17 declined (step 16)
  - `e489bec` — unit installed, linger on (CF-6), 1 h run started (step 18)
  - `e42cfee` — v1.0 gate is a 1 h run (owner, A7)
  - `d0d4437` — M5 docs (step 19); `2e8d9bf` — step 20 declined
  - `9d7945e`, `0d30228` — 1 h run measured (step 21)
  - `5ef5ce1`, `94b79b2` — phase gate and the pause-POST timing (step 22)
- Files / artifacts: `src/web/`, `src/store/read.rs`, `src/store/writer.rs`,
  `src/config.rs`, `src/metrics/localtime.rs`, `templates/`, `assets/`,
  `deploy/bluetti-controller.service`, `deploy/INSTALL.md`, `Makefile`
  (`build`, `dist`), ROADMAP §8.4, §10.3, §10.5, §12 M4/M5, ADR-009, README
  (API, Deploy, uPlot credit), `HUMAN-CHECKLIST.md` H9–H10.
- Milestone tag: `v1.0-mvp`

## Pending / Follow-up
- [ ] Owner: H1–H3 (app comparison, then `verified: true` on the core fields),
      H4 (outage test — candidates: phase-4 `grid_event` 7, and the real house
      outage 12:13:12–12:27:12 UTC in this run's DB, step 21 LOG), H5–H8.
- [ ] Owner: H9 (phone UI and pause with the official app), H10 (note any manual
      intervention during the 1 h run; the loop saw none).
- [ ] Homelab install (ROADMAP §10.2, hardened system unit) — human-only, after
      v1.0, interactive session (owner, A7). Long-run stability is proven there.
- [ ] The DB-size rate cannot be measured in 1 h (pre-checkpoint WAL, step 21);
      phase 4's 0.341 MB per station-day stays the evidence.
- [ ] Energy table coverage (close-review M1, `src/web/history.rs:73`): a day
      with no samples, and the part of a day before the first or after the last
      sample, show `missing 0 s` — the live table claims full coverage for
      2026-10-09..14 although the DB starts 2026-10-15 12:13 UTC. Count those
      edges and empty days as missing, or render "no data". Fix task after v1.0.
- [ ] The user unit keeps running on the dev laptop (step 21, Q7); stop it with
      `systemctl --user stop bluetti-controller` when no longer wanted.

## Amendments
- A1–A6, A8–A14 — fixed during the task; each disposition is in its PLAN entry.
- A7 — owner decision (1 h run instead of the 7-day soak), applied: steps 17/20
  declined, 18/21 run and measured the 1 h run.
- `make amendments` → 14 finding(s), all disposed.

## Lessons / Notes
- Owner decisions that change a gate mid-task (A7) are cheapest as one PLAN
  amendment that names every affected step's new fate; the later steps then
  read as written with "(declined — A7)" lines, and no step text is rewritten.
- A test clock that mixed the real `unix_now()` into paused tokio time (A11)
  failed 1 run in ~400. Looping the gate and keeping its output named it; one
  red gate with the output lost cost a whole investigation step.
- A measurement step must time every route the gate names, including the ones
  that change state: step 21 skipped the pause POST, and only the batched
  reviewer caught it (H1, fixed with a harmless `minutes=0`).
- No grader verdict was rejected as false this task.

<!-- Calibration proposals for the owner (AGENTS.md §8b): -->
- reviewer · missed: a coverage column that reports 0 s missing for windows
  with no samples (`history::energy`, step 7) — it read the §7.4 gap rule as
  sufficient; the close-reviewer found it on the live page.
