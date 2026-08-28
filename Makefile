## Sensei monorepo — root build coordinator
##
## Components:
##   app/         — Tauri + SvelteKit desktop app
##   crates/      — All Rust crates (single workspace)
##     senseid    — HTTP daemon (API server)
##     cli        — sensei CLI (binary: sensei)
##     mcp        — MCP server
##     bootstrap  — installer/prereq checker
##     logger     — structured logging crate
##   website/     — Marketing website
##   docs/        — Documentation
##
## Sibling repos (pulled in as git deps, not in this tree):
##   sensei-hq/gateway — LLM routing library (git dep `gateway-embedded`)
##
## Versioning:
##   VERSION file is the single source of truth.
##   `make bump v=patch|minor|major|0.3.0` updates VERSION + all manifests, commits, tags, pushes,
##   installs locally, then cleans target/ (so the next dev build is cold).
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
        db-backup db-backup-essential db-backup-rotate \
        app-dev app-check \
        website-dev website-build \
        test test-fast test-crates test-crates-fast \
        test-app test-app-unit test-app-e2e test-app-e2e-cold app-e2e-build \
        _e2e-cold-pre _e2e-cold-post reset-e2e-db \
        setup-hooks update bump dbd-cache-clear tap-push marketplace-push \
        clean clean-cache

VERSION := $(shell cat VERSION)

# Health-bypass is decided at runtime via `window.__TAURI__` — Tauri
# injects it before any user script runs (`withGlobalTauri: true`).
# `vite dev`/`vite preview` outside Tauri never sees it, so they auto-
# bypass. No env vars, no build-time flags.

# ── Rust crates ───────────────────────────────────────────────────────────────

# Embedded inference (in-process llama.cpp) is built into ALL daemon builds by
# default (#76): the gateway serves chat + embeddings locally without the
# external Ollama daemon, and the seed chains list the embedded adapter first.
# It compiles llama.cpp natively, so the build needs a C/C++ toolchain
# (cmake + clang) on the build host. Opt OUT with `make <target> EMBED=0` for a
# lean Ollama-only build on a host without that toolchain.
CRATE_FEATURES := $(if $(filter 0 no off,$(EMBED)),,--features senseid/embedded-llama-cpp)

crates:
	cargo build --release -p senseid -p sensei-cli -p sensei-mcp $(CRATE_FEATURES)

crates-debug:
	cargo build -p senseid -p sensei-cli -p sensei-mcp $(CRATE_FEATURES)

# Full-coverage build — exercises every Rust crate, including the Tauri
# sidecar (which lives in its own `[workspace]` and so is skipped by the
# root `cargo build --workspace`). Use this as the compile-time gate
# before bumping or releasing: catches symbol drift in any crate that
# wasn't part of the daemon/CLI/MCP build set.
crates-all:
	cargo build --release --workspace
	cargo build --release --manifest-path app/src-tauri/Cargo.toml
	# Embedded inference is built into shipped daemons by default (#76), so the
	# release gate must compile that path too — catches drift in the
	# embedded-llama-cpp code that the plain --workspace build skips.
	cargo build --release -p senseid --features senseid/embedded-llama-cpp

# ── Dōjō local auth stack (localhost supabase — un-parks the console) ──────────
# Boots a LOCAL-ONLY supabase stack (auth + db + studio + inbucket) for developing
# the Dōjō SaaS console. Config + seed live in supabase/. No real secrets — every
# credential resolves via env(). Nothing leaves the machine.
#   Studio            → http://127.0.0.1:54323
#   Inbucket (mail)   → http://127.0.0.1:54324   (magic-link emails land here)
.PHONY: supabase-up supabase-down
supabase-up:  ## Boot the local Dōjō supabase auth stack (localhost only)
	supabase start
supabase-down:  ## Stop the local Dōjō supabase auth stack
	supabase stop

# ── Install ───────────────────────────────────────────────────────────────────
#
# `make install` does the full install: the service binaries (CLI, daemon,
# MCP) overlaid into the brew prefix, plus the desktop .app bundle copied
# into /Applications/. The two halves are exposed as `install-service` and
# `install-app` for when you only need one. `install-debug` is the fast
# iteration variant of `install-service` — same brew-prefix overlay path,
# debug binaries.

install: install-service install-app

# Snapshot the sensei DB before any install* runs. Custom-format pg_dump
# (-F c) is binary, compressed, and supports `pg_restore -d sensei -c …`
# for clean+restore. No-op when the DB doesn't exist yet (first install).
# Make's target memoisation guarantees this runs exactly once per
# top-level `make install` invocation even though install-service/-app/
# -debug each depend on it.
#
# Restore the latest backup:
#   pg_restore -d sensei -c $$(ls -t database/backup/backup-*.dump | head -1)
db-backup: db-backup-rotate
	@mkdir -p database/backup
	@# Keep Spotlight from indexing the multi-hundred-MB .dump files. Without
	@# this, every backup write triggers mds indexing → CPU spike (observed at
	@# 94%, which starved the e2e health-bootstrap gate and made the suite flaky).
	@touch database/backup/.metadata_never_index
	@if psql -d sensei -c "SELECT 1" >/dev/null 2>&1; then \
	  ts=$$(date +%Y%m%d-%H%M%S); \
	  out="database/backup/backup-$${ts}.dump"; \
	  echo "Backing up sensei DB to $$out..."; \
	  pg_dump -d sensei -F c -f "$$out" && \
	  echo "DB backed up: $$out ($$(ls -lh $$out | awk '{print $$5}'))"; \
	else \
	  echo "sensei DB not present — skipping backup (first-time install)"; \
	fi

# db-backup-rotate — keep only the 5 most recent full backups. Runs before
# `db-backup` so the new dump always fits inside the retention window.
# Each backup is ~350MB compressed; 5 is the sweet spot between rollback
# headroom and disk consumption.
db-backup-rotate:
	@if [ -d database/backup ]; then \
	  keep=5; \
	  ls -t database/backup/backup-*.dump 2>/dev/null \
	    | tail -n +$$((keep + 1)) \
	    | xargs -I{} rm -f "{}"; \
	fi

# db-backup-essential — narrow backup that exports ONLY the tables whose
# contents can't be reconstructed from the source tree:
#   • assistant_events / sessions / transcript_turns / turns — captured user
#     activity that has no other source of truth
#   • memories / detected_patterns / recommendations / reasoning_traces /
#     corrections / drift_items — derived signals distilled by the analyzer that
#     survive raw-event pruning
#   • tool_insights / session_process_evidence / playbook_rules /
#     consolidated_rulesets / memory_outcomes — LLM-derived and accumulated
#     learning. Re-running the analyzer costs real inference spend AND returns
#     different text, so these are not reproducible even in principle.
#   • projects / folders / folders_to_watch / repositories / folder_path_aliases
#     — user identity + scope mapping. The aliases are load-bearing: they are
#     what re-attributes a renamed repo's history (dbd-rs → dbd), so losing them
#     orphans sessions that currently resolve.
#
# NOT exported (rebuildable):
#   • nodes / edges / scan_state — comes from a full scan
#   • library_pages — re-fetched by lib_indexer
#   • logs — pruned by the structured-log TTL (#74 partial)
#   • task_executions — job history; noisy and self-contained
#
# Writes JSONL under database/backup/essential/<ts>/<schema>.<name>.jsonl.
# Runs `dbd export -n <table> --format jsonl` per table so the file layout
# matches import/staging/*.jsonl for future re-import.
db-backup-essential:
	@mkdir -p database/backup/essential
	@if ! psql -d sensei -c "SELECT 1" >/dev/null 2>&1; then \
	  echo "sensei DB not present — skipping essential backup"; \
	  exit 0; \
	fi
	@# Rotate essential-backup snapshots: keep only the 5 most recent so a
	@# 650MB assistant_events export doesn't accumulate unbounded.
	@keep=5; \
	  ls -1t database/backup/essential 2>/dev/null | tail -n +$$((keep + 1)) \
	    | while read d; do rm -rf "database/backup/essential/$$d"; done
	@ts=$$(date +%Y%m%d-%H%M%S); \
	  outdir="database/backup/essential/$${ts}"; \
	  mkdir -p "$$outdir"; \
	  echo "Essential backup → $$outdir"; \
	  for t in activity.assistant_events activity.sessions activity.transcript_turns activity.turns \
	           activity.session_process_evidence \
	           sensei.memories sensei.memory_outcomes sensei.tool_insights \
	           sensei.playbook_rules sensei.consolidated_rulesets \
	           inference.detected_patterns inference.recommendations \
	           inference.reasoning_traces inference.corrections inference.drift_items \
	           sensei.projects sensei.folders sensei.folders_to_watch \
	           sensei.repositories sensei.folder_path_aliases; do \
	    DATABASE_URL="postgres://localhost/sensei" \
	      dbd export -n "$$t" --format jsonl \
	                 --output "$$outdir" \
	                 --source database >/dev/null 2>&1 || \
	      echo "  skip $$t (dbd export failed — table may not be tracked in design.yaml)"; \
	  done; \
	  echo "Essential backup complete — $$(du -sh "$$outdir" | awk '{print $$1}')"

# Overlay freshly-built release binaries into the brew prefix.
#
# `bin.install` in the brew Formula sets the destination mode to 0555
# (read+exec, no write), so cp-overwrite fails with EACCES; `rm -f` unlinks
# the read-only file (needs write on parent dir, not on the file itself).
# Re-sign with hardened runtime so the Tauri sidecar can spawn them (macOS
# Sequoia Code Signing Monitor level 2 requires this).
install-service: db-backup crates
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
	@$(MAKE) mcp-refresh-note

# Build the desktop .app bundle and install it to /Applications/.
# Stop any running instance first — `cp -R` over a running .app would mix
# old code with new resources, and the next launch would crash with a
# code-signature mismatch.
install-app: db-backup
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
install-debug: db-backup crates-debug
	@# Stop via brew services FIRST, then pkill any stragglers. `pkill` alone is
	@# not enough: launchd's keep_alive in the brew service plist respawns the
	@# daemon within a few ms, so the cp lands while a stale process is still up
	@# and launchd keeps serving the OLD binary. Restart at the end so the new
	@# binary is the one serving requests.
	@echo "Stopping sensei service (so the new binary actually takes effect)..."
	-@brew services stop sensei >/dev/null 2>&1
	-@pkill -x senseid 2>/dev/null
	@sleep 1
	@DEST=$$(brew --prefix sensei)/bin && \
	rm -f "$$DEST/senseid" "$$DEST/sensei" "$$DEST/sensei-mcp" && \
	cp target/debug/senseid    "$$DEST/senseid" && \
	cp target/debug/sensei     "$$DEST/sensei" && \
	cp target/debug/sensei-mcp "$$DEST/sensei-mcp" && \
	codesign --sign - --options runtime --force "$$DEST/senseid" && \
	codesign --sign - --options runtime --force "$$DEST/sensei" && \
	codesign --sign - --options runtime --force "$$DEST/sensei-mcp" && \
	echo "Overlaid debug binaries into $$DEST (codesigned)"
	@echo "Restarting sensei service so the new daemon is live..."
	-@brew services start sensei
	@$(MAKE) mcp-refresh-note

# Post-install MCP/plugin refresh (shared by install-service + install-debug).
# The sensei MCP is a long-lived stdio subprocess owned by the Claude Code
# session — NOT a brew service — so `brew services restart` above never touches
# it; it keeps the OLD in-memory binary until the client reconnects. Kill any
# straggler so the next session/reconnect execs the freshly-overlaid binary,
# best-effort refresh each assistant's plugin via `sensei upgrade` (runs
# `claude plugin update sensei` daemon-side; the daemon is up from the restart
# above), then print the reminder for the plugin/tool-surface-change case.
#
# We deliberately DO NOT kill sensei-mcp here. It's a thin stdio proxy with a
# STATIC tool list (lib::handle_list_tools) and per-call HTTP to :7744, so a
# RUNNING proxy stays correct across a daemon restart with zero client action.
# Killing it only matters when the tool SURFACE changed (new mcp binary), and
# even then relies on the client auto-respawning the stdio server — which it
# often doesn't, silently dropping the tools until a manual /mcp (the bug this
# used to cause). So a daemon-only upgrade is truly live-immediately, and a
# tool-surface change is covered by the plugin-update reminder below.
.PHONY: mcp-refresh-note
mcp-refresh-note:
	-@"$$(brew --prefix sensei)/bin/sensei" upgrade >/dev/null 2>&1 \
	  && echo "  ✓ ran 'sensei upgrade' — assistant plugins refreshed" || true
	@echo ""
	@echo "  ℹ  Daemon-only upgrades are LIVE IMMEDIATELY — the sensei MCP is a thin"
	@echo "     proxy to :7744, so a running session keeps working, no reconnect needed."
	@echo "     Only if the MCP TOOL SURFACE or plugin hooks changed, refresh the plugin:"
	@echo "       claude plugin update sensei@sensei-marketplace"
	@echo "     (marketplace qualifier REQUIRED; a running session picks it up on /mcp"
	@echo "     reconnect or restart.)"
	@echo ""

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

test: test-crates test-app-unit test-dojo

test-crates:
	cargo test --workspace

test-app: test-app-unit

test-app-unit:
	cd app && bun run test:unit

# The dōjō's vitest suite. Was in no aggregate target at all, so it only ran when
# someone remembered to — which is part of how two dead code paths sat behind a
# green suite (spec dojo-auth-provisioning §VIII.4). Needs no database.
test-dojo:
	cd dojo && bun run test

# SQL assertions against a REAL Postgres — RLS, constraints, policy grants: the
# things a mocked supabase-js client cannot check and therefore cannot fail on.
# Deliberately NOT part of `make test`: it needs the local Supabase up
# (`supabase start`), and a missing service should not read as a test failure.
#   DATABASE_URL overrides the target; defaults to the local Supabase.
test-db:
	./database/tests/run.sh

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
	@# Safety net: globalSetup stops the real brew `sensei` daemon and runs an
	@# e2e daemon (--instance e2e, throwaway sensei_e2e DB) on the SAME :7744.
	@# globalTeardown restores it on a clean exit, but a SIGTERM/interrupt to
	@# Playwright skips teardown → the system is left with the real daemon down
	@# and the empty-DB e2e daemon squatting on :7744 (looks like total data
	@# loss; see reference_e2e_7744_leftover). This trap restores the real daemon
	@# on ANY exit — redundant with teardown on success, the actual fix on kill.
	cd app && trap 'pkill -f "instance e2e" 2>/dev/null; brew services start sensei >/dev/null 2>&1 || true' EXIT INT TERM; bun run test:e2e

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
#
# Then, locally: `make install` (so the machine that cut the release is running
# it — the daemon once sat on v0.2.29 for ten releases) and `make clean` (so
# target/, which reaches tens of GB, does not survive the release).
# NOTE: the next dev build after a bump is therefore COLD.

# ALIAS for `bump`, kept for muscle memory and any script that calls it.
#
# `ship` existed because `bump` did not install, which left the running daemon
# stale behind a release (it once ran v0.2.29 for ten releases). `bump` now
# installs as its final step, so the two are the same thing — and this must
# DELEGATE rather than call `bump` and then `install` again, which would build
# and install the same version twice.
# Usage: make ship v=patch
.PHONY: ship
ship:
	@if [ -z "$(v)" ]; then echo "Usage: make ship v=patch|minor|major|<version>"; exit 1; fi
	@$(MAKE) bump v=$(v)

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
	@# Dōjō web app — the version is stamped into the build (vite reads
	@# package.json) and shown in the console footer / served at /version.
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' dojo/package.json
	@# Tauri app manifest + Cargo.toml
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' app/src-tauri/tauri.conf.json
	@sed -i '' "s/^version = \"[^\"]*\"/version = \"$(_v)\"/" app/src-tauri/Cargo.toml
	@# Rust crates — every published or internally-pathed crate in the
	@# workspace tracks the monorepo version. Adding a new crate? Append
	@# its directory name here and to the git-add list below.
	@for crate in senseid cli mcp bootstrap logger sensei-config; do \
	  f="crates/$$crate/Cargo.toml"; \
	  sed -i '' "s/^version = \"[^\"]*\"/version = \"$(_v)\"/" "$$f"; \
	done
	@# Refresh Cargo.lock files so the new versions are reflected before we
	@# stage. The pre-commit hook runs `cargo build` and would otherwise
	@# modify these AFTER `git add` and leave them dirty in the working tree.
	@cargo check --workspace --offline --quiet 2>/dev/null || cargo check --workspace --quiet
	@(cd app/src-tauri && cargo check --offline --quiet 2>/dev/null || cargo check --quiet)
	@# Homebrew formula and cask (SHA256s updated by GitHub Actions after release)
	@sed -i '' "s/version \"[^\"]*\"/version \"$(_v)\"/" homebrew/Formula/sensei.rb
	@sed -i '' "s/version \"[^\"]*\"/version \"$(_v)\"/" homebrew/Casks/senseihq.rb
	@# Marketplace — package.json/catalog.json are tooling metadata; the two
	@# .claude-plugin manifests are what Claude Code actually reads to decide
	@# whether an installed plugin has an update available. If those two stay
	@# frozen, `claude plugin update sensei` will always report "up to date"
	@# even after a real bump.
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/package.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/catalog.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/.claude-plugin/marketplace.json
	@sed -i '' 's/"version": "[^"]*"/"version": "$(_v)"/' marketplace/plugins/sensei/.claude-plugin/plugin.json
	@# Website footer version
	@sed -i '' 's/v[0-9]*\.[0-9]*\.[0-9]*<\/div>/v$(_v)<\/div>/' website/src/routes/+page.svelte
	@# Capture the schema as a versioned snapshot — but ONLY once the project is
	@# released. Pre-release nothing is written, so `database/snapshots/` does not
	@# exist and the folder stays clean until there is a baseline worth diffing
	@# against.
	@#
	@# `dbd release` (run once, at the first public release) sets `released: true`
	@# in database/design.yaml, disables `reconcile`, and writes the baseline. From
	@# that point this step becomes load-bearing: `dbd deploy` migrates from the
	@# committed snapshot, and a release whose DDL changed WITHOUT one deploys
	@# nothing while reporting success. So post-release, a missing dbd is a hard
	@# error rather than a skipped step.
	@#
	@# Self-skipping even then: dbd prints "No schema changes detected — snapshot
	@# skipped" when the design is unchanged. Needs no database — a snapshot is a
	@# diff against the PREVIOUS snapshot, not a live server.
	@if grep -qE '^[[:space:]]*released:[[:space:]]*true' database/design.yaml 2>/dev/null; then \
	  command -v dbd >/dev/null || { \
	    echo "Error: dbd not found — a released project must carry its schema snapshot."; \
	    echo "Install: cargo install --git https://github.com/sensei-hq/dbd dbd"; \
	    exit 1; \
	  }; \
	  (cd database && dbd snapshot --name "v$(_v)"); \
	else \
	  echo "dbd: pre-release — no snapshot (run 'dbd release' once at first public release)"; \
	fi
	@# Commit everything
	@git add VERSION Cargo.lock app/src-tauri/Cargo.lock \
	  app/package.json app/src-tauri/tauri.conf.json app/src-tauri/Cargo.toml \
	  website/package.json website/src/routes/+page.svelte dojo/package.json \
	  crates/senseid/Cargo.toml crates/cli/Cargo.toml crates/mcp/Cargo.toml \
	  crates/bootstrap/Cargo.toml crates/logger/Cargo.toml crates/sensei-config/Cargo.toml \
	  homebrew/Formula/sensei.rb homebrew/Casks/senseihq.rb \
	  marketplace/package.json marketplace/catalog.json \
	  marketplace/.claude-plugin/marketplace.json \
	  marketplace/plugins/sensei/.claude-plugin/plugin.json
	@# Schema snapshot artefacts, when the project is released (see above).
	@if [ -d database/snapshots ]; then git add database/design.yaml database/snapshots; fi
	@git commit -m "chore: bump to v$(_v)"
	@git tag v$(_v)
	@git push origin HEAD
	@git push origin v$(_v)
	@echo "Pushed v$(_v) — GitHub Actions will build release artifacts and update tap SHA256s"
	@# Invalidate the dbd schema-source cache now that the DDL version changed,
	@# so the next daemon deploy fetches v$(_v)'s DDL instead of serving the
	@# previous version's cached bundle (which would re-apply the old schema).
	@$(MAKE) dbd-cache-clear
	@echo "Syncing homebrew-tap and marketplace..."
	@$(MAKE) tap-push marketplace-push
	@# Install the version we just tagged, so the machine that cut the release is
	@# running it. Deliberately AFTER the push: the tag is what CI builds from, so
	@# a local toolchain problem must not strand a release that is already valid
	@# everywhere else. It re-runs db-backup, release-builds the crates, and
	@# replaces /Applications/Sensei.app (quitting a running instance first).
	@echo "Installing v$(_v) locally..."
	@$(MAKE) install
	@# ...then reclaim the tree that build needed. MUST come after install —
	@# reversed, clean would delete target/ and install would rebuild it all from
	@# cold for nothing.
	@#
	@# This supersedes the `clean-cache` prune that used to run here: that kept the
	@# 5 newest incremental caches, and `clean` removes target/ outright, so doing
	@# both would just be pruning something a moment before deleting its parent.
	@#
	@# The trade is deliberate: the next dev build after a bump is COLD (a full
	@# rebuild, several minutes). A release is infrequent and target/ reaches tens
	@# of GB, so paying the rebuild once per release beats carrying the disk.
	@echo "Reclaiming the build tree..."
	@$(MAKE) clean

# Clear the dbd schema-source cache. The daemon resolves its DDL from
# `sensei-hq/sensei/database@v<VERSION>` and dbd caches resolved sources per
# version under ~/Library/Caches/dbd/. While we're pre-stable and NOT yet using
# dbd snapshots/migrations (the schema is reapplied declaratively each deploy),
# a stale cache entry makes a post-bump deploy reapply the PREVIOUS version's
# DDL — recreating dropped objects / reverting view changes. `bump` invokes this
# so the next deploy refetches the freshly tagged DDL. Revisit once migrations
# land (then dbd tracks state in _dbd_meta and the cache is no longer a hazard).
dbd-cache-clear:
	@rm -rf "$(HOME)/Library/Caches/dbd"/sensei-hq-sensei-* 2>/dev/null || true
	@echo "Cleared dbd schema-source cache (next deploy refetches the tagged DDL)"

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
#
# Two clean levels:
#
#   make clean       — nuke everything reproducible: target/ (both workspaces),
#                      app build artifacts, DB backups older than 7 days.
#                      Full rebuild after this (~2 min for crates alone).
#
#   make clean-cache — quick prune: keep target/debug artifacts warm but drop
#                      stale rustc incremental caches (target/debug/incremental
#                      accumulates ~800MB per rustc invocation and cargo doesn't
#                      GC it). Called by `make bump` so releases don't ship a
#                      2-week-old cache.
#
# target/ bloat: cargo's target/debug/incremental grows by ~1GB per rustc
# invocation without upper bound. On a busy day this pushes target/ to 100+ GB.
# `clean-cache` keeps only the 5 most recent incremental caches per crate.

clean:
	@echo "Cleaning target/ (both workspaces)..."
	cargo clean
	@if [ -d app/src-tauri/target ]; then \
	  (cd app/src-tauri && cargo clean); \
	fi
	@echo "Cleaning app build artifacts..."
	rm -rf app/.svelte-kit app/build
	@echo "Pruning DB backups older than 7 days..."
	@if [ -d database/backup ]; then \
	  find database/backup -name 'backup-*.dump' -mtime +7 -delete -print; \
	fi
	@echo "Clean complete."

clean-cache:
	@echo "Pruning stale rustc incremental caches (keeping 5 newest per crate)..."
	@for base in target app/src-tauri/target; do \
	  inc="$$base/debug/incremental"; \
	  if [ ! -d "$$inc" ]; then continue; fi; \
	  keep=5; \
	  find "$$inc" -mindepth 1 -maxdepth 1 -type d -print0 \
	    | xargs -0 -I{} stat -f "%m %N" "{}" 2>/dev/null \
	    | sort -rn \
	    | tail -n +$$((keep + 1)) \
	    | awk '{print $$2}' \
	    | xargs -r -I{} rm -rf "{}"; \
	  echo "  $$inc: kept last $$keep, rest pruned"; \
	done
	@echo "Cache prune complete."
