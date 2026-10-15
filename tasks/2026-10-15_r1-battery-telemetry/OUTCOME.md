# Outcome — R1 battery health + R1a telemetry extras

**Status: done (2026-10-15).** v1.2.0 runs on the homelab host.

## Deliverables
- dd67149: ROADMAP §2.2 decision record and the task plan.
- aa4c9cd (R1): `metrics::capacity`, `store::read::discharge_buckets`, and the "Battery health" block on `/telemetry`.
- 63f1849 (R1a): V2 extras, u32 fields, slow blocks every 10 min that never fail a poll, and `read` printing the extras.
- 23836cd: the reviewer fixes (ranges in the decision text, slot timed from poll start, gappy buckets split a run), CHANGELOG 1.2.0 and HUMAN-CHECKLIST H12.
- ef605a2 / `v1.2.0`: the release, deployed once with no rollback.

## Follow-ups
- H12 (owner): compare the extras with the app. Is the temperature in °F? 104 and 105 read the same value; the AC180 AC-out counter reads 0.
- The capacity factor depends on load: low-load discharges (~20 W on the AC180) give a low factor from inverter overhead. Judge it only after a few discharges at normal load.
- SOH register: none known. A read-only sweep of the 6000 block stays an R1 discovery item.
- The EB3A has intermittent `le-connection-abort-by-local` connect failures, present before this release too. Ethernet for the host frees the shared radio.
- A1 declined: `device.rs` is 461 lines.

## Lessons / Notes
- reviewer · caught: a schedule timed from the poll end slips one poll per slot, and paused-time tests hid it. Give fakes a non-zero poll duration.
- The step-5 live read was dropped: it needed a production polling pause, which the owner ruled out. Post-deploy checks of `/telemetry` and the journal covered it.
