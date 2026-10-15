# Log

> Append-only. Never edit a previous entry. Newest at bottom. Keep it factual —
> commands run, evidence, decisions. This is the loop's memory.
>
> **This is the ONE place per-step detail is written.** `loop/STATE.md` and
> `tasks/INDEX.md` are pointers and change only when the gate verdict changes, a
> decision is made, a carry-forward moves, or a task opens/closes. `loop/where.sh`
> computes step position from `PLAN.md` and the last result from this file's newest
> `## ` heading — so restating an entry in a pointer file is prose every later
> iteration pays to re-read.
>
> **Budget: ≤40 lines per entry** (`make loop-hygiene` warns). Bullets, not
> narrative. Cite a spec/doc section, never re-quote it. Record the decision and
> the evidence, not the reasoning that produced them. The shape below is the shape.

## 2026-10-15 19:00 — Session start
Context loaded per `loop/where.sh --json`. Starting at step 1.

<!-- Entry template — copy, fill, keep it under 40 lines:

## Step <N> <✓|blocked> YYYY-MM-DD — <the headline result, one line>

**Changed:** <file/module — what, one line each>

**Check:** <the check that proves this step> — failed first for <the intended
reason>, then passed. <or, when it is green-on-arrival: the mutation that proves
the assertion has teeth, in one line>

**Evidence:** written below this entry by `loop/step-done.sh` (the gate tail,
fenced in `gate:begin/gate:end`). Add by hand only what no script captures: a
smoke result, measured acceptance numbers — the decisive lines.

**reviewer:** <n> CRITICAL / <n> HIGH / <n> MEDIUM · or `batched` (low-risk,
reviewed with the next batch before the push)
- CRITICAL|HIGH <id>: <finding> → fixed in <where>.
- MEDIUM <id>: <finding> → deferred to PLAN `## Amendments` A<n>.

**Decision:** <a choice the next iteration must not relitigate · none>
**Carry-forward:** <raised/discharged in loop/STATE.md · none>
**Next:** step <N+1> — <title>.

Commit: via `loop/step-done.sh --commit "<type>(<scope>): <subject>"`   ·   Marker: <<LOOP:CONTINUE>>
-->

## Step 1 ✓ 2026-10-15 — ROADMAP: extras postponed, release/rollout design settled

**Changed:** `ROADMAP.md` — §2.2 R1a + Appendix B: telemetry extras postponed, not approved (owner); §10.2 rewritten as a pointer (root:bluetti 0640, `bluetooth` group, D-Bus check via `systemd-run`); new §10.6: version/`check`/layout/system unit/ship/apply/rollback/status/retention/test.

**Check:** sections written with a recommendation; reviewer, two rounds.

**reviewer:** round 1 — 0 CRITICAL / 3 HIGH / 3 MEDIUM; round 2 — 2 HIGH / 2 MEDIUM; all fixed in the text:
- HIGH stale/future samples pass a DB-age criterion → `/healthz` `samples_since_start` (in-process count) ≥ 1.
- HIGH failure after the stop leaves monitoring dark → EXIT trap from the stop to the start restores current, previous, unit file, starts the old build; free-space preflight.
- HIGH DB restore (stale WAL, root owner, data loss) → live DB moved aside as pre-rollback, backup installed bluetti 0640.
- HIGH root `check` leaves root-owned -wal/-shm → `check` runs as the service identity (`runuser`).
- MEDIUM rollback outcomes (unhealthy, config rejected) → version-only check, config restore root:bluetti, exit 3.
- MEDIUM §10.2 contradicted §10.6 → pointer; MEDIUM `ssh -t`, root-owned temp dir, laptop-side hash.
- MEDIUM backup size arithmetic → bounded by 30-day raw retention (~27 MB DB).

**Decision:** §10.6 as written (owner request 2026-10-15; no CI/CD per 2026-10-13).
**Next:** step 2 — `--version`, `/healthz` fields, `check`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T16:26:00Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-10-15 — build id, `check` subcommand, `/healthz` version + own-sample count

**Changed:**
- `src/lib.rs` — `BUILD` (`BLUETTI_BUILD`, empty/unset → `v` + crate version); `--version` uses it; Makefile `build` passes `git describe`.
- `src/cli/check.rs` (new) + `src/main.rs` — `check`: config + keys (main, exit 1), prints `build`, `database`, `health <url>` (unspecified IP → loopback), schema verdict; exit 2 when the DB is newer. Read-only, no BLE, never creates the DB.
- `src/store/schema.rs` — `SCHEMA_VERSION`, `schema_version(path)`.
- `src/web/{mod,state}.rs` + `cli/serve.rs` — `/healthz` `version`, `samples_since_start` (in-memory count of the writer's `Sample` updates; §10.6 H1).

**Check:** `make check` green; new tests `check::tests::*` (missing/current/newer schema, file not created; health URL v4/v6/specific), `web::tests::healthz_counts_only_this_process_samples`, healthz body tests. Smoke on the laptop config: `check` exit 0 "database v1, binary v1 (ok)"; `/nonexistent` config exit 1; `--version` = `v1.0-mvp-8-g88dc783-dirty`.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM; 2 LOW fixed (empty `BLUETTI_BUILD`; counter doc said "until the sender closes").

**Next:** step 3 — system unit, `bluetti-update`, stubbed test, Makefile targets.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T16:28:29Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-10-15 — system unit, bluetti-update, stubbed test, release/deploy targets

**Changed:**
- `deploy/bluetti-controller.system.service` — `User=bluetti`, `bluetooth` group, `/opt/…/current`, `StateDirectory`, hardening, `MemoryMax=128M`.
- `deploy/bluetti-update` — apply (checksums, preflight `check` as `bluetti`, space, stop, EXIT-trap undo, backup to `/var/backups/bluetti-controller`, switch, unit sync, start, health = version + `samples_since_start` ≥ 1, save healthy config, prune) / rollback (config on check exit 4, DB on exit 2, start always, exit 3) / status.
- `deploy/deploy.sh` — laptop-side hash, `ssh -t`, root-owned 0755 temp dir, hash re-checked after the copy.
- `deploy/test/update-test.sh` — temp root, stub systemctl/curl, fake releases; 39 checks over every §10.6 path.
- `Makefile` — `dist` (BUILD, SHA256SUMS, normalised owner/mode, glibc ≤ 2.36 guard), `release`, `deploy`, `test-deploy` (in `check`), `lint-shell` (shellcheck v0.10.0 via Docker).
- `src/main.rs`, `src/cli/check.rs` — `check` exit 4 = config rejected.
- `ROADMAP.md` §10.6 — saved-config restore, backups outside the state dir, exit 4, refusal of an installed build.

**Check:** `make check` green incl. `update-test: all checks passed`; `make lint-shell` clean. Mutations: no EXIT trap → "trap restarted the old build" fails; no `inherit_errexit` → 3 checks of the failed-backup case fail. Laptop binary needs ≤ GLIBC_2.34. Tarball: root/root, 0644/0755, `sha256sum -c` OK, `--version` = BUILD.

**reviewer:** 0 CRITICAL / 2 HIGH / 3 MEDIUM, all fixed:
- HIGH backup errors inside `$(…)` ignored → `inherit_errexit` + test.
- HIGH root following symlinks in the service-writable state dir → backups in `/var/backups/bluetti-controller` (root 0700), `no_links` refusal + test.
- MEDIUM trap stopped at its first failure → `set +e` in `undo_apply`.
- MEDIUM re-applying `previous` deleted its saved config/BACKUP → refused + test.
- MEDIUM every check exit 1 treated as config → exit 4 only; other failures start the old build anyway, exit 3.

**Next:** step 4 — HOMELAB.md, CHANGELOG.md, README.
- Re-check: all five fixed, 0 CRITICAL/HIGH/MEDIUM; 3 wording slips fixed (exit-3 cases in the header and §10.6, test comments).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T16:40:40Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-10-15 — HOMELAB runbook, CHANGELOG, README; deploy.sh test; release dry-run

**Changed:** `deploy/HOMELAB.md` (rode in be82b60 with `git add deploy`) — host requirements, one-time setup, config/keys, move from the laptop (laptop unit stopped first), first release, every update, rollback, troubleshooting. `CHANGELOG.md` — 1.1.0 and 1.0.0. `README.md` Deploy — homelab vs laptop. `deploy/test/deploy-test.sh` — stub ssh/scp/sudo: 7 checks (remote quoting, flag, exit pass-through, corrupted copy refused before apply), in `test-deploy` and `lint-shell`. `Makefile` `release` undoes the version bump when `check` fails.

**Check:** `make check` green (update-test + deploy-test); `make lint-shell` clean. Release dry-run in a scratch clone: `make release VERSION=1.1.0` → commit `chore(deploy): release 1.1.0` (Cargo.toml + Cargo.lock only), tag `v1.1.0`, `dist/bluetti-controller-v1.1.0-x86_64-linux.tar.gz` with `BUILD` = `v1.1.0`, clean tree; clone deleted. HOMELAB commands cross-checked against `bluetti-update`, `deploy.sh`, the unit and `config.example.toml`.

**Amendment:** A1 — no hand bump; the owner's `make release VERSION=1.1.0` does it.
**reviewer:** batched (low-risk docs + test) before the push.
**Next:** close the task; owner questions (host details) in one batch.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-15T16:45:50Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-29b9cfa85dc29102)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 99.82% (min 80%)
>> src/metrics/ line coverage 99.4% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
