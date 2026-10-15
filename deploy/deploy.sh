#!/usr/bin/env bash
# deploy/deploy.sh — ship one release tarball to the homelab host and apply it
# (ROADMAP.md §10.6). Runs on the dev laptop, by the owner (human-only: the
# host side needs sudo). `make deploy HOST=…` calls it.
#
#   deploy/deploy.sh <ssh-host> <tarball> [--no-poll-check]
#
# The tarball's sha256 is computed here. On the host the tarball is copied
# into a root-owned 0755 `mktemp -d` under sudo, checked against that hash
# there (after the copy, so a swap in the user-writable upload dir cannot slip
# through), unpacked, and its own `bluetti-update apply` runs: the newest
# update logic always runs. Exit = the apply's exit (0 ok, 1 rolled back, 3).
set -euo pipefail

die() { printf '!! %s\n' "$*" >&2; exit 1; }

host=${1:-}
tarball=${2:-}
flag=${3:-}
[[ -n $host && -f $tarball ]] || die "usage: deploy/deploy.sh <ssh-host> <tarball> [--no-poll-check]"
[[ -z $flag || $flag == --no-poll-check ]] || die "unknown option $flag"
name=$(basename "$tarball")
[[ $name =~ ^bluetti-controller-[A-Za-z0-9._+-]+-x86_64-linux\.tar\.gz$ ]] || die "unexpected tarball name $name"
sum=$(sha256sum "$tarball" | cut -d ' ' -f 1)

upload=$(ssh "$host" mktemp -d)
trap 'ssh "$host" rm -rf -- "$upload" || true' EXIT
printf '>> copying %s to %s:%s\n' "$name" "$host" "$upload"
scp -q "$tarball" "$host:$upload/$name"

# Runs as root on the host; $1 upload path, $2 sha256, $3 flag (may be empty).
# shellcheck disable=SC2016 # expanded by the remote bash, not here
remote='
set -euo pipefail
d=$(mktemp -d)
trap "rm -rf -- \"$d\"" EXIT
chmod 0755 "$d"
cp -- "$1" "$d/release.tar.gz"
echo "$2  $d/release.tar.gz" | sha256sum --quiet --strict -c -
tar -xzf "$d/release.tar.gz" -C "$d" --no-same-owner
dir=$(find "$d" -mindepth 1 -maxdepth 1 -type d)
[[ -n $dir && $(printf "%s\n" "$dir" | wc -l) == 1 ]] || { echo "!! tarball must hold one directory" >&2; exit 1; }
"$dir/bluetti-update" apply "$dir" ${3:+"$3"}
'
printf '>> applying on %s (sudo will ask for your password)\n' "$host"
set +e
ssh -t "$host" "sudo bash -c $(printf '%q' "$remote") _ $(printf '%q' "$upload/$name") $sum $(printf '%q' "$flag")"
code=$?
set -e
case $code in
    0) printf '>> deployed %s to %s\n' "$name" "$host" ;;
    1) printf '!! deploy to %s failed: the output above says whether it stopped before any change or rolled back\n' "$host" >&2 ;;
    3) printf '!! update AND rollback unhealthy on %s: check it now (journalctl -u bluetti-controller)\n' "$host" >&2 ;;
    *) printf '!! deploy to %s ended with exit %s\n' "$host" "$code" >&2 ;;
esac
exit "$code"
