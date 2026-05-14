---
title: Debug vs Release Build — System-wide Specification
description: How dev and release builds coexist across all sensei components
date: 2026-05-12
status: implemented
---

# Debug vs Release Build — System-wide Specification

## Why this matters

Developing sensei means running modified daemon code while also keeping the stable release daemon running for production use. Without explicit separation, a dev build would overwrite the release binary, connect to the production database, and corrupt production data.

This spec documents the contracts that keep dev and release completely isolated.

---

## The Contract: Cargo Feature Flag as Mode Determinant

**Mode is determined at compile time by the `dev` Cargo feature. No runtime detection.**

- `cargo build --features dev` → dev mode (port 7745, `~/.sensei-dev/`, database `sensei_dev`)
- `cargo build` → release mode (port 7744, `~/.sensei/`, database `sensei`)

There is no `--mode` flag, no `SENSEI_MODE` env var, no binary name detection. The binary knows what it is because the decision was made at compile time.

---

## Compile-time Configuration

All mode-sensitive values live in `sensei-bootstrap/src/config.rs`:

```rust
#[cfg(feature = "dev")]
pub const COMPILE_DEV: bool = true;
#[cfg(not(feature = "dev"))]
pub const COMPILE_DEV: bool = false;

const DAEMON_PORT: u16 = if COMPILE_DEV { 7745 } else { 7744 };
```

`SenseiConfig::from_env()` reads these constants:

```rust
pub fn from_env() -> Self {
    let mode = if COMPILE_DEV { SenseiMode::Dev } else { SenseiMode::Prod };
    let daemon_port = DAEMON_PORT;
    let db_name = if COMPILE_DEV { "sensei_dev" } else { "sensei" };
    let db_url = format!("postgresql://localhost:5432/{db_name}");
    let dir_suffix = if COMPILE_DEV { ".sensei-dev" } else { ".sensei" };
    ...
}
```

### Feature propagation

Every crate that depends on `sensei-bootstrap` forwards the feature:

```toml
# crates/senseid/Cargo.toml
[features]
dev = ["sensei-bootstrap/dev"]
```

The Makefile passes `--features dev` for dev builds:

```makefile
crates-dev:
    cargo build --features dev -p senseid -p sensei-cli -p sensei-mcp

crates-release:
    cargo build --release -p senseid -p sensei-cli -p sensei-mcp
```

---

## Port Allocation

| Mode | Daemon port | Directory | Database |
|------|-------------|-----------|----------|
| Release | 7744 | `~/.sensei/` | `sensei` |
| Dev | 7745 | `~/.sensei-dev/` | `sensei_dev` |

The ports are fixed. There is no dynamic port selection.

---

## Binary Naming Convention

### Installed binaries

| Release binary | Dev binary | Purpose |
|---------------|-----------|---------|
| `senseid` | `senseid-dev` | Daemon |
| `sensei` | `sensei-dev` | CLI |
| `sensei-mcp` | `sensei-mcp-dev` | MCP server |

Dev binaries are installed to `~/.local/bin/` alongside release binaries. They coexist without conflict because they have different names.

### Build targets

```makefile
# Release binaries → ~/.local/bin/senseid, sensei, sensei-mcp
make install-release

# Dev binaries → ~/.local/bin/senseid-dev, sensei-dev, sensei-mcp-dev
make install-dev

# Run dev daemon from build dir (mode baked in by --features dev)
make daemon-dev
```

---

## Desktop App (Tauri + SvelteKit)

### Rust sidecar

The Tauri sidecar forwards the `dev` feature:

```toml
# app/src-tauri/Cargo.toml
[features]
dev = ["sensei-bootstrap/dev"]
```

`make app-dev` and `make app-dev-bundle` pass `--features dev`.
`make app-release` builds without the feature (prod).

### Frontend (SvelteKit)

`vite.config.ts` injects the daemon port at build time:

```typescript
const daemonPort = process.env.TAURI_ENV_DEBUG || process.env.NODE_ENV !== 'production' ? 7745 : 7744;

define: {
    __SENSEI_DEFAULT_PORT__: JSON.stringify(daemonPort),
}
```

`appState.port` defaults to this build-time value.

---

## Database

### Schema separation

| Database | Used by |
|----------|---------|
| `sensei` | Release daemon, release CLI, release Tauri app |
| `sensei_dev` | Dev daemon, dev CLI, Tauri dev build |

Both databases share the same schema definition — they just hold different data.

### Schema deployment

Bootstrap deploys schema on daemon startup. In dev builds, if `SENSEI_DB_SCHEMA_PATH` is set (Tauri sidecar sets it), the local `database/` directory is used. Otherwise, schema is downloaded from GitHub at the matching release tag.

For local dev without the Tauri sidecar, apply schema manually:

```bash
DATABASE_URL="postgresql://localhost/sensei_dev" dbd apply
```

---

## Homebrew Distribution (Release Only)

Homebrew distributes release binaries only. No `-dev` binaries are packaged for distribution. Dev binaries are built locally by contributors using `make install-dev`.

---

## Uninstall Scope

`sensei remove all --purge` (release binary) removes `~/.sensei/` and release binaries.
`sensei-dev remove all --purge` (dev binary) removes `~/.sensei-dev/` and dev binaries.

Each binary only removes its own scope.

---

## Decision Log

**Why compile-time Cargo feature and not runtime detection?**
Runtime detection (env vars, binary name checks) is fragile — env vars can be unset, binary names can be wrong. Compile-time guarantees the binary knows what it is. `make install-dev` builds with `--features dev`; the result is always a dev binary regardless of what it's named.

**Why hardcoded ports (7744/7745) and not dynamic?**
Hook scripts, Tauri apps, and the CLI all need to know the port without negotiation. Dynamic port selection would require a discovery mechanism. Two well-known ports is simpler.
