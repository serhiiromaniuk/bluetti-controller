# Homelab host — first install, migration, updates, rollback

The runbook for running `bluetti-controller` on the homelab host as a hardened
system service. The design and its reasons are in ROADMAP.md §10.6. **Every
command marked `[host]` runs with sudo on the host, and only the owner runs it**
(RULES.md, human-only boundary). Commands marked `[laptop]` run in the repo on
the dev laptop.

Contents: [1 Host requirements](#1-host-requirements) ·
[2 One-time host setup](#2-one-time-host-setup) ·
[3 Config and keys](#3-config-and-keys) ·
[4 Move from the laptop](#4-move-from-the-laptop) ·
[5 First release](#5-first-release) · [6 Every later update](#6-every-later-update) ·
[7 Rollback](#7-rollback) · [8 Troubleshooting](#8-troubleshooting)

## 1. Host requirements

- x86_64, Debian 12 or 13 (`make dist` refuses a binary that needs a newer
  glibc than 2.36), BlueZ ≥ 5.63, a Bluetooth adapter in range of all three
  stations (a USB adapter on a short extension helps, HUMAN-CHECKLIST H6).
- Powered from the Elite 200 V2 or its own battery, never the EB3A (ROADMAP §10.1).
- SSH from the laptop with a user that may `sudo`. Below the host is called
  `homelab`: add a `Host homelab` entry to the laptop's `~/.ssh/config`.

## 2. One-time host setup

```bash
# [host]
sudo apt install bluez curl
sudo systemctl enable --now bluetooth
bluetoothctl show | grep -E 'Powered|Name'           # Powered: yes
sudo useradd --system --home-dir /var/lib/bluetti-controller --no-create-home \
  --shell /usr/sbin/nologin --groups bluetooth bluetti
sudo install -d -m 0750 -o root -g bluetti /etc/bluetti-controller
sudo install -d -m 0755 -o root -g root /opt/bluetti-controller

# The service identity can talk to BlueZ (prints the adapter's object tree):
sudo systemd-run --pipe --wait -p User=bluetti -p SupplementaryGroups=bluetooth \
  busctl --system tree org.bluez
```

If the last command says "access denied", the BlueZ D-Bus policy on this
Debian does not grant the `bluetooth` group: stop here and tell the loop
(ROADMAP §10.6 assumes it does).

## 3. Config and keys

Start from the laptop's working files; they hold the station MACs and the keys
(never commit them, golden rule #3).

```bash
# [laptop]
scp ~/.config/bluetti-controller/config.toml ~/.config/bluetti-controller/keys.toml homelab:
```

```bash
# [host] — edit the copy first: absolute host paths.
#   [storage] path      = "/var/lib/bluetti-controller/data.db"
#   [crypto]  keys_file = "/etc/bluetti-controller/keys.toml"
#   [ble]     adapter   = "hci0"   (the host's adapter: `bluetoothctl list`)
#   keep control.enabled = false (golden rule #2)
nano ~/config.toml
sudo install -m 0640 -o root -g bluetti ~/config.toml ~/keys.toml /etc/bluetti-controller/
shred -u ~/config.toml ~/keys.toml
```

## 4. Move from the laptop

**Only one machine may poll the stations** (one BLE connection per station,
golden rule #5). Stop the laptop first, then (optionally) carry the history.

```bash
# [laptop]
systemctl --user disable --now bluetti-controller
systemctl --user is-active bluetti-controller        # inactive
# Optional — keep today's history: copy the DB after the stop.
cd ~/.local/share/bluetti-controller && ls data.db*
scp data.db* homelab:
```

```bash
# [host] — only if you copied the DB
sudo install -d -m 0750 -o bluetti -g bluetti /var/lib/bluetti-controller
sudo install -m 0640 -o bluetti -g bluetti ~/data.db* /var/lib/bluetti-controller/
rm ~/data.db*
```

Without a copied DB the service starts a fresh one.

## 5. First release

The first deploy is the same command as every later one.

```bash
# [laptop] — CHANGELOG.md already has the 1.1.0 section
make release VERSION=1.1.0        # bump, make check, commit, tag v1.1.0, dist/
git push && git push --tags
make deploy HOST=homelab          # asks for your sudo password on the host
```

`make deploy` prints every step: the preflight `check` (run as `bluetti`), the
backup, the switch, the start, and `healthy: <build> stored N sample(s) since
its start` once the new build polled a station (up to 5 min). Then, once:

```bash
# [host] — start on boot
sudo systemctl enable bluetti-controller
```

Check it from anywhere on the LAN:

```bash
curl -s http://homelab:8080/healthz      # "version":"v1.1.0", samples_since_start > 0
ssh -t homelab sudo /opt/bluetti-controller/current/bluetti-update status
```

Open `http://homelab:8080/` on the phone (HUMAN-CHECKLIST H9).

## 6. Every later update

```bash
# [laptop]
#  1. Write the changes under a new `## X.Y.Z — YYYY-MM-DD` section in
#     CHANGELOG.md (say "config: …" when the host config needs an edit) and commit.
#  2. If the release needs a config change, edit /etc/bluetti-controller/config.toml
#     on the host first: the running build keeps its loaded config until restart,
#     and the preflight checks the new build against the edited file.
make release VERSION=X.Y.Z
git push && git push --tags
make deploy HOST=homelab
```

What `bluetti-update apply` does (ROADMAP §10.6): verify the checksums → the
new build's `check` (nothing changes if it fails) → free-space check → stop →
back up `data.db` and `config.toml` to `/var/backups/bluetti-controller/`
→ switch `/opt/bluetti-controller/current` → start → wait for a sample stored
by the new build. On failure it rolls back on its own.

| `make deploy` exit | Meaning | Do |
|---|---|---|
| 0 | the new build is current and polling | nothing |
| 1 | it failed and rolled back; the old build runs | read the output and `journalctl -u bluetti-controller -n 200` on the host, fix, release again |
| 3 | rolled back, but the old build is unhealthy too (left running) | look now: §8 |

Every station off on purpose (e.g. a maintenance window)? The sample check
cannot pass: `make deploy HOST=homelab NO_POLL_CHECK=1` checks the version only.

Deploying a commit that is not a release also works (`make deploy` refuses a
dirty tree); releases are the habit, so `/healthz` names a tag.

## 7. Rollback

Back to the previous release, any time:

```bash
ssh -t homelab sudo /opt/bluetti-controller/current/bluetti-update rollback
```

- If the newer build migrated the database, the backup taken before it went in
  is restored; samples stored since are **not** in the live DB (the newer DB is
  kept as `/var/backups/bluetti-controller/<time>-pre-rollback.db`). The
  command says so.
- If the older build rejects the config (a key added for the newer one), the
  config it last ran with is restored; the newer one is kept as
  `/var/backups/bluetti-controller/<time>-pre-rollback.config.toml`.
- Running `rollback` again goes forward to the release you left.

## 8. Troubleshooting

| Symptom | Do |
|---|---|
| Status | `sudo /opt/bluetti-controller/current/bluetti-update status` |
| Logs | `journalctl -u bluetti-controller -f` |
| exit 3 after a deploy | the old build runs but is unhealthy: `journalctl -u bluetti-controller -n 200`; often BLE — §10.5 runbook (`bluetoothctl`, adapter, the phone app holding a station) |
| "check exit 1" in the preflight | keys file or database unusable for the `bluetti` user: modes/owners (§3), disk |
| "check exit 2" in the preflight | the build is older than the database schema: deploy a newer one |
| "less than … free" | free disk space; old backups are in `/var/backups/bluetti-controller/` (root only) |
| "check exit 4" in the preflight | the new build rejects the config: compare with `config.example.toml` and the CHANGELOG's **config:** lines |
| "already installed" | that build is `previous` or older: `bluetti-update rollback` returns to `previous`; for an older one remove `/opt/bluetti-controller/releases/<build>` first |
| "… is a symlink: refusing" | something replaced a file in `/var/lib/bluetti-controller` with a link: treat the host as compromised, investigate before anything else |
| Host rebooted during an update | after the switch the new build starts without a health check: run `status`, then `rollback` if needed |
| Restore a backup by hand | `sudo systemctl stop bluetti-controller`, move `data.db*` aside, `sudo install -o bluetti -g bluetti -m 0640 /var/backups/bluetti-controller/<name>.db /var/lib/bluetti-controller/data.db` (no `-wal`/`-shm` from another set), start |
| Back to the laptop | stop + disable the host unit first (`sudo systemctl disable --now bluetti-controller`), then `systemctl --user enable --now bluetti-controller` on the laptop |
