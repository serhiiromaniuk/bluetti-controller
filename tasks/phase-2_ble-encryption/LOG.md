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

## YYYY-MM-DD HH:MM — Session start
Context loaded per `loop/where.sh --json`. Starting at step <N>.

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

## Plan ✓ 2026-10-13 — PLAN.md written (17 steps) and audited

**Changed:** `PLAN.md` — 17 steps, parallel group A (1–4), Q1–Q5 all `delegated 2026-10-13`; old steps 9–16 and A1–A8 carried (mapping in Risks).

**Check:** `plan-reviewer` audit — 0 CRITICAL / 2 HIGH / 4 MEDIUM / 1 LOW; all acted on before the commit:
- HIGH F1 (step 13 committed a session secret + station key, against golden rule #3 and BRIEF done-when) → fixed: step 13 commits key-free envelope structure only; step 1 decides the committable-artefact rule and whether the station key is ephemeral.
- HIGH F2 (key scan armed only at step 8, after steps 6–7 port key-bearing sources) → fixed: step 5 writes the local `keys.toml` and proves the scan refuses its values; step 8 only loads keys.
- MEDIUM F3 (replay `detect` vs `read` recording) → fixed: step 12 `read` runs `detect` + `poll` in one session.
- MEDIUM F4 (no owner for an EB3A V2 re-map) → fixed in step 11.
- MEDIUM F5 (step 12 oversized) → `--record-wire` moved to step 13.
- MEDIUM F6 (step 1 check lacks the fixture decision) → fixed in step 1's check. LOW F7 → step 9 check wording.

**reviewer (batch `@{u}..HEAD` = 001de51, 1f2809b, 63ca4e8):** INTENT satisfied; 0 CRITICAL / 0 HIGH / 3 MEDIUM → deferred to PLAN `## Amendments` A9 (ROADMAP §12 M1/M2/M5 bodies stale), A10 (H-rows lack the `verified` follow-through), A11 ("production deploys" undefined vs user-level units); all homed at step 17.

**Decision:** none beyond the delegated Q1–Q5 in PLAN.
**Carry-forward:** none.
**Next:** parallel group A — steps 1–4.

Commit: `docs(loop): plan phase-2 encryption task` · Marker: <<LOOP:CONTINUE>>

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:30:48Z

```text

test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-f1d778b563e65a98)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 1 ✓ 2026-10-13 — handshake designed in ROADMAP §4.4 from 3 MIT sources

**Changed:** `ROADMAP.md` §4.4 rewritten (sequence table, static keys by role, `keys.toml` schema, committable-artefact rule, test split, recommendation), §4.9 crypto row, sources S35–S39 (worktree branch, commit a7d96b5).

**Check:** sources pinned + licences MIT: nhurman `bluetti_mqtt` b746f17c1f6c, Patrick762 `bluetti-bt-lib` 0e11a80484a1, `rustytti` d2c651d54dcf (GitLab); GPL repo never opened. Added-line hex grep `[0-9a-fA-F]{32,}` → 0. Crates: p256 0.14.0, aes 0.9.3, cbc 0.2.1, md-5 0.11.0.
- Findings: station opens kex (type 01, 4-byte seed); temp layer AES-128 (md5(reverse(seed)) XOR local key); session layer AES-256 from raw ECDH x; zero padding; no MAC/SN in any message; BLUETTE/BLUETTF same handshake in all sources; station-key ephemerality unknown → step 13.

**Decision:** keys.toml = `local_aes_key` (32 hex), `host_signing_key` (64 hex), `station_verifying_key` (130 hex, SEC1 `04…`).
**Next:** see group close below.

## Step 2 ✓ 2026-10-13 — A2 + A3: config Debug masks address/token, metrics range-checked

**Changed:** `src/config.rs` (329 lines) — hand-written `Debug` for `StationConfig` (`<masked>`) and `WebConfig` (`<set>`/`None`); `validate` rejects factor outside (0,1] and non-finite/≤0 `grid_voltage_min` (commit ed4a881).

**Check:** failing first: `debug_masks_address_and_token ... FAILED` (`assertion failed: !format!("{config:?}").contains("DD:EE:01")`), `rejects_bad_metrics ... FAILED`; then `cargo test --lib config::` ok 11 passed. A2, A3 → fixed.

## Step 3 ✓ 2026-10-13 — A5 + A6: backoff pinned, anonymiser hardened

**Changed:** `Cargo.toml` dev-dep tokio `test-util`; `read/tests.rs` `backoff_doubles_between_retries` (start_paused, elapsed == 3 s); `anonymise.rs` masks both serial layouts for every model, refuses replies after an unanswered request (commit 085223e + review fix below).

**Check:** failing first: A5 `no method named start_paused` before test-util; A6 `fixture_masks_both_serial_layouts_for_every_model ... FAILED` (`serial at 116 leaked`). Mutation: deleting `backoff.saturating_mul(2)` → `left: 2s right: 3s`. Step text's two-filter command does not parse; ran `cargo test --lib -- transport::read transport::anonymise` → ok 23 passed. A5, A6 → fixed. Cargo.lock unchanged.

## Step 4 ✓ 2026-10-13 — Metric::range bounds + pure problems()

**Changed:** `src/protocol/device.rs` (309 lines) `Metric::range()` per metric with reasons (SOC 0–100, power 0–5000 W, AC V 0–300, Hz 0–70, DC V 0–150, flags 0–1, pack 0–60 V, current ±100 A); `Problem {Missing, OutOfRange}`, `problems(profile, registers)`; `decode` unchanged (commit 221140b).

**Check:** stub returning `vec![]` failed 3 new tests (`left: [] right: [OutOfRange { metric: BatterySoc, value: 101.0 }, …]`); then ok 14 passed; protocol coverage 100%.

## Group A close 2026-10-13 — merged, batched review

- Worktree branches p2-step1..4; `loop/merge-gate.sh` → `MERGE GATE OK` on the merged tree; cherry-picked linear onto main; worktrees removed.
- **reviewer (batch 5f40a76..HEAD):** INTENT 1–4 satisfied; 0 CRITICAL / 1 HIGH / 1 MEDIUM.
  - HIGH H1 `anonymise.rs:137`: an unanswered NON-serial request + same-count serial request shifts pairing, serial reply masked against wrong range → fixed: any unanswered request sets `stale`; test replaced with the shifted case (failed first at tests.rs:242, then ok 13 passed). Second pass on the fix: no findings.
  - MEDIUM M1 `device.rs:143` PackCurrent sign vs u16 decode → deferred to A14.
- Writer findings → A12 (session AES-256, zero padding; step 6), A13 (Write Request for msg 2; steps 9–10).
- Env note: cargo lives in `~/.cargo/bin`, not on the loop shell's PATH.

**Carry-forward:** none. **Next:** step 5 — A7 + key scan in `loop/step-done.sh`.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:42:23Z

```text

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-6a10f69a4d815929)

running 2 tests
test cli::scan::tests::unknown_station_still_prints ... ok
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 5 ✓ 2026-10-13 — scan refuses `-`/`_` MACs and keys.toml values; local keys.toml written

**Changed:**
- `loop/step-done.sh` `builtin_scan`: MAC regex `[0-9A-Fa-f]{2}([:_-])([0-9A-Fa-f]{2}\1){4}[0-9A-Fa-f]{2}` (one separator per MAC); exemption `^AA([:_-])BB\1CC\1`, case-insensitive (A7).
- Same function: when `<main worktree>/keys.toml` exists (via `git rev-parse --git-common-dir`), count added lines holding any of its ≥32-hex values (`grep -ciF`, case-insensitive); refuse on >0; values never printed.
- `keys.toml` (gitignored, mode 0600, not committed): §4.4 schema from nhurman/bluetti_mqtt `b746f17c1f6c` `encryption.py` (LICENSE: MIT). Lengths 32/64/130 hex; `station_verifying_key` = last 65 B of the 91-B DER, whose 26-B prefix equals the public P-256 SPKI header (checked, True). Fetched copy deleted.

**Check:**
- `bash -n loop/step-done.sh` → ok; `git check-ignore -q keys.toml` → ignored.
- Scratch repo (plain bash, GNU grep 3.12, copy of keys.toml): `12-34-…` refused (1 MAC), `12_34_…` refused (1), `12:34:…` refused (1), `AA-BB-CC-DD-EE-01` pass, `aa_bb_cc_dd_ee_02` pass, mixed separators pass (not a MAC), uppercased real 64-hex value refused (1 line), clean line pass. Scratch dir removed.
- Pitfall: the interactive tool shell's `grep` is a ugrep wrapper without `\1` support — first scratch run errored and passed everything; the script runs under plain bash and gets GNU grep.
- **reviewer:** INTENT satisfied; 0 CRITICAL / 0 HIGH / 2 MEDIUM. M1 (empty pattern list passes silently) → deferred to A15; M2 (byte-array / split-line key forms pass) → deferred to A16. Both homed at step 6, before any crypto port commits.

**Carry-forward:** none. **Next:** step 6 — crypto envelope (fix A15 + A16 first in that increment).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-13T08:45:42Z

```text

test result: ok. 72 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/llvm-cov-target/debug/deps/bluetti_controller-6a10f69a4d815929)

running 2 tests
test cli::scan::tests::anonymised_line_masks_mac_and_serial ... ok
test cli::scan::tests::unknown_station_still_prints ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 100% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->
