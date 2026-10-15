#!/usr/bin/env bash
# shellcheck disable=SC2016,SC2034 # `check` evals its single-quoted conditions
# deploy/test/deploy-test.sh — runs deploy/deploy.sh against stub ssh, scp and
# sudo that execute locally, with a tarball whose `bluetti-update` only records
# its arguments. Proves the remote quoting (printf %q through ssh and sudo
# bash -c), the hash check after the copy and the exit-code pass-through.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
DEPLOY=$HERE/../deploy.sh
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT
mkdir -p "$T/bin" "$T/pkg"

# ssh [-t] <host> <command…>: run the command string locally (as ssh would on
# the host: the words joined by spaces, given to a shell).
cat > "$T/bin/ssh" <<'EOF'
#!/usr/bin/env bash
[[ $1 == -t ]] && shift
shift
bash -c "$*"
EOF
# scp [-q] <src> <host>:<dst>
cat > "$T/bin/scp" <<'EOF'
#!/usr/bin/env bash
[[ $1 == -q ]] && shift
cp "$1" "${2#*:}"
EOF
printf '#!/usr/bin/env bash\nexec "$@"\n' > "$T/bin/sudo"
chmod +x "$T/bin/"*
export PATH=$T/bin:$PATH

name=bluetti-controller-v9.9.9-x86_64-linux
mkdir -p "$T/pkg/$name"
cat > "$T/pkg/$name/bluetti-update" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" > "$T/args"
exit \${FAKE_EXIT:-0}
EOF
chmod +x "$T/pkg/$name/bluetti-update"
tar czf "$T/$name.tar.gz" -C "$T/pkg" "$name"

fails=0
check() { if eval "$2"; then printf 'ok   %s\n' "$1"; else printf 'FAIL %s\n' "$1"; fails=$((fails + 1)); fi; }

set +e
"$DEPLOY" homelab "$T/$name.tar.gz" > "$T/out" 2>&1; code=$?
set -e
check "deploy exits 0" '(( code == 0 ))'
check "apply got the unpacked dir" '[[ $(sed -n 1p "$T/args") == apply && $(sed -n 2p "$T/args") == */$name ]]'
check "no flag passed by default" '[[ $(wc -l < "$T/args") == 2 ]]'

set +e
"$DEPLOY" homelab "$T/$name.tar.gz" --no-poll-check > "$T/out" 2>&1; code=$?
set -e
check "--no-poll-check reaches apply" '(( code == 0 )) && [[ $(sed -n 3p "$T/args") == --no-poll-check ]]'

set +e
FAKE_EXIT=3 "$DEPLOY" homelab "$T/$name.tar.gz" > "$T/out" 2>&1; code=$?
set -e
check "apply exit 3 passes through" '(( code == 3 )) && grep -q "rollback unhealthy" "$T/out"'

# A tarball changed after the laptop hashed it (stub scp corrupts the copy).
rm -f "$T/args"
printf '#!/usr/bin/env bash\n[[ $1 == -q ]] && shift\ncp "$1" "${2#*:}"; echo x >> "${2#*:}"\n' > "$T/bin/scp"
set +e
"$DEPLOY" homelab "$T/$name.tar.gz" > "$T/out" 2>&1; code=$?
set -e
check "corrupted copy is refused before apply" '(( code != 0 )) && [[ ! -e $T/args ]]'

set +e
"$DEPLOY" homelab "$T/$name.tar.gz" --bogus > "$T/out" 2>&1; code=$?
set -e
check "unknown option refused" '(( code != 0 ))'

(( fails == 0 )) || { cat "$T/out"; echo "$fails check(s) failed"; exit 1; }
echo "deploy-test: all checks passed"
