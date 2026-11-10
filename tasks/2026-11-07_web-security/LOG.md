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
