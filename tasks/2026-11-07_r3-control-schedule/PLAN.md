# Plan — R3 AC/DC output control + scheduled switching

> Written before work starts. **Do not edit the steps once work begins** —
> append changes under `## Amendments`. One step = one loop increment.

Owner's framing (BRIEF + 2026-11-07 addition): "super easy", "now just the
code, we will test it later", and "control schedule + manual control will be
useful from UI, super simple and effective". So the web UI is the primary
surface for both manual on/off and the schedule — one small control panel per
station, minimal JS, no new framework; the CLI `set` stays the minimal BRIEF
item for the first hardware checks. No step sends a write frame to a station,
enables `control.enabled` on any host or deploys (human-only, RULES.md); every
hardware check is a `HUMAN-CHECKLIST.md` row (step 14). Production (homelab,
v1.3.4) is untouched.

## Questions for the owner

All answered 2026-11-07 (relayed at the owner's request; Q3 and Q5 are the
delegated defaults the owner accepted).

- Q1 — Where is the schedule defined: in `config.toml` only (a change needs a
  restart) or editable in the web UI (a schedule table in the database, a
  form, a second write path)? · blocks: 5, 11, 12, 13a, 13b · answer:
  2026-11-07 (owner) — editable in the web UI, which is the primary surface
  for manual control too. Lean design: entries live in the database
  (migration 002, one row per station output, carrying its `last_due`), edits
  go through the single writer as `Record`s, the scheduler reads the entries
  from a read-only connection on every tick (no reload signalling), each
  station card on the status page carries one control panel (AC/DC toggle
  showing the read-back state, its `on`/`off` times inline), JSON routes
  `GET/POST/DELETE /api/schedule`, POST/DELETE behind the bearer like
  pause/control. Editing is allowed while `control.enabled = false`; the
  scheduler then sends nothing.
- Q2 — Schedule format: one entry per station output with daily local-time
  `on`/`off` (`HH:MM`, `general.timezone`), every day, no weekday filter, at
  most one entry per output, either time optional? · blocks: none · answer:
  2026-11-07 (owner) — kept, as UI fields: the smallest form that covers
  "router off at night, mobile on in the morning"; weekdays are a one-field
  addition later.
- Q3 — Missed-run rule (restart, station unreachable, host off at the due
  time): catch up to the latest due state, or skip the missed run? · blocks:
  none · answer: delegated 2026-11-07, accepted by the owner — catch up: per
  output, the most recent due time ≤ now is applied once when it is newer than
  the persisted `last_due`; a failed attempt is retried every scheduler tick,
  at most 10 times per due slot, until it succeeds or a later due time
  supersedes it. Skipping would "silently drop a run" (BRIEF done-when 4); a
  restart inside a slot never repeats it because `last_due` is in the
  database. On the very first run with a schedule the latest due state is
  applied too (the schedule declares the wanted state).
- Q4 — Which station outputs power the homelab host and the router (H11 is
  still pending: Elite AC outlet or AC180 USB-C for the host; the EB3A AC
  outlet feeds the router)? Switching one off from the LAN cuts the service or
  the LAN itself, and nothing switches it back on. · blocks: none · answer:
  2026-11-07 (owner) — "none — allow all": no protected outputs, no
  `deny_off`; the owner accepts that any output, the router's included, can
  be switched off from the LAN (see Risks).
- Q5 — Is `web.token` mandatory once `control.enabled = true` (ROADMAP.md §12
  R3 says "mandatory token"; today an unset token leaves the pause POST open)?
  · blocks: none · answer: delegated 2026-11-07, accepted by the owner — yes:
  config validation fails fast on `control.enabled = true` without a token; a
  write to a station is not a pause.

## Steps

- [x] 1. Spec: record the R3 slice in ROADMAP.md before any code (RULES.md
  "Decision process") — a new `ADR-010` line in §3 and the design in `spec §2.2`
  (R3 slice: what this task delivers, what waits; the UI is the primary
  surface, the CLI `set` is for hardware checks), `spec §4.2` (the 0x06 frame
  `01 06 <reg> <value> <crc>`, its 8-byte echo reply, exception = 5 bytes;
  encrypted stations: the plaintext frame inside the existing session envelope,
  **assumption, `verified: false`**, bt PR #59), `spec §4.3` (the allow-list: EB3A
  write 3007 AC / 3008 DC, read-back 48 / 49; AC180 and Elite 200 V2 write
  2011 AC / 2012 DC, read-back 2011 / 2012; value 1 = on, 0 = off; nothing
  else is ever written), `spec §6` (one table `schedule (station_id, output,
  on_min, off_min, last_due)` — minutes after local midnight, nullable, at
  least one set, one row per station output, `last_due` the unix time of the
  due slot last applied, `NULL` until the first run; a schedule edit upserts
  `on_min`/`off_min` and keeps `last_due`, a run updates `last_due` only, a
  delete removes the row; all through the single writer; the scheduler and the
  web read it from read-only connections), `spec §8.2` (`POST
  /api/control/output?station=&output=&state=on|off`, bearer required, replies
  the read-back state; `GET /api/schedule` (open), `POST /api/schedule?station=
  &output=&on=HH:MM&off=HH:MM` and `DELETE /api/schedule?station=&output=`
  (bearer; validated at the boundary: station exists, output per `switches()`,
  `HH:MM`, `on != off`, at least one; `202` — the writer applies it in order,
  so a `GET` may lag by a moment); `/api/status` cards carry
  `ac_output_on`/`dc_output_on` from the last sample when the profile reads
  them; one control panel per station card: an AC and a DC row, each a toggle
  showing the last known state (sample, then the POST's read-back; "—" when
  unknown, e.g. the Elite's profile has no 2011 block) and its `on`/`off`
  times inline with Save/Clear, the bearer taken from the pause form's one
  token field; the toggle is disabled with a "control disabled" note when
  `control.enabled = false`, the times stay editable), `spec §9` (`[control]
  enabled` only; token rule per Q5; no schedule in the config, Q1), `spec §R3`
  (this slice's boxes, the missed-run rule per Q3, editing allowed while
  disabled, the serialisation rule: in `serve` the collector executes commands
  between poll rounds over its own link and refuses them while polling is
  paused; the scheduler's retry cap (10 per due slot); the CLI `set` takes a
  config station name only and opens its own session, so with a running
  service on the same host the documented rule is "pause polling first"; Q4:
  no protected outputs). Each section ends with a stated recommendation. —
  check: the sections read back with `loop/where.sh --context` on a step
  citing `spec §R3`; `reviewer` pass (decision record = risky) with no
  CRITICAL/HIGH open.

- [x] 2. Protocol allow-list, pure (`spec §4.3`): new `src/protocol/control.rs`
  (+ `pub mod control` in `src/protocol/mod.rs`): `Output { Ac, Dc }` with
  `key()` (`"ac"`/`"dc"`) and `from_key`; `OutputSwitch` with **private**
  fields `output`, `register`, `read_back`, `verified` and getters only — no
  public constructor, so the only way to hold one is `pub fn switches(model)
  -> &'static [OutputSwitch]`, which lists exactly the four register pairs of
  step 1 per model, all `verified: false` (golden rule #2 enforced by the
  type). TDD: a test per model asserts the registers and that every switch is
  `verified() == false`; a test that `switches()` never lists a register
  outside {3007, 3008, 2011, 2012}. — check: `cargo test protocol::control`
  red then green, `make check`.

- [x] 3. [parallel: A] Write frame, pure (`spec §4.2`): in `src/protocol/
  modbus.rs` add `write_request(register, value) -> [u8; 8]` (function 0x06),
  `WRITE_RESPONSE_LEN = 8`, `expected_write_len(prefix) -> Option<usize>` (5 on
  the exception flag, else 8) and `parse_write_response(frame, register, value)
  -> Result<(), FrameError>` (CRC, function 0x06 or its exception, the echo must
  equal the request; else `FrameError::EchoMismatch`). TDD in `src/protocol/
  modbus/tests.rs` with hand-computed bytes (e.g. `01 06 0B BF 00 01` + CRC for
  EB3A AC on). No other file changes. — check: the new tests red then green,
  `make check`.

- [x] 4. [parallel: A] Config (`spec §9`, Q5): `Config::validate` rejects
  `control.enabled = true` without `web.token` (the error names both keys);
  `config.example.toml`'s `[control]` and `[web]` comments state the rule and
  that control and the schedule are driven from the web UI, never a config
  block. No new config keys (Q1, Q4). TDD in `src/config.rs` tests: the
  example still parses; enabled + no token is rejected; enabled + token
  parses. — check: the new tests red then green, `make check`.

- [x] 5. [parallel: A] Schedule table in the store (`spec §6`: single writer,
  numbered migrations) (needs Q1): `migrations/002_schedule.sql` creating
  `schedule (station_id INTEGER REFERENCES station, output TEXT, on_min
  INTEGER, off_min INTEGER, last_due INTEGER, PRIMARY KEY (station_id,
  output), CHECK (on_min IS NOT NULL OR off_min IS NOT NULL))`;
  `SCHEMA_VERSION` follows; `Record::ScheduleSet { station, output, on_min,
  off_min }` (upsert of the two times, `last_due` untouched),
  `Record::ScheduleDelete { station, output }` (deletes the row) and
  `Record::ScheduleRun { station, output, due }` (`UPDATE … SET last_due`,
  a no-op when the row is gone) through the writer;
  `store::read::schedule_entries(conn) -> Vec<ScheduleEntry { station_id,
  output, on_min, off_min, last_due: Option<i64> }>`. TDD in
  `src/store/writer/tests/` and `src/store/read/tests.rs` on a temp db:
  set/update keeps `last_due`, run updates it, delete, run after delete is a
  no-op, migration from version 1. — check: the new tests red then green,
  `make check`.

- [x] 6. [parallel: B] Transport write path (`spec §4.2`, golden rule #5): new
  `src/transport/write.rs` with `set_output(gatt, switch: &OutputSwitch, on:
  bool, policy: &ReadPolicy) -> Result<bool, WriteError>`: drain, one write
  attempt (no retry on the write itself: a timed-out write may have landed),
  reassemble up to `expected_write_len` within `policy.timeout`, check the
  echo, then `read_block(switch.read_back(), 1)` with the normal retry policy
  and return the read-back state; `WriteError::ReadBackMismatch { wanted, got }`
  when it differs. Tests in `src/transport/write/tests.rs` over `ReplayGatt`
  with hand-built fixtures (a `pub(crate) fn eb3a_ac_on_fixture()` that step 9
  reuses): echo ok + read-back 1; exception reply; echo mismatch; read-back
  mismatch; timeout. Only this file and `transport/mod.rs` (`pub mod write`)
  change. — check: the new tests red then green, `make check`.

- [x] 7. [parallel: B] The gate (`spec §R3`, golden rule #2): new
  `src/control/mod.rs` (+ `pub mod control` in `src/lib.rs`) with `Command {
  station: String, output: Output, on: bool, switch: &'static OutputSwitch }`
  whose fields are private and whose only constructor `Command::new(control:
  &ControlConfig, station: &StationConfig, output, on) -> Result<Command,
  Refused>` returns `Refused::Disabled` unless `control.enabled` and
  `Refused::NotAllowed` when `switches(model)` has no entry for the output
  (no protected list, Q4); plus `Reply = oneshot::Sender<Result<bool,
  String>>` and `Request { command, reply }`. Read-only accessors `station()`,
  `output()`, `on()`, `switch()` — callers reach `set_output` only through
  `command.switch()`. No I/O here. TDD: disabled refuses every station/output;
  enabled + listed builds with the right switch. — check: the new tests red
  then green, `make check`.

- [x] 8. Collector executes commands between rounds (`spec §R3`, `spec §5.4`,
  golden rule #5): `Collector` gains `commands: mpsc::Receiver<Request>`; the
  tick `select!` gains a branch that receives a request and runs it at once
  (no round is in progress then); requests arriving during a round wait for
  its end. New `src/collector/control.rs` with `impl Collector { async fn
  execute(&self, request) }`: resolve the target by name, `connect.open`
  (`retry` per the station's failure count), `with_disconnect { set_output }`
  under the poll deadline, `release` when not disconnected, log at `info` with
  station/output/state/read-back (never the MAC), answer the oneshot. `serve`
  creates the channel and passes the receiver (the sender is unused until
  step 10). `retry` is a `bool` passed in from the loop's failure count
  (`failures` is a local of `run`). While `*control.borrow()` is `Pause {
  until }` with now < until, `execute` answers `Err("polling paused")`
  without `connect.open` (the pause-first rule must hold for scheduler and
  web too). A request whose `reply.is_closed()` (the caller gave up) is
  dropped before opening a link. `set_output` is called only with
  `command.switch()`. Tests in `src/collector/tests/control.rs` with the
  existing `Fake`: the fake gatt records the exact 0x06 frame and answers the
  echo + read-back; a request during a round runs after the round's last
  disconnect, one link at a time; an unreachable station answers an error
  without a frame; a request during a pause answers "polling paused" without a
  frame; a closed-reply request opens no link. —
  check: the new tests red then green, `make check`.

- [ ] 9. [parallel: C] CLI `set`, minimal (`spec §R3`): `Command::Set { station,
  output, state }` (`bluetti-controller set router ac off`, needs `--config`;
  `station` is a config station name only — a MAC or an unknown name exits
  non-zero before any BLE), dispatched from `main.rs` with the owned config
  like `serve`; new `src/cli/set.rs`: `control::Command::new` first (a
  `Refused` exits non-zero **before** any BLE), then `session::run` around
  `pub(super) async fn apply(gatt: &mut impl Gatt, command: &Command, policy:
  &ReadPolicy) -> anyhow::Result<bool>` (calls `set_output(command.switch(),
  …)`), printing the read-back state. No extras (no status subcommand, no
  flags): it exists for the first hardware checks (H14–H16); the UI is the
  primary surface. Help text and `main.rs`' module doc drop the "only 0x03"
  claim and name the gate. Tests: argument parsing; a MAC and an unknown name
  fail before `session::run`; a disabled config fails before `session::run`;
  `apply` over `ReplayGatt` with step 6's `eb3a_ac_on_fixture()` returns the
  read-back `true` (the CLI line of done-when 3). — check: the new tests red
  then green, `make check`.

- [ ] 10. [parallel: C] Web control route (`spec §8.2`): `AppState` gains
  `commands: mpsc::Sender<Request>`, `control: ControlConfig` and `stations:
  Vec<Target>` (the `targets` `serve` already builds before `AppState::new`,
  `src/cli/serve.rs`; the id ↔ name/model map every later step uses); new
  `src/web/control.rs`, `POST /api/control/output?station=&output=&state=on|off`
  reusing `pause.rs`'s bearer check (`same` and `error` become `pub(super)`
  there, nothing moves — a sibling web-security task replaces that check with
  a router layer), `control::Command::new` → `403 {"error": "control is
  disabled"}`, unknown station/output `400`, `401` without the bearer, then
  send the request and await the reply with a bounded wait (poll deadline +
  one interval → `504` on timeout, `503` while shutting down); `200
  {"station","output","state"}` with the read-back. `serve` wires the sender.
  Also `/api/status` cards gain `ac_output_on` / `dc_output_on`
  (`Option<bool>` from the last sample's metrics, `null` when the profile lacks
  them) in `src/web/status.rs`. Tests in `src/web/control/tests.rs` with a
  fake receiver task: 401, 403 when disabled, 400, 200, 504; a status test for
  the two new fields. — check: the new tests red then green, `make check`.

- [ ] 11. Web schedule routes (`spec §8.2`, `spec §6`) (needs Q1): `AppState`
  gains `records: mpsc::Sender<Record>` (`serve` creates the record channel
  before the state); new `src/web/schedule.rs`: `GET /api/schedule` (open;
  `schedule_entries` from the reader in `spawn_blocking`, station ids mapped
  to names through `state.stations`, times as `HH:MM`), `POST /api/schedule?
  station=&output=&on=&off=` and `DELETE /api/schedule?station=&output=`
  behind the bearer, validated at the boundary (station in `state.stations`,
  `Output::from_key` and `switches(model)` has it, `HH:MM` → minutes, `on !=
  off`, at least one; else `400` naming the field) and sent as
  `Record::ScheduleSet` / `ScheduleDelete` with the target's id → `202` with
  the entry; editing is accepted while control is disabled. Tests in
  `src/web/schedule/tests.rs` with a fake record receiver: 401, each 400, 202
  with the right `Record`, GET lists what the temp db holds. — check: the new
  tests red then green, `make check`.

- [ ] 12. Per-station control panel on `/` (`spec §8.2`) (needs Q1): in each
  station card (`src/web/pages.rs`, the assets JS — plain DOM, no framework)
  an AC row and a DC row: a toggle button whose label shows the known state
  (the card's `ac_output_on`/`dc_output_on`, replaced by the POST's read-back
  after a click; "—" when unknown) that POSTs the step-10 route, and the
  row's `on`/`off` `HH:MM` inputs (filled from `GET /api/schedule`) with Save
  (POST) / Clear (DELETE) on the step-11 routes; the bearer is read from the
  pause form's single token field (no token input per row); while a request
  runs the row is disabled and an error reply is shown inline. With
  `control.enabled = false` the toggles are disabled with a "control disabled —
  the schedule will not run" note; the times stay editable. If `pages.rs`
  passes ~400 lines the panel template moves to `src/web/pages/control.rs`.
  Tests in `src/web/pages/tests.rs`: the panel renders both rows for every
  station, the toggles are disabled and the note shown only when control is
  disabled. — check: the new tests red then green, `make check`.

- [ ] 13a. Scheduler arithmetic, pure (`spec §R3`, Q3) (needs Q1): new
  `src/control/schedule.rs` with `fn due(entries: &[ScheduleEntry], now:
  Zoned) -> Vec<(station_id, Output, on: bool, due_ts)>` — per entry the
  latest `on`/`off` occurrence ≤ now (today's time in `general.timezone`, else
  yesterday's; a time skipped by a DST gap resolves with `jiff`'s default
  disambiguation), returned when newer than the entry's `last_due` (`None` =
  never run → the latest due state applies). TDD with hand-computed instants
  in `src/control/schedule/tests.rs` (the done-when 4 proof): due exactly at
  `off`; a restart 30 min after a missed `off` (`last_due` = yesterday's) is
  due once, and not with `last_due` = that slot; a restart 5 min after a
  recorded run is not due; `on` and `off` both missed → only the latest; an
  `off` moved earlier in the UI is due at once when already past; a DST-gap
  time resolves. — check: the new tests red then green, `make check`.

- [ ] 13b. Scheduler task + wiring (`spec §R3`, golden rule #5) (needs Q1): in
  `src/control/schedule.rs`, `pub async fn run(reader: Connection, stations:
  Vec<Target>, tz, control: ControlConfig, commands, records, shutdown)`:
  every 60 s read `schedule_entries` on its own read-only connection
  (`open_reader`, in `spawn_blocking`), take `last_due` as the greater of the
  row's value and this process's own recorded successes (so a writer that has
  not yet applied the `UPDATE` cannot cause a repeat), call `due`,
  `Command::new` each (a `Refused` is logged once, never retried that tick),
  send the request, on `Ok` send `Record::ScheduleRun` and remember the slot;
  on `Err` log and leave it (retried next tick, at most 10 consecutive
  failures per due slot — then one `warn!` and wait for the next slot). An
  entry deleted or edited in the UI is simply absent or new on the next tick.
  `serve` spawns the task only when `control.enabled` and stops it on
  shutdown. Tests in `src/control/schedule/tests.rs` with a temp db, a fake
  request receiver and paused time: a due entry sends one request and one
  `ScheduleRun` on `Ok`; a failed run is retried on the next tick and not
  after a newer slot; the 11th consecutive failure in one slot sends nothing;
  `control.enabled = false` sends nothing. — check: the new tests red then
  green, `make check`.

- [ ] 14. Docs batch + close evidence: README "Control" section (enable rule,
  token rule, the control panel: toggle + times per output, the missed-run
  rule, `set` for hardware checks, the pause-first rule, every register
  `verified: false`, Q4: no protected outputs); `HUMAN-CHECKLIST.md` rows H14
  (EB3A `router`: first `set router dc on` with the owner present — does the
  encrypted write frame go through, echo and read-back 49, the app shows the
  change; then `ac` with a lamp, not the router), H15 (AC180 `mobile`: the
  same on 2011/2012), H16 (Elite `living-room`: the same), H17 (from the UI:
  one toggle click shows the read-back, and one saved `on`/`off` time fires
  within a minute, once, with its `last_due` updated); ROADMAP `spec §R3`
  boxes ticked for this slice and the `verified: false` note. Then map each
  BRIEF done-when line to the test names (done-when 2: steps 7, 9, 10, 13b;
  3: steps 6, 8, 9 (`apply` over `ReplayGatt`), 10, 13b; 4: steps 5, 13a) and
  `make test`'s coverage line in `LOG.md`. — check: `make check` green and
  `src/protocol/` coverage ≥ 80 % in its output; `reviewer` batch pass over
  the docs.

## Risks / Dependencies

- **Encrypted write frames are unconfirmed** (bt PR #59): every station
  advertises `BLUETTF`, and the plan assumes a 0x06 frame inside the existing
  session envelope. If the first hardware check (H14) fails, the fix is in
  `src/transport/write.rs` / `crypto` only; the gate, CLI, web and scheduler stay.
  All four registers stay `verified: false` until H14–H16.
- **Any output can be switched off from the LAN, the router's included** —
  the owner's call (Q4, 2026-11-07): no protected list. If the switched outlet
  powers the router or the host, nothing switches it back on until the owner
  presses the station's button. What remains: golden rule #2 (default off,
  human-only enable), `OutputSwitch` obtainable only from `switches()`, the
  bearer on every write and schedule edit, the read-back shown in the panel,
  and the scheduler sending nothing while control is disabled.
- **The UI toggle's state can be stale or unknown**: it comes from the last
  poll (≤ one interval old) and the Elite's profile has no output-state block,
  so its rows show "—" until a click returns the read-back. Adding `(2011, 2)`
  to the Elite's poll blocks is not done here: an unsupported address fails
  the whole poll on that station (ROADMAP §4.3), a production risk for a
  cosmetic gain; a HUMAN-CHECKLIST row can test it later.
- **The schedule is data, not config**: a second write path into the store
  (web → writer `Record`s) and a reader in the scheduler. Kept lean: one
  table, no reload signal (one read per 60 s tick), no SSE update, `202` on
  edit; a stale `GET` for a moment after a `POST` is accepted in the spec, and
  the scheduler overlays its own successes on the row's `last_due` so writer
  lag cannot repeat a run.
- **A timed-out write may have landed**: step 6 never retries the write frame
  itself; the read-back is the truth and a mismatch is an error, not a retry
  storm (golden rule #5).
- **Two BLE owners**: `set` from the CLI while `serve` runs on the same host
  would fight for the link. Documented rule "pause polling first" (step 1/14),
  and the collector refuses commands while paused (step 8); on different hosts
  (laptop vs homelab) it is the existing "pause prod before dev reads" practice.
- **File sizes**: `src/collector/mod.rs` (426) and `src/protocol/device.rs`
  (462) are already past ~400, so new logic goes into new files
  (`collector/control.rs`, `protocol/control.rs`, `web/schedule.rs`,
  `control/schedule.rs`, `web/pages/control.rs` if needed); `mod.rs` files
  gain a field, a branch or a `pub mod`.
- **Clock and DST**: the scheduler uses `general.timezone` through `jiff`; a
  due time inside a DST gap is tested once in step 13a (default
  disambiguation), nothing more.
- **Parallel groups**: A = {3, 4, 5} write disjoint files (`protocol/modbus*`,
  `config.rs` + example, `store*` + migration) and consume only step 2's
  `Output`. B = {6, 7} (`transport/write*` vs `control/mod.rs` + `lib.rs`)
  consume steps 2–4 only. C = {9, 10} (`cli/set.rs` + `cli/mod.rs` + `main.rs`
  vs `web/control*` + `web/status.rs` + `web/mod.rs` + `web/state.rs` +
  `cli/serve.rs`) both consume step 8; step 9 also reuses step 6's fixture.
  Steps 11–13b are sequential: 11 shares `web/mod.rs`, `web/state.rs` and
  `cli/serve.rs` with 10; 12 consumes 10's and 11's routes; 13a consumes 5's
  `ScheduleEntry`; 13b consumes 13a, 11's record channel and the `serve`
  wiring. Each group gets one batched `reviewer` pass. Risky steps (1, 6, 7,
  8, 11, 13b — decision record, write path, gate, BLE owner, store write path,
  scheduler task) are reviewed before their commit; the rest in batch.
- **Out of scope stays out**: no charging mode / ECO / power-off register, no
  weekday filter, no CLI extras, no deploy, no real write (BRIEF).

## Escape hatches

- If a step fails 3 times: stop, log the blocker, set status: blocked, emit `<<LOOP:BLOCKED>>`.
- If a destructive / secret-touching / irreversible external action is required
  — sending any write frame to a station, setting `control.enabled = true` in
  a config a service reads, adding a register beyond the four output switches,
  deploying to the homelab — stop and hand to a human (`<<LOOP:BLOCKED>>`).
- If the gate fails (`make check` red, `src/protocol/` coverage < 80 %, a
  done-when line without a green test): `<<LOOP:GATE_FAILED>>` on the third
  attempt — fix the work, never the threshold.
- If H14–H16 later show the encrypted write frame needs a different envelope:
  a fix task against `src/transport/write.rs` / `crypto`, not an amendment here.

## Amendments
<!-- Append here during work; never edit the steps above. Two kinds of entry:
       - YYYY-MM-DD — plan: <a change to the plan, e.g. step 4 split into 4a/4b>
       - A1 · YYYY-MM-DD · MEDIUM `<file:line>` — <deferred reviewer finding> · disposition: open
     A1, A2, … are deferred reviewer findings (PROMPT §4b), one list item each.
     The increment that settles one rewrites its field: `disposition: fixed in <sha>`
     · `re-targeted → <CF-n or task>` · `declined — <reason>`. `make amendments`
     counts the ones still open; at the close every open one gets a disposition
     here and in OUTCOME.md. Only `- A<n>` items are counted. -->
- 2026-11-07 — plan (batch reviewer H1): `POST`/`DELETE /api/schedule` require `web.token` to be set,
  whatever `control.enabled` is: with no token they answer `403` "schedule edits need web.token". Why: with
  `src/web/pause.rs:29`'s optional-bearer pattern, anyone on the LAN could save an entry while control is
  off, and Q3 would fire it as a real write the moment the owner enables control. Applies to Q1's "behind
  the bearer like pause" and step 11: step 11's tests add "403 with no token configured" next to 401.
  Step 1's `spec §8.2` states the rule. Does not depend on the web-security task landing first.
- A1 · 2026-11-07 · MEDIUM `PLAN.md` step 12 vs `tasks/2026-11-07_web-security/PLAN.md` step 7 — R3's panel reads
  the bearer from the pause form's token field, which web-security step 7 removes. Step 12 sends the bearer only
  when that field exists and is non-empty, so either task order works (web-security A1 covers its side) · disposition: open
- A2 · 2026-11-07 · MEDIUM `PLAN.md` step 1 (`spec §4.3`), BRIEF :26-27 — the Elite 200 V2's 2011/2012 write and
  read-back are extrapolated from the AC180 / V2 base map; ROADMAP §4.3 has no Elite source. Step 1 marks the Elite
  pair `verified: false` with that source, and the Elite stays out of `switches()` until H16 confirms the pair, so no
  read-back failure can make the scheduler resend a write up to 10 times per slot · disposition: open
- A3 · 2026-11-10 · MEDIUM `tasks/2026-10-27_ble-reliability/BUG-station-ble-invisible.md:100-101` (batch review of a650183) —
  the verdict's "(fewer aborted connects, no burst before it)" is unsupported: the journal was not read (line 92) and
  daily misses rose to 30/47/51. Delete the clause or mark the abort count unknown · disposition: open
- A4 · 2026-11-10 · MEDIUM `tasks/2026-10-27_ble-reliability/BUG-station-ble-invisible.md:97` (batch review of a650183) —
  "Station uptime … 47 h (10-25)" is time since adverts resumed on 10-23 10:19 (~46 h), not a power-on; relabel or
  drop it · disposition: open
- A5 · 2026-11-10 · MEDIUM `ROADMAP.md` §R3 "Scheduler" + §8.2 panel (step-1 review M1) — a new entry has `last_due = NULL`,
  so saving a single past-due time (e.g. "router AC off 23:00" saved at 23:30) switches the output within 60 s while
  control is enabled, and with no `on` time nothing turns it back on. State it in the panel note / ADR-010, or ask the
  owner to refine Q3 (seed `last_due` on first save so an entry fires from its next slot) · disposition: open
- A6 · 2026-11-10 · MEDIUM `PLAN.md` step 12 (step-1 review M2) — step 12's "an AC row and a DC row" and its test "both rows
  for every station" contradict spec §8.2 / §4.3 (A2): rows only for outputs in `switches(model)`, none for the Elite until
  H16. Step 12 renders and tests rows per `switches(model)` · disposition: open
- A7 · 2026-11-10 · MEDIUM `src/control/mod.rs:20` (group B review M1) — "the gate cannot be skipped" holds by convention
  only: `set_output` and `switches()` are `pub`, so `set_output(gatt, &switches(model)[0], …)` compiles without a
  `Command`. Reword the doc, and the reviews of steps 8, 9 and 13b check every `set_output` call takes
  `command.switch()` (or `set_output` takes `&Command`) · disposition: open
- A8 · 2026-11-10 · MEDIUM `src/collector/tests/control.rs:76` (step-8 review M1) — no test reaches `failures[i] >=
  UNREACHABLE_AFTER`, and no test asserts the `retry` field of `opens`, so "retry per the station's failure count" and the
  index into `failures` are unchecked. Add a request to a station past 3 misses (`opens.last()` has `retry == false`, no
  write) and assert `retry == true` in the between-rounds test · disposition: open
- A9 · 2026-11-10 · MEDIUM `src/collector/control.rs:97` (step-8 review M2) — commands call `connect.open` once; polls retry
  a failed key exchange once on a fresh connection (A23). §R3 says "the station's normal retry". Share the handshake-retry
  block between `poll` and the command path, or state in §R3 that commands skip it · disposition: open
- A10 · 2026-11-10 · MEDIUM `src/collector/control.rs:78-80` (step-8 review M3) — the release-by-address branch of a command
  (deadline or shutdown mid-write) has no test. Add one: open, then a stalled read-back; `Err` names the poll deadline,
  `releases == 1`, `live == None`, the next round opens · disposition: open
