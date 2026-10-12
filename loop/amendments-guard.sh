#!/usr/bin/env bash
# loop/amendments-guard.sh — make deferred findings arrive as a NUMBER.
#
# PROMPT §4b defers every MEDIUM reviewer finding (the reviewer reports no LOW)
# to the active PLAN's `## Amendments`. Deferral only works if something counts
# what is still OPEN; without a count, "deferred" and "dropped" are the same
# state and nothing tells them apart at the close.
#
# HOW IT DECIDES. A disposition is written IN the amendment, by the increment
# that dealt with it — local and exact. (A first version of this guard matched
# amendment ids against commit subjects instead; its one field result, "16 of 16
# discharged", was false at both ends: the ids were cross-references to other
# tasks' amendments, the "evidence" bare numbers inside unrelated strings.)
# Substring verbs are not enough either: "never closed" is not closed. So the
# format is explicit, one entry per deferred finding:
#
#   - A3 · 2026-08-23 · MEDIUM `src/x.py:42` — <the finding> · disposition: open
#
# and the increment that settles it rewrites the field:
#
#   … · disposition: fixed in a1b2c3d
#   … · disposition: re-targeted → CF-2          (or → <task folder>)
#   … · disposition: declined — <the reason>
#
# DISPOSED = the LAST `disposition:` word of the entry is one of: fixed, done,
# discharged, re-targeted, retargeted, declined, withdrawn, superseded, folded,
# obsolete, moot. `open`, `deferred`, any other word, or no field at all is
# OPEN. Only `- A<n>` list items are findings; other items in the section (dated
# plan notes, a step split into sub-steps) are not counted. HTML comments and
# fenced blocks are ignored. Continuation lines (up to a blank line or the next
# item) belong to their entry.
#
# WARN-ONLY, like entry-size-guard: an amendment may legitimately stay open for a
# whole task. The point is that the number is visible at every close (PROMPT §1).
#
# Usage:  make amendments                    # the active task's PLAN.md
#         loop/amendments-guard.sh --all     # every tasks/*/PLAN.md, closed included
#         loop/amendments-guard.sh --strict  # exit 1 when any finding is open (CI)
set -uo pipefail
SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
REPO_ROOT="$(dirname "$(dirname "$SCRIPT")")"
cd "$REPO_ROOT" || exit 1
WHERE="${LOOP_WHERE:-$(basename "$(dirname "$SCRIPT")")/where.sh}"
TASKS_DIR="${LOOP_TASKS_DIR:-tasks}"

STRICT=0
ALL=0
for a in "$@"; do
  case "$a" in
    --strict) STRICT=1 ;;
    --all) ALL=1 ;;
    -h | --help)
      awk 'NR == 1 {next} /^#/ {sub(/^# ?/, ""); print; next} {exit}' "$SCRIPT"
      exit 0
      ;;
    *)
      echo "unknown arg: $a (try --help)" >&2
      exit 2
      ;;
  esac
done

DISPOSED='fixed|done|discharged|re-targeted|retargeted|declined|withdrawn|superseded|folded|obsolete|moot'

plans=()
if [ "$ALL" -eq 1 ]; then
  for f in "$TASKS_DIR"/*/PLAN.md; do
    case "$f" in */_template/*) continue ;; esac
    [ -f "$f" ] && plans+=("$f")
  done
else
  p="$("$WHERE" --read 2>/dev/null | grep -E '/PLAN\.md$' | head -1)"
  [ -n "$p" ] && [ -f "$p" ] && plans+=("$p")
fi
[ "${#plans[@]}" -gt 0 ] || {
  echo ">> no PLAN.md to check"
  exit 0
}

warnings=0
for plan in "${plans[@]}"; do
  # → "<findings> <open> <other items> <open id list>"
  read -r total open_n other open_list <<<"$(awk -v disp="^($DISPOSED)\$" '
    function flush(   s, w, last) {
      if (buf == "") return
      total++; s = tolower(buf); last = ""
      while (match(s, /disposition:[ \t*_`]*[a-z-]+/)) {
        w = substr(s, RSTART, RLENGTH); sub(/^disposition:[ \t*_`]*/, "", w)
        last = w; s = substr(s, RSTART + RLENGTH)
      }
      if (last !~ disp) open[++o] = id
      buf = ""
    }
    /^## / { if (f) { flush(); f = 0 } }
    /^## Amendments/ { f = 1; next }
    !f { next }
    /^[ \t]*(```|~~~)/ { if (!c) fence = !fence; next }
    fence { next }
    /<!--/ { c = 1 }
    c { if (/-->/) c = 0; next }
    /^[ \t]*$/ { flush(); next }
    /^- (\*\*)?A[0-9]+(\*\*)?([^0-9A-Za-z]|$)/ {
      flush(); buf = $0
      match($0, /A[0-9]+/); id = substr($0, RSTART, RLENGTH)
      next
    }
    /^([-*] |[0-9]+\. )/ { flush(); others++; next }
    { if (buf != "") buf = buf " " $0 }
    END {
      if (f) flush()
      list = ""
      for (i = 1; i <= o && i <= 14; i++) list = list open[i] " "
      if (o > 14) list = list "(+" (o - 14) " more)"
      if (list == "") list = "-"
      print total + 0, o + 0, others + 0, list
    }' "$plan")"

  if [ "${open_n:-0}" -gt 0 ]; then
    printf '>> WARN  %-42s %s finding(s), %s OPEN: %s\n' "$plan" "$total" "$open_n" "$open_list"
    warnings=$((warnings + 1))
  elif [ "${total:-0}" -eq 0 ] && [ "${other:-0}" -gt 0 ]; then
    # shellcheck disable=SC2016 # reason: the backticks are literal Markdown
    printf '>> ok    %-42s 0 findings in `- A<n>` form (%s other item(s): plan notes — or findings in another form; look)\n' "$plan" "$other"
  else
    printf '>> ok    %-42s %s finding(s), all disposed\n' "$plan" "${total:-0}"
  fi
done

if [ "$warnings" -gt 0 ]; then
  printf '>> %d plan(s) with open amendments. Each is a deferred reviewer finding: at the\n' "$warnings"
  printf '   task close every one owes a disposition — in its PLAN entry and in OUTCOME.md\n'
  # shellcheck disable=SC2016 # reason: the backticks are literal Markdown
  printf '   `## Amendments`: fixed, re-targeted to a carry-forward, or declined WITH a\n'
  printf '   reason. Never dropped (AGENTS.md §2).\n'
  [ "$STRICT" -eq 1 ] && exit 1
fi
exit 0
