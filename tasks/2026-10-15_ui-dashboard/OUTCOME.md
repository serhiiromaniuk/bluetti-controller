# Outcome

## Summary
The status page is now a Grafana-style dashboard: a time picker (last 12 h by
default, quick ranges up to 30 d, custom from/to, drag-to-zoom), manual refresh
by default with optional 1 m–1 h auto-refresh that pauses in a hidden tab, a
dark theme by default with a persisted toggle, compact station cards above three
cross-station charts with house-outage bands, and a data-age line that turns into
a STALE banner after 3 poll intervals. One `/api/chart` request feeds every chart;
with refresh off, the page makes no request after load.

## Deliverables
- Commits: `0e760d3` backend (`/api/chart`, no-data days, meter-floor forecast text) · `66c597e` frontend · `f9c90dc` deploy + Chrome evidence
- Deployed: `~/.local/bin/bluetti-controller`, user unit restarted
- Milestone tag: none (ad hoc task)

## Pending / Follow-up
- [ ] Owner: phone + look check, `HUMAN-CHECKLIST.md` H9 (rewritten for the new UI).
- [ ] Homelab install will pick this build up with v1.0's interactive install.

## Amendments
None — every reviewer finding (step 1: 3 MEDIUM; step 2: 1 HIGH, 4 MEDIUM) was fixed in its own step.

## Lessons / Notes
- An open page used to make an estimated 45 chart requests and 3 reloads per minute: the old `app.js` reloaded on each station's `sample` event (seen 4–5 s apart on `/events`), 15 series fetches each. The QA report estimated 15 and 1, assuming one sample group per minute. Not measured with the old build. Now it makes 0 unless the owner refreshes or turns on auto-refresh (measured).
- Page weight budget rebalanced by the owner (ROADMAP §8.4 i): JS 12 000 / CSS 5 000 / HTML 18 000; actual 10 914 / 4 113 / ~7 100.
- uPlot synced charts replay mouseup on every chart in the sync group; a select hook must check `u.cursor.event`.
- reviewer · missed: none known. The step-2 HIGH was confirmed independently by the CDP run.
