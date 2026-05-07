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
##   `make bump v=0.3.0` updates VERSION + all manifests, commits, tags, and pushes.
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

.PHONY: build-dev build-release install-dev install-release \
        crates-dev crates-release \
        app-dev app-dev-bundle app-release app-check \
        website-dev website-build \
        test test-fast test-crates test-crates-fast \
        test-app test-app-unit test-app-e2e test-app-sidecar \
        setup-hooks update bump tap-push marketplace-push clean

VERSION := $(shell cat VERSION)

# Load dev environment from .env.dev (DATABASE_URL, SENSEI_MODE, VITE_BYPASS_HEALTH)
ifeq (,$(wildcard .env.dev))
  $(error .env.dev not found — create it with DATABASE_URL, SENSEI_MODE, and VITE_BYPASS_HEALTH)
endif
include .env.dev
export DATABASE_URL SENSEI_MODE VITE_BYPASS_HEALTH

# ── Rust crates ───────────────────────────────────────────────────────────────

crates-dev:
	cargo build -p senseid -p sensei-cli -p sensei-mcp

crates-release:
	cargo build --release -p senseid -p sensei-cli -p sensei-mcp

install-dev: crates-dev
	@mkdir -p ~/.local/bin
	cp target/debug/senseid    ~/.local/bin/senseid
	cp target/debug/sensei     ~/.local/bin/sensei
	cp target/debug/sensei-mcp ~/.local/bin/sensei-mcp
	@echo "Installed dev binaries to ~/.local/bin"
	@echo "Make sure ~/.local/bin is before /opt/homebrew/bin in PATH"

install-release: crates-release
	@mkdir -p ~/.local/bin
	cp target/release/senseid    ~/.local/bin/senseid
	cp target/release/sensei     ~/.local/bin/sensei
	cp target/release/sensei-mcp ~/.local/bin/sensei-mcp
	@echo "Installed release binaries to ~/.local/bin"

build-dev: crates-dev
	@echo "Dev build complete — binaries in target/debug/"

build-release: crates-release
	@echo "Release build complete — binaries in target/release/"

# ── Desktop app ───────────────────────────────────────────────────────────────

# Tauri dev with Vite HMR — pre-builds Rust backend then starts tauri dev
app-dev:
	cd app && cargo build --manifest-path src-tauri/Cargo.toml && bunx tauri dev

# Build debug .app bundle and launch it (full native bundle, slower than app-dev)
app-dev-bundle:
	cd app && SENSEI_DB_SCHEMA_PATH=../database bunx tauri build --debug && SENSEI_DB_SCHEMA_PATH=../database ./src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop

app-release:
	cd app && bunx tauri build

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
#   or override: make test TEST_DATABASE_URL=postgresql://localhost:5432/sensei_dev

test-fast: test-crates-fast test-app-unit

test-crates-fast:
	cargo test -p sensei-bootstrap

test: test-crates test-app-unit test-app-sidecar

test-crates:
	cargo test --workspace

test-app: test-app-unit test-app-sidecar

test-app-unit:
	cd app && bun run test:unit

test-app-e2e:
	cd app && bun run test:e2e

test-app-sidecar:
	cd app && bun run test:sidecar

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
# Usage: make bump v=0.3.0
#
# Updates all version strings, commits, creates a git tag, pushes the commit
# and tag (which triggers the GitHub Actions release workflows), then syncs
# the updated Homebrew formula version to the tap and marketplace.
# GitHub Actions will fill in the real SHA256s once artifacts are built.

bump:
	@if [ -z "$(v)" ]; then echo "Usage: make bump v=<version>"; exit 1; fi
	@echo "$(v)" > VERSION
	@# Node manifests
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' app/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' website/package.json
	@# Tauri app manifest + Cargo.toml
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' app/src-tauri/tauri.conf.json
	@sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" app/src-tauri/Cargo.toml
	@# Rust crates (excludes bootstrap which has its own cadence)
	@for crate in senseid cli mcp gateway; do \
	  f="crates/$$crate/Cargo.toml"; \
	  sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" "$$f"; \
	done
	@# Homebrew formula and cask (SHA256s updated by GitHub Actions after release)
	@sed -i '' "s/version \"[^\"]*\"/version \"$(v)\"/" homebrew/Formula/sensei.rb
	@sed -i '' "s/version \"[^\"]*\"/version \"$(v)\"/" homebrew/Casks/sensei.rb
	@# Marketplace
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' marketplace/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' marketplace/catalog.json
	@# Commit everything
	@git add VERSION \
	  app/package.json app/src-tauri/tauri.conf.json app/src-tauri/Cargo.toml \
	  website/package.json \
	  crates/senseid/Cargo.toml crates/cli/Cargo.toml crates/mcp/Cargo.toml crates/gateway/Cargo.toml \
	  homebrew/Formula/sensei.rb homebrew/Casks/sensei.rb \
	  marketplace/package.json marketplace/catalog.json
	@git commit -m "chore: bump to v$(v)"
	@git tag v$(v)
	@git push origin HEAD
	@git push origin v$(v)
	@echo "Pushed v$(v) — GitHub Actions will build release artifacts and update tap SHA256s"
	@echo "Syncing homebrew-tap and marketplace..."
	@$(MAKE) tap-push marketplace-push

# Sync homebrew/ files to the tap repo (sensei-hq/homebrew-tap).
# Uses a temporary clone so it works regardless of subtree/squash history.
tap-push:
	@tmpdir=$$(mktemp -d) && \
	git clone git@github.com:sensei-hq/homebrew-tap.git "$$tmpdir" 2>&1 && \
	cp homebrew/Formula/sensei.rb "$$tmpdir/Formula/" && \
	cp homebrew/Casks/sensei.rb "$$tmpdir/Casks/" && \
	cd "$$tmpdir" && \
	git add -A && \
	git diff --cached --quiet && echo "homebrew-tap already up to date" || \
	  (git commit -m "chore: sync from sensei monorepo" && git push origin main) && \
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
