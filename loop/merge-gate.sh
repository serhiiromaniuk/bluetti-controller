#!/usr/bin/env bash
# loop/merge-gate.sh — the prerequisite for parallel work.
#
# AGENTS.md §8a fans out independent work: a `[parallel: A]` group of plan steps,
# or decision records drafted side by side, each on its own branch in its own
# git worktree. That is the largest available speed-up on a serial loop, where a
# research phase done one area after another can dominate the whole run.
#
# WHY THIS SCRIPT MUST EXIST FIRST. Two branches that are each green can be RED
# together: a global size/LOC budget, a single coverage or test-count ratchet,
# committed generated code, two records claiming the same number, one index row
# written twice. Verifying per branch proves nothing about the merge. The
# single-instance lock in loop/loop.sh exists for exactly this class of
# collision; fan-out is only safe with a gate on the MERGED tree.
#
# Usage:  loop/merge-gate.sh <branch> [<branch> ...]
#         MERGE_GATE_CMD="make check"  — the gate (default: make check)
#           The gate runs in a fresh throwaway worktree, which has none of the
#           ignored dependencies (node_modules, .venv, build caches). If the gate
#           needs them, set e.g. MERGE_GATE_CMD="make install && make check", or a
#           RED here can be the environment, not the merge.
#         MERGE_GATE_BASE=<branch>     — the base (default: the branch origin/HEAD
#                                        names, else main, else master)
#
# It merges each branch into a throwaway worktree off the base, in order, and
# runs the full gate ONCE on the result. Nothing is pushed and the base is never
# touched: the caller merges for real only after this exits 0. The throwaway
# merges skip commit hooks and use a fixed identity — their commits are
# discarded, and a hook refusing one is not a merge conflict.
#
# Exit: 0 green together · 1 conflict, merge failure or gate red · 2 usage.
set -uo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT" || exit 1
GATE="${MERGE_GATE_CMD:-make check}"
[ "$#" -ge 1 ] || {
  echo "usage: loop/merge-gate.sh <branch> [<branch> ...]" >&2
  exit 2
}

fail() {
  echo "!! $*" >&2
  exit 1
}
[ -z "$(git status --porcelain)" ] || fail "working tree is dirty — the gate needs a clean base"

if [ -n "${MERGE_GATE_BASE:-}" ]; then
  BASE="$MERGE_GATE_BASE"
else
  BASE=""
  for b in "$(git symbolic-ref --short -q refs/remotes/origin/HEAD 2>/dev/null | sed 's#^origin/##')" main master; do
    if [ -n "$b" ] && git rev-parse --verify --quiet "refs/heads/$b" >/dev/null; then
      BASE="$b"
      break
    fi
  done
fi
[ -n "$BASE" ] || fail "no base branch: neither origin/HEAD, main nor master exists — set MERGE_GATE_BASE"
git rev-parse --verify --quiet "$BASE^{commit}" >/dev/null || fail "no such base: $BASE (MERGE_GATE_BASE)"

# Resolve every ref BEFORE merging anything: a typo'd branch name must not be
# reported as a merge conflict.
for b in "$@"; do
  git rev-parse --verify --quiet "$b^{commit}" >/dev/null \
    || fail "no such branch/commit: $b"
done

WT="$(mktemp -d "${TMPDIR:-/tmp}/merge-gate-XXXXXX")"
# `rm -rf` after a failed `worktree remove` would leave a stale registration in
# .git/worktrees, so prune unconditionally.
# shellcheck disable=SC2317 # reason: invoked by the EXIT trap, not unreachable
cleanup() {
  git worktree remove --force "$WT" >/dev/null 2>&1 || true
  rm -rf "$WT"
  git worktree prune >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo ">> throwaway worktree off $BASE: $WT"
git worktree add --detach "$WT" "$BASE" >/dev/null || fail "could not create the worktree"

for b in "$@"; do
  echo ">> merging $b"
  if ! out="$(git -C "$WT" -c user.name=merge-gate -c user.email=merge-gate@localhost \
    merge --no-ff --no-verify --no-edit "$b" 2>&1)"; then
    while IFS= read -r l; do printf '   %s\n' "$l"; done <<<"$out" >&2
    if [ -n "$(git -C "$WT" diff --name-only --diff-filter=U 2>/dev/null)" ]; then
      fail "$b does not merge cleanly onto $BASE + the branches before it (conflicting
   paths above). Resolve on the BRANCH, never inside this throwaway worktree —
   its whole purpose is that the result is discarded."
    fi
    fail "merging $b failed, and NOT on a conflict (git output above)."
  fi
done

echo ">> the gate, on the MERGED tree (this is the only run that proves anything)"
if (cd "$WT" && bash -c "$GATE"); then
  echo ">> MERGE GATE OK — these branches are green TOGETHER: $*"
  echo ">> now merge them into $BASE for real, in this order, and push."
  exit 0
fi
fail "$GATE is RED on the merged tree though each branch may be green alone.
   Usual causes: a global budget crossed only in the union, a ratchet both
   branches raised, generated code regenerated on both sides, or two records
   claiming the same number.
   Fix on the branches, never on the merge."
