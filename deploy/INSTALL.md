# Install — user-level unit on the dev laptop

This installs `bluetti-controller` as a **user** systemd unit (ROADMAP.md §10.3):
no `sudo`, no system unit. The hardened install on the homelab host is a
separate, human-only procedure (ROADMAP.md §10.2).

Prerequisites: BlueZ ≥ 5.63 running, a BLE adapter (`bluetoothctl show` →
`Powered: yes`), and the user in a session that can reach the system D-Bus.

## 1. Build or unpack

From the repo: `make dist` (tarball in `dist/`), then unpack it. §2–§3 run
inside the unpacked directory: the bare names below (`bluetti-controller`,
`config.example.toml`, `bluetti-controller.service`) exist only at its top.

```bash
tar xzf bluetti-controller-<version>-x86_64-linux.tar.gz
cd bluetti-controller-<version>-x86_64-linux
```

## 2. Binary, config, data directory

```bash
install -D -m 0755 bluetti-controller ~/.local/bin/bluetti-controller
install -d -m 0700 ~/.config/bluetti-controller ~/.local/share/bluetti-controller
install -m 0600 config.example.toml ~/.config/bluetti-controller/config.toml
```

Edit `~/.config/bluetti-controller/config.toml`:

- **Use absolute paths.** The loader does not expand `~` or `$HOME`:
  - `storage.path = "/home/<user>/.local/share/bluetti-controller/data.db"`
  - `crypto.keys_file = "/home/<user>/.config/bluetti-controller/keys.toml"`
    (copy your local `keys.toml` there with mode 600; never commit it).
- `web.listen = "0.0.0.0:8080"` serves the LAN only (no port forwarding).
  Leave `web.token` unset for an open pause POST, or set a non-empty token.
- Add one `[[station]]` per station with its MAC address.
- Keep `control.enabled = false` (golden rule #2).

Check the config before installing the unit:

```bash
~/.local/bin/bluetti-controller scan --config ~/.config/bluetti-controller/config.toml
```

## 3. The unit

```bash
install -D -m 0644 bluetti-controller.service \
  ~/.config/systemd/user/bluetti-controller.service
systemctl --user daemon-reload
systemctl --user enable --now bluetti-controller
systemctl --user status bluetti-controller
```

`ExecStart` runs under `systemd-inhibit --what=sleep:idle:handle-lid-switch`,
so the laptop does not suspend while the service runs, lid closed included.
Check the lock with `systemd-inhibit --list`.

## 4. Keep it running after logout

A user unit stops when the last session ends unless lingering is on:

```bash
loginctl enable-linger
loginctl show-user "$USER" -p Linger   # Linger=yes
```

If polkit refuses, stay logged in; do not work around it with `sudo`.

## 5. Logs and status

```bash
journalctl --user -u bluetti-controller -f
curl -s http://<laptop-ip>:8080/healthz
```

The status page is `http://<laptop-ip>:8080/`.

## 6. Upgrade

Stop the unit, replace the binary, start it again. The database migrates on
start.

```bash
systemctl --user stop bluetti-controller
install -m 0755 bluetti-controller ~/.local/bin/bluetti-controller
systemctl --user start bluetti-controller
```

## 7. Uninstall

```bash
systemctl --user disable --now bluetti-controller
rm ~/.config/systemd/user/bluetti-controller.service ~/.local/bin/bluetti-controller
systemctl --user daemon-reload
loginctl disable-linger   # only if nothing else needs it
```

The config and the database stay in `~/.config/bluetti-controller/` and
`~/.local/share/bluetti-controller/`; remove them by hand if you want them gone.

## Homelab host

The system-wide install (service user, `/etc/bluetti-controller/`, hardened
system unit) is ROADMAP.md §10.2. It needs `sudo` and is done by a human.
