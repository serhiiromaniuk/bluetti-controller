# RULES.md — `bluetti-controller`

Working agreement for this repo. Read before changing anything. This file is the
durable governance layer (structure, conventions, safety, git). The full spec,
research and milestone plan live in [`ROADMAP.md`](./ROADMAP.md).

---

## Agentic workflow (how work gets executed)

This file is governance (*what is true*). [`AGENTS.md`](./AGENTS.md) is the
operating manual (*how to execute*), [`WORKFLOW.md`](./WORKFLOW.md) is the worked
example, and [`loop/`](loop/) is the long-running **agent loop** (re-feed
one fixed prompt; progress lives on disk, not in context). Work is tracked as
per-phase task folders under [`tasks/`](tasks/) (`BRIEF/PLAN/LOG/OUTCOME`);
specialist subagents live in [`.claude/agents/`](.claude/agents/). `CLAUDE.md`
`@`-imports this file and `AGENTS.md`, so both are already in every session's
context.

**At the start of any agentic session, run `loop/where.sh --json` and read only
the files it lists** (AGENTS.md §3a) — it computes the phase, task, step and gate
from disk, and leaves out what `CLAUDE.md` already loaded. The loop is dev-time
tooling only. It tags a milestone and rolls into the next phase on its own **only
once that phase's gate has objectively passed**, and it always stops at a
human-only boundary (see AGENTS.md §6).

---

## What this is

A single-binary Rust service that runs on a low-power x86_64 homelab host
(Debian) and monitors three Bluetti power stations (EB3A, AC180, Elite 200 V2)
over local Bluetooth LE. It polls each station, stores the readings in an
embedded SQLite database, derives a power-outage log, runtime forecasts and
energy accounting, and serves a minimal read-only web UI on the LAN. Its top
constraint: it must keep working during a power outage with no internet, which
is exactly when it is needed.

### Golden rules (never violate)

> Drafted on 2026-10-13 from the owner's answers in the planning session.
> **Approved by the owner on 2026-10-13** (with the phase table and the
> acceptance thresholds below). Changes need the owner's approval again.

1. **Local-first runtime.** Monitoring never depends on the internet or the
   Bluetti cloud. No cloud SDK, cloud API call or phone-home enters the runtime
   path. The device is reached only over local BLE.
2. **Device writes are off by default.** Any frame that changes a station's state
   (function codes 0x06/0x10: AC/DC output, charging mode, power-off, ECO) is sent
   only when `control.enabled = true` in the config **and** the register is on an
   explicit allow-list. The default is `false` and never changes. Tests,
   discovery and register sweeps send read-only frames (0x03) only.
3. **Secrets and personal identifiers stay out of the repo.** Encryption keys,
   Bluetti account credentials, the owner's device MAC addresses and serial
   numbers live only in the local config or environment. Recorded frame fixtures
   are anonymised before commit. Never hardcode a key.
4. **MVP scope is monitoring.** Until v1.0-mvp: outage log, runtime forecast,
   energy accounting and extra telemetry only. Out of scope for now: device
   control, outage-schedule integration, battery-health analytics, notifications,
   Prometheus, cloud access (ROADMAP.md §2).
5. **Be gentle with the hardware.** One BLE connection per station at a time,
   polls serialised across stations, a timeout on every GATT operation, bounded
   retries, and connect-read-disconnect so the official phone app can still
   connect. Never hold a station connection open indefinitely.
6. **Phase 1's foundation gate gates everything.** No later-phase production
   logic until the foundation (research, ROADMAP, scaffold, protocol layer) is
   complete and reviewed by the owner.

---

## Phase workflow

Build strictly in order; commit + tag at each milestone (see Git). An agent (the
loop) **may tag a milestone and roll into the next phase autonomously, but only
once that phase's gate has objectively passed** (`make check` is green for code
phases; the documented acceptance criteria are met otherwise). The one boundary
it never crosses on its own is the **human-only boundary** — see "Before crossing
a human-only boundary" below. Detailed tasks per phase: ROADMAP.md §12.

| Phase | Deliverable | Milestone tag |
|-------|-------------|---------------|
| 1 | Foundation: research, ROADMAP, agent scaffold, protocol layer (M0) — owner-reviewed | `v0.1-foundation` |
| 2 | BLE transport + CLI (`scan`, `detect`, `read`, `dump`) validated on the EB3A (M1) | `v0.2-ble` |
| 3 | Encryption handshake + V2 profiles validated on AC180 and Elite 200 V2 (M2) | `v0.3-encryption` |
| 4 | Collector, SQLite storage, outage log, runtime forecast, energy accounting (M3) | `v0.4-collector` |
| 5 | Web UI + deployment on the homelab host, 7 days unattended (M4 + M5) | `v1.0-mvp` |

**Acceptance gate.** Every code phase: `make check` green. Phase-specific
criteria (never weaken them; fix the work, not the threshold):

- **v0.2 / v0.3 — reading is correct.** For each station in scope, 20
  consecutive polls at a 60 s interval: ≥ 19 succeed (95%), and battery % equals
  the official app's value at the same moment, power readings within ±5 W.
  *Arithmetic:* BLE links drop now and then (community reports, ROADMAP.md §4.6),
  so 1 miss in 20 is tolerated; ±5 W is the display resolution of the app plus
  the time skew of two reads a few seconds apart.
- **v0.4 — the collector is reliable.** A 24 h run: ≥ 95% successful polls per
  station (≤ 72 misses of 1440); a deliberate unplug of a station's AC input is
  recorded as an outage within 2 poll intervals (≤ 2 min); the database stays
  below 10 MB per station-day. *Arithmetic:* 2880 rows/day × ~100 B ≈ 0.3 MB, so
  10 MB is a 30× headroom that only a bug can exceed.
- **v1.0 — it runs unattended.** 7 days on the homelab host with ≤ 1 manual
  intervention, resident memory ≤ 50 MB, the status page served in ≤ 1 s on the
  LAN. *Arithmetic:* tokio + axum + SQLite typically use under 20 MB; 50 MB
  leaves room for caches without hiding a leak over 7 days.

A threshold that turns out to measure the wrong thing is re-scoped only via the
falsified-metric path (`AGENTS.md` §6): a decision record and a human, never a
self-applied edit.

---

## Architecture rules

- **The BLE stack sits behind one adapter.** Core logic depends on our own
  `Transport`/`Gatt` trait (`src/transport/`), never on `bluer` directly. The
  `bluer` import lives in the adapter module only, so a replay transport can
  stand in for tests.
- **The protocol layer is pure.** `src/protocol/` (framing, encryption, register
  maps, decoding) does no I/O and has no async; it is unit-tested with byte
  vectors.
- **All config flows through one module** (`src/config.rs`). Validate at
  startup, fail fast on missing or invalid config. No scattered environment
  reads.
- **One writer for the database.** A single task owns the write connection;
  readers use a separate read-only connection. Schema changes go through
  numbered migrations.
- **State survives restart.** Open outage events and rollup progress are
  persisted, so a restart never loses or double-counts an event.
- **Many small files** (≤ ~400 lines, 800 hard max). One responsibility each.

---

## Conventions

- Rust edition 2024. The exact toolchain (compiler + rustfmt + clippy) is pinned
  in `rust-toolchain.toml` and installed by rustup; `rust-version` in
  `Cargo.toml` is only the minimum supported version. Upgrade = bump the
  channel in `rust-toolchain.toml`, run `make check`, commit. Doc comments on
  public items.
- **No CI/CD.** The owner decided against it: `make check` run locally before
  every commit is the only gate.
- **Structured logging** via `tracing`, no `println!` debugging left in.
- **Every BLE operation** has a timeout and bounded retry with backoff. Never
  silently swallow errors.
- Validate all external data at the boundary: frame CRC and length, register
  plausibility (e.g. battery % ≤ 100), config values.
- Format with `rustfmt`, lint with `clippy -D warnings`.
- **Third-party code:** port only from MIT/Apache-licensed sources and credit
  them in `README.md`. Never copy GPL code (e.g. `bluetti-encryption`,
  `Bluetti_ESP32_Bridge`) into this repo.
- Register fields not yet confirmed on real hardware are marked
  `verified: false` and are never used for alerts or gates until confirmed.

## Local development (isolated + reproducible)

- **Never install into system-wide tooling.** Cargo builds into the project's
  `target/`. The `Makefile` is the only sanctioned entry point.
  - `make install` → fetch dependencies · `make check` → format check + clippy +
    tests · `make lock` → refresh `Cargo.lock`.
- **Reproducibility is a priority.** Pin dependency versions; commit
  `Cargo.lock` (this is a binary crate).

## Decision process

Any choice not settled in ROADMAP.md §3 (Decisions) is **resolved in ROADMAP.md
before implementation**, not decided ad hoc in code. Golden rule #6 gates this.

---

## Testing

- Coverage target **≥ 80%** for `src/protocol/` and the derived-metrics module,
  measured with `cargo llvm-cov` once it is wired into `make check` (M1). TDD
  where practical (red → green → refactor).
- Must-test: framing and CRC, decoding with hand-computed values, the
  encryption handshake against recorded vectors, outage detection edge cases
  (flapping input, missing polls), runtime and energy arithmetic.
- Hardware tests are `#[ignore]` and run only by a human next to the stations.
- Run via `make test`.

---

## Git

**Commit message** — Conventional Commits:

```
<type>(<scope>): <subject>
```

- types: `feat fix refactor docs test chore perf ci build`
- scopes: `protocol transport crypto collector store metrics web config cli deploy docs loop`

**Identity:** this is a personal repo. Commits use the owner's personal git
identity set in the repo's local config, never a corporate address.

**Branching & parallel work**
- The main branch is the working branch by default: atomic, separate commits
  directly to it. No feature branches / PRs unless the project needs them.
- **Git worktrees** are the exception, for work that runs concurrently (a
  `[parallel: X]` plan group, AGENTS.md §8a): read-only discovery and steps that
  only create their own new files share the tree; any other concurrent edit gets
  its own short-lived branch in its own worktree, merged back only after
  `loop/merge-gate.sh` is green on the merged tree.

**Cadence & milestones**
- **Commit after every completed change, scope, or task** — do not batch
  unrelated work. **Push once it is reviewed** (AGENTS.md §8): risky changes at
  once, low-risk ones with their batched review, so the remote lags by at most
  three reviewed-pending commits and never holds an unreviewed one.
- Tag milestones per the phase table (`git tag v0.1-foundation` then `git push
  --tags`). The agent applies these tags itself once the phase gate is
  observed-green (a tag asserts the gate passed — never tag to "make progress").

**Never commit:** `config.toml` or any local config, keys, `.env` files, SQLite
databases (`*.db`, `*.sqlite*`), raw frame captures that contain MAC addresses or
serial numbers, `target/`. Enforced by `.gitignore` — if `git status` ever shows
one of these, stop and fix it.

---

## Before crossing a human-only boundary

This checklist is **human-only — never an agent/loop action.** The loop stops at
the boundary (`<<LOOP:BLOCKED>>`, or `<<LOOP:DONE>>` at the final milestone) and
hands over here. Human-only actions:

- Anything that needs the physical stations: hardware validation runs, pairing,
  unplugging a station to simulate an outage, capturing phone traffic.
- Enabling device control (`control.enabled = true`) or adding a register to the
  write allow-list.
- Putting encryption keys, credentials, MACs or serial numbers into a config on
  the host.
- Installing or changing anything on the homelab host (packages, systemd units,
  BlueZ settings, firewall).
- Changing repo visibility, publishing releases, or anything else outside this
  repo.

Do the documented pre-flight checklist in ROADMAP.md §10, then a human performs
the action (golden rules #2/#3/#5).
