# BLE reliability on the homelab host — investigation (2026-10-27)

Read-only investigation of v1.3.2 on the IdeaPad (Debian 13, BlueZ 5.82, QCA
combo chip). Nothing on the host was changed. Stations are named, never
addressed. Owner facts (2026-10-27): all three stations are powered and in use
(AC180 `mobile` powers this host; EB3A `router` was off Oct 24–26 waiting for a
DC cable — ignored; Elite 200 V2 `living-room` powers the TV).

The owner's three problems: **#1** polling disruptions across the fleet,
**#2** stations unreachable to us while powered, **#3** a station invisible to
both the official app and us (the Elite). #1 and #2 are ours and have a plan
below. #3 is not pinned down by the evidence; it has its own bug report,
[`BUG-station-ble-invisible.md`](./BUG-station-ble-invisible.md).

## 1. Timeline and measured evidence

Journals: `sudo journalctl -u bluetti-controller --since 2026-10-23` (10,283
lines), `-u bluetooth` (12,665 lines). The service has run as PID 202686 since
Oct 16 21:20 (no restart in the window).

**Poll failures by day / station / stage** (`grep "poll failed" | stage`):

| day | router | living-room | mobile | notes |
|---|---|---|---|---|
| 23 | — | discover 619, connect 6 | — | Elite silent 00:00–10:19, came back alone |
| 24 | discover 1355 | connect 10, read 9 | connect 1 | EB3A off (owner) |
| 25 | discover 1498 | discover 916, connect 9 | connect 3 | Elite silent from 08:42 |
| 26 | discover 1084 | discover 1439 | 0 | AC180 100 % |
| 27 | discover 343, connect 24, **handshake 55, read 25** | discover 712, connect 67, handshake 30, read 19 | **connect 117, handshake 24, read 60, discover 24** | all 3 degrade from 09–10 h |

Oct 27 by hour: 00–08 h only `discover` (absent stations); from 09 h on,
`connect`/`handshake`/`read` failures and poll-deadline cuts on every station:
"dropped while connected" + "no result within the 20s poll deadline" per hour:
`09:17 10:59 11:53 12:80 13:51 14:53 15:64 16:56 17:67 18:58` (Oct 23–26 total:
17). `unreachable`/`reachable again` flapped 45 times today, 5 times in Oct 23–26.

**Service memory (the decisive host finding).** Unit has `MemoryMax=128M`
(RULES.md gate: ≤ 50 MB resident):

```
systemctl status: Memory: 127.8M (max: 128M, available: 188K, peak: 128M, swap: 116.4M)
/proc/202686/status: VmRSS 138884 kB  VmSwap 59572 kB  (smaps: Pss_Anon 129 MB)
cgroup memory.events: max 42656   memory.stat: anon 132.7 MB, pgmajfault 2105212
cgroup memory.pressure: full avg60=6.24 avg300=14.10 total=9073 s
cgroup io.pressure:     full avg60=24.63 avg300=35.68 total=15601 s
/proc/202686/io: read_bytes 9.5 GB (a 13 MB database)   bluetoothd: VmRSS 11 MB
90 s sample: pgmajfault +3997 (44 swap-ins/s), memory.current pinned at 134.0 MB
```

The full-stall totals (9,073 s mem / 15,601 s io) at the current rates (14 % /
35 %) accumulate in 12–18 h, so the thrashing began late Oct 26 / early Oct 27 —
the same morning the connect/handshake/read failures started on all stations.
Host RAM is 7.7 GB, 5 GB free: only the unit cap forces the swap.

**btmon, 130 s** (`sudo timeout 130 btmon -w`, 2 polls + the start of a third):

```
3.80  LE Create Connection → 3.87 connected (handle 21)
3.94–4.88  BlueZ GATT discovery (ATT reads)            ← 1 s, normal
4.88–24.50 NOTHING on the link, no ff02 write          ← our process idle 19.6 s
24.51 Disconnect (our 20 s poll deadline)
79.48 LE Set Scan Enable (11.25/11.25 ms, accept all)  ← our find() scan
79.84 scan off (station heard after 0.36 s)
80.37 Create Connection → 80.41 connected; 80.45–81.42 GATT discovery
81.42–86.11 nothing (4.7 s stall) → 86.11–86.89 handshake → 87.98–88.36 reads → 90.51 Disconnect
```

A healthy poll is connect 0.1 s + GATT 1 s + handshake 0.8 s + reads 0.4 s ≈
2.5 s; the 20 s budget is eaten by process stalls. All 3 links in the capture
were closed by us (`Reason: Remote User Terminated` from the host), none left up.

**Connect aborts.** 1,247 `le-connection-abort-by-local` in 5 days (347 / 428 /
224 / 114 / 194). They come back fast: attempt-1→attempt-2 gap minus the 2 s
backoff is p50 0.0 s, p90 1.0 s, max 4.0 s (n=173) — local cancellations, not
15 s timeouts. bluetoothd: `No matching connection for device` 12,122×
(≈ 1 per poll), `device_add_connection() Device … is already connected` 542×
(30 in the 09 h burst today). dmesg today: `ACL packet for unknown connection
handle 12` (14:28). No btusb resets, no hci errors (`hciconfig`: RX/TX errors 0).

**Adapter state now:** `Discovering: no`, 0 connected devices, 1 device object in
the BlueZ cache (the AC180), `/etc/bluetooth/main.conf` all defaults. Wi-Fi:
`ath10k … Spurious quick kickout` bursts since Oct 17 (background, predates the
problem; one burst today 13:00).

**Code facts (v1.3.2; `src/transport`, `src/collector` unchanged since
2026-10-15, before v1.1.0 — `git log v1.0-mvp..HEAD -- src/transport src/collector`):**
- One poll = `find` (BlueZ cache, else an LE scan that stops when the station is
  heard, ≤ 15 s) → `connect` (≤ 3 attempts, backoff 2 s/4 s, 15 s each) →
  handshake (≤ 1 retry on a fresh link) → reads (3 tries/block) →
  `Gatt::disconnect` → `release()` = a second `Device1.Disconnect` by address.
  All under a 20 s deadline (`interval / n_stations`). Polls are serialised.
- `unreachable` only drops connect attempts to 1 (`collector/bluez.rs:57`); the
  15 s scan still runs every minute (`gatt.rs:44 find`) — no poll backoff.
- Every BlueZ call opens a **new `bluer::Session`** (D-Bus connection + 3
  tokio tasks): `cached`, `scan_for`, `connect`, `release` → 3–4 per poll,
  ≈ 15,000 per day. In bluer 0.17.4 `Session::new` spawns an event loop
  (`session.rs:441`) that, once the session drops, awaits
  `connection.remove_match(…)` ×3 on a connection whose I/O task was just
  aborted (`session.rs:150`) — a reply that may never come. `busctl list`
  shows 11–18 names still registered for the service user at any moment.

## 2. Root causes, ranked

**R1 — The service is swap-thrashing against its 128 MB cap (confidence ~90 %).
Causes #1 and most of #2.** RSS 139 MB + 122 MB swapped, 44 major faults/s,
`memory.max` hit 42,656×, btmon shows our process idle for 19.6 s on an open
link and 4.7 s before a handshake. Onset ≈ Oct 27 morning = onset of
connect/handshake/read failures on all three stations, including the AC180
that sits beside the host and powers it. It explains every Oct-27 signature:
deadline cuts (382 "no result within 20s"), `handshake out of order: expected
message 6, got 4` (we resend message 5 after a 1 s stall, the station restarts),
read timeouts, 163 "dropped while connected", the AC180 flapping unreachable.
Working set > 128 MB after 11 days vs ~20 MB expected ⇒ a leak. Leading suspect:
the per-call `bluer::Session` (above), ~1.5 KB/session × 15 k/day ≈ 20 MB/day
fits the growth. Unconfirmed until step 1 measures it.

**R2 — Fast local connect aborts from stale link state in BlueZ/controller
(confidence ~60 %). The pre-Oct-27 part of #2.** 1,247 aborts return in < 1 s;
bluetoothd sees "already connected" 542× and a disconnect for a connection it
does not have once per poll; the kernel saw an ACL packet for a dead handle.
Our pattern feeds it: two `Disconnect`s per poll, a retry 2 s after an abort,
and under R1 a deadline cut tears the link down mid-GATT. Not a station
problem: aborts hit all three stations alike. Amplified, not caused, by R1.

**R3 — No backoff for absent stations (confidence: certain as behaviour, low as
a cause).** An absent station costs a 15 s active scan at 100 % duty
(interval = window = 11.25 ms) every minute; Oct 25–26 with two absent stations
that was 30 s/min of radio on the Wi-Fi/BT combo chip. It touches no station,
and the AC180 stayed at 100 % through it, so it is load, not the cause.

**Hypothesis "we are hammering the stations" — refuted with numbers.** Max 7
connect attempts in any minute fleet-wide (busiest minute in 5 days), ≤ 3 per
poll, ≥ 2 s apart, 1 poll/min/station; handshake retries ≤ 1 per poll, 6–26 per
day; discovery runs 0.36 s when the station answers and is off between polls
(`Discovering: no`); every poll ends with a disconnect (btmon: 3 of 3 links
closed by us; 0 connected devices). The official app holds one link and polls
over it at ~1 Hz — far more traffic than ours. What *is* abnormal is the
quality of our attempts since Oct 27 (R1): links held open and idle for 20 s,
then cut.

**Elite invisible to app and controller (#3):** both silences (Oct 23 00:00,
Oct 25 08:42) were preceded by 8–13 aborted connects in the previous 15–35 min;
the AC180 and EB3A survive the same bursts. Station-side; cause not pinned —
see the bug report.

## 3. Fix plan

The smallest set that fixes #1 and #2; each step cites its evidence. TDD with
`Fake`/`ReplayGatt` (`src/collector/tests.rs`, `src/transport/replay.rs`);
check = `make check` unless stated. **Code** = agent; **Human** = owner.

1. **One `bluer::Session` per process (R1). Code.** `src/transport/bluer/gatt.rs`
   (`find`, `cached`, `release`, `connect`) and `src/transport/bluer/scan.rs`
   (`le_adapter`, `listen_for`, `discover`) take an `&bluer::Adapter` instead
   of opening a `Session` each; `BluezConnect` (`src/collector/bluez.rs`) holds
   the one adapter, created in `src/cli/session.rs` for `serve` and the CLI.
   Test first: `#[ignore]` `session_churn_does_not_grow_rss` in
   `src/transport/bluer/gatt.rs` — 2,000 × `Session::new()` + drop, RSS from
   `/proc/self/statm` grows < 2 MB (red on 0.17.4 if the suspect holds; if it
   stays green, the leak is elsewhere: run `serve` 2 h on the laptop under
   `heaptrack` before touching more code). Check: `make check` + a 2 h laptop
   `serve`, RSS at 0/1/2 h in the LOG — flat.
2. **Poll backoff for unreachable stations (R3; also the "backoff" the owner
   asked for). Code.** `src/collector/mod.rs` `Collector::run`: once `failed ≥
   UNREACHABLE_AFTER`, skip the station for 1, 2, 4 … 9 ticks (cap 10 min),
   reset on success; skipped ticks store nothing. Test in
   `src/collector/tests.rs` (paused time, always-failing `Fake`): polls at
   ticks 0,1,2,4,8,16,26,36…; a success returns it to every tick; the other
   stations keep every tick. Effect: 15 s of 100 % duty scan per 10 min, not
   per minute, per absent station.
3. **One `Disconnect` per poll (R2's bluetoothd noise, 12,122 lines). Code.**
   `src/collector/mod.rs:145`: call `release` only when `Gatt::disconnect` did
   not succeed (deadline, error before the GATT handle, shutdown). Test:
   `a_clean_poll_releases_once` on `Fake` — 0 `release` calls after a success,
   1 after a deadline cut. Before/after metric: `No matching connection for
   device` per day in `journalctl -u bluetooth`.
4. **Release 1.3.3. Code; deploy on owner go.** CHANGELOG + `make release`;
   gate after 24 h: ≥ 95 % polls per powered station, `systemctl status`
   `Memory:` flat and < 50 MB, zero "dropped while connected".
5. **Human — now.** `sudo systemctl restart bluetti-controller` clears the
   thrash; the AC180 and EB3A should return to ~100 % until the cap refills
   (~5–6 days at the observed rate). Keep `MemoryMax=128M`: it is what made
   the leak visible.

**Not doing (why):** connect-backoff tweaks or a wait-for-disconnect after an
abort — the abort mechanism (R2) is 60 % confidence; re-count aborts/day after
1.3.3 first · handshake retry limit — already 1/poll, the Oct-27 spike is R1 ·
longer scan window / continuous discovery — btmon: the station is heard in
0.36 s, the budget is lost to stalls · per-station counters, `rss_kb` in
`/healthz` — the journal and `systemctl status` already give the before/after
numbers · a watchdog restart on memory — symptom, not cause · BlueZ config,
`-E`, cache `remove`, another adapter — 0 HCI errors, no resets, one stale
object that expires in 30 s · Wi-Fi coexistence — predates the problem.

## 4. Owner actions needed now

1. Restart `bluetti-controller` (step 5) and note `Memory:` in `systemctl
   status` once a day until 1.3.3 lands — a steady climb confirms R1's leak.
2. Nothing physical for #1/#2. For #3 see the bug report (Elite).
