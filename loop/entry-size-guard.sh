#!/usr/bin/env bash
# loop/entry-size-guard.sh — keep the control plane from becoming a journal.
#
# The compaction that `loop/where.sh` documents is a one-off; this is the ratchet
# that keeps it. Nothing stops pointer files growing back except a style rule, and
# a style rule is exactly what drifts. Budgets:
#
#   newest <task>/LOG.md entry   <=  40 lines   (detail, but bounded; the
#                                               step-done.sh gate block is not prose)
#   loop/STATE.md, whole file    <=  40 lines   (a pointer, not a report)
#   every tasks/INDEX.md row     <= 200 bytes   (a ledger row, not a history;
#                                               bytes, not characters)
#
# WARN-ONLY, and deliberately NOT wired into the project's `check`/`verify` gate:
# that gate is for correctness and must never fail on prose style. A warning
# nobody reads is worthless, so loop/PROMPT.md §5 calls this in the agent's own
# pre-commit sequence — that is what gives it a reader every iteration.
#
# Usage:
#   make loop-hygiene                       # or: loop/entry-size-guard.sh
#   loop/entry-size-guard.sh --strict       # exit 1 on any warning (CI)
#
# The active LOG.md is located via loop/where.sh, so this script owns no duplicate
# knowledge of where the control plane lives.
#
# NEVER raise a budget to silence a warning — shorten the prose. Growing the
# threshold is the same move as weakening a test to go green.

set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
REPO_ROOT="$(dirname "$(dirname "$SCRIPT")")"
cd "$REPO_ROOT" || exit 1

STRICT=0
case "${1:-}" in
  "") ;;
  --strict) STRICT=1 ;;
  -h|--help) awk 'NR == 1 {next} /^#/ {sub(/^# ?/, ""); print; next} {exit}' "$SCRIPT"; exit 0 ;;
  *) echo "unknown arg: $1 (try --help)" >&2; exit 2 ;;
esac

INDEX="${LOOP_INDEX:-tasks/INDEX.md}"
STATE="${LOOP_STATE:-loop/STATE.md}"
WHERE="${LOOP_WHERE:-$(basename "$(dirname "$SCRIPT")")/where.sh}"

MAX_LOG_ENTRY=40
MAX_STATE=40
MAX_INDEX_ROW=200

warnings=0
warn() { printf '>> WARN  %s\n' "$1"; warnings=$((warnings + 1)); }
ok()   { printf '>> ok    %s\n' "$1"; }

# ── 1. the newest LOG entry ──────────────────────────────────────────────────
LOG=""
# From the task folder, not `--read`: LOG.md is deliberately not in the read list.
if [[ -x "$WHERE" ]]; then
  folder="$("$WHERE" --json 2>/dev/null | sed -n 's/^  "folder": "\(.*\)",$/\1/p')"
  [[ -n "$folder" ]] && LOG="$folder/LOG.md"
fi
if [[ -n "$LOG" && -f "$LOG" ]]; then
  # The newest real entry: `## ` headings inside HTML comments (the template's
  # entry skeleton) or fenced blocks are not entries.
  start="$(awk '/^[ \t]*(```|~~~)/ { if (!c) fence = !fence }
                /<!--/ { if (!fence) c = 1 }
                { if (!c && !fence && /^## /) print NR }
                /-->/ { c = 0 }' "$LOG" | tail -n1)"
  if [[ -n "$start" ]]; then
    # The gate block loop/step-done.sh appends is machine-written evidence of a
    # fixed size (its tail length + 7 lines), not prose — it is not counted.
    entry="$(awk -v s="$start" 'NR >= s && /<!-- gate:begin/ {g = 1}
                                 NR >= s && !g && !c && /<!--/ && !/-->/ {c = 1}
                                 NR >= s && !g && !c {n++}
                                 c && /-->/ {c = 0}
                                 /<!-- gate:end -->/ {g = 0}
                                 END {print n + 0}' "$LOG")"
    if [[ "$entry" -gt "$MAX_LOG_ENTRY" ]]; then
      warn "$LOG newest entry is $entry lines (max $MAX_LOG_ENTRY)"
      printf '         Bullets, not narrative: what changed · the test and its observed RED ·\n'
      printf '         evidence (the verbatim gate tail) · reviewer dispositions · next.\n'
      printf '         Cite a spec section; never re-quote it.\n'
    else
      ok "$LOG newest entry $entry/$MAX_LOG_ENTRY lines"
    fi
  fi
else
  ok "no active LOG.md to check"
fi

# ── 2. the live pointer ──────────────────────────────────────────────────────
if [[ -f "$STATE" ]]; then
  n="$(wc -l < "$STATE")"
  if [[ "$n" -gt "$MAX_STATE" ]]; then
    warn "$STATE is $n lines (max $MAX_STATE) — it is a pointer, not a report"
    printf '         Step position, the governing spec and the last result are COMPUTED by\n'
    printf '         %s. Keep only what cannot be derived: the gate, the\n' "$WHERE"
    printf '         decision log, and carry-forwards aimed at a task that has no folder yet.\n'
  else
    ok "$STATE $n/$MAX_STATE lines"
  fi
fi

# ── 3. ledger rows ───────────────────────────────────────────────────────────
if [[ -f "$INDEX" ]]; then
  fat="$(LC_ALL=C awk -v m="$MAX_INDEX_ROW" 'length($0) > m {printf "%d(%dB) ", NR, length($0)}' "$INDEX")"
  if [[ -n "$fat" ]]; then
    warn "$INDEX rows over ${MAX_INDEX_ROW}B: $fat"
    printf '         One line per task — started · phase · title · folder+OUTCOME · status · tag.\n'
    printf '         Detail belongs in that task OUTCOME.md.\n'
  else
    ok "$INDEX all rows <= ${MAX_INDEX_ROW}B"
  fi
fi

# ── verdict ──────────────────────────────────────────────────────────────────
if [[ "$warnings" -gt 0 ]]; then
  printf '>> %d warning(s). Never grow a budget to silence this — shorten the prose.\n' "$warnings"
  [[ "$STRICT" -eq 1 ]] && exit 1
  exit 0
fi
printf '>> loop hygiene OK\n'
exit 0
