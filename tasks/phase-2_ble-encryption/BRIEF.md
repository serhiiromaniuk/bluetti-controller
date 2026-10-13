# Phase 2 — BLE transport + encryption, EB3A and Elite 200 V2  |  phase-2_ble-encryption

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
Finish the BLE transport and read-only CLI (`detect`, `read`, `dump`) and add
the encryption handshake, so both encrypted stations in range — EB3A and Elite
200 V2 — are read over local BLE (milestones M1 + M2, merged).

## Why
Phases 2 and 3 were merged by the owner on 2026-10-13 (RULES.md phase table):
`scan` showed the EB3A advertises encrypted (`BLUETTF`), so no station can be
read without the handshake. Continues `phase-2_ble-transport` (closed as
superseded after its step 8).

## Scope
**In:** `phase-2_ble-transport` PLAN steps 9–16 and its amendments A1–A8;
ROADMAP.md §12 M2 for the EB3A and Elite 200 V2: the handshake as a pure state
machine in `src/protocol/crypto` (port from the MIT sources in §4.4, never the
GPL ones), keys loaded from `crypto.keys_file`, handshake vectors recorded
from our own stations and anonymised; Elite 200 V2 `detect` + `dump` of
100–200, 1300–1520, 6000–6020 against PR #49 and the V2 maps; firmware and
encryption flag of each station in ROADMAP.md §14 Q5; an acceptance run of
20 polls at 60 s per station.
**Out:** the AC180 (off by owner decision; its validation is a
`HUMAN-CHECKLIST.md` row); collector, storage, metrics (phase 4); web UI and
deploy (phase 5); any write frame (0x06/0x10), pairing or device control
(golden rule #2).

## Done when
- [x] `make check` green with `src/protocol/` ≥ 80% line coverage — check:
      `make check` output in LOG.md
- [x] `bluer` is used only under `src/transport/bluer/` — check: grep in LOG.md
- [x] Handshake unit-tested against recorded, anonymised vectors — check: test
      passes; fixtures hold no real MAC/SN/key
- [x] Replay test drives `detect` + `read` for the EB3A and the Elite 200 V2
      from anonymised fixtures — check: tests pass
- [x] Firmware and encryption flag of the EB3A and Elite 200 V2 in ROADMAP.md
      §14 Q5 — check: `scan`/`detect` output in LOG.md
- [x] Acceptance run: ≥ 19/20 polls at 60 s per station, every decoded value
      inside its plausibility range — check: run output in LOG.md
- [x] Official-app comparison per station is a row in `HUMAN-CHECKLIST.md`
      (deferred, does not block) — check: the rows exist
- [x] the phase gate passes (`v0.3-encryption`) — see RULES.md / AGENTS.md §6

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #2/#3/#5, phase table, standing owner delegation,
  deferred physical checks) · ROADMAP.md §4.1–4.6, §4.9, §9, §11, §12 (M1, M2),
  §14 Q5 · loop/STATE.md CF-2
- Previous task: `tasks/phase-2_ble-transport/` (PLAN steps 9–16, amendments A1–A8)
- Related files: `src/transport/`, `src/protocol/`, `src/cli/`, `config.toml` (local)
