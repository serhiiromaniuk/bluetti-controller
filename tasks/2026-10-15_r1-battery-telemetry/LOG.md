# Log

## 2026-10-15 23:30 — task open, plan, decision
- Owner asked for R1 + R1a in parallel, direct work, one deploy, no production downtime.
- Decision record: ROADMAP.md §2.2 (extras never fail a sample; slow blocks every 10 min, one try, half the deadline; capacity from `rollup_5m` segments ≥ 10 SOC points).
- No migration needed: extras use the existing `sample_extra` table, so rollback stays a binary swap.
- A copy of the production DB to calibrate the estimator was denied by the permission classifier; the estimator is tested on hand-computed synthetic rollups instead.

## 2026-10-15 23:55 — steps 2 + 3 (parallel), step 4 merge
- Step 3 (R1) ran in a worktree subagent: `metrics::capacity` (segments, estimate), `store::read::discharge_buckets`, "Battery health" on `/telemetry`; 6 hand-computed tests + a page test (aa4c9cd).
- Step 2 (R1a) in the main tree (63f1849): `Field::wide` (u32, low word first), `Profile::extras` / `slow_blocks` / `SLOW_EVERY_S`, `transport::read::read_slow` (one try per block), collector slow schedule cut at half the deadline, `read` prints extras.
- New tests: u32 decode on the Elite dump values, a bad or missing extra is never a problem, a garbled slow block gets no retry, slow blocks on poll 1 and 11 only, a refused slow block drops only its extras, a stalled slow block is cut at 30 s and the sample kept.
- Fixtures: registers 104 and 105 read the same value on both V2 stations (AC180 6376, Elite 228 min); kept as two metrics until H12.
- Merge clean (ce2f7c7); `make check` green on the merged tree: protocol 98.86 %, metrics 99.48 %.
- Step 5 changed: no laptop live read. It needs a production polling pause (a data gap), and the owner asked for no interruption. The deploy's health check is the live check, and `/telemetry` after deploy shows the extras. Slow-block failures cannot fail a poll (tests above).
- HUMAN-CHECKLIST H12 added (extras vs the app); CHANGELOG 1.2.0 written.

## 2026-10-16 00:20 — step 4 review
- Reviewer (batch e4bf48b..ce2f7c7): 0 CRITICAL, 1 HIGH, 2 MEDIUM, 1 MEDIUM dropped.
- HIGH: §2.2 lacked the extras' ranges. Fixed: the ranges and the slot rule are now in the decision text.
- MEDIUM: the slow slot slipped to every 11th poll, because it was timed from the poll end. Fixed: it is timed from poll start to poll start, with half an interval of slack. The test now has a 5 s poll release.
- MEDIUM: a bucket with a long poll gap undercounts energy. Fixed: `discharge_buckets` requires `n ≥ 300/interval − 1`. The page test now has a gappy run that splits.
- Dropped MEDIUM: device.rs is 461 lines. Logged as A1, declined.
- No write frames, rollback safe (no migration), no 500 path found. `make check` green.

## 2026-10-15 23:55 host time — steps 5 + 6, release and deploy
- `make release VERSION=1.2.0` (ef605a2, tag v1.2.0), pushed.
- `make deploy HOST=debian@192.168.0.192`: preflight ok (schema v1 = v1), backup, "healthy: v1.2.0 stored 1 sample(s)"; v1.1.1 is now the rollback target. Gap: a restart of about 1 s, so no poll tick was missed.
- 11 rounds watched (to `samples_since_start` = 33): 33 of 34 polls ok. The 1 failure is the EB3A `le-connection-abort-by-local`, also seen on v1.1.1 at 20:29–20:40 UTC. 0 slow-block warnings. RSS 12.4 MB.
- `/telemetry`, Elite: 198.4 kWh AC out, 227.5 kWh grid in, 40.4 kWh feedback, 69 °F, time 5994 min.
- `/telemetry`, AC180: both slow blocks read (6007 = 69 °F; the AC out counter reads 0.0 kWh, unverified, H12).
- Battery health, AC180: ≈ 477 Wh of 1152, factor 0.41, from 1 segment (tonight's ~20 W outage load). The inverter's own draw dominates at a load this low. Not used to change config.
