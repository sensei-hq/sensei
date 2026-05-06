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

.PHONY: build-dev build-release install-dev install-release test bump \
        daemon-dev daemon-release app-dev app-release \
        website-dev website-build clean

VERSION := $(shell cat VERSION)

# ── Dev builds ────────────────────────────────────────────────────────────────

build-dev: daemon-dev
	@echo "Dev build complete (daemon binaries in daemon/target/debug/)"

daemon-dev:
	$(MAKE) -C daemon dev

daemon-release:
	$(MAKE) -C daemon release

install-dev:
	$(MAKE) -C daemon install-dev

install-release:
	$(MAKE) -C daemon install-release

# ── Release builds ────────────────────────────────────────────────────────────

build-release: daemon-release
	@echo "Release build complete (daemon binaries in daemon/target/release/)"

app-dev:
	cd app && bun run tauri:vite-dev

app-release:
	cd app && bun run tauri:build

# ── Website ───────────────────────────────────────────────────────────────────

website-dev:
	cd website && bun run dev

website-build:
	cd website && bun run build

# ── Tests ─────────────────────────────────────────────────────────────────────

test: test-daemon test-app

test-daemon:
	$(MAKE) -C daemon test

test-app:
	cd app && bun run test:unit

# ── Version bump ──────────────────────────────────────────────────────────────
# Usage: make bump v=0.3.0

bump:
	@if [ -z "$(v)" ]; then echo "Usage: make bump v=<version>"; exit 1; fi
	@echo "$(v)" > VERSION
	@# Update app/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(v)"/' app/package.json
	@# Update daemon workspace Cargo.toml (top-level version field)
	@sed -i '' 's/^version = "[^"]*"/version = "$(v)"/' daemon/Cargo.toml
	@echo "Bumped to $(v) in VERSION, app/package.json, daemon/Cargo.toml"
	@echo "Review and commit: git add VERSION app/package.json daemon/Cargo.toml"

# ── Clean ─────────────────────────────────────────────────────────────────────

clean:
	$(MAKE) -C daemon clean
	rm -rf app/.svelte-kit app/build
