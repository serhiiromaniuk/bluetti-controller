# Outcome

## Summary
Every future update now ships the same way: `make release VERSION=X.Y.Z`
(changelog section required, `make check`, version bump, tag, tarball with
BUILD + SHA256SUMS), then `make deploy HOST=homelab`. On the host
`bluetti-update apply` checks the release, runs the new build's `check` as the
service user, backs up the DB and config to a root-only dir, switches
`current` atomically, and keeps the release only when `/healthz` shows the new
build polled a station; otherwise it rolls back on its own (restoring the DB
when the schema moved, the config when the old build rejects it). The telemetry
extras are recorded as postponed (not approved), with nothing built.

## Deliverables
- Commits: `29f3141` design (ROADMAP §2.2 R1a, §10.2, §10.6) · `4ac4f12` build id, `check`, `/healthz` version + `samples_since_start` · `be82b60` system unit, `bluetti-update`, `deploy.sh`, stubbed test, Makefile targets, HOMELAB.md · `08faeff` CHANGELOG, README, deploy.sh test, release rollback-on-failure
- Tests in `make check`: `update-test.sh` (39 checks), `deploy-test.sh` (7); `make lint-shell` clean
- Milestone tag: none (ad hoc). The first release tag `v1.1.0` is the owner's (`deploy/HOMELAB.md` §5).

## Pending / Follow-up
- [ ] Owner: answer the host questions (model/arch, Debian version, BT adapter, which station powers it, SSH name), then run `deploy/HOMELAB.md` §2–§5 (sudo, human-only).
- [ ] First real install validates what the stubs cannot: the hardened unit (`SystemCallFilter`, `MemoryDenyWriteExecute`) with BlueZ, the `bluetooth` group D-Bus policy, `runuser` on the host.

## Amendments
- A1 (no hand version bump; `make release` does it) · fixed in the plan.

## Lessons / Notes
- A stubbed harness found what reading did not: `du` on a missing DB killed the script silently under `pipefail`; mutations proved the EXIT trap and `inherit_errexit` checks have teeth.
- reviewer · caught: the DB-age health criterion passing without a poll; root following symlinks in a service-writable dir; set -e not applying inside `$(…)`.
