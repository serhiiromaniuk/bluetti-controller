# Outcome

## Summary
The web UI and API are enclosed: one owner login (argon2id hash in
`web.password_hash`, in-memory `HttpOnly; SameSite=Strict` session, lockout
after 5 failures) guards every route except `/login`, `/assets/*` and `/healthz`;
a Host allow-list refuses DNS rebinding and an exact-origin check refuses
cross-site unsafe requests; four security headers ride on every response;
query spans are capped and advertised names are sanitised. Design in ADR-011 /
`spec §8.5`; an owner-added test suite (8 types) holds every security file at
100 % line coverage and zero surviving mutants. Not deployed: the homelab still
runs v1.4.0 (owner deploys, H18).

## Deliverables
- Commits: `4ff0cdd` ADR-011 + spec · `3db47e5` argon2id + `hash-password` ·
  `e3fc8cc` config rules + file-mode warning · `7f8f810` session store + lockout ·
  `c937ab9` login/logout · `14e21eb` guard layers · `23ae4f9` headers + CSP-clean
  pages · `1e29d8d` span caps + printable names · `2053e11` docs + H18/H19 ·
  `2216f03` step 10a added · `fad3c78` A3/A4/A5/A6/A9 · `b3fa70e`, `b0c4b40`,
  `c0292e0`, `eef9946`, `815ebc9` security suite types 1–8 + `COV_SECURITY` gate +
  `make mutants`; the close commit settles A7, A8, A11, A12.
- Files: `src/web/{guard,headers}.rs`, `src/web/auth/{mod,session,password}.rs`,
  `src/web/api/window.rs`, `src/config/{web,mode}.rs`,
  `src/cli/{hash_password,printable}.rs`, `templates/login.html`,
  `assets/theme.js`, `tests/security_e2e.rs` + `tests/security_e2e/server.rs`;
  `SECURITY.md`, `README.md`, `deploy/HOMELAB.md`, `deploy/INSTALL.md`,
  `HUMAN-CHECKLIST.md` H18–H19.
- Milestone tag: none (ad-hoc task).

## Pending / Follow-up
- [ ] H18 (owner): add `password_hash` on the homelab, deploy, log in from the
      phone, log out, re-login after a restart. H19: R3 control only after H18.
- [ ] Out of scope: BLE reply binding (`SECURITY.md` "Known limitations" 1),
      the `bluetti-update` items ("Planned hardening"), persistent sessions
      (limitation 3 "Sessions end on restart"; a later slice, Q3).
- [ ] close-review M1 · MEDIUM `SECURITY.md:144` — "Planned hardening" still
      lists the step-10a test suite + 100 % gate, now done: delete the bullet.
- [ ] close-review M2 · MEDIUM `ROADMAP.md:2040` — scope row "Single-user web
      login, Host/Origin guard" still says `in progress`: mark done (not
      deployed, H18).

## Amendments
- A1–A6, A9, A10 — fixed during the task (see their PLAN entries).
- A7 — fixed at the close: `SECURITY.md` and `deploy/HOMELAB.md` §7a say
  `password_hash` stays set behind the proxy.
- A8 — fixed at the close: ADR-011 consequences say the preflight `check`
  exits 4 and nothing changes.
- A11 — fixed at the close: `Fill` and `Sessions::with_rng` are `pub(crate)`.
- A12 — fixed at the close: the e2e server binds `127.0.0.1:0` and reads its
  address from the `serving listen=…` log line (`NO_COLOR=1`).

## Lessons / Notes
- 100 % line coverage still left 5 surviving mutants (a dead `cfg(not(unix))`
  stub, an XOR-equivalent fold, an equivalent default, a `>`/`>=` boundary):
  mutation testing finds what coverage cannot.
- `cargo mutants` copies the tree to /tmp; without `--gitignore true` the copy
  held the local `config.toml`/`keys.toml` (GR #3).
- A spec item (§8.5 h dev-mode `warn!`) no step named went unbuilt until step
  10's docs pass (A6): a plan step list should be cross-checked against every
  spec sub-item.

<!-- Calibration proposals for the owner (AGENTS.md §8b):
- plan-reviewer · missed: a spec sub-item (§8.5 h `warn!`) that no PLAN step built — it audits steps against the BRIEF, not against the spec the first step writes
-->
