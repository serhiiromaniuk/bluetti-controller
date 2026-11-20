# Log

## Task opened 2026-11-20 — outage and missing station

- User requested both problems investigated and fixes deployed under repo rules.
- Plan review: 2 HIGH fixed before work: oracle-readable questions and explicit release/deployment checks.
- Read-only evidence: EB3A input dropped 19 Nov 20:49:48; AC180 simultaneously measured mains but does not vote. Elite last sampled 17 Nov 19:34:54; subsequent discovery fails.
- The false house event persists on restart and overlaps distinct AC180 input-loss intervals. No production changes or historical edits made.
- Human-only boundary and physical status limit live verification; questions requested in chat.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-20T15:30:41Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-11-20 — traced house vote and missing Elite

- **Evidence:** read-only SSH config/SQLite query; timezone Europe/Kyiv; `grid_voltage_min=180`, debounce 2, poll interval 60 s (`ROADMAP.md` §7.1–7.2).
- Sensors: router EB3A and living-room Elite enabled; mobile AC180 disabled. Router 220 V at 20:48:48 → 0 V at 20:49:48 and 20:50:48 on 19 Nov. AC180 222.0 V at 20:49:54, 221.5 V at 20:50:56.
- DB opened router and house events 19 Nov 20:49:48; both remain open. AC180 events: 20 Nov 07:25:54–09:13:54 and 15:07:54–open. Input loss is not independently proof of a house outage.
- Elite last sample 17 Nov 19:34:54 (220.4 V); latest failure 20 Nov 17:31:54 at discovery. At 17:35 mobile read 2.2 V and router 0 V.
- `src/metrics/house.rs`: absent plus unknown votes outage; `src/store/writer.rs`: restores open house events; `src/web/status.rs` and `templates/status.html` include every station card. Whether the missing UI item is a card or chart remains unverified.
- **Check:** observed values and sensor roles reproduce the vote and banner; configured event persistence explains restart behavior. No authenticated UI or physical Elite check available.
- **reviewer:** batched (read-only evidence / prose). **Decision:** none; sensor role and historical correction depend on physical context.
- **Next:** step 2, reproduce the indicated missing UI view when Q2 is answered.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-20T15:37:18Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 2026-11-20 — show unknown when every sensor is stale

- **Changed:** `src/web/status.rs`, tests and `ROADMAP.md` §8.4: show unknown if every configured grid sensor is unknown; keep any open outage event unchanged (§6).
- **Check:** `make test` first failed the new stale-sensor regression (outage vs unknown); `make check` then passed, including the regression, formatting, clippy and coverage.
- **Evidence:** Elite discovery failures since 17 Nov start at discovery; cached BlueZ target absent. No claim that device is off or on. Original false event still needs sensor-role correction and history reconciliation.
- **Lenses:** engineer: current state separated from persisted event; risk: no device or production changes; security: no credentials touched; future: chart uptime gap recorded as amendment A1.
- **reviewer:** 0 critical / 0 high / 0 medium; original false event and A1 remain separate.


<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-20T15:48:17Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->

## A1 mitigation 2026-11-20 — qualify recorded outages

- **Changed:** dashboard summary no longer presents an unsupported uptime percentage; chart summary and daily house table identify event counts and durations as records that can be inaccurate when sensor coverage or settings change.
- **Evidence:** `station.grid_sensor` is overwritten on sync; old raw readings are purged and rollups omit exact grid readings. Historical sensor coverage cannot be reconstructed reliably.
- **Check:** `node --check assets/app.js` and Node VM scenarios for no events and an ongoing event pass; `make check` green. A1 remains open for actual coverage accounting.
- **Lenses:** engineer: retain event data and API; risk: do not imply complete coverage; security: no secrets touched; future: precise uptime needs timestamped sensor roles and known/unknown intervals.
- **reviewer:** 0 critical / 0 high / 0 medium; live UI not inspected.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-20T15:52:14Z

```text

    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> src/web/guard.rs line coverage 100% (min 100%)
>> src/web/headers.rs line coverage 100% (min 100%)
>> src/web/auth/mod.rs line coverage 100% (min 100%)
>> src/web/auth/session.rs line coverage 100% (min 100%)
>> src/web/auth/password.rs line coverage 100% (min 100%)
>> src/web/api/window.rs line coverage 100% (min 100%)
>> src/config/web.rs line coverage 100% (min 100%)
>> src/config/mode.rs line coverage 100% (min 100%)
>> src/cli/hash_password.rs line coverage 100% (min 100%)
>> src/cli/printable.rs line coverage 100% (min 100%)
>> make check: all gates green
```

<!-- gate:end -->
