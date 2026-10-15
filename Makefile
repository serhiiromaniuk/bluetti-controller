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
check:                          ## fmt-check + clippy + tests + deploy test, in parallel (run before committing)
	@$(MAKE) --no-print-directory --jobs=$(CHECK_JOBS) $(OUTPUT_SYNC) check-gates
	@echo ">> make check: all gates green"

check-gates: fmt-check lint test test-deploy

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
# Added with their milestones (ROADMAP.md §12): build (M1), run (M3), dist (M5).
# The build id baked into the binary (`--version`, /healthz; ROADMAP.md §10.6).
BUILD_ID := $(shell git describe --tags --always --dirty)
.PHONY: build
build:                          ## release binary: target/release/bluetti-controller
	BLUETTI_BUILD=$(BUILD_ID) $(CARGO) build --release --locked

DIST_NAME = bluetti-controller-$(BUILD_ID)-x86_64-linux
# The newest glibc symbol version the binary may need: Debian 12 ships 2.36,
# Debian 13 2.41. A build on a newer laptop that needs more fails `dist`.
GLIBC_MAX ?= 2.36
DIST_FILES = target/release/bluetti-controller deploy/bluetti-update \
	deploy/bluetti-controller.system.service deploy/bluetti-controller.service \
	deploy/HOMELAB.md deploy/INSTALL.md CHANGELOG.md config.example.toml
.PHONY: dist
dist: build                     ## release tarball in dist/ (binary, bluetti-update, units, docs, BUILD, SHA256SUMS)
	@need=$$(objdump -T target/release/bluetti-controller | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -n 1); \
	  [[ $$(printf '%s\n%s\n' "$$need" "$(GLIBC_MAX)" | sort -V | tail -n 1) == "$(GLIBC_MAX)" ]] \
	  || { echo "!! the binary needs glibc $$need > $(GLIBC_MAX) (the host's): build on an older base" >&2; exit 1; }
	rm -rf dist/$(DIST_NAME) && mkdir -p dist/$(DIST_NAME)
	cp $(DIST_FILES) dist/$(DIST_NAME)/
	echo "$(BUILD_ID)" > dist/$(DIST_NAME)/BUILD
	cd dist/$(DIST_NAME) && sha256sum $(notdir $(DIST_FILES)) BUILD > SHA256SUMS
	tar czf dist/$(DIST_NAME).tar.gz --owner=0 --group=0 --mode='u+rwX,go+rX,go-w' --sort=name -C dist $(DIST_NAME)
	rm -rf dist/$(DIST_NAME)
	@echo ">> wrote dist/$(DIST_NAME).tar.gz"

# ── Releases and the homelab host (ROADMAP.md §10.6, deploy/HOMELAB.md) ──
.PHONY: release
release:                        ## cut release VERSION=X.Y.Z: bump, make check, commit, tag, dist (CHANGELOG section first)
	@[[ "$(VERSION)" =~ ^[0-9]+\.[0-9]+\.[0-9]+$$ ]] || { echo "!! usage: make release VERSION=X.Y.Z" >&2; exit 1; }
	@[[ $$(git rev-parse --abbrev-ref HEAD) == main ]] || { echo "!! releases are cut on main" >&2; exit 1; }
	@[[ -z $$(git status --porcelain) ]] || { echo "!! the tree is not clean" >&2; exit 1; }
	@! git rev-parse -q --verify "refs/tags/v$(VERSION)" >/dev/null || { echo "!! tag v$(VERSION) exists" >&2; exit 1; }
	@grep -q '^## $(VERSION) ' CHANGELOG.md || { echo "!! CHANGELOG.md has no '## $(VERSION) …' section" >&2; exit 1; }
	sed -i '0,/^version = ".*"/s//version = "$(VERSION)"/' Cargo.toml
	$(CARGO) update --workspace --offline
	$(MAKE) --no-print-directory check
	git commit -q -am "chore(deploy): release $(VERSION)"
	git tag -a "v$(VERSION)" -m "release $(VERSION)"
	$(MAKE) --no-print-directory dist
	@echo ">> released v$(VERSION). Push: git push && git push --tags"

.PHONY: deploy
deploy:                         ## ship the current build to HOST=<ssh name> and apply it (human-only: sudo on the host)
	@[[ -n "$(HOST)" ]] || { echo "!! usage: make deploy HOST=<ssh name> [NO_POLL_CHECK=1]" >&2; exit 1; }
	@[[ "$(BUILD_ID)" != *-dirty ]] || { echo "!! the tree is dirty: commit (or cut a release) first" >&2; exit 1; }
	@test -f dist/$(DIST_NAME).tar.gz || $(MAKE) --no-print-directory dist
	deploy/deploy.sh "$(HOST)" dist/$(DIST_NAME).tar.gz $(if $(NO_POLL_CHECK),--no-poll-check)

SHELLCHECK_IMAGE ?= koalaman/shellcheck:v0.10.0
.PHONY: lint-shell
lint-shell:                     ## shellcheck the deploy scripts (official pinned Docker image)
	docker run --rm -v "$(CURDIR):/mnt" -w /mnt $(SHELLCHECK_IMAGE) \
		deploy/bluetti-update deploy/deploy.sh deploy/test/update-test.sh

.PHONY: test-deploy
test-deploy:                    ## stubbed test of deploy/bluetti-update (every apply/rollback path)
	@bash deploy/test/update-test.sh

CONFIG ?= config.toml
.PHONY: run
run:                            ## poll the stations and serve /healthz with a local config (CONFIG=config.toml)
	$(CARGO) run --release --locked -- --config $(CONFIG) serve

.PHONY: help
help:                           ## list targets
	@grep -hE '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) \
		| awk '{ i = index($$0, "## "); t = substr($$0, 1, index($$0, ":") - 1); \
		         printf "  \033[36m%-16s\033[0m %s\n", t, substr($$0, i + 3) }'
