# Log

## 2026-10-15 23:30 — task open, plan, decision
- Owner asked for R1 + R1a in parallel, direct work, one deploy, no production downtime.
- Decision record: ROADMAP.md §2.2 (extras never fail a sample; slow blocks every 10 min, one try, half the deadline; capacity from `rollup_5m` segments ≥ 10 SOC points).
- No migration needed: extras use the existing `sample_extra` table, so rollback stays a binary swap.
- A copy of the production DB to calibrate the estimator was denied by the permission classifier; the estimator is tested on hand-computed synthetic rollups instead.
