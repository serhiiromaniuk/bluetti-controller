#!/usr/bin/env bash
# loop/step-done.sh — close one increment in ONE command.
#
# WHY THIS EXISTS. Closing a step by hand costs many model round-trips — spawn a
# verifier to re-run a deterministic gate, read its report, paste the tail into
# LOG.md, tick the PLAN box, run hygiene, run the staged scan, commit, push —
# each one a full model turn. Across a long plan that ceremony alone adds up to
# hours. A script does the mechanical part in
# one call, and the evidence gets BETTER: the gate tail in LOG.md is written by
# the script from the real exit code, not retyped by the agent that wants green.
#
# What it does, in order (stops at the first failure):
#   1. runs the gate ($STEP_DONE_GATE, default `make check`). Red → prints the
#      tail and exits 1. Nothing has been changed at that point.
#   2. appends the gate tail (last $STEP_DONE_TAIL lines, ANSI stripped) to the
#      active task's LOG.md, fenced between `<!-- gate:begin -->` and
#      `<!-- gate:end -->` so entry-size-guard.sh can tell machine evidence from
#      prose. Write your step's LOG entry BEFORE calling this: the block lands
#      at its bottom.
#   3. ticks the first unchecked `- [ ]` box in PLAN.md — or, with --steps
#      "3 4 5", exactly those step ordinals (a parallel group closed together,
#      AGENTS.md §8a). Skip with --no-tick.
#   4. runs the prose budget (`make loop-hygiene`, warn-only).
#   5. stages LOG.md + PLAN.md (plus everything with --all; otherwise stage the
#      step's own files yourself first), runs the staged-diff secret scan
#      (`make staged-scan` when the Makefile has it, else a built-in scan of
#      forbidden paths and key-shaped strings), and commits with --commit.
#      Commit hooks run: this script never passes --no-verify.
#   6. pushes, only with --push. Push only reviewed commits (AGENTS.md §8):
#      unpushed == unreviewed is the invariant the batched review relies on.
#
# Usage:
#   loop/step-done.sh --commit "<message>" [--push] [--all] [--no-tick | --steps "N M …"]
#   loop/step-done.sh --dry-run            # run the gate, print the plan, change nothing
#   make step-done MSG="<message>" [PUSH=1]
#
# Exit: 0 committed (or dry-run green) · 1 gate red, nothing changed · 2 usage
#       · 3 precondition, scan or commit failed — LOG.md and PLAN.md are restored
#         to how they were before the call, so fixing the cause and re-running
#         is safe (it will not tick a second step). A push failure also exits 3,
#         but after the commit: the commit stays, push it by hand.
#       Run the script directly when the exit code matters: `make step-done`
#       reports every failure as make's own exit 2.

set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"
HERE="$(basename "$SCRIPT_DIR")" # the loop dir, e.g. loop
SCRIPT="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]}")"

GATE="${STEP_DONE_GATE:-make check}"
TAIL="${STEP_DONE_TAIL:-15}"
WHERE="${LOOP_WHERE:-$HERE/where.sh}"

MSG=""
PUSH=0
ALL=0
TICK=1
DRY=0
STEPS=""
usage() { sed -n '/^# Usage:/,/^# Exit:/p' "$SCRIPT" | sed 's/^# \{0,1\}//' >&2; }
while [[ $# -gt 0 ]]; do
  case "$1" in
    --commit)
      [[ $# -ge 2 && -n "$2" ]] || {
        echo "--commit needs a message" >&2
        exit 2
      }
      MSG="$2"
      shift 2
      ;;
    --push) PUSH=1 && shift ;;
    --all) ALL=1 && shift ;;
    --no-tick) TICK=0 && shift ;;
    --steps)
      [[ $# -ge 2 && "$2" =~ ^[0-9]+([ ,][0-9]+)*$ ]] || {
        echo "--steps needs step numbers, e.g. --steps \"3 4 5\"" >&2
        exit 2
      }
      STEPS="${2//,/ }"
      shift 2
      ;;
    --dry-run) DRY=1 && shift ;;
    -h | --help)
      awk 'NR == 1 {next} /^#/ {sub(/^# ?/, ""); print; next} {exit}' "$SCRIPT"
      exit 0
      ;;
    *)
      echo "unknown arg: $1" >&2
      usage
      exit 2
      ;;
  esac
done
if [[ "$DRY" -eq 0 && -z "$MSG" ]]; then
  echo "refusing to commit without --commit \"<message>\" (or use --dry-run)" >&2
  exit 2
fi

die() {
  echo "!! $*" >&2
  exit 3
}

# From step 2 until the commit lands, LOG.md and PLAN.md are edited in place.
# Any failure in that window (scan, hook, an unexpected error under `set -e`)
# puts both files back and unstages them: otherwise a re-run after exit 3 would
# append a second gate block and tick the NEXT, undone step.
ARMED=0
BAK_LOG=""
BAK_PLAN=""
out=""
restore() {
  cat "$BAK_LOG" >"$LOG"
  [[ "$TICK" -eq 1 ]] && cat "$BAK_PLAN" >"$PLAN"
  git reset -q -- "$LOG" 2>/dev/null || true
  git reset -q -- "$PLAN" 2>/dev/null || true
  echo "!! $LOG and $PLAN restored and unstaged — fix the cause, then re-run" >&2
}
on_exit() {
  local rc=$?
  [[ "$rc" -ne 0 && "$ARMED" -eq 1 ]] && restore
  rm -f ${out:+"$out"} ${BAK_LOG:+"$BAK_LOG"} ${BAK_PLAN:+"$BAK_PLAN"}
  exit "$rc"
}
trap on_exit EXIT

# ── where is the active task? ────────────────────────────────────────────────
json="$("$WHERE" --json 2>/dev/null)" || die "$WHERE found no active task — nothing to close"
field() { sed -n "s/^  \"$1\": //p" <<<"$json" | sed -E 's/,$//; s/^"//; s/"$//'; }
folder="$(field folder)"
[[ -n "$folder" && -d "$folder" ]] || die "no task folder in $WHERE output"
LOG="$folder/LOG.md"
PLAN="$folder/PLAN.md"
[[ -f "$LOG" ]] || die "no $LOG"
if [[ "$TICK" -eq 1 ]]; then
  [[ -f "$PLAN" ]] || die "no $PLAN"
  grep -qE '^- \[ \] ' "$PLAN" || die "no unchecked step in $PLAN (use --no-tick for a non-step commit)"
  for n in $STEPS; do
    grep -E '^- \[[ xX]\] ' "$PLAN" | sed -n "${n}p" | grep -qE '^- \[ \] ' \
      || die "--steps: step $n is not an unchecked step of $PLAN"
  done
fi

# ── 1. the gate ──────────────────────────────────────────────────────────────
out="$(mktemp)"
echo ">> gate: $GATE"
set +e
bash -c "$GATE" >"$out" 2>&1
rc=$?
set -e
esc=$'\033' # BSD sed has no \x escapes
# shellcheck disable=SC2016 # reason: literal backticks in a sed script, not an expansion
evidence="$(sed -E "s/${esc}\[[0-9;]*[A-Za-z]//g"'; s/```/` ` `/g' "$out" | tail -n "$TAIL")"
if [[ "$rc" -ne 0 ]]; then
  printf '%s\n' "$evidence" >&2
  echo "!! gate RED (EXIT=$rc) — nothing changed. Fix the work, never the check." >&2
  exit 1
fi
echo ">> gate green (EXIT=0)"

if [[ "$DRY" -eq 1 ]]; then
  echo ">> dry-run: would append the gate tail to $LOG$([[ "$TICK" -eq 1 ]] && echo ", tick $PLAN step(s) ${STEPS:-$(field step)}"), stage, scan, commit$([[ "$PUSH" -eq 1 ]] && echo ', push')"
  exit 0
fi

# ── 2. machine-written evidence ──────────────────────────────────────────────
BAK_LOG="$(mktemp)"
BAK_PLAN="$(mktemp)"
cat "$LOG" >"$BAK_LOG"
[[ -f "$PLAN" ]] && cat "$PLAN" >"$BAK_PLAN"
ARMED=1
{
  echo
  echo "<!-- gate:begin (written by $HERE/step-done.sh — do not edit) -->"
  echo "**Gate:** \`$GATE\` EXIT=0 · $(date -u +%FT%TZ)"
  echo
  echo '```text'
  printf '%s\n' "$evidence"
  echo '```'
  echo
  echo "<!-- gate:end -->"
} >>"$LOG"

# ── 3. tick the step ─────────────────────────────────────────────────────────
if [[ "$TICK" -eq 1 ]]; then
  tmp="$(mktemp)"
  if [[ -n "$STEPS" ]]; then
    awk -v want=" $STEPS " '/^- \[[ xX]\] / {o++; if (index(want, " " o " ")) sub(/^- \[ \] /, "- [x] ")} {print}' "$PLAN" >"$tmp"
  else
    awk '!d && /^- \[ \] / {sub(/^- \[ \] /, "- [x] "); d = 1} {print}' "$PLAN" >"$tmp"
  fi
  cat "$tmp" >"$PLAN" && rm -f "$tmp"
  echo ">> ticked step(s) ${STEPS:-$(field step)} in $PLAN"
fi

# ── 4. prose budget (warn-only) ──────────────────────────────────────────────
if make -n loop-hygiene >/dev/null 2>&1; then
  make --no-print-directory loop-hygiene || true
elif [[ -x "$HERE/entry-size-guard.sh" ]]; then
  "$HERE/entry-size-guard.sh" || true
fi

# ── 5. stage, scan, commit ───────────────────────────────────────────────────
if [[ "$ALL" -eq 1 ]]; then git add -A; fi
# PLAN.md is staged with --no-tick too: an increment that ticks nothing (a
# close, a spec, an amendment fix) still owes the commit its own PLAN edits —
# the amendments it appended, a `close-hold` tag. Left unstaged, they read as an
# interrupted iteration next time, and the reconcile step may restore them away.
git add -- "$LOG" "$PLAN"

builtin_scan() { # forbidden paths and key-shaped strings in the staged diff; never prints a value
  local bad=0 f
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    if [[ "$f" =~ (^|/)\.env(\.[^/]*)?$ && ! "$f" =~ \.example$ ]] \
      || [[ "$f" =~ \.(pem|key|p12|pfx)$ ]] || [[ "$f" =~ (^|/)id_(rsa|dsa|ecdsa|ed25519) ]]; then
      echo "!! forbidden path staged: $f" >&2
      bad=1
    fi
  done < <(git diff --cached --name-only --diff-filter=ACMR)
  local re='AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9]{36}|glpat-[A-Za-z0-9_-]{20}|xox[abprs]-[A-Za-z0-9-]{10,}'
  local hits
  hits="$(git diff --cached -U0 --diff-filter=ACMR | grep -vE '^\+\+\+ ' | grep -E '^\+' | grep -cE "$re" || true)"
  if [[ "$hits" -gt 0 ]]; then
    echo "!! $hits added line(s) look like a secret (key/token pattern) — values not shown" >&2
    bad=1
  fi
  return "$bad"
}
if make -n staged-scan >/dev/null 2>&1; then
  make --no-print-directory staged-scan || die "staged-scan failed — unstage the offending file"
else
  builtin_scan || die "staged diff failed the built-in secret scan — unstage the offending file"
fi

if [[ -n "$(git status --porcelain --untracked-files=normal | grep -vE '^[MADRC] ' || true)" ]]; then
  echo ">> WARN unstaged or untracked files remain (not in this commit):" >&2
  git status --short | grep -vE '^[MADRC] ' | sed 's/^/     /' >&2
fi
git commit -q -m "$MSG" || die "git commit failed (a hook refused it, or no git identity?)"
ARMED=0
echo ">> committed $(git rev-parse --short HEAD): $(git log -1 --format=%s)"

# ── 6. push ──────────────────────────────────────────────────────────────────
if [[ "$PUSH" -eq 1 ]]; then
  git push -q || die "push failed — the commit is local; push by hand"
  echo ">> pushed"
fi
