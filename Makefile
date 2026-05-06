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
##   `make bump v=0.3.0` updates VERSION + all manifests.
##
## Distribution:
##   Homebrew tap: sensei-hq/homebrew-tap
##   macOS install: brew tap sensei-hq/tap && brew install sensei

.PHONY: build-dev build-release install-dev install-release \
        daemon-dev daemon-release \
        app-dev app-dev-bundle app-release app-check \
        website-dev website-build \
        test test-daemon test-app test-app-unit test-app-e2e test-app-sidecar \
        update bump clean

VERSION := $(shell cat VERSION)

# Env vars for desktop app dev builds
APP_DEV_ENV  := DATABASE_URL=postgresql://localhost:5432/sensei_dev SENSEI_MODE=dev VITE_BYPASS_HEALTH=true
APP_BUNDLE_ENV := DATABASE_URL=postgresql://localhost:5432/sensei_dev SENSEI_MODE=dev SENSEI_DB_SCHEMA_PATH=../daemon/database

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
	cd app && $(APP_DEV_ENV) cargo build --manifest-path src-tauri/Cargo.toml && $(APP_DEV_ENV) tauri dev

# Build debug .app bundle and launch it (full native bundle, slower than app-dev)
app-dev-bundle:
	cd app && $(APP_BUNDLE_ENV) tauri build --debug && $(APP_BUNDLE_ENV) ./src-tauri/target/debug/bundle/macos/Sensei.app/Contents/MacOS/sensei-desktop

app-release:
	cd app && tauri build

# Type-check SvelteKit sources
app-check:
	cd app && bun run check

# ── Website ───────────────────────────────────────────────────────────────────

website-dev:
	cd website && bun run dev

website-build:
	cd website && bun run build

# ── Tests ─────────────────────────────────────────────────────────────────────

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

bump:
	@if [ -z "$(v)" ]; then echo "Usage: make bump v=<version>"; exit 1; fi
	@echo "$(v)" > VERSION
	@# Node manifests
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' app/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' website/package.json
	@# Daemon Rust crates (excludes bootstrap which has its own cadence)
	@for crate in senseid cli mcp; do \
	  f="daemon/crates/$$crate/Cargo.toml"; \
	  sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" "$$f"; \
	done
	@# Gateway Rust crate
	@sed -i '' "s/^version = \"[^\"]*\"/version = \"$(v)\"/" gateway/crates/gateway/Cargo.toml
	@echo "Bumped to $(v) in:"
	@echo "  VERSION"
	@echo "  app/package.json, website/package.json"
	@echo "  daemon/crates/{senseid,cli,mcp}/Cargo.toml"
	@echo "  gateway/crates/gateway/Cargo.toml"
	@echo "Review: git diff VERSION app/package.json website/package.json daemon/crates/*/Cargo.toml gateway/crates/gateway/Cargo.toml"

# ── Clean ─────────────────────────────────────────────────────────────────────

clean:
	$(MAKE) -C daemon clean
	rm -rf app/.svelte-kit app/build
