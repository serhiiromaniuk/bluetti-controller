# bluetti-controller

Local monitor for Bluetti power stations (EB3A, AC180, Elite 200 V2) over
Bluetooth LE. One Rust binary on a low-power homelab PC: polls the stations,
keeps the history in SQLite, logs power outages, forecasts runtime, accounts
energy, and serves a small web page on the LAN. Works with the internet down.

**Status:** Phase 2 — BLE transport + encryption. The read-only CLI (`scan`,
`detect`, `read`, `dump`) reads the EB3A and Elite 200 V2 over encrypted local
BLE; collector, storage and web UI come in phases 4–5.

## Read first

- [`ROADMAP.md`](./ROADMAP.md) — spec, research with sources, decisions,
  architecture, milestones, risks, open questions.
- [`RULES.md`](./RULES.md) — golden rules, phase gates, conventions.
- [`AGENTS.md`](./AGENTS.md) — how agents work in this repo (`loop/`, `tasks/`).

## Develop

```bash
make tools   # once per clone: pinned cargo-llvm-cov into .tools/
make check   # fmt-check + clippy + tests (needs rustfmt, clippy, make tools)
make test    # tests under cargo llvm-cov; fails if src/protocol/ < 80% lines
make help    # all targets
```

## Credits

Protocol knowledge comes from these MIT-licensed community projects:
[bluetti-bt-lib](https://github.com/Patrick762/bluetti-bt-lib) (Patrick762),
[bluetti_mqtt](https://github.com/warhammerkid/bluetti_mqtt) (Stephen Augenstein)
and the [nhurman fork](https://github.com/nhurman/bluetti_mqtt),
[bluetti-monitor](https://github.com/j4m3z0r/bluetti-monitor) (James),
[rustytti](https://gitlab.com/spock/rustytti). Not affiliated with Bluetti.

The encryption layer (`src/protocol/crypto/`: envelopes and the key-exchange
state machine in `handshake.rs`) is ported from rustytti `crypto.rs` (MIT) and
cross-checked against the nhurman fork's `encryption.py` (MIT); ROADMAP.md §4.4
pins the commits read.
