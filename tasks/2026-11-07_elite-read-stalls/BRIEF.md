# Elite read stalls  |  2026-11-07_elite-read-stalls

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
Find why the Elite 200 V2 (`living-room`) stops answering reads mid-poll
(~5 "dropped while connected" a day), and stop the drops or prove they are the
station's own.

## Why
Re-targeted from `2026-10-27_ble-reliability` done-when 4 by the owner
(2026-11-07). 34 events on 2026-11-01 – 11-06: 33 on the Elite (3× "no
notification within 5s", then the link drops; the next poll succeeds) and 1 on
the EB3A during a burst of failed connects (11-04 19:56Z, in a grid outage
with the host on battery; the owner excluded it, not in scope here). The AC180 shows none. Golden rule #5; the owner's goal is ~100 % polls.

## Scope
**In:** read-only diagnosis on the host (journal, `btmon` during an event
when the owner allows it), community tools' handling of the same symptom, a
host-side fix if one exists.
**Out:** the station going fully invisible (`BUG-station-ble-invisible.md`,
owner-handled); any write frame.

## Done when
- [ ] The cause is named with evidence (journal + one captured event), recorded in LOG.md
- [ ] Either a fix ships and the host shows zero "dropped while connected" over 24 h with the Elite on,
      or the evidence shows the station stops answering by itself and the owner accepts it on record
      after the event captured in done-when 1 (the 2026-11-07 verdict that opened this task does not count)
- [ ] `make check` green for any code change

## References
- Governing spec: `ROADMAP.md`
- `tasks/2026-10-27_ble-reliability/` (LOG.md last two entries, BUG-station-ble-invisible.md)
- RULES.md golden rule #5
