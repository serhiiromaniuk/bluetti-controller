# Plan

Run as one Workflow: three builders in their own worktrees, each followed by a
reviewer and, on a CRITICAL/HIGH finding, a fixer. Then merge, gate, release,
deploy (owner approved the deploy).

## Questions for the owner
None.

## Steps
- [ ] 1. [parallel: A] Cards: temperature °C, mode name, station time left (status.rs, card markup) — check: `make check`, card tests.
- [ ] 2. [parallel: A] Chart: `temp_c` in `/api/chart`, 4th panel (chart.rs, app.js `PANELS`, panel markup) — check: `make check`, chart test, JS budget.
- [ ] 3. [parallel: A] Energy: counter deltas per station-day + lifetime line (history.rs, pages.rs, energy markup) — check: `make check`, hand-computed test.
- [ ] 4. Merge the three branches, `make check`, CHANGELOG 1.3.0, `make release VERSION=1.3.0`, `make deploy` — check: `/healthz` v1.3.0, page shows the new parts.

## Risks / Dependencies
- All three edit `templates/status.html`, in separate sections: expect clean or trivial merges.
- `app.js` has ~1.1 KB of budget left: the chart adds one `PANELS` entry, nothing more.
- Extras are unverified: every new value carries the existing unverified styling or text.

## Escape hatches
- A builder fails `make check` 3 times: drop that feature from 1.3.0, ship the rest, log it.

## Amendments
