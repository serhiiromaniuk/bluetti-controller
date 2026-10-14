# BLE reliability research (phase 4, after the failed 12 h run)

Read-only research. No station, radio or binary was touched. Web claims not confirmed by a primary source are marked `unverified`.

## Findings

Our run (LOG step 18 ✗, `tasks/phase-4_collector/LOG.md`):
- AC180 99.72 %, Elite 91.81 %, EB3A 86.81 %. EB3A: all 95 misses are `discover` ("not found within 15s"), a steady 6-12 per hour for 12 h, JBL off the adapter — LOG step 18 "Miss shape".
- Elite: 48 `discover`, 6 `handshake` (signature does not verify), 5 `read` (20 s deadline). Its `discover` misses fall from 7/h to 1/h over the run — LOG step 18. Not explained; the other two stations do not fall, so it is not simply adapter warm-up (unverified).
- AC180 misses: 2 `handshake` ("ff01: notification stream closed"), no discover misses — LOG step 18.

How discover works today:
- Every poll: check the BlueZ cache, else open a brand-new LE discovery (`set_discovery_filter(Transport=Le)` + `discover_devices()`), poll RSSI every 250 ms for up to 15 s, then drop the discovery when the function returns — `src/transport/bluer/scan.rs:59-100`, `src/transport/bluer/gatt.rs:42-51`.
- BlueZ drops an unheard device ~30 s after disconnect (`TemporaryTimeout`, default 30 s; 0 = keep) — [BlueZ main.conf](https://raw.githubusercontent.com/bluez/bluez/master/src/main.conf), ROADMAP §5.4. At a 60 s interval the cache is therefore always cold, so every poll depends on hearing one advert inside a 15 s window.
- The scan waits for `rssi.is_some()` (a fresh sighting), and a discovery started with BlueZ default LE discovery scan params (the `[LE] ScanInterval/WindowDiscovery` entries are commented out, so kernel defaults apply — main.conf above). On a shared combo chip the effective duty cycle is not known here (unverified).

Community practice:
- warhammerkid/bluetti_mqtt keeps ONE persistent connection per device (state machine NOT_CONNECTED → CONNECTED → READY; link dropped only on error), no keepalive, response timeout 5 s, 5 retries, 5 s sleep after a disconnect, 1 s after a connect failure. It addresses by MAC with no scan after the first blocking `BleakScanner.discover()`; a not-found is retried in a tight loop with no backoff — [client.py](https://raw.githubusercontent.com/warhammerkid/bluetti_mqtt/main/bluetti_mqtt/bluetooth/client.py), [manager.py](https://raw.githubusercontent.com/warhammerkid/bluetti_mqtt/main/bluetti_mqtt/bluetooth/manager.py).
- Its README: one command monitors several devices but only one is talked to at a time; a polling interval (e.g. 60 s) and systemd `Restart=always`/`RestartSec=30` are the documented mitigations — [PyPI bluetti-mqtt](https://pypi.org/project/bluetti-mqtt).
- HA users: AC180 only stable at 60 s poll / 120 s timeout / 10 retries (ROADMAP §4.6, S19: hassio-bluetti-bt#176, bluetti-bt-lib#86). I could not retrieve those issues' text this session (unverified here; cited from ROADMAP).
- Forum reports: EB3A switches Bluetooth off after ~1 h idle with no solar/load ([Bluetti community](https://community.bluettipower.com/t/eb3a-bluetooth-shuts-off-while-unit-still-powered/24788), single user, unverified). This matches no pattern in our data (misses are steady, the EB3A answers between misses), but check EB3A load/solar state during misses.
- I found NO public source on Bluetti advert interval or on stopping advertising after a connection ends. ROADMAP §4.6 only says adverts stop while a client (phone) IS connected. Advert interval is unknown: measure it (below).

BlueZ facts:
- `Adapter1.ConnectDevice` connects "without need of performing General Discovery"; experimental; needs `AddressType` ("public"/"random", default BR/EDR); returns the device path; errors incl. AlreadyExists — [BlueZ Adapter API](https://bluez.readthedocs.io/en/latest/adapter-api/), [org.bluez.Adapter.rst](https://raw.githubusercontent.com/bluez/bluez/master/doc/org.bluez.Adapter.rst). It is experimental, so bluetoothd must run with `-E`; not checked here (unverified). The bluer wrapper's signature/availability I could not read (docs.rs page truncated): unverified.
- Alternative without discovery: `Device1.Connect` on an existing object. BlueZ creates the object only after seeing the device (or via ConnectDevice), so a cold cache needs one of the two.
- `SetDiscoveryFilter`: `Transport=le`, `DuplicateData` (default on = PropertiesChanged on each ManufacturerData sighting; we already read properties by polling). A second discovery client may merge filters; bluer says `set_discovery_filter` fails if discovery is already active — [docs.rs bluer Adapter](https://docs.rs/bluer/latest/bluer/struct.Adapter.html).
- Combo Wi-Fi/BT chips abort connections and lose scan time; community advice is a USB adapter — ROADMAP §4.6 (S20/S21, [Bleak troubleshooting](https://bleak.readthedocs.io/en/latest/troubleshooting.html)). The owner's goal host is a homelab box; the dev laptop chip is not the production target (RULES.md "What this is"), so a laptop-only result may not transfer.

"Signature does not verify" (handshake message 4, ECDSA over station point ‖ temp_iv — ROADMAP §4.4; error text `src/protocol/crypto/handshake.rs:109`):
- No public source found on its cause. Plausible, unverified: a truncated/interleaved notification from a link not fully torn down after the previous poll (our own session overlapped by ≤ 5 s cleanup disconnects), a corrupted/stale challenge from a half-closed station session, or the high-S signature open risk noted in ROADMAP §4.4. 6 in 720 on Elite only, 0 on EB3A/AC180, argues for a station-side or timing cause, not a key bug (a bad key fails every time).
- A fresh connect + handshake is a safe retry (new keys per connection; read-only). It is not a replay.

## Gaps and risks

- Root cause of `discover` misses is unmeasured. Rank by evidence:
  1. Cold scan per poll with a short, single window (strongest: only EB3A/Elite fail; AC180 never misses; cache always cold at 60 s; misses uniform over 12 h, so not an event). Station advert interval likely long on EB3A/Elite (hypothesis, unverified).
  2. Scan scheduling on the shared combo chip: discovery start/stop each poll, kernel default LE scan params (unverified duty cycle). JBL removal did not help — LOG step 18.
  3. Station-side: advertising pause/slowdown near a prior connection (no source) or after-disconnect state.
  4. Bluetti Bluetooth sleep (forum, one user) — weak.
- A18's "JBL coexistence" is falsified for EB3A (LOG step 18); keep measuring.
- A persistent connection fights golden rule #5 and the phone app (adverts stop while connected, ROADMAP §4.6); 3 persistent links from one laptop chip is also untested (the chip may cap concurrent LE links; unverified).
- Retry/longer scan inside the 20 s per-poll budget (ROADMAP §5.4) is tight: connect+handshake+read ≈ 4-5 s (§5.4), leaving ~15 s for discovery.

## Open questions

- Q1: may golden rule #5 change from "connect-read-disconnect per poll" to "hold the link, release on demand for the phone app"? (Trade-off below.)
- Q2: may we ask for a USB BT adapter (or is the target the homelab host's adapter, making the laptop numbers moot)?
- Q3: may bluetoothd run with `-E` (experimental, for `ConnectDevice`) or `TemporaryTimeout=0` in `/etc/bluetooth/main.conf`? Both are system-wide changes (human-only, RULES.md).

## Recommendation

Ranked, cheapest and least rule-bending first. Do 1-3 before touching rule #5.

1. **Measure first (A22 18a).** Passive: run one continuous LE discovery for ≥ 1 h (read-only, between runs, not during one) and log every sighting per station — gives advert interval distribution and the true p95 time-to-heard. Expected: tells whether 15 s is below p95 for EB3A. Source for method: our own `scan.rs`; no external source.
2. **Keep one discovery alive across polls** (one long-lived `discover_devices()` in the transport, shared by `find`). Removes per-poll scan start/stop and the cold cache: a station heard any time in the last 60 s is already cached with RSSI; `find` becomes a lookup. Keeps rule #5 (the station link is still per poll; only the host scan persists). Cost: continuous scan load on the shared combo chip, which may reduce connect/Wi-Fi throughput (unverified); BlueZ forbids changing the filter while discovery is active (bluer docs above) — set it once at start. Expected: the largest cut of `discover` misses if the cause is #1/#2 above. Phone app unaffected (scanning is not a connection).
3. **Extend the cache lifetime instead**: `TemporaryTimeout=0` ([main.conf](https://raw.githubusercontent.com/bluez/bluez/master/src/main.conf)) so the Device object survives between polls, and use `Device1.Connect` on it (no discovery). Caveat: a cached object still needs the station to be advertising at connect time (a connect is a scan for that device's adverts with BlueZ's connect-scan params), so on its own this may only move the miss from `discover` to `connect` (unverified). System-wide change, human-only (Q3). Better combined with 2.
4. **Bounded poll retries within the interval** instead of one 20 s deadline: after a `discover` miss, rescan (continuing, not restarting) until the round's budget ends; `ConnectDevice` with `AddressType` as a fallback for a cold cache once bluer support is confirmed (Q3). Expected: converts most single-window misses to successes without any rule change; budget is the limit (A22 requires a §5.4 decision first).
5. **Retry a handshake failure once** with a fresh connection (same poll if budget allows, else next tick counts as a miss anyway). Rationale: bluetti_mqtt retries 5× on parse errors; our 6/720 on Elite is a transient (cause unverified). Add the failing frame's length and time-since-previous-disconnect to the failure log first, so the cause can be settled (no secrets: mask MACs).
6. **Persistent connection (rule #5 change), only if 1-4 plateau below the goal.** Evidence for: bluetti_mqtt and the ESPHome bridges hold the link; removes discover+connect+handshake per poll (the `discover` failures disappear by construction; handshake runs once). Evidence against: the phone app cannot connect while we hold the link (adverts stop, ROADMAP §4.6) — mitigated by the planned pause-polling button, which must then also disconnect all three links; stations drop links over hours (HA AC180 reports), so reconnect logic returns with the same discovery weakness at each reconnect; 3 simultaneous links on one combo chip are untested; an outage (laptop on battery, stations switching) could drop a link at exactly the moment data matters. Net: do not adopt before 2+4 are measured; if adopted, it needs the owner's explicit rule change (Q1) and a hybrid: hold links, reconnect via the same long-lived discovery.
7. **Hardware**: a USB adapter on a USB 2.0 port with a short extension (ROADMAP §4.6; [Bleak troubleshooting](https://bleak.readthedocs.io/en/latest/troubleshooting.html)) removes the combo-chip variable. Cheap; ask the owner (Q2).

## Measurement plan

- Between runs (radio free), 1 h passive discovery log: per station, inter-advert gaps (p50/p95/max) and RSSI. Confirms or kills the "long advert interval" hypothesis. Also run it once with Wi-Fi off, once on (coexistence), and once with each station's phone app closed.
- After change 2: repeat the step 14 smoke, then the step 15 12 h run unchanged; compare per-station `discover` misses/hour (baseline above: EB3A 6-12/h, Elite 7→1/h, AC180 ~0).
- Log per failure: stage, elapsed in stage, time since the station's last successful poll, whether the discovery was warm (new fields; schema owner decision).
- Gate unchanged: ≥ 95 % per station (RULES.md); owner goal ~100 % tracked as a second column, not a gate.
