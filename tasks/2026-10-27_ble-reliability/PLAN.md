# Plan

> Steps from INVESTIGATION.md §3 (Fable, 2026-10-27). Owner: no over-engineering.

## Questions for the owner
- Q1 — deploy 1.3.3 to production when green? · blocks: 4 · answer: yes, owner 2026-10-27 ("go run and fix everything")

## Steps
- [x] 1. One `bluer::Session` per process: a lazily opened shared session in `src/transport/bluer/` used by `find`/`cached`/`release`/`connect`/`le_adapter` — check: `#[ignore]` churn test red on `Session::new` per call, green on the shared session; `make check`
- [x] 2. Poll backoff for unreachable stations in `Collector::run` (skip 1, 2, 4 … ticks, cap 10 min, reset on success) — check: collector test on paused time; `make check`
- [x] 3. One Disconnect per poll: `release` only when `Gatt::disconnect` did not succeed — check: collector test counting `release` calls; `make check`
- [x] 4. Release 1.3.3 and deploy (Q1) — check: `make deploy` exit 0; after ≥ 1 h `systemctl status` Memory < 50 MB, `/api/status` ≥ 95 % per station, no "dropped while connected"

## Risks / Dependencies
- The leak may not be the session (INVESTIGATION R1 ~90 %): step 1's test proves it; if it stays green, stop and profile before more code.
- A shared D-Bus connection that dies would fail every poll: bluer reconnects nothing. Mitigation: the unit restarts on exit; noted as a shortcut in code.

## Escape hatches
- A step fails 3 times: stop, log, hand back to the owner.
- Host config changes (sudo) stay the owner's.

## Amendments
- 2026-10-27 — plan: step 1's "2 h laptop serve" dropped: a laptop `serve` competes with production for each station's one link (owner: no prod interruption). RSS is measured on the host after the deploy instead (step 4).
- 2026-10-27 — plan: step 1 gains `start_discovery` (reviewer H1: a cancelled discovery start strands StartDiscovery on the shared session); the churn and cancel checks share one `#[ignore]` test, since the shared session lives on the runtime of the test that opened it.
