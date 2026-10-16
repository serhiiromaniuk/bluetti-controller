# Changelog

One section per release, newest first: `## X.Y.Z — YYYY-MM-DD`. `make release
VERSION=X.Y.Z` refuses to cut a release without its section (ROADMAP.md
§10.6). Mark anything the host needs before the deploy with **config:** or
**host:**, so `deploy/HOMELAB.md` §6 step 2 is never missed.

## 1.3.0 — 2026-10-16

Dashboard panels for the R1a extras (ROADMAP §2.2 R1b), AC180 and Elite 200
V2, all marked unverified until HUMAN-CHECKLIST H12:

- **Cards:** pack temperature in °C, charging mode (standard / silent / turbo),
  and the station's own time left beside our forecast while it discharges.
  Extras older than 20 min are hidden. `/api/status` adds `pack_temp_c`,
  `charging_mode` and `station_left`.
- **Chart:** a 4th panel, pack temperature °C per station (`temp_c` in
  `/api/chart`); the 10-min readings join into one line, and the axis goes
  below 0 °C.
- **Energy table:** daily in / out from the station counters beside our
  integrated values, and a lifetime line per station (in, out, out ÷ in).
  `/api/energy` adds `counter_in_wh` and `counter_out_wh`.
- No schema change: rollback to 1.2.0 is a binary swap.

## 1.2.0 — 2026-10-15

- **Battery health (R1):** `/telemetry` estimates each station's usable
  capacity from observed discharges (runs of 5-min buckets with metered output,
  no input and a SOC drop of 10 points or more), lists the discharges, and
  suggests a `usable_capacity_factor` (median of the newest 5 ÷ nominal). The
  config is not changed automatically.
- **Telemetry extras (R1a), AC180 and Elite 200 V2, all unverified:** charging
  status and the station's own time to full / to empty on every poll; lifetime
  energy counters (kWh), charging mode and pack temperature (°F as read) from
  two slow blocks read every 10 min. A slow block that fails or runs long is
  skipped and never fails the poll.
- `read` prints the extras.
- No schema change: rollback to 1.1.1 is a binary swap.

## 1.1.1 — 2026-10-15

- Battery % is verified against the official app on all three stations, and
  the AC180's AC output too (HUMAN-CHECKLIST H1–H3, partial): `/telemetry`
  shows them as verified. The other powers read 0 W during the check (grid
  outage) and stay unverified until compared at a non-zero value, so the
  cards keep their "unverified" badge.

## 1.1.0 — 2026-10-15

First release for the homelab host.

- **Dashboard** (`/`): Grafana-style time picker (last 12 h by default, quick
  ranges up to 30 d, custom, drag to zoom), manual refresh by default with
  optional auto-refresh that pauses in a hidden tab, dark theme with a toggle,
  Battery % / Output W / Input W charts across stations with house-outage
  bands, a data-age line that turns STALE after 3 missed polls. One
  `/api/chart` request per refresh replaces 15 `/api/series` calls; the page no
  longer reloads on every sample.
- **API:** `GET /api/chart?from=&to=&step=`; `/healthz` adds `version` and
  `samples_since_start`.
- Days without samples read "no data" in the energy and house tables, never 0.
- The forecast says why it is missing when an output reads 0 W (the EB3A's
  meter floor).
- **CLI:** `--version` names the build (`git describe`); `check` validates the
  config, the keys and the database schema without touching BLE.
- **Deploy:** hardened system unit, `bluetti-update` (apply with health check
  and automatic rollback, rollback, status), `make release`, `make deploy`,
  `deploy/HOMELAB.md`.
- **host:** first install — follow `deploy/HOMELAB.md` §2–§5.

## 1.0.0 — 2026-10-15

The MVP (`v1.0-mvp` tag; the crate version was still 0.1.0): BLE polling of
the EB3A, Elite 200 V2 and AC180 with the encryption handshake, SQLite storage,
outage log, runtime forecast, energy accounting and the web UI, run as a user
unit on the dev laptop.
