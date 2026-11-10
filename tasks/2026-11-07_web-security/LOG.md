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

## 2026-11-10 — Task opened
- Opened from the ledger: previous task (R3 control + schedule) closed 2026-11-10; this is the first `todo` row.
- `PLAN.md` written + batch-reviewed 2026-11-07 (`171edda`, `862f2e1`); Q1–Q8 delegated, none pending.
- R3 landed first (`a9f2abc`…`af7f3dc`): `src/web/control.rs` and `src/web/schedule.rs` carry their own
  bearer checks — the "R3 ordering" risk's first branch applies to step 6 (plan amendment, 2026-11-10).
- No carry-forward in `loop/STATE.md` names this task.
- **Next:** step 1 — spec first (ADR-011, `spec §8.5`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:48:09Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-11-10 — spec: ADR-011 + `spec §8.5` security design

**Changed:**
- `ROADMAP.md` §3: ADR-011 (context, decision, rejected options, consequences; argon2id m = 19 MiB, t = 2,
  p = 1, 16-byte salt, 32-byte tag, serialised); supersedes ADR-006's "no auth".
- `ROADMAP.md` §2.3 wording; §8.2 table gains an Auth column + `/login`, `/logout`, `/assets` rows, R3 rows
  drop "bearer required"; §8.3 auth rule; new §8.5 (a)–(h); §9 `password_hash`, `allowed_hosts`, Q7 rules,
  mode warning; Appendix B gains the three auth/HTTPS rows §2.3 claimed it had.
- `SECURITY.md` "Planned hardening": this task's items grouped as in progress.
- `PLAN.md`: Q1–Q8 marked settled; amendment (2026-11-10) records the spec refinements for steps 3, 6, 10.

**Spec refinements beyond the step text** (in the amendment):
- §8.5 d also accepts `Origin: https://<Host>` — the Q4 TLS proxy would otherwise fail every POST.
- §8.5 h dev mode keeps the bearer rule for unsafe methods when `web.token` is set (R3 schedule edits).
- §9 warns on `mode & 0o027`, so `0640 root:bluetti` (SECURITY.md) passes.

**Check:** `spec_sections_of` (extracted from `loop/where.sh`) on step 2's and step 6's text resolves
`8.5 Security design` (L1273–1360) and `9. Configuration`; `where.sh --json` → 0 open/unasked questions.

**reviewer:** 0 CRITICAL / 1 HIGH / 1 MEDIUM (covered this diff + unpushed `3f4cc1d`; INTENT: satisfied)
- HIGH H1: R3 prose in §8.2 (control 401, schedule 401, panel "bearer from the pause form") and §12 R3
  "Editing" still said bearer-only → fixed: each now says session + Origin, or bearer (§8.5).
- MEDIUM M1: `next` rule bypassable with `/%09/` → deferred to PLAN `## Amendments` A2 (step 5).
- Note: headers on `/healthz` (§8.5 a/f) follow step 7, not step 1's "(a)" wording; harmless, no change.

**Decision:** ADR-011 + `spec §8.5` settle the design; later steps follow the spec where it refines them.
**Carry-forward:** none
**Next:** step 2 — password hashing + `hash-password` CLI.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:54:11Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 2 ✓ 2026-11-10 — argon2id hashing + `hash-password` CLI

**Changed:**
- `Cargo.toml`: `argon2 = "=0.6.0"`, default features off (`alloc`, `password-hash`): the salt comes from
  the existing `getrandom` 0.4, not the crate's own RNG path.
- `src/web/auth/password.rs`: `hash` (argon2id, m = 19456 KiB, t = 2, p = 1, 32-byte tag stated explicitly,
  16-byte `getrandom` salt) and `verify` (params from the PHC string; any error → `false`).
- `src/web/auth/mod.rs`: module stub (step 5 adds the handlers); `src/web/mod.rs` declares it.
- `src/cli/hash_password.rs`: first stdin line, terminator stripped, empty refused; prints the PHC string and
  the `password_hash = "…"` line. `cli/mod.rs` dispatches `HashPassword`; `main.rs` unchanged (no config).
- `README.md` Credits: argon2 (RustCrypto, MIT/Apache).

**Check:** `cargo test --lib web::auth` — 4 of 5 failed first against a `bail!` stub (malformed passes
trivially), then 5/5 green; `cli::hash_password` tests 2/2. Smoke: `printf 'test\n' | … hash-password`
prints `$argon2id$v=19$m=19456,t=2,p=1$…`; empty line → `Error: empty password refused`, exit 1.
`cargo tree -e normal -i argon2`: only `bluetti-controller` depends on it; its tree is MIT/Apache except
`subtle` (BSD-3-Clause, already a p256 dependency) — no GPL.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM (INTENT: satisfied). Note for step 3: config validation parses
`password_hash` but does not bound its argon2 params (a hand-edited huge `m` would be honoured at login);
owner-controlled config, not raised as a finding.

**Carry-forward:** none
**Next:** step 3 — config (`password_hash`, `allowed_hosts`, Q7 rules, mode warning).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T16:59:04Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-58e9097d0cf349c8)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 3 ✓ 2026-11-10 — config: `password_hash`, `allowed_hosts`, Q7 rules, mode warning

**Changed:**
- `src/config/web.rs` (new): `WebConfig` moved here with `password_hash: Option<String>` and
  `allowed_hosts: Vec<String>` (`serde(default)`); `validate(control_enabled)` holds every `[web]` rule:
  empty token, hash must be an argon2id PHC string with salt, tag and argon2-valid params (stricter than
  "parses as PHC": any other string never verifies at login), non-loopback listen ⇒ hash, `control.enabled`
  ⇒ token and hash, each `allowed_hosts` entry `[A-Za-z0-9.-]+`. No error quotes the token or the hash.
  `Debug` prints `password_hash: <set>`. PHC type path in argon2 0.6: `password_hash::phc::PasswordHash`.
- `src/config/mode.rs` (new): `warn_loose` — one `warn!` when `mode & 0o027 != 0` (amendment 2026-11-10);
  an unstattable file stays silent (its reader reports it). `load` calls it for the config and `keys_file`.
  `main.rs` initialises tracing before `config::load` (L39–47), so no warning list is returned.
- `src/config.rs`: `mod web; mod mode; pub use web::WebConfig`; web tests moved out; 395 → 335 lines.
- `config.example.toml`: `listen = "127.0.0.1:8080"`, LAN form, `password_hash`, `allowed_hosts` commented;
  `[control]` note names both keys.

**Check:** `cargo test --lib config` — 6 failed first against a no-op `validate` / `None` `loose_mode`
(hash, LAN listener, hosts, token, control, mode), then 28/28 green. Test hash is `hash-password` of `"test"`.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM (INTENT: satisfied). Stricter hash check judged in scope (also
covers step 2's note on hand-edited params). Known risk restated: the homelab's LAN listener fails fast
until the owner adds `password_hash` — step 10 upgrade note, deploy stays the owner's.

**Carry-forward:** none
**Next:** step 4 — session store + lockout.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:03:53Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-b3f51157f6d25649)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-11-10 — session store, lockout and one-permit verification slot

**Changed:**
- `src/web/auth/session.rs` (new, no I/O, no HTTP): `Sessions` — `issue()` 32 `getrandom` bytes as 64 hex
  chars, expiry now + 30 d (not extended by use), drops expired ids then evicts the earliest expiry past 16;
  `check`; `revoke`. `Lockout` — `begin()` counts before verification; the 5th failure still verifies and sets
  `locked_until = now + 60 s`, the count restarts, so ≤ 5 verifications per lock window (§8.5 c); `succeeded()`
  clears count and lock. `Auth { hash, sessions, lockout, verify_slot: Semaphore(1) }`, no `Debug`. Every
  instant is `tokio::time::Instant`; a poisoned mutex is used as is (plain data), never a panic.
- `src/web/auth/mod.rs`: `pub mod session`.

**Check:** `cargo test --lib web::auth::session` — 7 failed first against a stub (zero constants, `Locked`
always, zero-permit semaphore), then 7/7 green under `start_paused`: issue/check, 30 d − 1 s valid / + 1 s
expired, 17th evicts oldest, revoke, 6th `begin()` locked and 60 s later not (then 5 more, locked again),
success resets the count and lifts a lock, one verification permit.

**reviewer:** 0 CRITICAL / 0 HIGH / 0 MEDIUM (INTENT: satisfied); the 5th-failure-verifies-then-locks design
judged to match §8.5 c.

**Carry-forward:** none
**Next:** step 5 — login and logout handlers (fold A2: reject bytes ≤ 0x20 / 0x7f in `next`).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:07:20Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-b3f51157f6d25649)

running 3 tests
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
