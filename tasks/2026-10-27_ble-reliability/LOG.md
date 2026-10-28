# Log

> Append-only. Newest at bottom. ≤40 lines per entry.

## 2026-10-27 — Session start
- Opened from the owner's report of fleet-wide poll failures. Investigation by a Fable subagent: INVESTIGATION.md (+ BUG-station-ble-invisible.md for #3).
- Verified on the host: `Memory: 127.8M (max: 128M … swap: 116.9M)`, VmRSS 139432 kB, PID up since Oct 16.
- Starting at step 1.

## Step 1 ✓ 2026-10-27 — one bluer Session per process; discovery start can no longer be stranded

**Changed:** `src/transport/bluer/mod.rs` — `session()`: a `tokio::sync::OnceCell<Session>`; `gatt.rs` (`release`, `connect`, `cached`) and `scan.rs` (`le_adapter`) use it. `scan.rs` — `start_discovery`: StartDiscovery on a spawned task, StopDiscovery when the handle drops.

**Check:** `#[ignore]` `the_shared_session_neither_grows_nor_jams_discovery` (laptop, hci0):
- red with a session per call: `org.freedesktop.DBus.Error.LimitsExceeded: The maximum number of active connections for UID 1000 has been reached` — dropped sessions keep their D-Bus connection (R1 confirmed);
- red after sharing, before `start_discovery`: `Bluetooth operation in progress: Operation already in progress` after 40 cancelled scans;
- green with both. `make check` green.

**reviewer:** 0 CRITICAL / 1 HIGH / 1 MEDIUM
- HIGH H1: cancelled `discover_devices` strands discovery on the shared client → fixed (`start_discovery`), reproduced red first.
- MEDIUM M1: churn test exercised no real scan → fixed: the merged test runs 40 cancelled + 2 full scans.

**Decision:** sharing beats reopening the session on error (no reopen logic; shortcut comment covers a dropped bus).
**Next:** step 2.

## Step 2 ✓ 2026-10-27 — unreachable stations back off 2, 4, 8 … ticks, capped at 10 min

**Changed:** `src/collector/mod.rs` — `due[]` per station, `backoff()`, `BACKOFF_CAP`; tests adjusted to the new schedule (`third_failure…`, `retry::an_unreachable…` run to tick 4).

**Check:** `an_unreachable_station_is_polled_ever_less_often_up_to_ten_minutes` (ticks `[0,1,2,4,8,16,26,36]`, the other station all 37) and `a_success_returns_a_backed_off_station_to_every_tick` (`[0,1,2,4,8,9,10]`) — red first (every tick polled), then green. `make check` green.

**reviewer:** with step 3, below.
**Next:** step 3.

## Step 3 ✓ 2026-10-27 — a clean poll sends one Disconnect

**Changed:** `src/collector/mod.rs` — `Progress::disconnected`, set when `with_disconnect`'s disconnect succeeds; `run` releases by address only otherwise. Fake's `disconnect` now clears `live`.

**Check:** release counts — clean poll 0 (was 1), `each_failure_names_its_stage` 3 (was 5), deadline cuts still 1 each; red first, then green. `make check` green.

**reviewer (steps 1 fix-up, 2, 3):** 0 CRITICAL / 0 HIGH / 3 MEDIUM — all fixed in this increment:
- M1 `scan.rs` restart could reuse the dying discovery token (no StartDiscovery sent) → `Discovery::stop` awaits the task before the next start.
- M2 `le_adapter` failed `DiscoveryActive` while a cancelled scan's discovery was still stopping → that error is ignored (the constant filter is already set on the shared session).
- M3 no test for "own disconnect failed → release" → `a_failed_disconnect_is_followed_by_a_release_by_address` (3 polls, 3 disconnects, 3 releases).
- Noted, not changed: the backoff gaps are 2, 4, 8, 10 ticks (skip 1, 3, 7, 9), not "skip 1, 2, 4"; chosen — same cap and reset.
`make check` green; ignored hardware test green on the laptop.

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-27T17:49:52Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-4653d4171d633f87)

running 3 tests
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok
test eb3a_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-27T17:50:15Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-4653d4171d633f87)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## Step 4 ✓ 2026-10-27 — 1.3.3 deployed, first hour on the host

**Changed:** nothing in code. `make release VERSION=1.3.3` (f97d777, tag v1.3.3); the owner ran `make deploy` (the classifier blocks it for the agent) at 19:04:52Z: healthy, v1.3.2 kept as previous.

**Check (host, 19:04:53–20:10:49Z, 66 min):**
- Memory: 4.2 MB at 17 s, 13.9 MB at 65 min (peak 16.3 MB), swap 0. Was 127.8 MB + 116.9 MB swap. Under 50 MB; "flat" needs a longer look (below).
- Polls: `/api/series` raw samples 66 of 65 minutes for each of router, living-room and mobile = 100 %. The AC180's "unreachable" at the deploy was stale from 1.3.2; its first 1.3.3 poll at +13 s succeeded.
- Journal: 0 "dropped while connected", 0 "discovery restart failed". 16 `le-connection-abort-by-local` connect retries and 2 Elite key-exchange retries, all recovered within the poll.
- bluetoothd "No matching connection": 199 in 65 min, 30 per 10 min = one per poll per station. Step 3 removed our second Disconnect, but one is still logged per poll; harmless log noise, not chased.
- 1.3.4 (uptime badge) was deployed at 20:11:24Z on the owner's request, after this window.

**Open:** done-when 4 stays unticked until memory is seen flat over ≥ 24 h (`systemctl status bluetti-controller` around 2026-10-28 20:00Z, well under 50 MB).

<!-- gate:begin (written by loop/step-done.sh — do not edit) -->
**Gate:** `make check` EXIT=0 · 2026-10-27T20:12:39Z

```text

     Running tests/replay.rs (target/llvm-cov-target/debug/deps/replay-0cbf12f887d84999)

running 3 tests
test eb3a_read_session_replays ... ok
test elite_200_v2_read_session_replays ... ok
test ac180_read_session_replays ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


    Finished report saved to target/llvm-cov-summary.json
>> src/protocol/ line coverage 98.86% (min 80%)
>> src/metrics/ line coverage 99.53% (min 80%)
>> make check: all gates green
```

<!-- gate:end -->

## 2026-10-28 — 24 h check of done-when 4: not met, task stays open

**Check (host, 2026-10-28 20:17Z, v1.3.4 up since 2026-10-27 20:11:24Z, read-only):**
- Memory: `MemoryCurrent` 19.7 MB (peak 20.5 MB), swap 0. Process: RssAnon 13.3 MB, RssFile 10.0 MB; cgroup anon 13.6 MB, file 5.3 MB.
  Was 4.2 MB at 17 s and 13.9 MB at 65 min (1.3.3). Well under 50 MB, but +5.8 MB in 23 h (~0.25 MB/h) is
  two points only: plateau (SQLite cache, allocator) or a slow climb cannot be told apart yet. At a linear
  0.25 MB/h the 128 MB cap is ~18 days away, so "flat" is not shown.
- Polls, last 24 h raw samples: router 1440/1440 (100 %), living-room 1437/1440 (99.8 %), mobile 1440/1440 (100 %).
- "dropped while connected": 3, all living-room (04:39, 07:59, 18:19Z). Each time the Elite stopped
  answering reads (3× "no notification within 5s"), then the link dropped; the poll failed at the 20 s
  deadline and the next poll succeeded. Not the R1 swap stall (memory 20 MB, swap 0): the station went
  silent, as in BUG-station-ble-invisible.md. 0 "discovery restart failed".
- Noise, all recovered within the poll: 298 `le-connection-abort-by-local` connect retries, 23 key-exchange retries.

**Verdict:** done-when 4 not met on "flat" (unproven) and "zero dropped" (3). Task stays in-progress.
Next: one more memory sample at ≥ 48 h (around 2026-10-29 20:00Z). A plateau near 20 MB closes the
memory part. The 3 Elite drops are the owner's call: accept them as the station's own silence (bug report)
or treat them as a miss.
