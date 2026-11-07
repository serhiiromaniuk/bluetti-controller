# Security — enclose the web UI and API  |  2026-11-07_web-security

> Static once written. The contract for this task. If scope changes, write a new
> task — don't rewrite this.

## What
Close the web UI and API, which are open on the LAN today: every page, route and
SSE stream needs a login, cross-site and DNS-rebinding requests are refused, and
the known limitations in `SECURITY.md` are fixed.

## Why
Owner, 2026-11-07 (recorded at the owner's request): "plan a security module of
enclosing this app and making it more secure than plain open … scaffold this
task, we will start working on it later." R3 (`2026-11-07_r3-control-schedule`)
puts device writes and a schedule editor in the UI, which raises the stakes.
Today any LAN client can read everything, and the pause POST is open when
`web.token` is unset (`SECURITY.md` "Known limitations" 1–2). Golden rule #2
(device writes) and #3 (secrets) apply. ROADMAP.md §2.3 parked "multi-user auth,
HTTPS on the LAN"; this request reopens the auth half. Whether HTTPS is in scope
is an owner question.

## Scope
**In:**
- Spec first: the security design recorded in ROADMAP.md (new ADR, §8 web/API,
  §9 config) and `SECURITY.md` before code.
- One owner login, single user, for every route: pages, `/api/*`, SSE. The
  password is stored as a hash and never in plaintext. The session lives in an
  `HttpOnly`, `SameSite=Strict` cookie and expires. Failed logins are
  rate-limited. A bearer token stays available for scripts.
- Host/Origin allow-list (DNS rebinding) and an Origin/CSRF check on every
  state-changing request (pause, R3 control and schedule).
- Security headers: CSP, `X-Frame-Options`/`frame-ancestors`,
  `X-Content-Type-Options`, `Referrer-Policy`.
- `SECURITY.md` limitations 3 and 5: cap the query spans and use checked bounds;
  strip control characters from advertised names.
- Config validation: fail fast when the listener is not loopback and no
  credential is set (exact rule: owner question). Warn on loose modes for
  `config.toml`/`keys.toml`.
- Tests for each rule (`make check`); `SECURITY.md` updated.

**Out:**
- Multi-user accounts, roles, OAuth/SSO, cloud, internet exposure.
- Host-level changes (firewall, reverse-proxy install, system units, certificates
  on the host) — human-only (RULES.md). The task may ship config files and
  documentation for them.
- `SECURITY.md` limitation 4 (binding BLE replies to requests) and the
  `bluetti-update` hardening items, unless the planner folds them in as small
  steps.
- Deploying to the homelab (owner only).

## Done when
- [ ] ROADMAP.md and `SECURITY.md` hold the design (auth model, session, CSRF,
      Host allow-list, headers, TLS decision, config rules), filled in as doc
      sections
- [ ] Without a valid session or bearer, every route except login and static
      assets answers 401 or redirects to login. A test walks the router's full
      route list (`make check`)
- [ ] A POST with a foreign `Origin` or `Host` is refused, and so is a request
      with an unlisted `Host` (tests)
- [ ] Login: a wrong password is rate-limited, sessions expire, logout works,
      and the password hash never appears in logs or `Debug` output (tests)
- [ ] Every page carries the security headers (test). Query spans are capped
      (test). Advertised names are sanitised (test)
- [ ] `make check` green; `HUMAN-CHECKLIST.md` lists the hardware and browser
      checks (login from a phone on the LAN, the cookie survives restarts per
      the decision)

## References
- Governing spec: `ROADMAP.md`
- RULES.md (golden rules #2, #3; human-only boundary) · `SECURITY.md` (threat
  model, known limitations, planned hardening) · ROADMAP.md §2.3, §8, §9, §10
- Related files: `src/web/` (router, `pause.rs` bearer check, `state.rs`),
  `src/config.rs` (`WebConfig`), `src/cli/scan.rs` / `detect.rs` (names),
  `deploy/*.service`
