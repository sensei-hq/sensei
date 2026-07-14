# Build & Release

## Overview

The build system coordinates Rust crates, the Tauri desktop app, the marketing website, and Homebrew distribution from a single monorepo. Versioning uses a single `VERSION` file.

There is **one build mode**. Sensei previously bifurcated into "dev" (port 7745, `sensei_dev` DB, `~/.sensei-dev/`, `-dev`-suffixed binaries) and "prod" (port 7744, `sensei`, `~/.sensei/`). That split was a constant source of state-leak bugs and has been removed — one binary, one port, one DB.

---

## Configuration

All compile-time values live in `crates/bootstrap/src/config.rs::SenseiConfig`:

| Setting        | Value                                |
|----------------|--------------------------------------|
| Daemon port    | `7744`                               |
| Data directory | `~/.sensei/`                         |
| Database name  | `sensei`                             |
| Binary names   | `senseid`, `sensei`, `sensei-mcp`    |
| Brew tap       | `sensei-hq/tap/sensei`               |

### Runtime overrides

There is one knob, used for iterating on DDL without publishing a release tag:

```bash
SENSEI_DDL_DIR=/abs/path/to/database senseid start
```

When `SENSEI_DDL_DIR` is set, `SenseiConfig::db_schema_source()` resolves to that local directory instead of the GitHub-tagged release. No other runtime overrides exist.

### Why hardcoded port, not dynamic

Hook scripts, Tauri apps, and the CLI all need to know the port without negotiation. Dynamic port selection would require a discovery mechanism that the CLI and external hooks could not consult easily. One well-known port is simpler.

---

## Version management

`VERSION` at the repo root is the single source of truth.

`make bump v=X.Y.Z` (or `v=patch|minor|major`) updates all manifests in one atomic commit:

| File                                 | What changes        |
|--------------------------------------|---------------------|
| `VERSION`                            | Raw version string  |
| `app/package.json`                   | `version` field     |
| `app/src-tauri/tauri.conf.json`      | `version` field     |
| `app/src-tauri/Cargo.toml`           | `version` field     |
| `website/package.json`               | `version` field     |
| `website/src/routes/+page.svelte`    | Footer version      |
| `crates/{senseid,cli,mcp,gateway,bootstrap}/Cargo.toml` | `version` field |
| `homebrew/Formula/sensei.rb`         | `version` string    |
| `homebrew/Casks/senseihq.rb`         | `version` string    |
| `marketplace/{package,catalog}.json` | `version` field     |

After updating, `bump` commits, tags (`vX.Y.Z`), pushes the commit and tag, then syncs homebrew-tap and marketplace subtrees.

---

## Build targets

### Rust crates

```bash
make crates           # cargo build --release for senseid + sensei-cli + sensei-mcp
make crates-debug     # debug variant (faster compile, same code path)
make crates-all       # full-coverage: root workspace + Tauri sidecar workspace
```

The `crates-all` target is the verification gate before a bump or release. The Tauri sidecar (`app/src-tauri/`) declares its own `[workspace]`, so plain `cargo build --workspace` from the root skips it; `crates-all` builds both halves so a broken sidecar can't slip through.

### Install

Single umbrella target plus two halves and a fast-iteration variant.

```bash
make install            # full install: service binaries + desktop .app
make install-service    # overlay senseid/sensei/sensei-mcp into the brew prefix + codesign
make install-app        # build the .app bundle and cp it to /Applications/
make install-debug      # service overlay with debug binaries (no .app rebuild)
```

`install-service` overlays the freshly-built binaries onto the brew install (`$(brew --prefix sensei)/bin`) and re-codesigns with hardened runtime so the Tauri sidecar can spawn them on macOS Sequoia. `install-app` does the equivalent for the Tauri bundle, stopping any running Sensei.app first so the cp doesn't mix old code with new resources.

### Desktop app dev / e2e

```bash
make app-dev          # Tauri dev with Vite HMR
make app-e2e-build    # Debug .app with --features e2e-testing
```

### Website

```bash
make website-dev      # Vite HMR dev server
make website-build    # Static production build
```

---

## Key Makefile targets

| Target               | Purpose                                                   |
|----------------------|-----------------------------------------------------------|
| `setup-hooks`        | Configure git hooks path to `.githooks/`, enable pre-commit |
| `crates`              | Build senseid + cli + mcp (release)                      |
| `crates-debug`        | Same set, debug profile                                  |
| `crates-all`          | Full-coverage Rust build: root workspace + sidecar       |
| `install`             | Full install: service binaries + desktop .app            |
| `install-service`     | Overlay service binaries into brew prefix + codesign     |
| `install-app`         | Build .app bundle + cp to /Applications/                 |
| `install-debug`       | Service overlay with debug binaries (fast iteration)     |
| `app-dev`             | Tauri dev with Vite HMR                                  |
| `app-check`           | Type-check SvelteKit sources (`svelte-check`)            |
| `app-e2e-build`       | Debug .app with `--features e2e-testing`                 |
| `test`                | Full test suite (requires PostgreSQL test database)      |
| `test-fast`           | Fast tests only (no DB) — used by pre-commit hook        |
| `test-crates`         | `cargo test --workspace`                                 |
| `test-crates-fast`    | `cargo test -p sensei-bootstrap` (pure Rust, no DB)      |
| `test-app-unit`       | Vitest unit tests                                        |
| `test-app-e2e`        | Playwright E2E tests (optionally resets DB)              |
| `update`              | Update Rust + Node deps, run tests                       |
| `bump`                | Version bump across all manifests, commit, tag, push, sync |
| `tap-push`            | Sync `homebrew/` to `sensei-hq/homebrew-tap`             |
| `marketplace-push`    | Sync `marketplace/` to `sensei-hq/marketplace`           |
| `clean`               | `cargo clean` + remove SvelteKit build artifacts         |

---

## Release process

1. **Tests pass** — `make test` (full suite including database tests)
2. **Bump** — `make bump v=X.Y.Z` updates manifests, commits, tags, pushes
3. **CI builds** — tag push triggers GitHub Actions: build release artifacts for all platforms, compute SHA256 hashes, update the Homebrew tap's `Formula/sensei.rb` + `Casks/senseihq.rb`
4. **Subtree sync** — `bump` automatically runs `tap-push` and `marketplace-push`
5. **Distribution** — users receive the update via `brew upgrade sensei` or the app's update check

### Pre-commit hook

`.githooks/pre-commit` runs `make test-fast` (bootstrap unit tests + app Vitest unit tests). No external dependencies required. Configured via `make setup-hooks`.

### Uninstall scope

```bash
sensei remove all --purge   # removes ~/.sensei/ and binaries
```

The `sensei reset` CLI additionally sweeps legacy `senseid-dev` binaries and `~/.sensei-dev/` left behind by older installs.
