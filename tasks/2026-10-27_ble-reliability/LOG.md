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
