## Sensei monorepo — root build coordinator
##
## Components:
##   app/         — Tauri + SvelteKit desktop app
##   crates/      — All Rust crates (single workspace)
##     senseid    — HTTP daemon (API server)
##     cli        — sensei CLI (binary: sensei)
##     mcp        — MCP server
##     bootstrap  — installer/prereq checker
##     gateway    — LLM routing library
##   website/     — Marketing website
##   docs/        — Documentation
##
## Versioning:
##   VERSION file is the single source of truth.
##   `make bump v=patch|minor|major|0.3.0` updates VERSION + all manifests, commits, tags, and pushes.
##   The tag push triggers GitHub Actions which build release artifacts and
##   update the Homebrew tap SHA256s automatically.
##
## Distribution:
##   Homebrew tap: sensei-hq/homebrew-tap (tracked as git subtree at homebrew/)
##   macOS install: brew tap sensei-hq/tap && brew install sensei
##
## Subtrees (editable in-repo, synced to their own GitHub repos):
##   homebrew/    → sensei-hq/homebrew-tap   (make tap-push)
##   marketplace/ → sensei-hq/marketplace    (make marketplace-push)

.PHONY: crates crates-debug crates-all \
        install install-service install-app install-debug \
        app-dev app-check \
        website-dev website-build \
        test test-fast test-crates test-crates-fast \
        test-app test-app-unit test-app-e2e test-app-e2e-cold app-e2e-build \
        _e2e-cold-pre _e2e-cold-post reset-e2e-db \
        setup-hooks update bump tap-push marketplace-push clean

VERSION := $(shell cat VERSION)

# Health-bypass is decided at runtime via `window.__TAURI__` — Tauri
# injects it before any user script runs (`withGlobalTauri: true`).
# `vite dev`/`vite preview` outside Tauri never sees it, so they auto-
# bypass. No env vars, no build-time flags.

# ── Rust crates ───────────────────────────────────────────────────────────────

crates:
	cargo build --release -p senseid -p sensei-cli -p sensei-mcp

crates-debug:
	cargo build -p senseid -p sensei-cli -p sensei-mcp

# Full-coverage build — exercises every Rust crate, including the Tauri
# sidecar (which lives in its own `[workspace]` and so is skipped by the
# root `cargo build --workspace`). Use this as the compile-time gate
# before bumping or releasing: catches symbol drift in any crate that
# wasn't part of the daemon/CLI/MCP build set.
crates-all:
	cargo build --release --workspace
	cargo build --release --manifest-path app/src-tauri/Cargo.toml

# ── Install ───────────────────────────────────────────────────────────────────
#
# `make install` does the full install: the service binaries (CLI, daemon,
# MCP) overlaid into the brew prefix, plus the desktop .app bundle copied
# into /Applications/. The two halves are exposed as `install-service` and
# `install-app` for when you only need one. `install-debug` is the fast
# iteration variant of `install-service` — same brew-prefix overlay path,
# debug binaries.

install: install-service install-app

# Overlay freshly-built release binaries into the brew prefix.
#
# `bin.install` in the brew Formula sets the destination mode to 0555
# (read+exec, no write), so cp-overwrite fails with EACCES; `rm -f` unlinks
# the read-only file (needs write on parent dir, not on the file itself).
# Re-sign with hardened runtime so the Tauri sidecar can spawn them (macOS
# Sequoia Code Signing Monitor level 2 requires this).
install-service: crates
	@# Cold install: ensure the sensei formula is present. Try the release
	@# tarball first; fall back to --HEAD (build from main) when no release
	@# is tagged yet (typically right after `make bump` before CI publishes).
	@if ! brew list --formula sensei >/dev/null 2>&1; then \
	  echo "Cold install: brew install sensei-hq/tap/sensei (one-time)..."; \
	  brew tap sensei-hq/tap https://github.com/sensei-hq/homebrew-tap >/dev/null 2>&1 || true; \
	  brew install sensei-hq/tap/sensei || brew install --HEAD sensei-hq/tap/sensei; \
	fi
	@# Stop via brew services FIRST, then pkill any stragglers. `pkill`
	@# alone is not enough: launchd's keep_alive in the brew service plist
	@# respawns the daemon within a few ms, so the cp lands while a stale
	@# process is still up. After the install, restart so the new binary
	@# is the one serving requests.
	@echo "Stopping sensei service (so the new binary actually takes effect)..."
	-@brew services stop sensei >/dev/null 2>&1
	-@pkill -x senseid 2>/dev/null
	@sleep 1
	@DEST=$$(brew --prefix sensei)/bin && \
	rm -f "$$DEST/senseid" "$$DEST/sensei" "$$DEST/sensei-mcp" && \
	cp target/release/senseid    "$$DEST/senseid" && \
	cp target/release/sensei     "$$DEST/sensei" && \
	cp target/release/sensei-mcp "$$DEST/sensei-mcp" && \
	codesign --sign - --options runtime --force "$$DEST/senseid" && \
	codesign --sign - --options runtime --force "$$DEST/sensei" && \
	codesign --sign - --options runtime --force "$$DEST/sensei-mcp" && \
	echo "Overlaid fresh release binaries into $$DEST (codesigned)"
	@echo "Restarting sensei service so the new daemon is live..."
	-@brew services start sensei

# Build the desktop .app bundle and install it to /Applications/.
# Stop any running instance first — `cp -R` over a running .app would mix
# old code with new resources, and the next launch would crash with a
# code-signature mismatch.
install-app:
	cd app && bunx tauri build
	@if [ -d app/src-tauri/target/release/bundle/macos/Sensei.app ]; then \
	  if pgrep -x sensei-desktop > /dev/null; then \
	    echo "Stopping running Sensei.app (pid $$(pgrep -x sensei-desktop))..."; \
	    osascript -e 'tell application "Sensei" to quit' 2>/dev/null || pkill -x sensei-desktop || true; \
	    sleep 1; \
	  fi; \
	  rm -rf /Applications/Sensei.app; \
	  cp -R app/src-tauri/target/release/bundle/macos/Sensei.app /Applications/; \
	  echo "Installed Sensei.app to /Applications/"; \
	else \
	  echo "Warning: app/src-tauri/target/release/bundle/macos/Sensei.app not found — skipping /Applications copy"; \
	fi

# Fast iteration variant — debug binaries into the brew prefix (no app).
install-debug: crates-debug
	@if pgrep -x senseid > /dev/null; then \
	  echo "Stopping senseid (pid $$(pgrep -x senseid))..."; \
	  pkill -x senseid; \
	  sleep 1; \
	fi
	@DEST=$$(brew --prefix sensei)/bin && \
	rm -f "$$DEST/senseid" "$$DEST/sensei" "$$DEST/sensei-mcp" && \
	cp target/debug/senseid    "$$DEST/senseid" && \
	cp target/debug/sensei     "$$DEST/sensei" && \
	cp target/debug/sensei-mcp "$$DEST/sensei-mcp" && \
	codesign --sign - --options runtime --force "$$DEST/senseid" && \
	codesign --sign - --options runtime --force "$$DEST/sensei" && \
	codesign --sign - --options runtime --force "$$DEST/sensei-mcp" && \
	echo "Overlaid debug binaries into $$DEST (codesigned)"

# ── Desktop app dev / e2e ─────────────────────────────────────────────────────

# Tauri dev with Vite HMR — pre-builds Rust backend then starts tauri dev
app-dev:
	cd app && cargo build --manifest-path src-tauri/Cargo.toml && bunx tauri dev

# Build the debug .app bundle with the e2e-testing feature enabled
# (exposes the playwright IPC socket at /tmp/tauri-playwright.sock).
# Used by the Playwright globalSetup — kept here so the build recipe is
# discoverable and not buried in TypeScript.
app-e2e-build: install-debug
	cd app && bunx tauri build --debug --features e2e-testing

# Type-check SvelteKit sources
app-check:
	cd app && bun run check

# ── Website ───────────────────────────────────────────────────────────────────

website-dev:
	cd website && bun run dev

website-build:
	cd website && bun run build

# ── Tests ─────────────────────────────────────────────────────────────────────
#
# test-fast — no external dependencies; used by the pre-commit hook
#   - sensei-bootstrap unit tests (pure Rust, no DB)
#   - app Vitest unit tests (no DB)
#
# test — full suite; requires sensei_test PostgreSQL database with full schema
#   Set TEST_DATABASE_URL=postgresql://localhost:5432/sensei_test (default)

test-fast: test-crates-fast test-app-unit

test-crates-fast:
	cargo test -p sensei-bootstrap

test: test-crates test-app-unit

test-crates:
	cargo test --workspace

test-app: test-app-unit

test-app-unit:
	cd app && bun run test:unit

# E2E runs against the throw-away `sensei_e2e` DB (set by SENSEI_INSTANCE=e2e
# in the e2e globalSetup). Dropping it here guarantees each run starts clean.
# The user's real `sensei` DB is never touched.
reset-e2e-db:
	@echo "[e2e] Dropping sensei_e2e for a clean slate..."
	dropdb --if-exists sensei_e2e
	@echo "[e2e] Done — bootstrap will recreate via the DB resolver."

# reset=true  → drop and recreate e2e DB before running (default)
# reset=false → skip DB reset (use existing DB)
reset ?= true
test-app-e2e: app-e2e-build
	$(if $(filter true,$(reset)),$(MAKE) reset-e2e-db)
	cd app && bun run test:e2e

# ── Cold-start E2E ────────────────────────────────────────────────────────────
# Verifies the health page drives itself through the full check → resolve →
# land flow with no test-driven navigation. Setup stops postgres + ollama
# and drops the e2e DB so the resolvers have real work to do. Teardown
# always restarts services so the dev box returns to a working state,
# even if the test fails.

_e2e-cold-pre:
	@echo "[e2e-cold] Setup: drop sensei_e2e + stop all three services"
	-brew services start postgresql@17
	@sleep 2
	-dropdb --if-exists sensei_e2e
	-brew services stop postgresql@17
	-brew services stop ollama
	-brew services stop sensei
	-pkill -x senseid
	@sleep 1

_e2e-cold-post:
	@echo "[e2e-cold] Teardown: restart services (including user's brew sensei)"
	-brew services start postgresql@17
	-brew services start ollama
	-brew services start sensei

# Note: uses literal `make` (not $(MAKE)) inside the shell pipeline so
# `make -n test-app-e2e-cold` is an honest dry-run. With $(MAKE) inside
# a recipe shell, GNU make force-executes the line under -n.
test-app-e2e-cold: app-e2e-build _e2e-cold-pre
	@cd app && bun run test:e2e:cold ; \
	  RC=$$? ; \
	  cd .. ; \
	  make _e2e-cold-post ; \
	  exit $$RC

# ── Git hooks ─────────────────────────────────────────────────────────────────
# Run once after cloning: make setup-hooks

setup-hooks:
	git config core.hooksPath .githooks
	chmod +x .githooks/pre-commit
	@echo "Git hooks installed — pre-commit will run unit tests before each commit"

# ── Dependency updates ────────────────────────────────────────────────────────

update:
	@echo "Updating Rust dependencies..."
	cargo update
	@echo "Updating Node dependencies (app)..."
	cd app && bun update
	@echo "Updating Node dependencies (website)..."
	cd website && bun update
	@echo "Running tests to verify updates..."
	$(MAKE) test
	@echo "All dependencies updated and tests passed."
	@echo "Review: git diff Cargo.lock app/bun.lock website/bun.lock"

# ── Version bump ──────────────────────────────────────────────────────────────
# Usage:
#   make bump v=patch       — 0.2.13 → 0.2.14
#   make bump v=minor       — 0.2.13 → 0.3.0
#   make bump v=major       — 0.2.13 → 1.0.0
#   make bump v=0.5.0       — explicit version
#
# Safety: aborts if the target tag already exists (prevents duplicate bumps).
# Updates all version strings, commits, creates a git tag, pushes the commit
# and tag (which triggers the GitHub Actions release workflows), then syncs
# the updated Homebrew formula version to the tap and marketplace.
# GitHub Actions will fill in the real SHA256s once artifacts are built.

bump:
	@if [ -z "$(v)" ]; then echo "Usage: make bump v=patch|minor|major|<version>"; exit 1; fi
	$(eval _v := $(shell \
	  cur=$$(cat VERSION); \
	  if [ "$(v)" = "patch" ]; then echo "$$cur" | awk -F. '{printf "%s.%s.%s", $$1, $$2, $$3+1}'; \
	  elif [ "$(v)" = "minor" ]; then echo "$$cur" | awk -F. '{printf "%s.%s.0", $$1, $$2+1}'; \
	  elif [ "$(v)" = "major" ]; then echo "$$cur" | awk -F. '{printf "%s.0.0", $$1+1}'; \
	  else echo "$(v)"; \
	  fi))
	@# Safety: block if tag already exists
	@if git tag -l "v$(_v)" | grep -q .; then \
	  echo "Error: tag v$(_v) already exists. Current VERSION is $$(cat VERSION)."; \
	  echo "Did you mean: make bump v=patch ?"; \
	  exit 1; \
	fi
	@# Safety: block version downgrades
	@cur=$$(cat VERSION); \
	if [ "$$(printf '%s\n%s' "$$cur" "$(_v)" | sort -V | tail -1)" = "$$cur" ] && [ "$$cur" != "$(_v)" ]; then \
	  echo "Error: cannot bump down ($$cur → $(_v))"; \
	  exit 1; \
	fi; \
	if [ "$$cur" = "$(_v)" ]; then \
	  echo "Error: $(_v) is already the current version"; \
	  exit 1; \
	fi
	@echo "Bumping $$(cat VERSION) → $(_v)"
	@echo "$(_v)" > VERSION
	@# Node manifests
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' app/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' website/package.json
	@# Tauri app manifest + Cargo.toml
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' app/src-tauri/tauri.conf.json
	@sed -i '' "s/^version = \"[^\"]*\"/version = \"$(_v)\"/" app/src-tauri/Cargo.toml
	@# Rust crates
	@for crate in senseid cli mcp gateway bootstrap; do \
	  f="crates/$$crate/Cargo.toml"; \
	  sed -i '' "s/^version = \"[^\"]*\"/version = \"$(_v)\"/" "$$f"; \
	done
	@# Homebrew formula and cask (SHA256s updated by GitHub Actions after release)
	@sed -i '' "s/version \"[^\"]*\"/version \"$(_v)\"/" homebrew/Formula/sensei.rb
	@sed -i '' "s/version \"[^\"]*\"/version \"$(_v)\"/" homebrew/Casks/senseihq.rb
	@# Marketplace
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/catalog.json
	@# Website footer version
	@sed -i '' 's/v[0-9]*\.[0-9]*\.[0-9]*<\/div>/v$(_v)<\/div>/' website/src/routes/+page.svelte
	@# Commit everything
	@git add VERSION \
	  app/package.json app/src-tauri/tauri.conf.json app/src-tauri/Cargo.toml \
	  website/package.json website/src/routes/+page.svelte \
	  crates/senseid/Cargo.toml crates/cli/Cargo.toml crates/mcp/Cargo.toml crates/gateway/Cargo.toml crates/bootstrap/Cargo.toml \
	  homebrew/Formula/sensei.rb homebrew/Casks/senseihq.rb \
	  marketplace/package.json marketplace/catalog.json
	@git commit -m "chore: bump to v$(_v)"
	@git tag v$(_v)
	@git push origin HEAD
	@git push origin v$(_v)
	@echo "Pushed v$(_v) — GitHub Actions will build release artifacts and update tap SHA256s"
	@echo "Syncing homebrew-tap and marketplace..."
	@$(MAKE) tap-push marketplace-push

# Sync homebrew/ files to the tap repo (sensei-hq/homebrew-tap).
# Uses a temporary clone so it works regardless of subtree/squash history.
tap-push:
	@tmpdir=$$(mktemp -d) && \
	git clone git@github.com:sensei-hq/homebrew-tap.git "$$tmpdir" 2>&1 && \
	cp homebrew/Formula/sensei.rb "$$tmpdir/Formula/" && \
	cp homebrew/Casks/senseihq.rb "$$tmpdir/Casks/" && \
	rm -f "$$tmpdir/Formula/sensei-dev.rb" "$$tmpdir/Brewfile" "$$tmpdir/Brewfile-dev" && \
	cd "$$tmpdir" && \
	git add -A && \
	git diff --cached --quiet && echo "homebrew-tap already up to date" || \
	  (git commit -m "chore: sync from sensei monorepo (drop sensei-dev formula)" && git push origin main) && \
	rm -rf "$$tmpdir"

# Sync marketplace/ files to sensei-hq/marketplace.
marketplace-push:
	@tmpdir=$$(mktemp -d) && \
	git clone git@github.com:sensei-hq/marketplace.git "$$tmpdir" 2>&1 && \
	rsync -a --delete --exclude='.git' marketplace/ "$$tmpdir/" && \
	cd "$$tmpdir" && \
	git add -A && \
	git diff --cached --quiet && echo "marketplace already up to date" || \
	  (git commit -m "chore: sync from sensei monorepo" && git push origin main) && \
	rm -rf "$$tmpdir"

# ── Clean ─────────────────────────────────────────────────────────────────────

clean:
	cargo clean
	rm -rf app/.svelte-kit app/build
