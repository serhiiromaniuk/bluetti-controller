# Plan

## Questions for the owner
- Homelab host details (model, arch, Debian version, BT adapter, which station powers it, SSH name). Not blocking: needed only for the owner's install run. answer: asked at the task end, in one batch.

## Steps
- [x] 1. ROADMAP: §2.2 + Appendix B record the telemetry extras as postponed (not approved, owner 2026-10-15); §10.2 and a new §10.6 settle the release layout, versioning, update/rollback flow and health criterion — check: sections written with a recommendation; reviewer (decision record).
- [x] 2. Code: build version (`BLUETTI_BUILD`, else the crate version) in `--version` and `/healthz`; `check` subcommand (config + keys + DB schema vs binary, read-only, no BLE; exit 2 when the DB schema is newer) — check: `make check` with new tests; reviewer.
- [x] 3. deploy/: system unit, `bluetti-update` (apply / rollback / status), `deploy/test/update-test.sh` with stub systemctl/curl over a temp root, Makefile `release`, `deploy`, `test-deploy` (in `check`) — check: `make check` incl. the stub test; reviewer.
- [ ] 4. Docs: `deploy/HOMELAB.md` (first install + migration from the laptop + update + rollback + troubleshooting), `CHANGELOG.md`, README pointer, crate version 1.1.0 — check: commands cross-checked against the scripts; batched reviewer.

## Risks / Dependencies
- Everything on the homelab host needs sudo: the owner runs it; the agent never does.
- Two pollers fight over each station's single BLE connection (golden rule #5): the migration stops the laptop unit before the homelab one starts.
- An older binary refuses a newer schema (`store::schema::migrate`): rollback must restore the pre-update DB backup when the schema moved.

## Escape hatches
- A step fails 3 times → stop and log the blocker.
- Anything needing sudo or the homelab host → hand to the owner.

## Amendments
- A1 2026-10-15 — step 4: the crate version is not bumped by hand; `make release VERSION=1.1.0` bumps, tags and builds it (cutting and pushing a release is the owner's, HOMELAB.md §5). Step 4 ships the `## 1.1.0` CHANGELOG section it needs. · disposition: fixed (in the plan)
