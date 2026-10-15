#!/usr/bin/env bash
# shellcheck disable=SC2016,SC2034 # `expect` evals its single-quoted checks
# deploy/test/update-test.sh — drives deploy/bluetti-update through every path
# of ROADMAP.md §10.6 against a temp root: stub systemctl and curl, fake
# releases whose "binary" is a shell script. No root, no network, no BLE.
#
# The fake binary's `check` honours three files in its release dir:
#   SCHEMA   the schema it migrates to (exit 2 when data.db is newer)
#   NEEDS    a config line it requires (exit 4 when absent)
#   REJECTS  a config line it refuses (exit 4 when present)
# The stub `systemctl start` "runs" the current release: it migrates data.db
# to the release's SCHEMA and publishes /healthz unless the release has
# DEAD; NOPOLL publishes samples_since_start = 0.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
UPDATE=$HERE/../bluetti-update
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT

export BLUETTI_PREFIX=$T/opt BLUETTI_CONFIG=$T/etc/config.toml BLUETTI_UNIT_DIR=$T/units BLUETTI_BACKUPS=$T/backups
me="$(id -un):$(id -gn)"
export BLUETTI_DB_OWNER=$me BLUETTI_CONFIG_OWNER=$me
export BLUETTI_RUNAS='' BLUETTI_HEALTH_TIMEOUT=1 BLUETTI_HEALTH_EVERY=0.1
export SYSTEMCTL=$T/bin/systemctl CURL=$T/bin/curl STUB=$T
mkdir -p "$T/bin" "$T/etc" "$T/state" "$T/units" "$T/src"
printf 'web = 1\n' > "$BLUETTI_CONFIG"

cat > "$T/bin/systemctl" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "$STUB/systemctl.log"
case $1 in
  stop) rm -f "$STUB/health" ;;
  start)
    rel=$(readlink -f "$STUB/opt/current")
    schema=$(cat "$rel/SCHEMA")
    db=$(sed -n 's/^schema=//p' "$STUB/state/data.db" 2>/dev/null || echo 0)
    (( schema > ${db:-0} )) && echo "schema=$schema" > "$STUB/state/data.db"
    if [[ -e $rel/DEAD ]]; then rm -f "$STUB/health"; exit 0; fi
    n=1; [[ -e $rel/NOPOLL ]] && n=0
    printf '{"status":"ok","last_poll_age_s":1,"version":"%s","samples_since_start":%s}' \
      "$(cat "$rel/BUILD")" "$n" > "$STUB/health" ;;
  is-active) [[ -e $STUB/health ]] && echo active || echo inactive ;;
esac
EOF
cat > "$T/bin/curl" <<'EOF'
#!/usr/bin/env bash
[[ -s $STUB/health ]] || exit 7
cat "$STUB/health"
EOF
chmod +x "$T/bin/systemctl" "$T/bin/curl"

# release <build> <schema> [flag...]: flags DEAD, NOPOLL, NEEDS=line, REJECTS=line.
release() {
    local name=$1 d=$T/src/$1 flag
    mkdir -p "$d"
    echo "$1" > "$d/BUILD"
    echo "$2" > "$d/SCHEMA"
    shift 2
    for flag in "$@"; do
        case $flag in
            NEEDS=*) echo "${flag#NEEDS=}" > "$d/NEEDS" ;;
            REJECTS=*) echo "${flag#REJECTS=}" > "$d/REJECTS" ;;
            *) touch "$d/$flag" ;;
        esac
    done
    cat > "$d/bluetti-controller" <<EOF
#!/usr/bin/env bash
here=\$(dirname "\$0")
echo "build \$(cat "\$here/BUILD")"
echo "database $T/state/data.db"
echo "health http://stub/healthz"
cfg=\$2
[[ -e \$here/NEEDS ]] && ! grep -qxF "\$(cat "\$here/NEEDS")" "\$cfg" && { echo "config: missing key"; exit 4; }
[[ -e \$here/REJECTS ]] && grep -qxF "\$(cat "\$here/REJECTS")" "\$cfg" && { echo "config: unknown key"; exit 4; }
db=\$(sed -n 's/^schema=//p' "$T/state/data.db" 2>/dev/null || true)
(( \${db:-0} > \$(cat "\$here/SCHEMA") )) && { echo "schema newer"; exit 2; }
echo "schema ok"
EOF
    cp "$UPDATE" "$d/bluetti-update"
    echo "[Service] # $name" > "$d/bluetti-controller.system.service"
    echo "notes" > "$d/CHANGELOG.md"
    chmod +x "$d/bluetti-controller" "$d/bluetti-update"
    (cd "$d" && sha256sum bluetti-controller bluetti-update bluetti-controller.system.service CHANGELOG.md BUILD > SHA256SUMS)
}

fails=0
ok() { printf 'ok   %s\n' "$1"; }
bad() {
    printf 'FAIL %s\n' "$1"
    fails=$((fails + 1))
    if (( fails == 1 )); then sed 's/^/     | /' "$T/out"; fi
}
expect() { if eval "$2"; then ok "$1"; else bad "$1  [$2]"; fi; }
cur() { cat "$T/opt/current/BUILD" 2>/dev/null; }
prev() { cat "$T/opt/previous/BUILD" 2>/dev/null; }
schema() { sed -n 's/^schema=//p' "$T/state/data.db"; }
run() { set +e; "$UPDATE" "$@" > "$T/out" 2>&1; code=$?; set -e; }
stops() { grep -c '^stop' "$T/systemctl.log" 2>/dev/null || true; }

# 1. First install: nothing to back up a DB from, no previous.
release b1 1 REJECTS=new=1
run apply "$T/src/b1"
expect "first install exits 0" '(( code == 0 ))'
expect "first install is current" '[[ $(cur) == b1 && ! -e $T/opt/previous ]]'
expect "unit file installed" 'grep -q b1 "$T/units/bluetti-controller.service"'
expect "DB migrated to schema 1" '[[ $(schema) == 1 ]]'

# 2. A healthy update: backup set (DB + config), previous = b1.
release b2 1 REJECTS=new=1
run apply "$T/src/b2"
expect "update exits 0" '(( code == 0 ))'
expect "b2 current, b1 previous" '[[ $(cur) == b2 && $(prev) == b1 ]]'
expect "backup set has DB and config" 'ls "$T/backups/"*-b1.db "$T/backups/"*-b1.config.toml >/dev/null'
expect "release records its backup" '[[ -s $T/opt/releases/b2/BACKUP ]]'
expect "release saved its healthy config" 'cmp -s "$T/opt/releases/b2/config.toml" "$BLUETTI_CONFIG"'

# 3. Same build again: nothing to do, no stop.
before=$(stops)
run apply "$T/src/b2"
expect "same build is a no-op" '(( code == 0 )) && [[ $(stops) == "$before" ]]'

# 4. Tampered release: abort before the stop.
release b3 1
echo tampered >> "$T/src/b3/CHANGELOG.md"
before=$(stops)
run apply "$T/src/b3"
expect "bad checksum aborts" '(( code != 0 )) && [[ $(cur) == b2 && $(stops) == "$before" ]]'

# 5. Preflight: the new build rejects the config → abort before the stop.
release b4 1 NEEDS=missing=1
before=$(stops)
run apply "$T/src/b4"
expect "failed preflight aborts" '(( code != 0 )) && [[ $(cur) == b2 && $(stops) == "$before" ]]'

# 6. Unhealthy, same schema: rolled back, DB untouched.
release b5 1 DEAD
run apply "$T/src/b5"
expect "dead build exits 1" '(( code == 1 ))'
expect "rolled back to b2" '[[ $(cur) == b2 && $(prev) == b5 ]]'
expect "no DB restore without a schema move" '! ls "$T/backups/"*pre-rollback* >/dev/null 2>&1'
expect "old build healthy again" 'grep -q "\"version\":\"b2\"" "$T/health"'

# 7. Unhealthy after migrating to schema 2: DB restored, newer one kept.
release b6 2 DEAD
run apply "$T/src/b6"
expect "schema-moving dead build exits 1" '(( code == 1 ))'
expect "DB back at schema 1" '[[ $(schema) == 1 && $(cur) == b2 ]]'
expect "migrated DB kept as pre-rollback" 'grep -q "schema=2" "$T/backups/"*-pre-rollback.db'
expect "no stale WAL beside the restored DB" '[[ ! -e $T/state/data.db-wal ]]'

# 8. New config key + a build that never polls: rollback restores the config
#    b2 accepts (b2 rejects new=1).
release b7 1 NEEDS=new=1 NOPOLL
echo "new=1" >> "$BLUETTI_CONFIG"
run apply "$T/src/b7"
expect "non-polling build rolled back" '(( code == 1 )) && [[ $(cur) == b2 ]]'
expect "config restored to the one b2 ran with" '! grep -qx "new=1" "$BLUETTI_CONFIG"'
expect "newer config kept aside" 'ls "$T/backups/"*-pre-rollback.config.toml >/dev/null'

# 9. --no-poll-check passes a build that answers but has not polled yet.
release b8 1 NOPOLL
run apply "$T/src/b8" --no-poll-check
expect "--no-poll-check passes" '(( code == 0 )) && [[ $(cur) == b8 && $(prev) == b2 ]]'

# 10. Failure after the stop (releases dir unwritable): the trap restores b8.
release b9 1
chmod 0500 "$T/opt/releases"
run apply "$T/src/b9"
chmod 0755 "$T/opt/releases"
expect "failure after stop exits non-zero" '(( code != 0 ))'
expect "trap kept b8 current, b2 previous" '[[ $(cur) == b8 && $(prev) == b2 ]]'
expect "trap restarted the old build" 'grep -q "\"version\":\"b8\"" "$T/health"'
expect "trap restored the unit file" 'grep -q b8 "$T/units/bluetti-controller.service"'

# 10b. A failing DB copy (inside the backup's $(…)) also triggers the trap.
release b9b 1
chmod 000 "$T/state/data.db"
run apply "$T/src/b9b"
chmod 0644 "$T/state/data.db"
expect "failed DB backup exits non-zero" '(( code != 0 ))'
expect "failed DB backup keeps b8 running" '[[ $(cur) == b8 ]] && grep -q "\"version\":\"b8\"" "$T/health"'
expect "failed apply left no release dir" '[[ ! -e $T/opt/releases/b9b ]]'

# 10c. Re-applying an installed release is refused before the stop.
before=$(stops)
run apply "$T/src/b2"
expect "re-applying previous is refused" '(( code != 0 )) && [[ $(stops) == "$before" && -e $T/opt/releases/b2/config.toml ]]'

# 10d. A symlinked DB is refused.
mv "$T/state/data.db" "$T/state/real.db"; ln -s real.db "$T/state/data.db"
release b9c 1
run apply "$T/src/b9c"
rm "$T/state/data.db"; mv "$T/state/real.db" "$T/state/data.db"
expect "symlinked DB is refused" '(( code != 0 )) && [[ $(cur) == b8 ]] && grep -q symlink "$T/out"'

# 11. Manual rollback, then back again onto a dead build → exit 3, left running.
run rollback
expect "manual rollback exits 0" '(( code == 0 )) && [[ $(cur) == b2 && $(prev) == b8 ]]'
touch "$T/opt/releases/b8/DEAD"
run rollback
expect "unhealthy rollback exits 3" '(( code == 3 )) && [[ $(cur) == b8 ]]'
rm "$T/opt/releases/b8/DEAD"
"$SYSTEMCTL" start x

# 12. Pruning: 5 releases, 3 backup sets kept; current and previous survive.
export BLUETTI_KEEP_BACKUPS=3
for b in c1 c2 c3 c4 c5 c6; do release "$b" 1; run apply "$T/src/$b"; done
expect "last update exits 0" '(( code == 0 )) && [[ $(cur) == c6 && $(prev) == c5 ]]'
expect "≤ 5 releases kept" '(( $(ls "$T/opt/releases" | wc -l) <= 5 ))'
expect "≤ 3 backup sets kept" '(( $(ls "$T/backups" | sed -E "s/\.(db(-wal|-shm)?|config\.toml)$//" | sort -u | wc -l) <= 3 ))'

# 13. status.
run status
expect "status names current" '(( code == 0 )) && grep -q "current:  c6" "$T/out"'

(( fails == 0 )) || { echo "$fails check(s) failed"; cat "$T/out"; exit 1; }
echo "update-test: all checks passed"
