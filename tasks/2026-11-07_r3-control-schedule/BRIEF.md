# R3 — AC/DC output control + scheduled switching  |  2026-11-07_r3-control-schedule

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
A super-easy way to switch each station's AC and DC outputs on and off (e.g. the
`router` EB3A AC outlet, the `mobile` AC180), by hand and on a schedule — code
only; hardware validation is deferred.

## Why
Owner, 2026-11-07 (recorded at the owner's request): "plan new scope: 1)
controlling power stations, 2) scheduling tasks. Super easy method of turning
off and on AC/DC in stations, for example router, for example mobile. Stages:
now just the code, we will test it later." This is the first slice of ROADMAP.md
§2.2 / §12 R3. The MVP is done (`v1.0-mvp`), so golden rule #4 no longer
blocks device control; golden rule #2 governs every write.

## Scope
**In:**
- Spec first: the R3 design (write frames, allow-list, schedule format, auth,
  missed-run semantics) recorded in ROADMAP.md before code (RULES.md "Decision
  process").
- Protocol: pure write-single-register (0x06) request builder + echo-response
  check, byte-vector tests; a per-model allow-list of output registers only
  (EB3A 3007 AC / 3008 DC; AC180 and Elite 200 V2 2011 AC / 2012 DC, ROADMAP.md
  §4.3), all `verified: false`.
- Write path through the existing transport/encryption session, serialised with
  polls (golden rule #5), read-back after the write.
- Gating: nothing is written unless `control.enabled = true` **and** the output
  is allowed for that station (golden rule #2); the default stays `false`.
- Manual control: a CLI command (`bluetti-controller set <station> ac|dc on|off`
  or similar) and a minimal web toggle per output, POST protected by the token.
- Scheduling: simple daily on/off times per station output, executed by the
  running service, surviving a restart without double or lost runs.
- Unit/integration tests with fakes (`ReplayGatt` or the collector fake);
  `make check` green.
- `HUMAN-CHECKLIST.md` rows for every hardware check (first real write per
  station, encrypted write frame, read-back, schedule firing).

**Out:**
- Any real write frame sent to a station by the agent, enabling
  `control.enabled` on any host, deploying to the homelab — human-only (RULES.md).
- Charging mode, ECO, power-off, LED, power lifting (later R3 slices).
- Schedule-aware charging and staggered charging (need R2 outage schedules).
- Notifications, multi-user auth, cloud.

## Done when
- [x] ROADMAP.md holds the R3 control + schedule decision (registers, frames,
      allow-list, schedule format, missed-run rule, auth) — doc section filled
- [x] With `control.enabled = false` (default) no code path builds or sends a
      write frame: tests prove CLI, web and scheduler all refuse (`make check`)
- [x] With control enabled and a fake transport: manual CLI, web toggle and a
      scheduled run each send the right 0x06 frame and read back the state
      (`make check`)
- [x] A restart around a scheduled time neither repeats nor silently drops a run:
      test green (`make check`)
- [x] `HUMAN-CHECKLIST.md` lists the hardware checks for each station; README /
      config example document how to use it
- [x] `make check` green; `src/protocol/` coverage ≥ 80%

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #2, #3, #5; human-only boundary) · ROADMAP.md §2.2 R3,
  §4.2 frames, §4.3 register maps, §4.4 encryption, §8 web/API, §9 config, §12 R3
- Related files: `src/protocol/modbus.rs`, `src/protocol/device.rs`,
  `src/transport/`, `src/collector/mod.rs`, `src/cli/`, `src/web/pause.rs`
  (token pattern), `src/config.rs` (`ControlConfig`)
