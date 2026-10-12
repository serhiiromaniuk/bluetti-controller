#!/usr/bin/env bash
# loop/where.sh — the loop's position oracle.
#
# WHY THIS EXISTS (measured, not theoretical). A Ralph loop re-reads its control
# plane in every fresh iteration. Those files are append-only by nature, so they
# grow, and the growth is quadratic: each iteration re-reads everything every
# earlier iteration wrote. On one real project the pointer files reached 181 KB
# (a "you are here" section of 87,000 B; a ledger row of 41,717 B on ONE line)
# and iteration wall time went from 12-17 min to 47-211 min on the same model,
# same machine, same discipline. Compacting them is half the fix. The other half
# is this script: a *derived* answer cannot rot back into prose.
#
# Everything below is already machine-readable — no parsing of narrative:
#   the `in-progress` row of tasks/INDEX.md · the `- [ ]`/`- [x]` checkboxes of
#   the active PLAN.md · the `Governing spec:` line of its BRIEF.md · the newest
#   `## ` heading of its LOG.md · loop/STATE.md's gate row · git status.
#
# loop/PROMPT.md §1 consumes the JSON, reads ONLY the files listed in `.read`,
# and runs `--context` once for the step slice. That is what keeps a CLOSED
# task's LOG.md out of context structurally instead of by asking the agent
# nicely — closed folders are never named.
#
# THE READ CONTRACT IS LEAN ON PURPOSE. Once position is computed, most of a
# resume read is files read in full when a slice was enough: the rules files
# CLAUDE.md already loads (re-read in full), the whole active LOG.md although
# only its tail was wanted, and the whole governing spec although the step
# cited one section of it. So:
#   · a rules file CLAUDE.md `@`-imports (directly or through another import) is
#     reported in `.loaded` and left OUT of `.read`. Imports are parsed the way
#     Claude Code parses them: relative to the importing file, and never inside
#     a fenced block or a `code span`;
#   · LOG.md is never in `.read` — `--context` prints its newest two entries;
#   · the spec leaves `.read` when every `spec §<key>` the current step cites
#     resolves to a heading of it — `--context` prints just those sections.
#     Any unresolved citation, a stub spec or a plan still to write keeps the
#     whole spec in `.read`. A miss costs a bigger read, never a missing one.
#
# Usage:
#   loop/where.sh            # JSON (default) — what the loop consumes
#   loop/where.sh --read     # just the file list, one path per line
#   loop/where.sh --context  # the step slice: full step text, LOG tail, cited spec sections
#   loop/where.sh --human    # one-screen summary for a person (see /where)
#
# Exit 0 = position determined (read .needs_open / .needs_plan / .spec_stub to
# see what the iteration owes). Exit 2 = cannot determine; JSON still prints with
# .error set.
#
# ADAPTING IT: only the paths below are project-specific. If your layout
# differs (rules in a subdirectory, specs elsewhere), change them here and
# nowhere else.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT" || exit 1
SELF="$(basename "$SCRIPT_DIR")/$(basename "${BASH_SOURCE[0]}")" # e.g. loop/where.sh

INDEX="${LOOP_INDEX:-tasks/INDEX.md}"
STATE="${LOOP_STATE:-loop/STATE.md}"
TASKS_DIR="${LOOP_TASKS_DIR:-tasks}"
TEMPLATE_DIR="${LOOP_TEMPLATE_DIR:-tasks/_template}"
RULE_FILES="${LOOP_RULES:-RULES.md AGENTS.md}"
LOG_TAIL_MAX="${LOOP_LOG_TAIL_MAX:-80}"

MODE="json"
case "${1:-}" in
  "" | --json) MODE="json" ;;
  --read) MODE="read" ;;
  --context) MODE="context" ;;
  --human) MODE="human" ;;
  -h | --help)
    awk 'NR == 1 {next} /^#/ {sub(/^# ?/, ""); print; next} {exit}' "$REPO_ROOT/$SELF"
    exit 0
    ;;
  *)
    echo "unknown arg: $1 (try --help)" >&2
    exit 2
    ;;
esac

ERROR=""

# ── rules: already loaded by CLAUDE.md, or still to read ─────────────────────
# Claude Code loads CLAUDE.md and every file it `@`-imports, into the main
# session and into every subagent. Re-reading those is pure waste.
LOADED=()
RULES=()
[[ -f CLAUDE.md ]] && LOADED+=(CLAUDE.md)
imports_of() { # imports_of <file> → the repo-relative paths it `@`-imports, one per line
  # Resolved relative to the importing file's directory; fenced blocks and code
  # spans are skipped; home (~) and absolute imports are ignored. A token with
  # trailing punctuation (`@AGENTS.md.`) resolves to no file, so it is a miss:
  # the safe direction — a bigger read, never a missing one.
  awk -v d="$(dirname "$1")" '
    function norm(p,   n, i, a, k, s, out) {
      n = split(p, a, "/"); k = 0
      for (i = 1; i <= n; i++) {
        if (a[i] == "" || a[i] == ".") continue
        if (a[i] == "..") { if (k == 0) return ""; k--; continue }
        s[++k] = a[i]
      }
      out = ""
      for (i = 1; i <= k; i++) out = out (i > 1 ? "/" : "") s[i]
      return out
    }
    /^[ \t]*(```|~~~)/ { fence = !fence; next }
    fence { next }
    {
      line = " " $0
      gsub(/`[^`]*`/, "", line)
      while (match(line, /[ \t]@[^ \t]+/)) {
        tok = substr(line, RSTART + 2, RLENGTH - 2)
        line = substr(line, RSTART + RLENGTH)
        if (tok ~ /^[~\/]/) continue
        p = norm((d == "." ? "" : d "/") tok)
        if (p != "") print p
      }
    }' "$1" 2>/dev/null
}
pending=()
for f in $RULE_FILES; do [[ -f "$f" ]] && pending+=("$f"); done
# Transitive, bounded: CLAUDE.md -> RULES.md -> AGENTS.md is two hops.
for _ in 1 2 3; do
  IMPORTED=""
  for l in ${LOADED[@]+"${LOADED[@]}"}; do IMPORTED="$IMPORTED$(imports_of "$l")"$'\n'; done
  rest=()
  for f in ${pending[@]+"${pending[@]}"}; do
    if grep -qxF -- "$f" <<<"$IMPORTED"; then LOADED+=("$f"); else rest+=("$f"); fi
  done
  pending=(${rest[@]+"${rest[@]}"})
done
RULES=(${pending[@]+"${pending[@]}"})

# ── the ledger row ───────────────────────────────────────────────────────────
# The single row of tasks/INDEX.md whose status is in-progress. Its first
# markdown link is the task folder (`./phase-1_x/` and `phase-1_x/` alike).
row="$(grep -m1 -E '\|[^|]*in-progress[^|]*\|' "$INDEX" 2>/dev/null || true)"
folder=""
task=""
phase=""
if [[ -n "$row" ]]; then
  rel="$(printf '%s' "$row" | grep -oE '\]\([^)]+\)' | head -1 | sed -E 's#^\]\((\./)?##; s#\)$##; s#/$##')"
  [[ -n "$rel" ]] && folder="$TASKS_DIR/$rel"
  task="$(basename "${folder:-}")"
  phase="$(printf '%s' "$row" | awk -F'|' 'NF>3 {gsub(/^[ \t`]+|[ \t`]+$/,"",$3); print $3}')"
fi
if [[ -z "$row" ]]; then
  ERROR="no in-progress row in $INDEX — open the next todo task from $TEMPLATE_DIR"
fi
needs_open=false
[[ -n "$row" && (-z "$folder" || ! -d "$folder") ]] && needs_open=true

BRIEF=""
PLAN=""
LOG=""
if [[ -n "$folder" && -d "$folder" ]]; then
  [[ -f "$folder/BRIEF.md" ]] && BRIEF="$folder/BRIEF.md"
  [[ -f "$folder/PLAN.md" ]] && PLAN="$folder/PLAN.md"
  [[ -f "$folder/LOG.md" ]] && LOG="$folder/LOG.md"
fi

# ── step position: the PLAN's checkboxes ARE the state machine ───────────────
steps=0
step=0
step_title=""
step_text=""
all_done=false
if [[ -n "$PLAN" ]]; then
  steps="$(grep -cE '^- \[[ xX]\] ' "$PLAN" || true)"
fi
needs_plan=false
if [[ -z "$PLAN" || "$steps" -eq 0 ]]; then
  needs_plan=true
else
  # The ordinal of the FIRST unchecked box, not "checked + 1": the two differ
  # once any later box is ticked out of order.
  step="$(grep -E '^- \[[ xX]\] ' "$PLAN" | grep -nE '^- \[ \] ' | head -1 | cut -d: -f1)"
  if [[ -z "$step" ]]; then
    step="$steps"
    all_done=true
  else
    # The step's FULL text: its checkbox line plus every continuation line up to
    # the next checkbox or heading.
    step_text="$(awk '/^- \[ \] / && !f {f=1; print; next}
                      f && (/^- \[/ || /^#/) {exit}
                      f {print}' "$PLAN")"
    # A step title may wrap onto indented continuation lines and end at a bolded
    # `:**`. Join up to 4 lines of THIS step (never into the next checkbox, a
    # heading or a blank line), cut at `:**` or ` — check:`, strip
    # marker/number/emphasis.
    # NB: awk runs END on `exit`, so the fallback print is guarded by `p`.
    raw="$(awk '!f && /^- \[ \] / {f = 1; buf = $0; n = 1; if (buf ~ /:\*\*/) {print buf; p = 1; exit}; next}
                f && (/^- \[/ || /^#/ || /^[ \t]*$/) {exit}
                f {buf = buf " " $0; if (buf ~ /:\*\*/ || ++n >= 4) {print buf; p = 1; exit}}
                END {if (!p && f && buf != "") print buf}' "$PLAN")"
    step_title="$(printf '%s' "$raw" \
      | sed -E 's/:\*\*.*$//; s/ — check:.*$//; s/^ *- \[ \] *//; s/^[0-9]+[a-z]?\.? *//; s/^\[parallel: *[A-Za-z0-9_-]+\] *//; s/\*\*//g;
                s/[[:space:]]+/ /g; s/^ +//; s/ +$//' | cut -c1-160)"
  fi
fi

# ── parallel groups: `- [ ] N. [parallel: A] …` ──────────────────────────────
# Steps the planner tagged with the same group write disjoint files and do not
# consume each other's output, so the session may fan them out (AGENTS.md §8a).
# `.parallel_steps` lists the unchecked steps of the current step's group.
parallel_group=""
PAR_STEPS=()
if [[ -n "$step_text" ]]; then
  parallel_group="$(head -1 <<<"$step_text" | sed -nE 's/^- \[ \] [0-9]+[a-z]?\.? *\[parallel: *([A-Za-z0-9_-]+)\].*/\1/p')"
fi
if [[ -n "$parallel_group" ]]; then
  # Same tag grammar as the line above: `[parallel:A]` and `[parallel: A]` alike.
  while IFS= read -r n; do PAR_STEPS+=("$n"); done < <(grep -E '^- \[[ xX]\] ' "$PLAN" \
    | awk -v g="$parallel_group" '{o++}
        /^- \[ \] / && match($0, /^- \[ \] [0-9]+[a-z]?\.? *\[parallel: *[A-Za-z0-9_-]+\]/) {
          t = substr($0, RSTART, RLENGTH); sub(/^.*\[parallel: */, "", t); sub(/\]$/, "", t)
          if (t == g) print o
        }')
fi

# ── owner questions: asked in ONE batch, not discovered step by step ─────────
# PLAN.md `## Questions for the owner`: `- Q3 — <question> · blocks: 4, 7 ·
# answer: pending` (not yet asked) → `answer: asked <date>` → `answer: <text>`.
# `.unasked_questions` / `.open_questions` count the first / first two states;
# `.waiting_on` lists the open ones the current step names with `(needs Q3)`.
OPEN_Q=()
UNASKED_Q=()
WAIT_Q=()
questions() { # questions <state regex> → the Q ids in that state, in order
  awk '/^## / {s = ($0 ~ /^## Questions for the owner/)} s' "$PLAN" \
    | grep -iE "^- \**Q[0-9]+.*answer: *\**($1)" | grep -oE '^- \**Q[0-9]+' | grep -oE 'Q[0-9]+' || true
}
if [[ -n "$PLAN" ]]; then
  while IFS= read -r q; do [[ -n "$q" ]] && OPEN_Q+=("$q"); done < <(questions 'pending|asked')
  while IFS= read -r q; do [[ -n "$q" ]] && UNASKED_Q+=("$q"); done < <(questions 'pending')
fi
if [[ -n "$step_text" && ${#OPEN_Q[@]} -gt 0 ]]; then
  while IFS= read -r q; do
    for o in "${OPEN_Q[@]}"; do [[ -n "$q" && "$o" == "$q" ]] && WAIT_Q+=("$q"); done
  done < <(grep -oE 'needs Q[0-9]+(( *, *| and | */ *)Q[0-9]+)*' <<<"$step_text" | grep -oE 'Q[0-9]+' | sort -u)
fi

# ── the governing spec, and whether it is still a stub ───────────────────────
# BRIEF.md's `Governing spec:` line; the first backticked path on it. Optional —
# a project with no spec layer simply leaves the line out.
spec=""
spec_stub=false
if [[ -n "$BRIEF" ]]; then
  # shellcheck disable=SC2016 # reason: the backticks are literal Markdown, not expansions
  spec="$(grep -m1 -E '^[-*] *Governing spec:' "$BRIEF" \
    | grep -oE '`[A-Za-z0-9._/-]+\.md`' | head -1 | tr -d '`')"
fi
if [[ -n "$spec" && -f "$spec" ]]; then
  # Convention: an unwritten spec carries a STUB/TODO marker in its header block.
  # Scoped to the header on purpose — a finished spec may well mention the word
  # "STUB" further down, recording that it stopped being one.
  head -12 "$spec" | grep -qE '\*\*STUB|TODO\(spec\)' && spec_stub=true
elif [[ -n "$spec" ]]; then
  spec_stub=true # named but absent — writing it IS the iteration
fi

# ── the spec sections the current step cites ─────────────────────────────────
# A citation is `spec §<key>` where <key> is a heading's leading number
# (`## 3. Data` → `spec §3`) or its text before ` — ` (`## Wave 1 — edge` →
# `spec §Wave 1`), matched case-insensitively and not followed by a letter or
# digit (`§1` never matches `§10`). `spec` must start a word: a bare `§6`,
# `AGENTS.md §6` or `runtime-spec §6` names another document and is ignored —
# a foreign `§5` that happened to match heading `5.` would load the wrong
# section AND drop the whole spec. Whitespace is collapsed first, so a
# citation may wrap across lines. Headings inside
# fenced code are not headings. A section runs to the next heading of the same
# or a higher level. Prints `start end title` per section, `NONE` when the step
# cites nothing, `UNRESOLVED` when any citation matches no heading.
spec_sections_of() { # <spec> <step text>
  WHERE_STEP_TEXT="$2" awk -v q='spec §' '
    function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t]+$/, "", s); return s }
    /^[ \t]*(```|~~~)/ { fence = !fence; next }
    fence { next }
    /^#+[ \t]/ {
      n++; hl[n] = NR; t = $0; lev = 0
      while (substr(t, 1, 1) == "#") { lev++; t = substr(t, 2) }
      hlev[n] = lev; t = trim(t); htxt[n] = t; k1 = ""
      if (match(t, /^[0-9]+(\.[0-9]+)*\.?[ \t]/)) {
        k1 = trim(substr(t, 1, RLENGTH)); sub(/\.$/, "", k1); t = trim(substr(t, RLENGTH + 1))
      }
      p = index(t, " — "); if (p > 0) t = substr(t, 1, p - 1)
      p = index(t, " – "); if (p > 0) t = substr(t, 1, p - 1)
      sub(/:$/, "", t)
      key1[n] = tolower(k1); key2[n] = tolower(trim(t))
    }
    END {
      for (i = 1; i <= n; i++) {
        e = NR
        for (j = i + 1; j <= n; j++) if (hlev[j] <= hlev[i]) { e = hl[j] - 1; break }
        hend[i] = e
      }
      s = tolower(ENVIRON["WHERE_STEP_TEXT"]); gsub(/[ \t\n]+/, " ", s)
      ql = length(q); cites = 0; unres = 0
      while ((p = index(s, q)) > 0) {
        prev = (p > 1 ? substr(s, p - 1, 1) : ""); after = substr(s, p + ql); s = after
        if (prev ~ /[a-z0-9_-]/) continue
        cites++
        if (substr(after, 1, 1) == " ") after = substr(after, 2)
        best = 0; bl = 0
        for (i = 1; i <= n; i++) for (k = 1; k <= 2; k++) {
          key = (k == 1 ? key1[i] : key2[i]); L = length(key)
          if (L == 0 || L <= bl) continue
          if (substr(after, 1, L) == key && substr(after, L + 1, 1) !~ /[a-z0-9]/) { best = i; bl = L }
        }
        if (best) chosen[best] = 1; else unres++
      }
      if (cites == 0) { print "NONE"; exit }
      if (unres > 0) { print "UNRESOLVED"; exit }
      for (i = 1; i <= n; i++) if (chosen[i]) {
        inner = 0
        for (j = 1; j <= n; j++) if (j != i && chosen[j] && hl[j] < hl[i] && hend[i] <= hend[j]) inner = 1
        if (!inner) printf "%d %d %s\n", hl[i], hend[i], htxt[i]
      }
    }' "$1"
}
SECTIONS=()
if [[ -n "$step_text" && -n "$spec" && -f "$spec" && "$spec_stub" == false ]]; then
  res="$(spec_sections_of "$spec" "$step_text")"
  if [[ "$res" != NONE && "$res" != UNRESOLVED && -n "$res" ]]; then
    while IFS= read -r l; do SECTIONS+=("$l"); done <<<"$res"
  fi
fi

# ── tree state (PROMPT §1's reconcile branch) ────────────────────────────────
tree_clean=true
[[ -n "$(git status --porcelain 2>/dev/null)" ]] && tree_clean=false
# symbolic-ref first: it also works before the first commit; short sha when detached.
branch="$(git symbolic-ref --short -q HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null || echo '?')"

# ── unreviewed work: commits not yet pushed ──────────────────────────────────
# AGENTS.md §8: a commit is pushed only once reviewed, so "ahead of upstream"
# IS the batch still owed a reviewer pass. -1 = no upstream: review every step.
unreviewed="$(git rev-list --count '@{u}..HEAD' 2>/dev/null || echo -1)"

# ── last result: the newest LOG.md entry heading ─────────────────────────────
# Deliberately NOT read from the pointer files. Deriving it here is what lets a
# normal increment write prose in ONE place (its LOG) instead of three.
log_headings() { # log_headings <log> → line numbers of real `## ` entries (not in comments or fences)
  awk '/^[ \t]*(```|~~~)/ { if (!c) fence = !fence }
       /<!--/ { if (!fence) c = 1 }
       { if (!c && !fence && /^## /) print NR }
       /-->/ { c = 0 }' "$1"
}
last_result=""
if [[ -n "$LOG" ]]; then
  ln="$(log_headings "$LOG" | tail -n1)"
  if [[ -n "$ln" ]]; then
    last_result="$(sed -n "${ln}p" "$LOG" \
      | sed -E 's/^## *//; s/[[:space:]]+/ /g; s/ $//' | cut -c1-240)"
  fi
fi

# ── the non-derivable bits STATE.md still owns ───────────────────────────────
gate=""
blocked="unknown"
if [[ -f "$STATE" ]]; then
  gate="$(grep -m1 -E '\*\*Gate status\*\*' "$STATE" \
    | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/,"",$3); gsub(/`/,"",$3); print $3}' | cut -c1-200)"
  blocked="$(grep -m1 -E '\*\*Blocked\?\*\*' "$STATE" \
    | awk -F'|' '{gsub(/^[ \t]+|[ \t]+$/,"",$3); gsub(/`/,"",$3); print $3}' | cut -c1-80)"
fi

# ── what this iteration must read ────────────────────────────────────────────
READ=(${RULES[@]+"${RULES[@]}"})
[[ -f "$STATE" ]] && READ+=("$STATE")
if [[ "$needs_open" == true ]]; then
  READ+=("$TEMPLATE_DIR/BRIEF.md" "$TEMPLATE_DIR/PLAN.md")
else
  [[ -n "$BRIEF" ]] && READ+=("$BRIEF")
  [[ -n "$PLAN" ]] && READ+=("$PLAN")
fi
[[ -n "$spec" && ${#SECTIONS[@]} -eq 0 ]] && READ+=("$spec")

# ── output ───────────────────────────────────────────────────────────────────
jesc() { printf '%s' "$1" | sed -e 's/\\/\\\\/g' -e 's/"/\\"/g' | tr -d '\000-\037'; }
jarr() { # jarr <item>... → a JSON array of strings
  local i=0 x
  printf '['
  for x in "$@"; do
    [[ "$i" -gt 0 ]] && printf ', '
    printf '"%s"' "$(jesc "$x")"
    i=$((i + 1))
  done
  printf ']'
}
section_label() { # "start end title" → "title (L<start>-<end>)"
  local a b t
  read -r a b t <<<"$1"
  printf '%s (L%s-%s)' "$t" "$a" "$b"
}
LABELS=()
for s in ${SECTIONS[@]+"${SECTIONS[@]}"}; do LABELS+=("$(section_label "$s")"); done

log_tail() { # the newest two `## ` entries, skipping headings in comments and fences
  local start total
  start="$(log_headings "$LOG" | tail -n2 | head -n1)"
  [[ -n "$start" ]] || start=1
  total="$(sed -n "${start},\$p" "$LOG" | wc -l)"
  if [[ "$total" -gt "$LOG_TAIL_MAX" ]]; then
    echo "(… $((total - LOG_TAIL_MAX)) earlier lines of these entries omitted — open $LOG only if you need them)"
    sed -n "${start},\$p" "$LOG" | tail -n "$LOG_TAIL_MAX"
  else
    sed -n "${start},\$p" "$LOG"
  fi
}

case "$MODE" in
  read)
    [[ ${#READ[@]} -gt 0 ]] && printf '%s\n' "${READ[@]}"
    ;;
  context)
    if [[ -n "$step_text" ]]; then
      echo "==> step $step of $steps ($PLAN)"
      printf '%s\n' "$step_text"
    else
      echo "==> no current step (needs_open=$needs_open needs_plan=$needs_plan all_steps_done=$all_done)"
    fi
    if [[ -n "$LOG" ]]; then
      echo
      echo "==> newest LOG entries ($LOG)"
      log_tail
    fi
    for s in ${SECTIONS[@]+"${SECTIONS[@]}"}; do
      read -r a b _ <<<"$s"
      echo
      echo "==> spec section $(section_label "$s") of $spec"
      sed -n "${a},${b}p" "$spec"
    done
    ;;
  human)
    echo "phase       : ${phase:-?}"
    echo "task        : ${task:-<none>}  [${folder:-no folder}]"
    if [[ "$needs_open" == true ]]; then
      echo "step        : NO FOLDER — open the task from $TEMPLATE_DIR; that IS this iteration"
    elif [[ "$needs_plan" == true ]]; then
      echo "step        : NO PLAN — the planner subagent decomposes BRIEF+spec; that IS this iteration"
    elif [[ "$all_done" == true ]]; then
      echo "step        : all $steps steps ✓ — close the task (OUTCOME.md), check the gate, advance"
    else
      echo "step        : $step of $steps — $step_title"
    fi
    [[ -n "$parallel_group" ]] && echo "parallel    : group $parallel_group — unchecked steps ${PAR_STEPS[*]:-} may fan out (AGENTS.md §8a)"
    [[ ${#OPEN_Q[@]} -gt 0 ]] && echo "questions   : ${#OPEN_Q[@]} open (${OPEN_Q[*]}), ${#UNASKED_Q[@]} not yet asked — ask them in ONE batch$([[ ${#WAIT_Q[@]} -gt 0 ]] && echo "; this step waits on ${WAIT_Q[*]}")"
    [[ -n "$spec" ]] && echo "spec        : $spec$([[ "$spec_stub" == true ]] && echo '  [STUB — write it first]')"
    [[ ${#LABELS[@]} -gt 0 ]] && echo "cited       : ${LABELS[*]}"
    echo "gate        : ${gate:-<none recorded>}"
    echo "tree        : $([[ "$tree_clean" == true ]] && echo clean || echo 'DIRTY — reconcile before starting (PROMPT §1)') on $branch"
    echo "unreviewed  : $([[ "$unreviewed" -lt 0 ]] && echo 'no upstream — review every step' || echo "$unreviewed commit(s) ahead of upstream")"
    echo "last result : ${last_result:-<none>}"
    echo "loaded      : ${LOADED[*]:-<none>}"
    echo "read        : ${READ[*]:-<none>}"
    echo "context     : $SELF --context"
    [[ -n "$ERROR" ]] && echo "error       : $ERROR"
    ;;
  json)
    printf '{\n'
    printf '  "phase": "%s",\n' "$(jesc "$phase")"
    printf '  "task": "%s",\n' "$(jesc "$task")"
    printf '  "folder": "%s",\n' "$(jesc "$folder")"
    printf '  "needs_open": %s,\n' "$needs_open"
    printf '  "needs_plan": %s,\n' "$needs_plan"
    printf '  "step": %s,\n' "$step"
    printf '  "steps": %s,\n' "$steps"
    printf '  "all_steps_done": %s,\n' "$all_done"
    printf '  "step_title": "%s",\n' "$(jesc "$step_title")"
    printf '  "parallel_group": "%s",\n' "$(jesc "$parallel_group")"
    printf '  "parallel_steps": [%s],\n' "$(
      IFS=,
      echo "${PAR_STEPS[*]:-}"
    )"
    printf '  "open_questions": %s,\n' "${#OPEN_Q[@]}"
    printf '  "unasked_questions": %s,\n' "${#UNASKED_Q[@]}"
    printf '  "waiting_on": %s,\n' "$(jarr ${WAIT_Q[@]+"${WAIT_Q[@]}"})"
    printf '  "spec": "%s",\n' "$(jesc "$spec")"
    printf '  "spec_stub": %s,\n' "$spec_stub"
    printf '  "spec_sections": %s,\n' "$(jarr ${LABELS[@]+"${LABELS[@]}"})"
    printf '  "gate": "%s",\n' "$(jesc "$gate")"
    printf '  "blocked": "%s",\n' "$(jesc "$blocked")"
    printf '  "tree_clean": %s,\n' "$tree_clean"
    printf '  "branch": "%s",\n' "$(jesc "$branch")"
    printf '  "unreviewed": %s,\n' "$unreviewed"
    printf '  "last_result": "%s",\n' "$(jesc "$last_result")"
    printf '  "loaded": %s,\n' "$(jarr ${LOADED[@]+"${LOADED[@]}"})"
    printf '  "read": %s,\n' "$(jarr ${READ[@]+"${READ[@]}"})"
    printf '  "context_cmd": "%s",\n' "$(jesc "$SELF --context")"
    printf '  "error": "%s"\n' "$(jesc "$ERROR")"
    printf '}\n'
    ;;
esac

[[ -n "$ERROR" ]] && exit 2
exit 0
