# Homelab migration + release rollout  |  2026-10-15_homelab-deploy

## What
A repeatable, safe way to ship every future update to the homelab host, and
the first migration from the dev laptop to it: versioned releases, a hardened
system unit, an update command that backs up the database, switches releases
atomically, health-checks the new build and rolls back on its own when the
check fails, plus a manual rollback.

## Why
Owner, 2026-10-15: move the setup to the homelab server; "we will have a lot of
updates in future, so have a detailed and sustainable mechanism of rolling out
updates". The telemetry extras discussed the same day are NOT approved: record
them as postponed, build nothing for them.

## Scope
**In:** ROADMAP decisions (§2.2 postponed extras, §10 release/rollout design);
build version in the binary and `/healthz`; a `check` subcommand (config, keys,
schema, no BLE); the homelab system unit; `deploy/bluetti-update`
(apply/rollback/status) with a stubbed test run by `make check`; `make release`
/ `make deploy`; `CHANGELOG.md`; `deploy/HOMELAB.md` (first install, migration
from the laptop, updates, rollback).
**Out:** running anything with sudo or on the homelab host (human-only), new
telemetry registers, CI/CD (owner decision 2026-10-13), remote access setup.

## Done when
- [x] ROADMAP records the postponed extras and the rollout design before the code.
- [x] `bluetti-controller --version` and `/healthz` report the build; `check` validates config, keys and schema without touching BLE.
- [x] `bluetti-update apply` backs up the DB, switches the release, waits for a sample newer than the restart from the new version, and rolls back (release + DB when the schema moved) when that fails; `rollback` and `status` work; a stubbed test proves each path and runs in `make check`.
- [x] `deploy/HOMELAB.md` gives the owner exact commands for the first install, the migration and every later update.
- [x] `make check` green; reviewer findings fixed.

## References
- ROADMAP §10 · RULES.md human-only boundary, golden rules #3 and #5 · `deploy/INSTALL.md` (laptop unit)
