# Security

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub's private
vulnerability reporting (the repository's **Security** tab → **Report a
vulnerability**). Do not open a public issue for a security problem.

Include the affected version or commit, the component (`src/web`,
`src/transport`, `deploy/`, …), and the smallest reproduction you have. This is
a personal homelab project maintained on a best-effort basis; there is no
response-time guarantee.

Supported version: only the latest release on `main`.

## Threat model

`bluetti-controller` is a single binary that monitors Bluetti power stations
over local Bluetooth LE and serves a read-only web UI on the LAN. It is meant to
run on a trusted home network, not on the internet.

| Principal | Trust | What it can do by design |
|---|---|---|
| Local operator | Trusted | Owns `config.toml` and `keys.toml`, runs the CLI and deploy scripts |
| LAN web client | Semi-trusted | Reads all monitoring data; pauses polling (token-gated if `web.token` is set) |
| BLE station | Semi-trusted | Sends notifications, handshake frames and advertisements |
| Other BLE devices in radio range | Untrusted | Can advertise arbitrary names and data |
| Internet | Untrusted | No intended access; the service makes no outbound connections |

Protected assets: station encryption keys and the web token; station MAC
addresses and serial numbers; monitoring history; and the authority to send
device write frames (golden rule #2 in [`RULES.md`](./RULES.md)).

## Security controls in place

- **Local-first runtime.** No cloud SDK, no outbound network calls; stations are
  reached only over local BLE.
- **Device writes are off.** No Modbus write frame (0x06/0x10) is built or sent
  anywhere in the code. `control.enabled` defaults to `false`.
- **Secrets stay out of the repository.** `config.toml`, `keys.toml`, `*.db*`,
  `*.key`, `*.pem` and `captures/` are gitignored. Committed frame fixtures are
  anonymised (serials zeroed, ephemeral secrets dropped).
- **Secrets stay out of output.** `Debug` impls redact the token, station
  address and static keys; config and key errors never echo values; MAC
  addresses are masked on BLE error paths.
- **Input validation at boundaries.**
  - Config is parsed with `deny_unknown_fields` and validated at startup.
  - SQL uses bound parameters for every request value; metric names come from
    an allow-list.
  - HTML templates are autoescaped (askama), with no `|safe`.
  - Static assets are served from a compile-time table by exact name.
- **BLE protocol hardening.**
  - Frame length and CRC are checked before decoding.
  - Decoded values pass a plausibility gate before storage.
  - The handshake verifies the station's ECDSA signature before ECDH.
  - Every GATT operation has a timeout and bounded retries, and connections
    are released after each poll.
- **Web token.** When `web.token` is set, `POST /api/polling/pause` requires
  `Authorization: Bearer`, compared in constant time.
- **Service hardening.** The system unit
  (`deploy/bluetti-controller.system.service`) runs as a dedicated user, with
  an empty capability set, `NoNewPrivileges`, `ProtectSystem=strict`,
  `MemoryDenyWriteExecute`, a syscall filter and a memory cap.
- **Release integrity.** `deploy/deploy.sh` verifies the tarball SHA-256 on the
  host before extraction; `bluetti-update` checks `SHA256SUMS` before running a
  release.

## Known limitations

From the audit of 2026-11-05 (commit `e8331a9`). None is a confirmed
vulnerability; each depends on how the service is deployed.

1. **Unauthenticated reads, no Host/Origin check.** All GET routes and the SSE
   stream are open to anyone who can reach the listener. A malicious page in a
   LAN browser could use DNS rebinding to read monitoring data.
2. **Pause endpoint open by default.** With `web.token` unset, a cross-site form
   POST from a LAN browser can pause polling for up to 60 minutes per request.
   This affects monitoring only; no device frame is sent.
3. **Unbounded history queries.** `/api/outages` and `/api/series` accept any
   time span. `grid_event` rows are never pruned, so one request can read the
   whole outage history. A very large negative `from` causes an HTTP 500 in
   debug builds.
4. **BLE replies are not bound to requests.** A late reply within the same
   connection can be accepted as the answer to a newer read. Exploiting this
   needs a link-level attacker.
5. **Advertised names are printed raw.** `scan`, `detect` and the session
   banner print the advertised name without stripping control characters.

## Deployment recommendations

- Set `web.token` in `config.toml`.
- Bind `web.listen` to a specific LAN address, not `0.0.0.0`, and firewall the
  port from untrusted networks. Never expose it to the internet.
- Keep `keys.toml` and `config.toml` at mode `0600` (or `0640 root:bluetti` for
  the system unit). The binary does not check file modes.
- Prefer the system unit over the user unit on a shared host; the user unit has
  no sandboxing.
- Deploy with `deploy/deploy.sh` rather than running `bluetti-update apply` on
  a directory, so the release is checksummed against the tarball built on your
  machine.

## Planned hardening

- Add a Host/Origin allow-list and an Origin check on the pause route.
- Cap the time span on `/api/series` and `/api/outages`, and use checked
  arithmetic on request bounds.
- Strip control characters from advertised names before printing.
- Add a whole-body deadline to `with_disconnect`, and reopen the D-Bus session
  after a bus loss.
- In `bluetti-update`: copy the release to a root-owned staging directory
  before verifying and running it; refuse symlinks in the release tree; extract
  without preserving setuid bits.
- Before wiring any device write path: add a typed gate so a write frame can
  only be built when `control.enabled` is true and the register is on the
  allow-list.
