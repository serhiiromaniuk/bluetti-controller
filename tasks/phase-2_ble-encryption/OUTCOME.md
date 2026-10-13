# Outcome

> Written when the task closes — done **or** blocked.

## Summary
Milestones M1 + M2 are done. The BLE transport, the read-only CLI (`scan`,
`detect`, `read`, `dump`) and the encryption handshake (a pure state machine in
`src/protocol/crypto`, ported from the MIT sources in ROADMAP.md §4.4) read all
three stations over local BLE. Each station passed a 20/20 acceptance run at
60 s. CF-2 and CF-4 are discharged. The official-app comparisons are deferred to
`HUMAN-CHECKLIST.md` H1–H3, so every register field stays `verified: false`.

**Scope change against BRIEF Out (A28).** The BRIEF puts the AC180 out of scope.
The owner reversed that on 2026-10-13 at 14:05 (`loop/STATE.md` Decisions: the
AC180 is on and in range, read-only access as for the other stations). The PLAN
amendment `3bcd58a` added steps 18–22 for it. The BRIEF stays static; this line
records the change.

## Deliverables
- Commits (task-scoped, oldest first):
  - `5f40a76` — `docs(loop): plan phase-2 encryption task`
  - `a7d96b5` — `docs(docs): design the BLE encryption handshake in ROADMAP §4.4`
  - `ed4a881`, `085223e`, `c86dabe`, `221140b` — A2/A3/A5/A6 fixes, plausibility bounds (steps 2–4)
  - `a3a60f6`, `cb5151e` — secret scan refuses MACs and `keys.toml` values (step 5, A15/A16)
  - `315cef1` — `feat(protocol): add pure crypto envelope — kex frame, AES-CBC temp and session layers`
  - `b3e6ecb` — `feat(protocol): add pure handshake state machine with typed errors and test station`
  - `6b9b46c` — `feat(config): load static keys from crypto.keys_file with value-free errors`
  - `58dff18`, `ce3e1ca`, `285b929` — BluerGatt, `CryptoGatt`, encrypted `detect` (steps 9–11)
  - `a4d5a25` — `feat(cli): add read, dump and --record over one station session; settle A14, A20-A22`
  - `b649a97` — key-free handshake vectors (step 13)
  - `c40e84e` — Elite 200 V2 dump, §14 Q5 (step 14)
  - `2872cb2` — replay fixtures for EB3A and Elite 200 V2 (step 15)
  - `274a955` — acceptance run EB3A 20/20, Elite 200 V2 20/20 (step 16)
  - `468178c`, `774df8f`, `3bcd58a`, `5740d9c` — CF-4, AC180 decision, steps 18–22 amendment, batch review
  - `df984c0` — M1/M2 docs close-out (step 17)
  - `581ca45`, `a2a6884`, `20ab689` — AC180 detect, fixtures, acceptance 20/20 (steps 18–20)
  - `3483412` — `fix(protocol): EB3A low-load floor documented, reg 77 in whole volts (phase-2 step 21, CF-4)`
  - `3ab67ec` — AC180 docs close-out (step 22)
- Files / artifacts: `src/protocol/crypto/`, `src/transport/` (incl. `bluer/`),
  `src/cli/`, `src/config.rs`, `tests/replay.rs`, `tests/handshake_vectors.rs`,
  `tests/frames/*.json` (anonymised), ROADMAP.md §4.3–§4.5, §12, §14 Q5,
  `HUMAN-CHECKLIST.md` H1–H4.
- Milestone tag: `v0.3-encryption`

## Pending / Follow-up
- [ ] Owner works through `HUMAN-CHECKLIST.md` H1–H3 (app comparison); a pass
      opens a follow-up task that sets `verified: true` on the named fields.
- [ ] H1 also bounds the EB3A `ac_output_power` low-load floor (ROADMAP.md §4.3).
- [ ] Open phase 4 (collector, storage, metrics — M3, `v0.4-collector`).
- [ ] Close-review MEDIUM M2 → CF-5: Elite 200 V2 / AC180 `ac_input_voltage`
      (1314, ×0.1) and `ac_input_frequency` (1300) never read with the grid on;
      a ×10 error passes the range check. Grid-on `read` of both before phase-4
      outage logic relies on them.
- [ ] Close-review MEDIUM M3: the EB3A 20/20 run (step 16) used the binary
      before the reg-77 scale fix (A33, step 21); the post-fix decode rests on one
      live `read router` (223 V) and the replay test. The poll ratio does not
      depend on the scale, so the gate stands.

## Amendments
- A1–A27, A29–A33 — all fixed during the task; each disposition is in its PLAN entry.
- A28 — fixed: §12 preamble rewritten in step 22 (`3ab67ec`); the OUTCOME
  citation of the 14:05 scope change is the "Scope change" paragraph above.

## Lessons / Notes
- Run `scan` before planning: the EB3A advertised `BLUETTF`, which merged phases 2 and 3.
- Plausibility ranges cannot catch a register that reads 0 under a small load
  (CF-4); an owner observation found it. Compare against a known load early.
- A community map's scale can be wrong for our firmware (EB3A reg 77 is whole
  volts, not ×0.1); a live dump with the grid on caught it.
- Forward-homed amendments (A28/A29 at steps 18–22) made step 17's
  "none open" check ambiguous (A30); a check should state which amendments it excludes.
- No grader verdict was rejected as false this task.

<!-- Calibration proposals for the owner (AGENTS.md §8b): -->
- reviewer · missed: register scale taken from a community map with no live read
  with the grid on or under a real load — a plausibility range accepts a ×10 error
  (EB3A reg 77: 22.2 V passes 0..300, A33) and a 0 W under ~9 W (CF-4); passed at
  steps 4, 11, 12, 15, 16.
