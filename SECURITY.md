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
over local Bluetooth LE and serves a web UI on the LAN behind one owner login.
It is meant to run on a trusted home network, not on the internet.

| Principal | Trust | What it can do by design |
|---|---|---|
| Local operator | Trusted | Owns `config.toml` and `keys.toml`, runs the CLI and deploy scripts |
| LAN web client | Untrusted until logged in | Reaches `/login`, `/assets/*` and `/healthz`; everything else needs the session or the bearer |
| Logged-in browser or bearer script | Trusted | Reads all monitoring data, pauses polling, switches outputs and edits the schedule when `control.enabled = true` |
| BLE station | Semi-trusted | Sends notifications, handshake frames and advertisements |
| Other BLE devices in radio range | Untrusted | Can advertise arbitrary names and data |
| Internet | Untrusted | No intended access; the service makes no outbound connections |

Protected assets: station encryption keys, the web token and the password hash; station MAC
addresses and serial numbers; monitoring history; and the authority to send
device write frames (golden rule #2 in [`RULES.md`](./RULES.md)).

## Security controls in place

- **Local-first runtime.** No cloud SDK, no outbound network calls; stations are
  reached only over local BLE.
- **Device writes are off by default** (ADR-010). Only the AC/DC output
  switches of `src/protocol/control.rs` can ever be written, through a command
  that is built only when `control.enabled = true`; the default is `false`.
  `control.enabled = true` needs both `web.token` and `web.password_hash`.
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
- **Web login** (ADR-011, ROADMAP.md §8.5). One owner password, stored as an
  argon2id PHC string in `web.password_hash` (`bluetti-controller
  hash-password` reads it from stdin). A login sets an `HttpOnly;
  SameSite=Strict` session cookie for 30 days, kept in memory (at most 16).
  Five failed logins in a row lock `POST /login` with `429` for 60 s; each
  attempt is counted before verification, and one verification runs at a time.
  `next` is followed only for a same-site path of visible ASCII.
- **Request guard.** Two layers on the router, none per handler:
  - every route except `/login`, `/assets/*` and `/healthz` needs the session
    cookie or `Authorization: Bearer <web.token>` (constant-time compare);
    otherwise `401` for `/api/*` and `/events`, `303 /login` for pages;
  - every route except `/healthz` needs a `Host` that is an IP literal,
    `localhost` or an entry of `web.allowed_hosts` (`421` otherwise: DNS
    rebinding);
  - every unsafe method without a bearer needs an `Origin` equal to
    `http://<Host>` or `https://<Host>` (`403` otherwise: CSRF).
  A test lists every route as public or protected, so a new route fails the
  build until it is listed.
- **Security headers** on every response: `Content-Security-Policy` (`'self'`
  only, `frame-ancestors 'none'`), `X-Frame-Options: DENY`,
  `X-Content-Type-Options: nosniff`, `Referrer-Policy: same-origin`. The pages
  hold no inline script.
- **Config rules.** A listener that is not loopback, or `control.enabled =
  true`, fails at startup without `web.password_hash`. Without it (dev mode)
  every GET is open on loopback. A `config.toml` or
  keys file with a group-write or any other-user mode bit is warned about.
- **Bounded queries and clean output.** `/api/series`, `/api/outages` and
  `/api/chart` refuse a span over 366 days or a negative bound, with checked
  arithmetic. Advertised names have their control characters replaced before
  `scan`, `detect` or the session banner prints them.
- **Service hardening.** The system unit
  (`deploy/bluetti-controller.system.service`) runs as a dedicated user, with
  an empty capability set, `NoNewPrivileges`, `ProtectSystem=strict`,
  `MemoryDenyWriteExecute`, a syscall filter and a memory cap.
- **Release integrity.** `deploy/deploy.sh` verifies the tarball SHA-256 on the
  host before extraction; `bluetti-update` checks `SHA256SUMS` before running a
  release.

## Known limitations

From the audit of 2026-11-05 (commit `e8331a9`), updated by task
`2026-11-07_web-security`. None is a confirmed vulnerability; each depends on
how the service is deployed. Closed by that task: unauthenticated reads and no
Host/Origin check, the open pause endpoint, unbounded history queries, and raw
advertised names.

1. **BLE replies are not bound to requests.** A late reply within the same
   connection can be accepted as the answer to a newer read. Exploiting this
   needs a link-level attacker.
2. **The session cookie is not `Secure`.** The app speaks plain HTTP, so the
   cookie and the password cross the LAN in clear text unless a TLS proxy is in
   front (Deployment recommendations). Wi-Fi link encryption is the only
   protection without one.
3. **Sessions end on restart.** They live in memory; every release or restart
   needs a new login.
4. **One global lockout.** Five wrong passwords from any LAN client lock the
   login for everyone for 60 s. A LAN attacker can keep the owner locked out,
   but cannot guess faster than five tries a minute.
5. **Bidi controls pass through advertised names.** Only control characters
   (Cc) are replaced; Unicode bidi overrides such as U+202E (Cf) still reach
   the terminal (task amendment A5).

## Deployment recommendations

- Set `web.password_hash` (`bluetti-controller hash-password`) and a long
  random `web.token` for scripts. A LAN listener refuses to start without the
  hash.
- Reaching the UI by a DNS name (e.g. `ideapad.lan`) needs that name in
  `web.allowed_hosts`; IP addresses need no entry.
- For TLS or remote access, put a proxy in front and keep the listener on
  loopback: Caddy with its internal CA (`deploy/HOMELAB.md`), or Tailscale
  Serve / WireGuard. Never expose the port to the internet.
- Bind `web.listen` to a specific LAN address, not `0.0.0.0`, and firewall the
  port from untrusted networks.
- Keep `keys.toml` and `config.toml` at mode `0600` (or `0640 root:bluetti` for
  the system unit); every command warns about looser modes at config load.
- Prefer the system unit over the user unit on a shared host; the user unit has
  no sandboxing.
- Deploy with `deploy/deploy.sh` rather than running `bluetti-update apply` on
  a directory, so the release is checksummed against the tarball built on your
  machine.

## Planned hardening

- Replace Unicode bidi controls in advertised names, and the security test
  suite with a 100 % line-coverage gate on the security files (task
  `2026-11-07_web-security`, step 10a).
- Add a whole-body deadline to `with_disconnect`, and reopen the D-Bus session
  after a bus loss.
- In `bluetti-update`: copy the release to a root-owned staging directory
  before verifying and running it; refuse symlinks in the release tree; extract
  without preserving setuid bits.
- Confirm each output register on real hardware (`HUMAN-CHECKLIST.md`
  H14–H17) before enabling control on the homelab, and only after the login is
  deployed (H19).
