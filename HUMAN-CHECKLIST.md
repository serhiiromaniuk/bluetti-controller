# HUMAN-CHECKLIST.md — physical checks deferred to the owner

The loop never stops for these (RULES.md "Deferred physical checks"). It adds a
row when it reaches one and keeps going. The owner works through the list once,
at the end, and records each result here as `done <date>: <result>`. A failed
row becomes a fix task.

| # | Check | Added by | Result |
|---|-------|----------|--------|
| H1 | EB3A: `read router` vs the official app within seconds — battery % equal, power ±5 W (note the app value, force-quit the app, then run `read`) | owner, 2026-10-13 | pending |
| H2 | Elite 200 V2: the same comparison for `read living-room` | owner, 2026-10-13 | pending |
| H3 | AC180: `read mobile` vs the official app (the loop runs scan, detect and the acceptance run itself since 2026-10-13 14:05) | owner, 2026-10-13 | pending |
| H4 | Outage test: unplug the AC input of a grid-sensor station once during a collector run; recorded within 2 min. Skip if a real grid outage was already recorded | owner, 2026-10-13 | pending |

## Owner observations (for the loop's plausibility checks)

- 2026-10-13 ~12:00–13:00: grid outage. AC input 0 V on both stations is
  correct. The Elite 200 V2 ~445 W output is the owner's PS5 load. The EB3A
  powers only the router on its AC outlet (~9 W expected), yet `read router`
  showed every power at 0 W: check the EB3A AC output power register
  (unverified map, or a low-load floor), before H1.
