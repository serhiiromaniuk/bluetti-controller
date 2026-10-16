# Log

## 2026-10-16 — task open
- Decision: ROADMAP.md §2.2 R1b. Plan: one workflow, three worktree builders + review + fix.

## 2026-10-16 — workflow, merge, release, deploy
- Workflow wf_194a28f3-23b ran 3 builders, each in its own worktree, then a reviewer on each. All 3 `make check` green.
- Reviews: cards clean; energy clean; chart had 1 MEDIUM (the y axis clipped readings below 0 °C), fixed during the merge (`Math.min(0, lo)`).
- Merge: one conflict in `pages/tests.rs` (two new tests side by side); kept both. The card test counted "Pack " and also matched the new chart heading, so it now counts only the card line. `make check` green.
- v1.3.0 released and deployed: healthy. Cards, counter columns and lifetime lines confirmed live; `/api/chart` `temp_c` populated.
- Browser check found the temperature chart empty: the browser was running a cached 1.2.0 `app.js` (`max-age=86400`, fixed URL). Fix: asset URLs carry `?v=<build>`, with a test. Released and deployed as v1.3.1.
- After 1.3.1, in Chrome: all 4 charts draw, and the scripts load `?v=v1.3.1`.
- Finding: the AC180's counters (150–159) all read 0, so its counter columns show 0. The Elite's lifetime counters read in 227.8 kWh, out 198.5 kWh, ratio 87 %.
