# BLE reliability: why per-poll discovery misses, and what else exists

Scout page for phase-4 step 18a. Read-only. GPL projects were described, never copied.

## Findings

### Our code (what runs today)
- `find` tries the BlueZ cache, then `scan_for`: fresh `set_discovery_filter(transport=Le)`, `discover_devices()`, checks `Device1.Rssi` every 250 ms, 15 s cap, `HEARD_POLL`, "an RSSI means heard in this discovery" — `src/transport/bluer/scan.rs:82-114`, `src/transport/bluer/gatt.rs:41-48`.
- `DiscoveryFilter` leaves `duplicate_data` at its default (false) — `scan.rs:73-78`; bluer field at `bluer-0.17.4/src/adapter.rs:754`.
- ROADMAP §5.4 already records that BlueZ forgets the station ~30 s after disconnect, so at a 60 s interval every poll is uncached — `ROADMAP.md:645-651`.
- Host: `bluetoothd` 5.85 runs as root with no `-E` flag in its command line (`ps aux`); `/etc/bluetooth/main.conf:109` has `#TemporaryTimeout = 30` (default).

### Community tools (Q1)
- bluetti_mqtt keeps a **persistent** link per device: `connect` once, `start_notify` once, then a command loop; on a BleakError it disconnects, sleeps 5 s, reconnects. Per-command: 5 retries, 5 s wait after a parse error, 1 s sleep after a connect error, no exponential backoff, and `BleakDeviceNotFoundError` retries with no sleep — https://raw.githubusercontent.com/warhammerkid/bluetti_mqtt/master/bluetti_mqtt/bluetooth/client.py (lines 53-163 as summarised).
- bluetti_mqtt does not scan per poll: it takes a fixed address; scanning (`BleakScanner.discover()` with Bleak's default timeout) is only for `--scan` and the start-up `check_addresses` — https://github.com/warhammerkid/bluetti_mqtt/blob/master/bluetti_mqtt/bluetooth/__init__.py.
- Its sample systemd unit restarts the process after 30 s (`Restart=always`) — https://pypi.org/project/bluetti-mqtt/0.15.0 (via search snippet; unverified in the unit file itself).
- ROADMAP §4.6 (sources S2, S5, S17, S19): HA users need a 60 s poll / 120 s timeout / 10 retries on the AC180 and it still drops after hours; a station stops advertising while the phone app holds the link — `ROADMAP.md:480-495`.
- bluetti-bt-lib README and hassio-bluetti-bt README state nothing on connection strategy (the lib is based on bluetti_mqtt) — https://github.com/Patrick762/bluetti-bt-lib, https://github.com/Patrick762/hassio-bluetti-bt. Home Assistant's Bluetooth integration owns scanning there (a passive, always-on scanner); unverified for this integration, I did not read its source.
- Bluetti_ESP32_Bridge README says nothing on reconnect or advertising; only that the icon blinks while connecting — https://github.com/mariolukas/Bluetti_ESP32_Bridge. Its source was not read.
- Advertising interval of the stations: not found in any source. `unverified`. Only the "stops advertising while connected" behaviour is documented (§4.6 above, and a forum report that one client at a time holds the link — https://community.bluettipower.com/t/elite-200-v2-app3-0-bt-scanning-device-not-found/41590).
- No source says whether a station advertises right after a disconnect; none found. `unverified`.

### BlueZ and kernel internals (Q2)
- `DISCOV_LE_TIMEOUT 10240 /* msec */`, `DISCOV_INTERLEAVED_TIMEOUT 5120` — https://raw.githubusercontent.com/torvalds/linux/master/include/net/bluetooth/hci_core.h (~line 1010). Where the timeout is applied (does the kernel stop LE discovery after 10.24 s?) was not read: hci_sync.c fetch got HTTP 429. `unverified`.
- `bluetoothd` `adapter.c`: `IDLE_DISCOV_TIMEOUT (5)` seconds; `discovering_callback()` restarts discovery after the kernel reports it stopped, with delay 5 s, or **0** when `no_scan_restart_delay` is set (set from `current_discovery_filter != NULL`); a failed restart waits 10 s — https://raw.githubusercontent.com/bluez/bluez/master/src/adapter.c (~lines 95, 1080, 1100, 1290; line numbers approximate, from a summarising fetch). So a gap of 0-5 s with no scan can follow every kernel scan cycle; with our filter set the gap should be ~0, but that is `unverified` on this host.
- `TEMP_DEV_TIMEOUT 3*60` exists in `adapter.c`, but main.conf `TemporaryTimeout` default is 30 s: "How long to keep temporary devices around" — https://raw.githubusercontent.com/bluez/bluez/master/src/main.conf. `device.c` removes a temporary device after the timeout, skipped while connecting or connected, and cancelled when a connection is added — https://raw.githubusercontent.com/bluez/bluez/master/src/device.c.
- `SetDiscoveryFilter`: `DuplicateData` default false ("disables duplicate detection of advertisement data"); with any filter set the default RSSI delta threshold is not applied; sessions from several clients are merged and discovery stops only when all end — https://raw.githubusercontent.com/bluez/bluez/master/doc/org.bluez.Adapter.rst. So with a single session the RSSI property is not refreshed on every advert, but is set when the object is created/first heard.
- Our own comment says BlueZ drops the RSSI of devices no longer seen when discovery stops — `scan.rs:118-119`. Each poll therefore starts from "no RSSI", and any advert gap or restart gap lands in the 15 s window.
- Net: the observed 13 % (EB3A) and 7 % (Elite) misses are consistent with a station that advertises slowly or with gaps, plus a scan that is restarted every poll. This is a hypothesis; the cause is **not** proven. Step 18a's time-to-heard measurement is what decides it.

### Alternatives (Q3)
- **Direct `Device1.Connect` on a known object, no discovery.** `dev_connect` calls `btd_device_set_temporary(dev,false)` then `device_connect_le`, so a cached LE device connects without fresh adverts — device.c above. Needs the Device1 object to exist: it is removed ~30 s after disconnect while temporary. LE connect itself still needs the peer to advertise (connectable), so it can still fail; its failure shows as `le-connection-abort`/page timeout (`unverified` which error).
- **`Adapter1.ConnectDevice`.** Connects without general discovery; needs `Address`, optional `AddressType` public/random; **experimental**, so `bluetoothd -E` (or `Experimental = true` in main.conf) — a BlueZ config change, human-only — https://raw.githubusercontent.com/bluez/bluez/master/doc/org.bluez.Adapter.rst. bluer 0.17.4 exposes it as `Adapter::connect_device` and marks it experimental — `bluer-0.17.4/src/adapter.rs:385-399`. Current host runs bluetoothd without `-E`, so it would return NotSupported/UnknownMethod; `unverified` (not called, would connect).
- **One discovery session kept open across polls.** BlueZ sessions are refcounted, so a process-wide `discover_devices()` stream held for the service lifetime keeps the Device1 objects and RSSI alive and fresh; no restart gap per poll, no cache expiry (objects seen in discovery are not removed while heard). A scan is not a connection, so golden rule #5 (one link, no held connection) is not touched; its cost is continuous radio use on the host, and it must not collide with `connect` (BlueZ pauses/resumes scanning during LE connect; `unverified` for our adapter). No sudo, no config change. Shared state: one discovery handle for 3 stations, owned by one task.
- **Passive scan via AdvertisementMonitor (`org.bluez.AdvertisementMonitorManager1`).** Lets a client watch for manufacturer-data patterns without active scan requests; the `main.conf` `AdvMon*` scan settings exist (defaults 300/500 ms duty) — https://raw.githubusercontent.com/bluez/bluez/master/src/main.conf. Exposed in bluer as `adapter.register_advertisement_monitor` (`unverified`, not checked in the crate source); needs kernel/controller MSFT or AdvMon support (`unverified`) and possibly `-E` on older BlueZ; heavier code than the other options.

## Gaps and risks
- Cause of the misses is unproven: no measurement of the stations' advertising gaps. A scan-window increase might not help if the station goes quiet for tens of seconds after disconnect.
- Kernel LE discovery timeout use and the BlueZ restart delay were read through summaries, not line-by-line; line numbers above are approximate.
- Station advertising interval and post-disconnect behaviour are undocumented anywhere found; EB3A (13 %) vs AC180 (0.3 %) suggests firmware difference, not host radio.
- A held discovery session raises radio load and might starve the single-adapter connect; needs a trial, and it may mask a station that really stopped advertising (phone app attached).
- The 20 s per-poll budget leaves ~15 s after a ~5 s healthy poll; a longer scan window cannot exceed that without a §5.4 decision.

## Open questions
- Is a continuously running LE scan on the dev laptop acceptable under golden rule #5's spirit ("gentle with the hardware")? A scan is not a connection, but the owner may read it differently.
- Is enabling `bluetoothd -E` (for `ConnectDevice`) acceptable? It is a system-wide BlueZ change, human-only; only needed if the discovery-held option fails.

## Recommendation for 18b
1. First choice: **one process-wide discovery session kept open across polls** (start at service start, restart if its stream ends, owned by one task), with the existing `Device1` RSSI/advert check reading from live objects. Connect then needs no per-poll scan, removes the 0-5 s restart gap and the ~30 s cache expiry, stays within rule #5 (links remain connect-read-disconnect) and the 20 s budget, and needs no sudo or config.
2. Use 18a's p50/p95/max time-to-heard to confirm: if adverts arrive with gaps ≤ ~20 s, a held scan reaches ~100 %. If the max exceeds the budget, the station is the limit; then extend the deadline only via a §5.4 decision, and add a direct `Device.connect` attempt on the cached object (no wait for an advert) as a fallback before giving up.
3. Keep `ConnectDevice` and AdvertisementMonitor as later options; both need human-only host changes or more code.
4. Mirror bluetti_mqtt's lesson: retry connect with a short, bounded sleep, never a tight loop on "device not found".

## Measurement — second adapter (owner's homelab IdeaPad, 2026-10-14 15:12)

Passive `btmon` log of advertising reports from a second host (Qualcomm Atheros USB BT, Debian 13), active `bluetoothctl scan on`, no connection to any station; the laptop's loop was polling at the same time. The capture ran 12.3 min, not the planned 60 (cause not found).

| Station | Adverts | Gap p50 | p95 | p99 | Max | Gaps > 15 s |
|---|---|---|---|---|---|---|
| EB3A | 114 | 2.56 s | 16.13 s | 16.49 s | 16.5 s | 26 |
| AC180 | 216 | 0.67 s | 15.22 s | 15.91 s | 16.1 s | 14 |
| Elite 200 V2 | 395 | 0.72 s | 7.69 s | 12.11 s | 14.1 s | 0 |

- Silences of 15–16.5 s are routine for the EB3A (26 in 12 min) and occur on the AC180. A 15 s per-poll discovery window can fall entirely inside one, which matches the EB3A's `discover` misses on the laptop.
- The silences show on a second adapter too, so they are station-side (or BlueZ scan cycling), not the laptop's combo chip alone.
- Consequence for A23: a per-poll window must exceed ~17 s to be safe; a long-lived discovery that keeps the BlueZ cache warm (gaps < 30 s `TemporaryTimeout`) removes the problem without a longer window. Prefer the long-lived discovery.
- Not explained: why the AC180 had no laptop misses despite similar gaps.
