# Bug: a station becomes invisible to both the official app and the controller

Seen on the Elite 200 V2 (`living-room`), twice. Owner-handled; this page is
the evidence collected on 2026-10-27, nothing more. No MACs or serials.

## Symptom

The station is powered, in use (it powers the TV) and its own display works,
but it sends no BLE advertisements: the controller's `find` scan on the
homelab host never hears it, a `bluetoothctl scan le` on the dev laptop never
lists it, and the official Bluetti Android app cannot find it either (it did
10 days earlier; no app was connected at the time). The EB3A and AC180 are
heard by the same scans at the same moments.

## Timeline

| when (local) | event |
|---|---|
| Oct 22 23:18–23:54 | 13 aborted connects (`le-connection-abort-by-local`), 1 handshake "signature does not verify" |
| Oct 23 00:00 | silent; `unreachable`; 619 polls fail at `discover` |
| Oct 23 10:19 | heard again by itself; polls OK — no action taken by anyone |
| Oct 25 08:25–08:41 | 8 aborted connects in 16 min (2 polls fail at `connect`, station still advertising: BlueZ cache "warm") |
| Oct 25 08:42 | silent; `unreachable` from 08:44; 2,413 `discover` failures over the next 52 h |
| Oct 25–27 | host and laptop scans: no adverts; app: not found |
| Oct 27 ~13:05 | owner power-cycles it; heard at 13:05, first good poll 13:08 |

Both silences were preceded by a run of aborted connection attempts; the
Oct 23 silence ended on its own after ~10 h, the Oct 25 one needed a power
cycle (52 h).

## Observed from the host, the laptop and the app

- Host (`btmon`, Oct 27): the controller's scan is an active LE scan at 100 %
  duty (interval = window = 11.25 ms); it stops 0.36 s after a station is
  heard. During the silence it ran 15 s every minute and heard nothing from
  the Elite.
- Laptop: `bluetoothctl scan le` listed the EB3A and AC180, not the Elite.
- App: "device not found" (owner), no app connection open.
- After the power cycle: adverts resume, encrypted handshake and reads work.

## Ruled out

- Our scanning: the laptop's independent adapter saw the same silence.
- Our holding the link: every poll ends with a disconnect by handle and by
  address; `bluetoothctl devices Connected` empty; `btmon` shows every link
  closed by the host within 20 s.
- Connection storms: ≤ 3 connect attempts per poll, ≥ 2 s apart, 1 poll per
  minute; max 7 attempts in any minute across all stations in 5 days.
- The host adapter: 0 HCI RX/TX errors, no btusb resets in dmesg; the AC180
  was polled at 100 % through the whole Oct 25–26 silence.
- The app holding the link ("a station stops advertising while the app is
  connected", ROADMAP §4.6): the app was not connected and could not find it.

## Possible links to problem #2 (unreachable while powered)

The controller's aborted connects (`le-connection-abort-by-local`, returned in
< 1 s, 1,247 in 5 days) hit all three stations. Only the Elite goes silent
after a burst of them. A plausible but unproven reading: the Elite's BLE
stack stops advertising after N connection attempts that fail at the link
layer (the central sends CONNECT_IND, the link never settles), and only a
reboot of its BLE firmware clears it. INVESTIGATION.md steps 1–3 reduce the
aborts' causes (process stalls, double disconnects) but do not address the
station's own behaviour.

## Suggested next diagnostic steps (owner)

1. If it goes silent again: note the time, open `journalctl -u
   bluetti-controller --since "-1h" | grep living-room` and count the
   `le-connection-abort-by-local` lines before the silence — a third
   "burst then silence" makes the link to #2 strong.
2. While silent: pause polling (`POST /api/polling/pause`, 30 min) and check
   whether adverts come back on their own (`bluetoothctl scan le` on the
   laptop); if they do, our attempts keep it silent; if not, it is wedged.
3. While silent, before power-cycling: try the app with Bluetooth on the phone
   toggled off/on, then try the station's own BLE reset if the manual has one
   (some Bluetti models re-enable BLE from the settings menu) — tells a
   firmware hang from an advertising stop.
4. Check the Elite's firmware version in the app and whether an update exists;
   community reports (ROADMAP §4.6 S17) mention Elite 200 V2 "app 3.0 BT
   scanning device not found" — same symptom, no cause given.
5. If it recurs with 1.3.3 deployed (fewer aborts), it is station-side
   regardless of us; if it stops recurring, the aborts were the trigger.
