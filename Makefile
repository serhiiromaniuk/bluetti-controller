# bluetti-controller — task runner.
# Cargo builds into the project's own target/ (never touches system-wide
# tooling). The `check` target is the single sanctioned gate; run it before every
# commit. `make help` lists everything.

# The Rust toolchain is pinned in ONE place: rust-toolchain.toml (rustup).
CARGO ?= cargo

# Dev tools live in the MAIN worktree's .tools/ (never system-wide); resolving it
# through the git common dir lets loop/merge-gate.sh worktrees share one install.
TOOLS_DIR := $(abspath $(shell git rev-parse --path-format=absolute --git-common-dir)/..)/.tools
LLVM_COV_VERSION := 0.9.1
LLVM_COV := $(TOOLS_DIR)/bin/cargo-llvm-cov
# Minimum line coverage (%) of each gated directory (RULES.md Testing).
COV_MIN ?= 80
COV_DIRS := src/protocol/ src/metrics/
COV_JSON := target/llvm-cov-summary.json
# Unit tests live in <module>/tests.rs and are excluded, so test code never
# covers itself. jq: summed line coverage of $dir's files against $min.
define COV_FILTER
[.data[0].files[] | select(.filename | contains("/" + $$dir)) | .summary.lines]
| (map(.covered) | add) as $$c | (map(.count) | add) as $$n
| if ($$n // 0) == 0 then error("no \($$dir) files in coverage") else 100 * $$c / $$n end
| (. * 100 | floor / 100) as $$pct
| if . < $$min then error("\($$dir) line coverage \($$pct)% < \($$min)%")
  else ">> \($$dir) line coverage \($$pct)% (min \($$min)%)" end
endef
export COV_FILTER

# Use bash and fail loudly.
SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

# ── Local dev ─────────────────────────────────────────────────────────────
.PHONY: install
install:                        ## fetch all dependencies (locked)
	$(CARGO) fetch --locked

.PHONY: lock
lock:                           ## refresh Cargo.lock after a dependency change
	$(CARGO) generate-lockfile
	@echo ">> wrote Cargo.lock"

.PHONY: fmt-check
fmt-check:                      ## format check (rustfmt)
	$(CARGO) fmt --all -- --check

.PHONY: format
format:                         ## auto-format
	$(CARGO) fmt --all

.PHONY: lint
lint:                           ## clippy, warnings are errors
	$(CARGO) clippy --all-targets --locked -- -D warnings

.PHONY: tools
tools:                          ## install pinned dev tools (cargo-llvm-cov) into .tools/
	$(CARGO) install --locked --root $(TOOLS_DIR) cargo-llvm-cov --version $(LLVM_COV_VERSION)

# Tests run once, under coverage; the gate fails below COV_MIN on any of COV_DIRS.
.PHONY: test
test:                           ## run tests under coverage; fail if src/protocol/ or src/metrics/ < COV_MIN % (hardware tests are #[ignore])
	@test -x $(LLVM_COV) || { echo "!! $(LLVM_COV) missing — run: make tools" >&2; exit 1; }
	PATH="$(TOOLS_DIR)/bin:$$PATH" $(CARGO) llvm-cov --all-targets --locked --ignore-filename-regex '/tests\.rs$$' --json --summary-only --output-path $(COV_JSON)
	@for dir in $(COV_DIRS); do jq -r --arg dir "$$dir" --argjson min $(COV_MIN) "$$COV_FILTER" $(COV_JSON); done

# The single sanctioned gate — the loop and every commit run THIS.
# Gates run in parallel; cargo serialises on its build-directory lock, so they
# never corrupt each other. Any failing gate fails `check`; the green line
# prints only when all passed.
CHECK_JOBS ?= 3
OUTPUT_SYNC := $(if $(filter 4.% 5.% 6.%,$(MAKE_VERSION)),--output-sync=target)

.PHONY: check check-gates
check:                          ## fmt-check + clippy + tests, in parallel (run before committing)
	@$(MAKE) --no-print-directory --jobs=$(CHECK_JOBS) $(OUTPUT_SYNC) check-gates
	@echo ">> make check: all gates green"

check-gates: fmt-check lint test

.PHONY: clean
clean:                          ## remove build output
	$(CARGO) clean

# ── Loop hygiene (control-plane prose budget) ─────────────────────────────
# NOT dependencies of `check` on purpose: `check` is the correctness gate and must
# never fail on prose style. loop/PROMPT.md §5 calls loop-hygiene in the agent's
# own pre-commit sequence, which is what gives the warning a reader.
.PHONY: where
where:                          ## print the loop's position (loop/where.sh --human)
	@./loop/where.sh --human

.PHONY: loop-hygiene
loop-hygiene:                   ## warn if the newest LOG entry / STATE.md / INDEX rows exceed their budgets
	@./loop/entry-size-guard.sh

# make reports any failure of the script as its own exit 2; the loop calls
# loop/step-done.sh directly when it needs the 1 (gate red) / 3 (refused) split.
.PHONY: step-done
step-done:                      ## close a step: gate → LOG evidence → tick → hygiene → scan → commit (MSG=…, PUSH=1)
	@test -n "$(MSG)" || { echo 'usage: make step-done MSG="<type>(<scope>): <subject>" [PUSH=1]' >&2; exit 2; }
	@./loop/step-done.sh --commit "$(MSG)" $(if $(PUSH),--push)

.PHONY: amendments
amendments:                     ## warn which deferred reviewer findings (the PLAN Amendments section) are still open
	@./loop/amendments-guard.sh

# ── Project-specific targets ──────────────────────────────────────────────
# Added with their milestones (ROADMAP.md §12): build (M1), run (M3).
.PHONY: build
build:                          ## release binary: target/release/bluetti-controller
	$(CARGO) build --release --locked

.PHONY: help
help:                           ## list targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| awk '{ i = index($$0, "## "); t = substr($$0, 1, index($$0, ":") - 1); \
		         printf "  \033[36m%-16s\033[0m %s\n", t, substr($$0, i + 3) }'
