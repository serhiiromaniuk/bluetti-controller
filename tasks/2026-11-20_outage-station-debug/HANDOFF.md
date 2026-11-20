# Owner handoff: grid sensors and Elite discovery

Status: **prepared, not applied**. The host is a production service; all
commands that edit the host, invoke `sudo`, or deploy are **owner-run**
(`RULES.md` and `deploy/HOMELAB.md` §6–7). The config and its backup contain
sensitive values; never paste them into chat or commit them.

## 1. Select the grid sensors

The owner reports the AC180 normally remains plugged into a mains outlet. The
router EB3A is intentionally disconnected from mains, so it must not vote on
house power. Keep the Elite as a sensor while it is normally connected to
mains. In `/etc/bluetti-controller/config.toml`, change only the `grid_sensor`
values under these existing `[[station]]` sections:

| `name` | Model | `grid_sensor` |
|---|---|---|
| `router` | EB3A | `false` |
| `mobile` | AC180 | `true` |
| `living-room` | Elite200V2 | `true` |

Before editing, create a backup on the host. It retains the original config's
owner and access mode. Choose a new backup filename if it already exists; do
not replace an existing backup.

```bash
# Owner, on the laptop: save a protected copy and edit the host file.
ssh -t debian@192.168.0.192 'sudo sh -c '\''umask 077; test ! -e /etc/bluetti-controller/config.pre-sensor-fix.toml && cp -p /etc/bluetti-controller/config.toml /etc/bluetti-controller/config.pre-sensor-fix.toml'\'''
ssh -t debian@192.168.0.192 sudoedit /etc/bluetti-controller/config.toml
```

Verify and validate without displaying credentials or the rest of the config:

```bash
ssh -t debian@192.168.0.192 'sudo -u bluetti python3 -c '\''import tomllib; c=tomllib.load(open("/etc/bluetti-controller/config.toml","rb")); print(*[(s["name"], s.get("grid_sensor")) for s in c["station"]], sep="\n")'\'''
ssh -t debian@192.168.0.192 'sudo -u bluetti /opt/bluetti-controller/current/bluetti-controller --config /etc/bluetti-controller/config.toml check'
```

Check the printed names and values against the table and confirm `check`
exits 0. The running process reads its existing config until restarted. If
validation fails, restore the backup before restarting or deploying:

```bash
ssh -t debian@192.168.0.192 'sudo cp -p /etc/bluetti-controller/config.pre-sensor-fix.toml /etc/bluetti-controller/config.toml'
```

## 2. Deploy the reviewed dashboard fixes

The documented release route is `deploy/HOMELAB.md` §6: add and review the
appropriate changelog entry, run `make release VERSION=<version>`, push the
reviewed release and tag, then run the following owner-only deploy from the
clean checkout. A reviewed commit build can also be deployed without a release
as the runbook permits. Check that the intended fixes are in `HEAD` first.

```bash
# Owner, on the laptop, in the repository.
make deploy HOST=debian@192.168.0.192
curl -fsS http://192.168.0.192:8080/healthz
```

Expect deploy exit 0, the intended `version`, and `samples_since_start > 0`.
`last_poll_age_s` only shows the newest sample from **any** station: it does
not prove that the Elite is working. Sign in to the dashboard and verify the
individual station cards' `Last seen` and health, including AC180. Its input
voltage represents its socket, and unplugging it can again resemble an outage.
The new UI reports unknown when no configured sensor is reachable and labels
house-event summaries as recorded estimates.

If deploy exits 1 or 3, follow `deploy/HOMELAB.md` §6 before retrying. For a
rollback, use its §7 procedure:

```bash
ssh -t debian@192.168.0.192 sudo /opt/bluetti-controller/current/bluetti-update rollback
```

A rollback can restore an earlier config and, after a schema migration, an
older database. Recheck both config roles and data; see the runbook's warning
about samples recorded after the backup.

## 3. The already-open false event

Changing roles or deploying does **not** retroactively repair the open house
event that began 19 Nov 20:49:48 Kyiv time. While the AC180 input remains low,
the open event can still show that old start; it may close when a configured
sensor observes mains. The AC180 recorded input-loss intervals on 20 Nov
07:25:54–09:13:54 and from 15:07:54 onward. Its readings show socket power,
not whether each loss was a grid outage or a disconnection.

Do not rewrite the event or claim corrected historical durations until the
owner confirms what happened during those intervals and a separately reviewed
reconciliation procedure has been tested against a disposable database. A
consistent SQLite backup and an explicit rollback are required first. Never
copy or modify only the main database file while its WAL-backed writer runs.

## 4. Elite Bluetooth

On 20 Nov a 20-second read-only host scan saw AC180 and EB3A, but not Elite;
the collector's Elite discovery attempts continue to fail. The owner says the
Elite is powered and Bluetooth is enabled. Check that the Elite is within
range of the host, its Bluetooth indicator is on, and the official phone app
or another client is not holding a connection. Close the app and wait for the
collector's next retry (up to 10 minutes); do not turn off the station's AC/DC
outputs. Refresh the dashboard and inspect the Elite card's `Last seen` and
health. If it still fails, preserve the failure time for further read-only
investigation; do not pair, reset Bluetooth, or send device writes as a test.
