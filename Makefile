## Sensei monorepo — root build coordinator
##
## Components:
##   app/      — Tauri + SvelteKit desktop app
##   daemon/   — Rust backend (senseid, sensei-cli, sensei-mcp)
##   website/  — Marketing website
##   gateway/  — LLM routing library
##   docs/     — Documentation
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

.PHONY: build-dev build-release install-dev install-release \
        daemon-dev daemon-release \
        app-dev app-dev-bundle app-release app-check \
        website-dev website-build \
        test test-fast test-daemon test-daemon-fast \
        test-app test-app-unit test-app-e2e test-app-sidecar \
        setup-hooks update bump tap-push clean

VERSION := $(shell cat VERSION)

# Load dev environment from .env.dev (DATABASE_URL, SENSEI_MODE, VITE_BYPASS_HEALTH)
ifeq (,$(wildcard .env.dev))
  $(error .env.dev not found — create it with DATABASE_URL, SENSEI_MODE, and VITE_BYPASS_HEALTH)
endif
include .env.dev
export DATABASE_URL SENSEI_MODE VITE_BYPASS_HEALTH

# ── Daemon ────────────────────────────────────────────────────────────────────

daemon-dev:
	$(MAKE) -C daemon dev

daemon-release:
	$(MAKE) -C daemon release

install-dev: daemon-dev
	$(MAKE) -C daemon install-dev

install-release: daemon-release
	$(MAKE) -C daemon install-release

build-dev: daemon-dev
	@echo "Dev build complete — binaries in daemon/target/debug/"

build-release: daemon-release
	@echo "Release build complete — binaries in daemon/target/release/"

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

test-fast: test-daemon-fast test-app-unit

test-daemon-fast:
	cd daemon && cargo test -p sensei-bootstrap

test: test-daemon test-app-unit test-app-sidecar

test-daemon:
	$(MAKE) -C daemon test

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
	@echo "Updating Rust dependencies (daemon)..."
	cargo update --manifest-path daemon/Cargo.toml
	@echo "Updating Rust dependencies (gateway)..."
	cargo update --manifest-path gateway/crates/gateway/Cargo.toml
	@echo "Updating Node dependencies (app)..."
	cd app && bun update
	@echo "Updating Node dependencies (website)..."
	cd website && bun update
	@echo "Running tests to verify updates..."
	$(MAKE) test
	@echo "All dependencies updated and tests passed."
	@echo "Review: git diff daemon/Cargo.lock app/bun.lock website/bun.lock"

# ── Version bump ──────────────────────────────────────────────────────────────
# Usage: make bump v=0.3.0
#
# Updates all version strings, commits, creates a git tag, pushes the commit
# and tag (which triggers the GitHub Actions release workflows), then syncs
# the updated Homebrew formula version to the tap via git subtree push.
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
	@# Daemon Rust crates (excludes bootstrap which has its own cadence)
	@for crate in senseid cli mcp; do \
	  f="daemon/crates/$$crate/Cargo.toml"; \
	  sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" "$$f"; \
	done
	@# Gateway Rust crate
	@sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" gateway/crates/gateway/Cargo.toml
	@# Homebrew formula and cask (SHA256s updated by GitHub Actions after release)
	@sed -i '' "s/version \"[^\"]*\"/version \"$(v)\"/" homebrew/Formula/sensei.rb
	@sed -i '' "s/version \"[^\"]*\"/version \"$(v)\"/" homebrew/Casks/sensei.rb
	@# Commit everything
	@git add VERSION \
	  app/package.json app/src-tauri/tauri.conf.json app/src-tauri/Cargo.toml \
	  website/package.json \
	  daemon/crates/senseid/Cargo.toml daemon/crates/cli/Cargo.toml daemon/crates/mcp/Cargo.toml \
	  gateway/crates/gateway/Cargo.toml \
	  homebrew/Formula/sensei.rb homebrew/Casks/sensei.rb
	@git commit -m "chore: bump to v$(v)"
	@git tag v$(v)
	@git push origin HEAD
	@git push origin v$(v)
	@echo "Pushed v$(v) — GitHub Actions will build release artifacts and update tap SHA256s"
	@echo "Syncing formula version to homebrew-tap..."
	@$(MAKE) tap-push

# Push the homebrew/ subtree to the tap repo (sensei-hq/homebrew-tap)
tap-push:
	git subtree push --prefix homebrew https://github.com/sensei-hq/homebrew-tap main

# ── Clean ─────────────────────────────────────────────────────────────────────

clean:
	$(MAKE) -C daemon clean
	rm -rf app/.svelte-kit app/build
