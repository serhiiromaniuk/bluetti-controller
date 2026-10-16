# Brief — R1b dashboard panels for the R1a extras

## What
Show the useful R1a extras on the dashboard (ROADMAP.md §2.2 R1b): card lines,
a temperature chart and counter-based energy in the 7-day table. One release
(1.3.0), one deploy.

## Why
Owner, 2026-10-16: "yes, go. parallel plan, make dynamic workflow and deploy at
once" (recorded at the owner's request), after the pick of cards, temperature
chart and counter energy.

## Scope
In: the three features exactly as in §2.2 R1b; tests; CHANGELOG 1.3.0; deploy.
Out: `charging_status`, `feedback_energy`, any budget raise, any schema change.

## Done when
- [ ] `make check` green on the merged tree, page-weight budget unchanged.
- [ ] v1.3.0 deployed once, `/healthz` reports it, the status page renders the
      new card lines, the 4th chart and the counter columns on production.

## Refs
ROADMAP.md §2.2 (R1a, R1b), §8.4 (status page), `src/web/{status,chart,history,pages}.rs`.
