#!/usr/bin/env bash
# loop/loop.sh — the bounded agent-loop harness.
#
# Re-feeds loop/PROMPT.md to a fresh `claude -p` agent each iteration. Progress
# lives on disk (tasks/ + loop/STATE.md) and in git, NOT in the context window.
# The loop reads where it left off, does ONE verified increment, commits, and
# emits a marker. See loop/README.md and AGENTS.md §4.
#
# Usage:
#   loop/loop.sh                                   # default cap (8), supervised
#   loop/loop.sh --max-iterations 20
#   loop/loop.sh --model opus                      # pin the driver model
#   loop/loop.sh --skip-permissions                # no permission prompts (unattended)
#   loop/loop.sh --continuous                      # DONE/BLOCKED/GATE_FAILED stop; retry transients
#   loop/loop.sh --stop-on-phase                   # ALSO stop at PHASE_COMPLETE (one milestone, then hand back)
#   loop/loop.sh --dry-run                         # show what would run, run nothing
#   loop/loop.sh --prompt loop/PROMPT.md
#
# Long unattended run (survives terminal/SSH close via tmux):
#   tmux new -s loop
#   loop/loop.sh --continuous --model opus --skip-permissions --max-iterations 1000
#   # detach: Ctrl-b then d   ·   reattach: tmux attach -t loop
#
# Exit codes (why the loop stopped):
#   0 <<LOOP:DONE>>          1 harness/agent failure (see stderr)   2 usage error
#   3 <<LOOP:BLOCKED>>       4 <<LOOP:GATE_FAILED>>                 5 <<LOOP:PHASE_COMPLETE>> under --stop-on-phase
#   6 --max-iterations reached                                       128+n interrupted by signal n
#
# Environment:
#   LIMIT_WAIT=1800    seconds to wait out a usage/rate limit before retrying
#   LIMIT_MAX=12       consecutive limit waits before giving up (exit 1)
#   FAIL_BACKOFF=15    --continuous: seconds × consecutive-failure count between retries
#   loop/env.sh        optional; sourced once at start — its exports reach every child agent
#
# CONFIG HYGIENE: run headless under a CLEAN CLAUDE_CONFIG_DIR with NO
# interactive/greeting plugins. Such plugins inject prompts/hooks that make the
# headless `claude -p` agent answer conversationally instead of executing
# PROMPT.md (a "…Ready. What do you need?" reply with no marker). The repo's own
# .claude/agents load regardless of config dir.
#
# Requires the `claude` CLI on PATH. Works from any directory inside the repo.

set -uo pipefail

SCRIPT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
REPO_ROOT="$(dirname "$(dirname "$SCRIPT")")"
cd "$REPO_ROOT" || exit 1

usage_error() {
  echo "$*" >&2
  echo "try: $SCRIPT --help" >&2
  exit 2
}

MAX_ITERS=8
PROMPT_FILE="loop/PROMPT.md"
MODEL=""
DRY_RUN=0
CONTINUOUS=0
STOP_ON_PHASE=0                    # --stop-on-phase: halt after one milestone instead of rolling on
PERM_FLAG=(--permission-mode auto) # default: supervised (classifier in the loop)
MAX_CONSEC=5                       # stop after this many consecutive hard failures
LIMIT_WAIT="${LIMIT_WAIT:-1800}"   # seconds to wait out a usage/rate limit (default 30m)
LIMIT_MAX="${LIMIT_MAX:-12}"       # consecutive limit waits before giving up (12 × 30m = 6h)
FAIL_BACKOFF="${FAIL_BACKOFF:-15}" # linear backoff unit for --continuous retries
# Signals meaning "session/usage/rate limit" — expected, NOT a failure. Checked
# ONLY on a failed run, so a successful iteration whose output happens to mention
# rate-limiting never false-trips. `429` must stand alone: a commit sha like
# a4291bc is not a rate limit. When seen: wait LIMIT_WAIT, retry the SAME
# iteration (no failure count, no iteration consumed), at most LIMIT_MAX times
# in a row — the loop is never unbounded.
LIMIT_RE='usage limit|rate limit|rate.?limited|(^|[^0-9A-Za-z])429([^0-9A-Za-z]|$)|Too Many Requests|quota|resets? at|Please try again|overloaded|capacity'

need_value() { [[ $# -ge 2 && -n "$2" && "$2" != --* ]] || usage_error "$1 needs a value"; }
while [[ $# -gt 0 ]]; do
  case "$1" in
    --max-iterations)
      need_value "$@"
      MAX_ITERS="$2"
      shift 2
      ;;
    --prompt)
      need_value "$@"
      PROMPT_FILE="$2"
      shift 2
      ;;
    --model)
      need_value "$@"
      MODEL="$2"
      shift 2
      ;;
    --skip-permissions) PERM_FLAG=(--dangerously-skip-permissions) && shift ;;
    --continuous) CONTINUOUS=1 && shift ;;
    --stop-on-phase) STOP_ON_PHASE=1 && shift ;;
    --dry-run) DRY_RUN=1 && shift ;;
    -h | --help)
      awk 'NR == 1 {next} /^#/ {sub(/^# ?/, ""); print; next} {exit}' "$SCRIPT"
      exit 0
      ;;
    *) usage_error "unknown arg: $1" ;;
  esac
done

[[ "$MAX_ITERS" =~ ^[1-9][0-9]*$ ]] || usage_error "--max-iterations needs a positive integer, got: $MAX_ITERS"
for v in LIMIT_WAIT LIMIT_MAX FAIL_BACKOFF; do
  [[ "${!v}" =~ ^[0-9]+$ ]] || usage_error "$v must be a non-negative integer, got: ${!v}"
done
[[ -f "$PROMPT_FILE" ]] || {
  echo "prompt not found: $PROMPT_FILE" >&2
  exit 1
}

# Optional: a project drops loop/env.sh to put its own toolchain on PATH (or set
# any env) for every child agent. `claude -p` spawns NON-interactive shells that
# don't source ~/.profile, so a user-space toolchain would otherwise be invisible
# and the verify step would fail spuriously. Sourced ONCE, here; its exports are
# inherited by every iteration. Keep it domain-specific and local.
if [[ -f "loop/env.sh" ]]; then
  # shellcheck disable=SC1091 # reason: project-provided, optional, not in this template
  source "loop/env.sh" || {
    echo ">> loop/env.sh failed (exit $?) — fix it before running the loop." >&2
    exit 1
  }
fi

# If CLAUDE_CONFIG_DIR is already exported, respect it so every iteration uses
# the intended config (agents, settings, permissions) regardless of which
# terminal launched the loop.
if [[ -n "${CLAUDE_CONFIG_DIR:-}" ]]; then export CLAUDE_CONFIG_DIR; fi

# Credentials in the environment reach every command the agent runs: one injected
# instruction — in a fetched page, an issue, a dependency's README — only has to
# print them, and a narrower token helps less than its absence. So the loop
# belongs where none are set (loop/README.md "Credentials"). Checked after
# loop/env.sh, whose exports count too. Names only, never values. A forwarded
# SSH_AUTH_SOCK counts: it opens every host the agent's keys open. The agent's own
# login (ANTHROPIC_*, CLAUDE_*) is left out: the CLI needs it. Warn-only.
CRED_RE='(TOKENS?|SECRETS?|PASSWORD|PASSWD|CREDENTIALS?)(_|$)|(^|_)(API_?KEY|ACCESS_KEY|PRIVATE_KEY)(_|$)|_(KEY|PAT|PWD)$|_DSN$|^(DATABASE_URL|SSH_AUTH_SOCK)$'
cred_vars="$(compgen -e | grep -E "$CRED_RE" | grep -vE '^(ANTHROPIC|CLAUDE)_' | sort | tr '\n' ' ')"
if [[ -n "$cred_vars" ]]; then
  echo ">> WARN credential-like variables are set; every command the agent runs can read them: ${cred_vars% }" >&2
  echo "   Run the loop where they are not set (loop/README.md \"Credentials\")." >&2
fi

MODEL_FLAG=()
[[ -n "$MODEL" ]] && MODEL_FLAG=(--model "$MODEL")

# Completion markers the agent emits (AGENTS.md §4).
#   STOP_ALWAYS => halt even in --continuous mode. All three need a human: DONE
#                  (goal reached), BLOCKED (stuck / boundary), and GATE_FAILED (a
#                  hard gate failed — often a real block the agent can't self-fix
#                  in scope; do NOT spin on it, hand back). Never suppress these.
#   CONTINUE    => keep looping (a step done, or a phase finished+tagged and the
#                  agent rolls into the next phase autonomously).
STOP_ALWAYS=("<<LOOP:DONE>>" "<<LOOP:BLOCKED>>" "<<LOOP:GATE_FAILED>>")
CONTINUE_MARKERS=("<<LOOP:CONTINUE>>" "<<LOOP:PHASE_COMPLETE>>")

# --stop-on-phase: by default PHASE_COMPLETE *continues* — the agent tags the
# milestone and rolls into the next phase autonomously (the point of the design).
# When you want exactly one milestone and then a human look, promote it to a hard
# terminal. The stop lands AFTER the tag: the agent tags, pushes and advances the
# pointer before emitting the marker, so nothing is left half-done.
if [[ "$STOP_ON_PHASE" -eq 1 ]]; then
  STOP_ALWAYS+=("<<LOOP:PHASE_COMPLETE>>")
  CONTINUE_MARKERS=("<<LOOP:CONTINUE>>")
fi
stop_code() {
  case "$1" in
    "<<LOOP:DONE>>") echo 0 ;;
    "<<LOOP:BLOCKED>>") echo 3 ;;
    "<<LOOP:GATE_FAILED>>") echo 4 ;;
    "<<LOOP:PHASE_COMPLETE>>") echo 5 ;;
    *) echo 1 ;;
  esac
}

# bash < 4.4 treats an empty array as unbound under `set -u`; ${a[@]+…} is the
# portable spelling (stock macOS ships bash 3.2).
echo ">> loop | repo=$REPO_ROOT | prompt=$PROMPT_FILE | max-iterations=$MAX_ITERS"
echo ">> model=${MODEL:-<config default>} | perms=${PERM_FLAG[*]} | continuous=$CONTINUOUS | stop-on-phase=$STOP_ON_PHASE"
echo ">> CLAUDE_CONFIG_DIR=${CLAUDE_CONFIG_DIR:-<default>}"
if [[ "$DRY_RUN" -eq 1 ]]; then
  echo ">> --dry-run: would run up to $MAX_ITERS times:"
  echo "   claude -p \"\$(cat $PROMPT_FILE)\" ${MODEL_FLAG[*]+"${MODEL_FLAG[*]}"} ${PERM_FLAG[*]}"
  echo "---- prompt ----"
  cat "$PROMPT_FILE"
  exit 0
fi
# Single-instance guard — two loops racing the same tree clobber each other's
# commits. Refuse to start if one is already running on this working tree. The
# lock lives in the git dir, which is per-worktree, so parallel worktrees of one
# repo do not block each other and unrelated repos never share a lock. (flock is
# best-effort; the guard is skipped where it is unavailable, e.g. stock macOS.)
LOCK_DIR="$(git rev-parse --absolute-git-dir 2>/dev/null || echo "${TMPDIR:-/tmp}")"
LOCK_FILE="$LOCK_DIR/.agent-loop.lock"
if { exec 9>"$LOCK_FILE"; } 2>/dev/null; then
  if command -v flock >/dev/null 2>&1 && ! flock -n 9; then
    echo ">> another loop is already running on this repo (lock: $LOCK_FILE). Refusing to start." >&2
    exit 1
  fi
else
  echo ">> WARN cannot open $LOCK_FILE — running without the single-instance guard." >&2
fi

command -v claude >/dev/null || {
  echo "claude CLI not on PATH" >&2
  exit 1
}

# The agent runs in the background so a signal to the harness reaches it: a
# `$(claude …)` capture is never forwarded TERM/INT, and an agent orphaned by a
# killed harness keeps acting with nobody watching — worst under
# --skip-permissions.
OUT_FILE="$(mktemp "${TMPDIR:-/tmp}/agent-loop.XXXXXX")" || exit 1
child=""
# shellcheck disable=SC2317 # reason: invoked by traps, not unreachable
on_signal() { # on_signal <name> <exit code>
  trap - TERM INT HUP
  if [[ -n "$child" ]]; then
    kill -TERM "$child" 2>/dev/null
    wait "$child" 2>/dev/null
  fi
  echo "" >&2
  echo ">> interrupted by SIG$1 — the running agent was stopped. Reconcile the tree before re-running (PROMPT §1)." >&2
  exit "$2"
}
trap 'on_signal TERM 143' TERM
trap 'on_signal INT 130' INT
trap 'on_signal HUP 129' HUP
trap 'rm -f "$OUT_FILE"' EXIT

run_agent() { # → sets $out and returns claude's exit code
  # 9>&- : the agent (and anything it leaves running, e.g. a dev server) must not
  # inherit the lock, or the next loop would refuse to start.
  claude -p "$(cat "$PROMPT_FILE")" ${MODEL_FLAG[@]+"${MODEL_FLAG[@]}"} "${PERM_FLAG[@]}" \
    >"$OUT_FILE" 2>&1 </dev/null 9>&- &
  child=$!
  wait "$child"
  local rc=$?
  child=""
  out="$(cat "$OUT_FILE")"
  return "$rc"
}

consec_fail=0
limit_waits=0
for ((i = 1; i <= MAX_ITERS; i++)); do
  echo ""
  echo "╭─────────── iteration $i/$MAX_ITERS · $(date '+%Y-%m-%d %H:%M:%S %Z') ───────────╮"

  # One fresh agent per iteration. Scope tools to what a step needs; widen only
  # if you trust the run. `auto` keeps a classifier in the loop for safety;
  # --skip-permissions removes it (unattended) — then the human-only boundary in
  # AGENTS.md + the BLOCKED marker are the only guardrail.
  out=""
  if ! run_agent; then
    echo "$out"
    # Session/usage/rate limit — checked ONLY on a failed run. Wait it out and
    # retry the SAME iteration (no failure count, no iteration consumed), but
    # only LIMIT_MAX times in a row.
    if grep -qiE "$LIMIT_RE" <<<"$out"; then
      limit_waits=$((limit_waits + 1))
      if [[ "$limit_waits" -gt "$LIMIT_MAX" ]]; then
        echo ">> usage/rate limit seen $limit_waits times in a row (LIMIT_MAX=$LIMIT_MAX) — stopping." >&2
        exit 1
      fi
      echo ">> [$(date '+%Y-%m-%d %H:%M:%S %Z')] usage/rate limit detected ($limit_waits/$LIMIT_MAX) — waiting ${LIMIT_WAIT}s, then retrying this iteration (not counted as a failure)."
      sleep "$LIMIT_WAIT"
      i=$((i - 1))
      continue
    fi
    limit_waits=0
    if [[ "$CONTINUOUS" -eq 1 ]]; then
      consec_fail=$((consec_fail + 1))
      echo ">> claude exited non-zero (consecutive failures: $consec_fail/$MAX_CONSEC)."
      if [[ $consec_fail -ge $MAX_CONSEC ]]; then
        echo ">> $MAX_CONSEC consecutive failures — stopping (likely auth/API/network)." >&2
        exit 1
      fi
      sleep $((FAIL_BACKOFF * consec_fail)) # linear backoff
      continue
    fi
    echo ">> claude exited non-zero on iteration $i — stopping for review." >&2
    exit 1
  fi
  limit_waits=0
  echo "$out"

  # Dispatch on the LAST marker line only. The protocol (PROMPT.md §7) puts THE
  # marker on the turn's last line; prose earlier in the turn may legitimately
  # *mention* markers — recapping a prior iteration's stop (the §1 dirty-tree
  # reconcile does exactly this), quoting the protocol — so a whole-output grep
  # false-stops on the agent's own narration. If the last marker-bearing line
  # somehow carries both kinds, stop wins (fail toward the human, never past one).
  marker_line="$(grep -F '<<LOOP:' <<<"$out" | tail -n1 || true)"

  # Hard terminals — always honoured (DONE / BLOCKED / GATE_FAILED).
  stop=""
  for m in "${STOP_ALWAYS[@]}"; do grep -qF "$m" <<<"$marker_line" && stop="$m"; done
  if [[ -n "$stop" ]]; then
    echo ""
    echo ">> stop marker: $stop  (iteration $i). Handing back to a human."
    exit "$(stop_code "$stop")"
  fi

  # A recognized continue marker resets the failure counter.
  matched=""
  for m in "${CONTINUE_MARKERS[@]}"; do grep -qF "$m" <<<"$marker_line" && {
    matched="$m"
    break
  }; done
  if [[ -n "$matched" ]]; then
    consec_fail=0
    echo ">> $matched — continuing."
    continue
  fi

  # No marker at all: a protocol miss.
  if [[ "$CONTINUOUS" -eq 1 ]]; then
    consec_fail=$((consec_fail + 1))
    echo ">> no completion marker (consecutive failures: $consec_fail/$MAX_CONSEC) — retrying."
    if [[ $consec_fail -ge $MAX_CONSEC ]]; then
      echo ">> $MAX_CONSEC consecutive protocol misses — stopping for review." >&2
      exit 1
    fi
    sleep $((FAIL_BACKOFF * consec_fail))
    continue
  fi
  echo ">> no completion marker on iteration $i — stopping (needs attention)." >&2
  exit 1
done

echo ""
echo ">> reached max-iterations ($MAX_ITERS). Stopping (safety cap)."
echo ">> review loop/STATE.md and the active task LOG.md, then re-run to continue."
exit 6
