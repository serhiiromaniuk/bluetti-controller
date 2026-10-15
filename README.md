# bluetti-controller

Local monitor for Bluetti power stations (EB3A, AC180, Elite 200 V2) over
Bluetooth LE. One Rust binary on a low-power homelab PC: polls the stations,
keeps the history in SQLite, logs power outages, forecasts runtime, accounts
energy, and serves a small web page on the LAN. Works with the internet down.

**Status:** Phase 5 — web UI and deploy. The read-only CLI (`scan`, `detect`,
`read`, `dump`) reads the EB3A, AC180 and Elite 200 V2 over encrypted local BLE;
`serve` polls them on a schedule into SQLite, derives the outage log, runtime
forecast and energy totals, and serves them on the LAN.

## Read first

- [`ROADMAP.md`](./ROADMAP.md) — spec, research with sources, decisions,
  architecture, milestones, risks, open questions.
- [`RULES.md`](./RULES.md) — golden rules, phase gates, conventions.
- [`AGENTS.md`](./AGENTS.md) — how agents work in this repo (`loop/`, `tasks/`).

## Develop

```bash
make tools   # once per clone: pinned cargo-llvm-cov into .tools/
make check   # fmt-check + clippy + tests (needs rustfmt, clippy, make tools)
make test    # tests under cargo llvm-cov; fails if src/protocol/ or src/metrics/ < 80% lines
make help    # all targets
```

## Run

Copy `config.example.toml` to `config.toml` (gitignored) and fill in your
stations and keys file, then:

```bash
make run                         # = bluetti-controller --config config.toml serve
make run CONFIG=data/other.toml  # another local config
```

`serve` polls every `[[station]]` read-only once per `poll_interval_secs`,
writes the SQLite database at `storage.path`, and serves the web UI on
`web.listen`. SIGINT/SIGTERM stops it cleanly: the current poll disconnects, the
writer flushes, open `/events` streams close.

## Web UI and API

Server-rendered, no JS build step, assets embedded in the binary, no external
URL (it works offline during an outage). LAN only: GET has no auth.

- `/` — house grid banner, one card per station (SOC, in/out W, grid state,
  runtime forecast, last seen, poll health), the last 20 outages, a 7-day
  energy table, and SOC/power charts with a 24 h / 7 d toggle. A "pause
  polling" form stops BLE polling for up to 60 min so the official app can
  connect; the page reloads on live updates.
- `/telemetry` — every stored metric per station, its latest value, its
  `verified` flag and a 24 h sparkline.

| Method | Path | Returns |
|---|---|---|
| GET | `/api/status` | stations + house, the same view as `/` |
| GET | `/api/series?station=&metric=&from=&to=&step=` | `{source, t:[…], v:[…]}`; raw ≤ 48 h, 5-min rollup beyond |
| GET | `/api/outages?from=&to=` | grid events, newest first (default last 7 days) |
| GET | `/api/energy?days=7` | daily energy per station and house outage totals |
| GET | `/events` | SSE: `sample`, `grid_event`, `poll_failure` |
| POST | `/api/polling/pause?minutes=N` | `N` 1..=60 pauses, 0 resumes; `Authorization: Bearer` when `web.token` is set |
| GET | `/healthz` | `{"status":"ok","last_poll_age_s":…}` |

`metric` is a register key such as `battery_soc` or `ac_output_power`
(`/telemetry` lists them). A bad parameter answers `400` with
`{"error": "…"}`. Details: ROADMAP.md §8.

## Deploy

`make build` builds the release binary; `make dist` packs it with the user
unit, `config.example.toml` and the install notes into `dist/`. Install as a
user-level systemd unit (no `sudo`): see [`deploy/INSTALL.md`](deploy/INSTALL.md).
The hardened homelab install is ROADMAP.md §10.2 and is done by a human.

## Credits

Protocol knowledge comes from these MIT-licensed community projects:
[bluetti-bt-lib](https://github.com/Patrick762/bluetti-bt-lib) (Patrick762),
[bluetti_mqtt](https://github.com/warhammerkid/bluetti_mqtt) (Stephen Augenstein)
and the [nhurman fork](https://github.com/nhurman/bluetti_mqtt),
[bluetti-monitor](https://github.com/j4m3z0r/bluetti-monitor) (James),
[rustytti](https://gitlab.com/spock/rustytti). Not affiliated with Bluetti.

The web UI charts use [uPlot](https://github.com/leeoniya/uPlot) 1.6.32
(Leon Sorokin, MIT), vendored in `assets/` with its license as
`assets/uPlot.LICENSE`.

The encryption layer (`src/protocol/crypto/`: envelopes and the key-exchange
state machine in `handshake.rs`) is ported from rustytti `crypto.rs` (MIT) and
cross-checked against the nhurman fork's `encryption.py` (MIT); ROADMAP.md §4.4
pins the commits read.
