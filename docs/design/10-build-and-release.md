# Build & Release

## Overview

The build system coordinates Rust crates, the Tauri desktop app, the marketing website, and Homebrew distribution from a single monorepo. Versioning uses a single `VERSION` file. Dev and release modes are separated at compile time via a Cargo feature flag — no runtime environment variables.

---

## Debug vs Release

Mode is determined at compile time by the `dev` Cargo feature. There is no `--mode` flag, no `SENSEI_MODE` env var, no binary name detection. The binary knows what it is because the decision was baked in at compile time.

```
cargo build --features dev    -> dev mode
cargo build                   -> release mode
```

### Compile-time configuration

All mode-sensitive values live in `sensei-bootstrap/src/config.rs`:

| Setting | Dev | Release |
|---------|-----|---------|
| Daemon port | 7745 | 7744 |
| Data directory | `~/.sensei-dev/` | `~/.sensei/` |
| Database name | `sensei_dev` | `sensei` |

### Feature propagation

Every crate that depends on `sensei-bootstrap` forwards the feature:

```toml
# crates/senseid/Cargo.toml
[features]
dev = ["sensei-bootstrap/dev"]
```

The Tauri sidecar does the same — `app/src-tauri/Cargo.toml` declares `dev = ["sensei-bootstrap/dev"]`. The SvelteKit frontend gets the daemon port injected at Vite build time via `__SENSEI_DEFAULT_PORT__`.

### Binary naming

| Release | Dev | Purpose |
|---------|-----|---------|
| `senseid` | `senseid-dev` | Daemon |
| `sensei` | `sensei-dev` | CLI |
| `sensei-mcp` | `sensei-mcp-dev` | MCP server |

Dev binaries are installed to `~/.local/bin/` alongside release binaries. They coexist without conflict. Dev binaries are re-signed with hardened runtime for macOS Code Signing Monitor compatibility.

### Why compile-time, not runtime

Runtime detection (env vars, binary name checks) is fragile — env vars can be unset, binary names can be wrong. Compile-time guarantees the binary knows what it is regardless of how it is invoked.

### Why hardcoded ports, not dynamic

Hook scripts, Tauri apps, and the CLI all need to know the port without negotiation. Dynamic port selection would require a discovery mechanism. Two well-known ports is simpler.

---

## Version management

`VERSION` at the repo root is the single source of truth. Current version: `0.2.2`.

`make bump v=X.Y.Z` updates all manifests in one atomic commit:

| File | What changes |
|------|-------------|
| `VERSION` | Raw version string |
| `app/package.json` | `version` field |
| `app/src-tauri/tauri.conf.json` | `version` field |
| `app/src-tauri/Cargo.toml` | `version` field |
| `website/package.json` | `version` field |
| `website/src/routes/+page.svelte` | Footer version string |
| `crates/senseid/Cargo.toml` | `version` field |
| `crates/cli/Cargo.toml` | `version` field |
| `crates/mcp/Cargo.toml` | `version` field |
| `crates/gateway/Cargo.toml` | `version` field |
| `crates/bootstrap/Cargo.toml` | `version` field |
| `homebrew/Formula/sensei.rb` | `version` string |
| `homebrew/Casks/senseihq.rb` | `version` string |
| `marketplace/package.json` | `version` field |
| `marketplace/catalog.json` | `version` field |

After updating, `bump` commits, tags (`vX.Y.Z`), pushes the commit and tag, then syncs homebrew-tap and marketplace subtrees.

---

## Build targets

### Rust crates

The workspace contains five crates: `senseid`, `cli`, `mcp`, `gateway`, `bootstrap`.

```bash
make crates-dev       # cargo build --features dev -p senseid -p sensei-cli -p sensei-mcp
make crates-release   # cargo build --release -p senseid -p sensei-cli -p sensei-mcp
```

### Desktop app

```bash
make app-dev          # Tauri dev with Vite HMR (--features dev)
make app-dev-bundle   # Build debug .app bundle and launch it
make app-release      # Production build (no dev feature)
```

`app-dev` pre-builds the Rust backend then starts `tauri dev`. `app-dev-bundle` builds a full native bundle, installs dev binaries, and launches the .app.

### Website

```bash
make website-dev      # Vite HMR dev server
make website-build    # Static production build
```

### Build order

No strict ordering between independent targets. Dependencies within the app build: Rust sidecar must compile before Tauri bundles the app. The website and crates are fully independent.

---

## Key Makefile targets

| Target | Purpose |
|--------|---------|
| `setup-hooks` | Configure git hooks path to `.githooks/`, enable pre-commit |
| `install-dev` | Build dev crates, install to `~/.local/bin/` with `-dev` suffix, codesign |
| `install-release` | Build release crates, install to `~/.local/bin/` |
| `daemon-dev` | Run dev daemon directly from build dir (port 7745) |
| `app-dev` | Tauri dev with Vite HMR |
| `app-dev-bundle` | Full debug .app bundle |
| `app-release` | Production app build |
| `app-check` | Type-check SvelteKit sources (`svelte-check`) |
| `test` | Full test suite (requires PostgreSQL test database) |
| `test-fast` | Fast tests only (no DB) — used by pre-commit hook |
| `test-crates` | `cargo test --workspace` |
| `test-crates-fast` | `cargo test -p sensei-bootstrap` (pure Rust, no DB) |
| `test-app-unit` | Vitest unit tests |
| `test-app-sidecar` | Sidecar integration tests |
| `test-app-e2e` | Playwright E2E tests (optionally resets DB) |
| `update` | Update Rust + Node dependencies, run tests |
| `bump` | Version bump across all manifests, commit, tag, push, sync subtrees |
| `tap-push` | Sync `homebrew/` to `sensei-hq/homebrew-tap` |
| `marketplace-push` | Sync `marketplace/` to `sensei-hq/marketplace` |
| `clean` | `cargo clean` + remove SvelteKit build artifacts |

---

## Release process

1. **Tests pass** — `make test` (full suite including database tests)
2. **Bump** — `make bump v=X.Y.Z` updates manifests, commits, tags, pushes
3. **CI builds** — tag push triggers GitHub Actions: build release artifacts for all platforms, compute SHA256 hashes, update Homebrew formula
4. **Subtree sync** — `bump` automatically runs `tap-push` and `marketplace-push`
5. **Distribution** — users receive the update via `brew upgrade sensei` or the app's update check

### Pre-commit hook

`.githooks/pre-commit` runs `make test-fast` (bootstrap unit tests + app Vitest unit tests). No external dependencies required. Configured via `make setup-hooks`.

### Uninstall scope

Each binary only removes its own scope:
- `sensei remove all --purge` removes `~/.sensei/` and release binaries
- `sensei-dev remove all --purge` removes `~/.sensei-dev/` and dev binaries

Homebrew distributes release binaries only. Dev binaries are built locally by contributors.
