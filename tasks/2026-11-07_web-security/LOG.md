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

## Step 5 ✓ 2026-11-10 — login and logout handlers

**Changed:**
- `src/web/auth/mod.rs`: `GET /login` (form), `POST /login` (`lockout.begin()` → `429`; one-permit
  `verify_slot`; `verify` in `spawn_blocking`; ok → `Set-Cookie: session=<id>; HttpOnly; SameSite=Strict;
  Path=/; Max-Age=2592000` + `303 next`; wrong → `401` + form), `POST /logout` (revoke, `Max-Age=0`, `303
  /login`). `safe_next` keeps `/…` only: second byte not `/`/`\`, every byte visible ASCII (A2).
  `session_id(headers)` parses the `Cookie` header (step 6 reuses it). Log lines name the outcome only.
- `templates/login.html`: standalone page, no inline script (CSP-ready for step 7).
- `AppState.auth: Option<Auth>` from `web.password_hash`; `with_test_password()` (hash of `"test"`,
  `auth::TEST_HASH`); `Auth.verified` counter `#[cfg(test)]`. Router: `/login`, `/logout` (unguarded until
  step 6). `web/tests.rs`: `request_with`, `header`, `start_shared`.
- ROADMAP.md §8.5 c: the `next` rule names the visible-ASCII bound; dev mode `/login` only redirects.
  A2 → fixed.

**Check:** `cargo test --lib web::auth::tests` — 9/9 failed first against stubs (`safe_next` identity,
handlers `501`), then green: cookie fields + live session, wrong 401 no cookie, 10 concurrent wrong → exactly
5 verified / 5×401 / 5×429 and the right password still 429 while locked, `//`, `/\`, `/%09/`, `http://`
next → `/`, logout revokes only its id, dev mode redirects, logs hold neither password nor hash.
- Log test was red deterministically beside the logout test: tracing-core's one-dispatcher shortcut fixes a
  callsite first hit on another thread at "never". Fix: a second live `Dispatch` in the test; 6/6 runs of
  `cargo test --lib web` green after.

**reviewer:** 0 CRITICAL / 0 HIGH / 1 MEDIUM (INTENT: satisfied; dev-mode redirect judged not creep)
- MEDIUM M1: permit released when a dropped connection cancels the handler while `spawn_blocking` still
  verifies → deferred to PLAN `## Amendments` A3.

**Decision:** dev mode `GET`/`POST /login` redirect to `next` (spec §8.5 c). The locked `429` renders the
HTML form (a page route), not JSON — §8.5 g's JSON rule is for `/api/*` and `/events`.
**Carry-forward:** none
**Next:** step 6 — the guard layers.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:26:45Z

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

## Step 6 ✓ 2026-11-10 — the guard layers

**Changed:**
- `src/web/guard.rs` (new): `host_guard` (outer `.layer` over the whole router, `/healthz` exempt by exact
  path, so 404s are Host-guarded too; IP literal / `localhost` / `allowed_hosts`, case-insensitive, port
  ignored → else `421`) and `auth_guard` (protected `route_layer`): valid bearer passes; login on → session
  else `401 {"error":"login required"}` for `/api/*`+`/events`, `303 /login?next=<percent-encoded path+query>`
  for pages; dev mode + `web.token` → unsafe method needs the bearer (`401`, §8.5 h); unsafe method without
  bearer → `Origin` byte-equal to `http://<Host>` or `https://<Host>` else `403`. `same` moved here.
- `src/web/mod.rs`: `router()` = public (`/login`, `/assets/{name}`, `/healthz`) + protected (rest, incl.
  `/logout`, R3 control/schedule) under `auth_guard`, all under `host_guard`.
- `pause.rs`, `control.rs`, `schedule.rs`: per-handler bearer checks removed (plan amendment 2026-11-10);
  schedule keeps R3's "`web.token` set" `403`. `AppState.allowed_hosts` from `web.allowed_hosts`.
- Tests: handler 401 tests (pause, control, schedule) moved into `guard/tests.rs`; `web/tests.rs` `send()`
  sends `Origin: http://localhost` (a same-origin page), `exchange()` takes a custom `Host` line; logout test
  sends `Origin`.

**Check:** `cargo test --lib web::guard` with both guards stubbed as pass-through — 5/10 failed (Origin,
Host, no-credential walk, dev mode, dev-mode token); restored → 10/10 green; `cargo test --lib web` 123/123.
Tests: route scan of `../mod.rs` (each `.route(` path in exactly one list, both directions), protected walk
none/session/bearer, public walk, Origin (`evil`, own IP `:9999`, no port, missing, `null` → 403; `http://`
and `https://` Host → 200), Host (`evil.example` on `/` and `POST /login`, unlisted name → 421; v4, `[::1]`,
`localhost`, listed name any case, `/healthz` under evil → 200), Host edge cases, dev mode.
`grep -n AUTHORIZATION src/web/*.rs` → `guard.rs:97` only.

**reviewer:** 0 CRITICAL / 0 HIGH / 1 MEDIUM (INTENT: satisfied; golden rules clean)
- MEDIUM M1: no login-mode test for A1's fall-through (wrong/empty bearer + session) → deferred to A4.
- Flake check: `cargo test --lib web` 3/3 runs green.

**Carry-forward:** none
**Next:** step 7 — security headers + CSP-clean pages.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:34:30Z

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

## Step 7 ✓ 2026-11-10 — security headers + CSP-clean pages

**Changed:**
- `src/web/headers.rs` (new): `security_headers` (`from_fn`) sets the four §8.5 f headers; `router()` adds it
  as the outermost `.layer`, so 404s and `host_guard`'s `421` carry them too.
- `assets/theme.js` (new, 681 B): `base.html`'s two inline scripts, loaded synchronously in `<head>`; the
  toggle binds with `addEventListener` on `DOMContentLoaded`.
- `assets/control.js` (new, 1 508 B): `status.html` had a **third** inline script (R3's output rows, inline
  because `app.js` sits at its 12 KB cap) — moved out too, else `script-src 'self'` blocks the controls. Its
  bearer-from-token-field lines dropped (cookie + `Origin` carry the request).
- `assets.rs` serves both; page-weight test caps `theme.js` 1 000 B, `control.js` 2 000 B; `spec §8.4` i sum
  updated: 90 938 B < 100 000 B (threshold unchanged).
- `app.js`: pause POST drops the token header; `status.html` loses the token input; `pages.rs` drops `token`;
  `base.html` header gains `<form method="post" action="/logout">` (every page).
- Tests: `guard/tests.rs` `every_response_carries_the_security_headers` (every `PROTECTED` route with a
  session, `/login`, `/assets/app.js`, `/healthz`, a 404, a `421`); `pages/tests.rs`
  `no_template_has_an_inline_script` (each `<script` is a `/assets/` src), `no_token_field_and_a_logout_form`
  (replaces `token_field_only_with_a_token`), asset-URL test lists the two new files; the R3 control-panel
  and telemetry tests now read `control.js` / expect only `theme.js`.

**Check:** red first — `cargo test --lib web::` 4 failed (headers, inline script, logout form, asset URLs);
then green 125/125; `grep -c "<script>" templates/*.html` → 0 in all four; no `Bearer` left in
`assets/*.js` or templates.

**reviewer:** batched (plan: steps 7–10 in one pass before the push).

**Carry-forward:** none
**Next:** steps 8 + 9 — parallel group A (query-span caps, printable names).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:40:46Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-b3f51157f6d25649)

running 3 tests
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 8 ✓ 2026-11-10 — query-span caps (parallel group A)
**Changed:**
- `src/web/api.rs`: `CHART_MAX_S` → `MAX_SPAN_S` (366 d); `window()` enforces `0 ≤ from < to` and
  `to.checked_sub(from) ≤ MAX_SPAN_S` for `/api/series`, `/api/outages`, `/api/chart`; a defaulted `from` is
  `to.checked_sub(span)` (overflow → 400) then `.max(0)`. Chart's own span check removed (now in `window()`).
- `src/web/history.rs` (`from - 1`, `to - from`, `duration_s`) and `src/store/read.rs` (`to - from`):
  `saturating_sub` — inputs come from `local_day_bounds` or DB rows, never overflow-reachable; no panic either way.
- Test `every_windowed_route_caps_the_span` (`src/web/api/tests.rs`): per route `from=-1`, 2-year span,
  1 y + 1 s, `from=i64::MIN`, `to=i64::MAX`, `to=i64::MIN` → 400; exactly 1 year → 200.
**Check:** red first — `/api/series … from=-1&to=10` answered 200; then green. Release build:
`cargo test --release --lib every_windowed_route_caps_the_span` → 1 passed.
**reviewer:** batched with steps 7, 9 and 2216f03 — 0 CRITICAL / 0 HIGH; INTENT satisfied (accepts
`saturating_sub` over `checked_sub` in history/read).
**Carry-forward:** none
**Next:** step 10 — docs batch + close evidence.

## Step 9 ✓ 2026-11-10 — printable advertised names (parallel group A)
**Changed:**
- `src/cli/mod.rs`: `pub(crate) fn printable` maps every `char::is_control` (C0, DEL, C1) to U+FFFD
  (`pub(crate)`: binary-crate module).
- `scan::line` prints `printable(name)` / `printable(&mask_name(name))`; `detect::line` passes the
  manufacturer-data flag through `printable`. The session banner (`src/cli/session.rs:41`) goes through
  `detect::line` → `scan::line`; no other print of `advert.name` in `src/cli` (`src/transport/session*` does
  not exist). `identity_line` (register data, not advertised) left as is.
- Tests: `scan` `control_characters_in_the_name_never_reach_the_terminal` (ESC, CR, U+009B; plain
  `EB3A2235000123` unchanged), `detect` `flag_control_bytes_are_replaced`.
**Check:** red first — `cannot find function printable`; then a test bug (flag is the first 7 bytes) fixed;
green 22/22 bin tests.
**reviewer:** batched (same pass) — 0 CRITICAL / 0 HIGH / 1 MEDIUM
- MEDIUM M1: `printable` misses Cf bidi controls (U+202A–E, U+2066–9), needed by step 10a → deferred to A5.
**Carry-forward:** none
**Next:** step 10 — docs batch + close evidence.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:47:09Z

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

## Step 10 ✓ 2026-11-10 — docs batch + close evidence
**Changed:**
- `SECURITY.md`: threat model (login), controls (writes per ADR-010, login, guard, headers, config rules,
  span caps, names); limitations renumbered — old 1, 2, 3, 5 closed; old 4 → 1; new 2 cookie not `Secure`,
  3 sessions end on restart, 4 one global lockout, 5 bidi controls pass (A5); deployment recs; hardening trimmed.
- `README.md`: login section (hash-password, bearer, dev mode, `allowed_hosts`), route table (login/logout,
  no per-route bearer), R3 "Login and token". `deploy/HOMELAB.md`: upgrade note first (preflight `check` exits 4,
  nothing changes — more precise than the plan's "rolls back"), §3 config lines, §7a Caddy, 421 row.
  `deploy/INSTALL.md`: §2 hash, §6 upgrade note. `HUMAN-CHECKLIST.md` H18, H19. ROADMAP §8.5 status line.
**Done-when → evidence:**
- spec/design: ADR-011, §8.5 a–h, §9 (step 1), SECURITY.md (this step).
- every route except login + static assets → 401/303: `guard::tests::every_route_of_the_router_is_listed_once`,
  `without_a_credential_every_protected_route_is_refused`, `a_session_cookie_or_the_bearer_passes_every_protected_route`,
  `public_routes_answer_without_a_credential`. `/healthz` is also public and Host-exempt (Q6: `bluetti-update`
  and the runbook curl it, by DNS name; it shows only build + poll age).
- foreign Origin/Host: `an_unsafe_method_needs_its_own_origin`, `the_host_rule_refuses_unknown_names_but_not_healthz`,
  `host_rule_edge_cases`, `dev_mode_leaves_get_open_but_checks_host_and_origin`.
- login: `a_flood_of_wrong_logins_verifies_at_most_five`, `session::tests::the_6th_attempt_is_locked_for_60_seconds`,
  `an_id_expires_after_30_days`, `logout_revokes_the_session_and_clears_the_cookie`,
  `login_logs_name_the_outcome_never_the_password_or_hash`, `config::web::tests::debug_hides_token_and_hash`.
- headers / spans / names: `every_response_carries_the_security_headers`, `pages::tests::no_template_has_an_inline_script`,
  `api::tests::every_windowed_route_caps_the_span`, `scan::tests::control_characters_in_the_name_never_reach_the_terminal`.
- `make check` green (gate below); H18/H19 added.
- `make test` coverage: `src/protocol/` 98.82 %, `src/metrics/` 99.53 %. Security files (baseline for 10a):
  guard 100, headers 100, config/web 100, session 96.6, api 93.9, auth/mod 88.2, password 85, config/mode 75,
  hash_password 52.9, cli/mod 20 (% lines).
**Check:** `grep -n "No auth for GET" ROADMAP.md SECURITY.md` → empty (rc 1).
**reviewer:** batched (this step only; no other unpushed commits) — INTENT satisfied; 0 CRITICAL / 1 HIGH / 3 MEDIUM
- HIGH H1: SECURITY.md claimed a dev-mode `warn!` that `src/` lacks (spec §8.5 h unbuilt) → verified (grep);
  claim removed here, the code re-targeted to step 10a as A6.
- MEDIUM M1 proxy docs must keep `password_hash` → A7; M2 ADR-011 "rolls back" wording → A8; M3 stale
  limitation numbers in code comments → A9.
**Carry-forward:** none
**Next:** step 10a — security test suite + 100 % gate (settles A3–A9).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:53:13Z

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

## Step 10a.1 ✓ 2026-11-10 — prerequisite fixes A3, A4, A5, A6, A9 (A10 splits 10a into 10a.1–10a.6)
**Changed:**
- `PLAN.md` A10: 10a split (eight test types + gate + mutation run exceed one increment; phase-4 A21
  precedent). Each sub-step risky, `--no-tick`; 10a.6 ticks 10a.
- A3 `src/web/auth/{mod,session}.rs`: `verify_slot: Arc<Semaphore>`, `acquire_owned`, permit moved into the
  `spawn_blocking` closure; test-only `running` (now, peak) counter around `verify`.
- A6 `auth::from_config`: builds `Option<Auth>` from `web.password_hash`, logs one `warn!` when unset;
  `AppState::new` uses it (once per `serve`).
- A5 `src/cli/mod.rs` `printable`: also replaces the Bidi_Control set (U+061C, U+200E/F, U+202A–E,
  U+2066–9). `SECURITY.md`: limitation 5 closed, controls line and planned-hardening bullet updated.
- A9 `src/web/api.rs`, `src/cli/mod.rs` comments cite SECURITY.md "Bounded queries and clean output".
- A1 settled: guard side proven by A4's test; panel side R3 A1 (a9f2abc).
**Check:** new tests
- `a_dropped_login_keeps_its_slot_until_the_hash_ends` — red with the permit left in the handler
  (`left: 5, right: 0`: five verifications running at once), green after.
- `printable_replaces_every_bidi_control` — red with the bidi arm disabled (`"EB3A\u{61c}x"`), green after.
- `a_bad_bearer_beside_a_valid_session_still_passes` (token set and unset), `dev_mode_warns_once_that_the_login_is_off`
  — green on arrival (A4 pins existing behaviour; A6 asserts exactly one WARN line).
- `make check` green.
**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 1 MEDIUM
- MEDIUM M1: A1's disposition landed in the HTML-comment template line, not the real A1 entry → fixed
  before the commit (mechanical: template restored, real entry carries it; `make amendments` 3 open: A7 A8 A10).
**Decision:** 10a runs as 10a.1–10a.6 (A10); none of A7/A8 (docs) belongs to 10a — settled at the close.
**Carry-forward:** none
**Next:** 10a.2 — negative, exploit catalogue (`oneshot`), boundary tests.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T17:59:05Z

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

## Step 10a.2 ✓ 2026-11-10 — security suite types 1–3: negative, exploit catalogue, boundary (A10)
**Changed:** tests only, no production code.
- `src/web/guard/tests/{negative,exploits,login_exploits,boundary}.rs` (new), mounted from `guard/tests.rs`
  (+ `with_server` helper). `src/config/web.rs`: 1 config test. `src/cli/scan/tests.rs`: 1 terminal test.
- Through the real `router()` over a raw socket (hyper's parser), not `tower::oneshot`: no new dev-dependency,
  and duplicate `Host`, absolute-form and non-UTF-8 bytes only exist below `http::Request`.
- One test fn per attack family, one labelled row per attack (a failure names the row).
**Types and counts** (tests / rows):
- (1) negative — 7 / 45: cookie 14 (missing, empty, malformed, forged, case, ±1 char, name tricks, first cookie
  decides), bearer 12 (`Bearer`, `bearer x`, two spaces, Basic, two headers…), no token 3, expired, logged
  out, page redirect 3; config 8 (argon2i/argon2d/bad params/no tag, `_`/Cyrillic/NUL host, misspelt key).
- (2) exploit — 15 / 141: path 15, method 10, CSRF 13 (incl. forced logout), rebinding 14, absolute-form and
  missing Host 3, NUL/non-UTF-8 bytes 5, hostile spans and `days` 36; `next` 12 × login on/off, double-encoded
  `next` 3, markup in `next` 1, fixation 2, spoofed-header brute force 1, oversized 2, form bytes 6;
  advertised-name escapes 6 (OSC 8, OSC 0, CSI 2J, C1 CSI, U+202E, U+2066/9).
  Already covered, not repeated: 10-way flood (verifies 5), lock refuses the right password, A3 dropped futures.
- (3) boundary — 4 / 21: 4 failures no lock then 5 lock; lock 59 s/60 s; session TTL −1 s/0/+1 s; span cap ±1,
  `from == to`, `from = -1`, `days` 1/30.
**Check:** failure-first by mutation — 7 hand mutants, each run against the new suites, sources restored:
- `same_origin` always true → CSRF test red; Host rule always true → rebinding + absolute-form red;
  `safe_next` identity → 3 `next` tests red; lockout `>=`→`>` → 2 boundary red; session expiry `>`→`>=` →
  TTL + expired red; lock end `<`→`<=` → lock-window red; bearer without scheme → bearer test red.
- First run found two test bugs (HTTP/1.0 status line; `/api/chart` refuses < 1 min windows) — test fixes,
  no product defect found. `make check` green.
**reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 2 MEDIUM; raw socket instead of `oneshot` accepted
- M1 `exploits.rs:316`: the 10 `/api/series?station=x` span rows passed via "unknown station" too → fixed
  before the commit (assert the 400 is not "unknown station"); red with `window()`'s checks disabled.
- M2 LOG row totals did not match the itemised rows → fixed: recounted from source (47→45, 151→141, 18→21).
**Next:** 10a.3 — types 4–6 (property-style, secret hygiene, headers on every response class).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T18:24:59Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-b3f51157f6d25649)

running 3 tests
test eb3a_read_session_replays ... ok
test ac180_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.82% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 10a.3 ✓ 2026-11-10 — security suite types 4–6: property-style, secret hygiene, response classes (A10)
**Changed:** tests only, no production code.
- `src/web/guard/tests/{properties,secrets,response_classes}.rs` (new), mounted from `guard/tests.rs`.
- `src/cli/scan/tests.rs`: exhaustive `printable` test (bin crate). `src/config/web.rs`: 1 config-error test.
- `Capture` (log sink) moved from `auth/tests.rs` to `web/tests.rs`, shared by both suites.
- Table-driven generation, no `proptest`: exhaustive small domains beat random sampling here, no new dep.
**Types and counts** (tests / generated rows):
- (4) property — 4: `safe_next` over 74 274 strings (≤4 of 16 symbols: `/ \ TAB LF CR SP a : % . @ ? # NUL é DEL`,
  plus `http:` + ≤3) against an independent browser model (WHATWG-strict: trim C0, drop TAB/LF/CR, `\`=`/`,
  scheme or `//` leaves the site) — sound and complete, 637 kept; Host rule 28 names × 11 ports = 308 rows, each
  also uppercased, + 28 with no allow-list; Origin 40 own hosts × 17 mutations = 680 pairs + 40 without Host;
  `printable` over all 1 112 064 scalars, exactly 77 replaced (65 Cc + 12 Bidi_Control).
- (5) secret hygiene — 2: 18 router requests under captured TRACE logs (current-thread runtime, so the server's
  tasks log to the test's subscriber) reaching 200/303/400/401/403/404/421/429: token, password, hash, salt,
  digest, `$argon2`, `Bearer` and every session id absent from bodies, headers (bar the login's Set-Cookie),
  redirects and logs; 4 refused configs (bad hash, open listen + token, control without hash/token) never quote
  a secret. `Debug` already covered (`debug_hides_token_and_hash`); `AppState`/`Auth` have no `Debug`.
- (6) response classes — 1 / 26 rows: 200 page/API/SSE/asset/healthz/login, 303 ×3 (page refusal, login, logout;
  the app uses 303, not 302), 400, 401 ×7, 429, 403, 404 ×2, 405, 415, 422, 421 ×2 — four §8.5 f headers on
  each; login cookie 64 lowercase hex + exact attributes, logout exact, no other row sets a cookie; SSE type.
  Inline `<script>`: already `pages/tests.rs` `no_template_has_an_inline_script`; templates have no `style=`/`on*=`.
**Check:** failure-first by mutation — 10 hand mutants, each turned only its target test red, sources restored:
- `safe_next` allows `/\` · allowed hosts case-sensitive · 6-digit port · Origin tolerates a trailing `/` ·
  U+061C dropped from `printable` · guard logs `Authorization` · logout logs the session id · no headers on 4xx ·
  cookie without `HttpOnly` · config error quotes the hash.
- First run: `kept` bound wrong (test arithmetic, now pinned to 637); clippy `type_complexity` → `Row` alias.
**reviewer:** INTENT shortfall → fixed; 1 HIGH / 0 MEDIUM; oracle independence, log capture, no vacuous asserts confirmed.
- H1 `src/config.rs` `parse`: `toml::from_str(text)?` — toml's `Display` quotes the source line, so a malformed
  token/hash line reached stderr/journal (`serve`) and stdout (`check`). Verified against source → fixed: error
  is `line <n>: <message>` (no snippet; `keys.rs` already line-only). +4 test cases (hash with junk, unterminated
  token, duplicate `token`, token as array): red on the old `parse` ("SECRET in TOML parse error…"), green now.
- Second pass on the H1 fix: 1 HIGH (H1b) — `e.message()` still quotes an unquoted numeric token (serde
  "invalid type: integer `98765…`"). Fixed: `without_value` keeps kind + "expected …" for serde invalid
  type/value/length; +2 cases (integer, float token) red on `e.message()`, green now. No third pass (§4b).
**Next:** 10a.4 — the `COV_SECURITY` 100 % gate in `make check`, red then green.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-11-10T18:39:09Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-b3f51157f6d25649)

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
