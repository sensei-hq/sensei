---
title: Debug vs Release Build — System-wide Specification
description: How dev and release builds coexist across all sensei components
date: 2026-05-07
status: implemented
---

# Debug vs Release Build — System-wide Specification

## Why this matters

Developing sensei means running modified daemon code while also keeping the stable release daemon running for production use. Without explicit separation, a dev build would overwrite the release binary, connect to the production database, and corrupt production data. Users on the other end would see a broken `senseid`.

This spec documents the contracts that keep dev and release completely isolated.

---

## The Contract: Binary Name as Mode Determinant

**Mode is determined by binary name, not a runtime flag.**

- Binaries ending in `-dev` → dev mode (port 7745, `~/.sensei-dev/`, database `sensei_dev`)
- Binaries without suffix → release mode (port 7744, `~/.sensei/`, database `sensei`)

There is no `--mode dev` flag. A flag would:
- Be invisible to `brew services` and launchd (they invoke the binary directly)
- Need to be passed by every caller (Homebrew service, Tauri sidecar, the CLI, systemd)
- Be easy to accidentally omit, putting a dev binary in production mode

Binary name is set at install time and never changes at runtime — it's a compile-time / deploy-time decision.

**Exception (CI only):** `SENSEI_MODE=dev` env var overrides binary-name detection. This exists solely for CI environments where running a separate binary per mode is inconvenient. It is not a user-facing feature.

---

## Mode Detection by Component

### Daemon (`senseid` / `senseid-dev`)

```rust
// crates/senseid/src/main.rs
let is_dev_binary = std::env::current_exe()
    .ok()
    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    .map(|name| name.ends_with("-dev"))
    .unwrap_or(false);

if is_dev_binary {
    paths::set_mode(paths::Mode::Dev);
}
paths::init_from_env(); // SENSEI_MODE env var override for CI
```

`OnceLock<Mode>` ensures mode is set exactly once at startup. All subsequent path and port lookups read from this lock.

### CLI (`sensei` / `sensei-dev`)

```rust
// crates/cli/src/main.rs
fn is_dev() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .map(|name| name.ends_with("-dev"))
        .unwrap_or(false)
}
fn daemon_url() -> &'static str { if is_dev() { "http://127.0.0.1:7745" } else { "http://127.0.0.1:7744" } }
fn default_port() -> u16        { if is_dev() { 7745 } else { 7744 } }
fn daemon_bin() -> PathBuf      { if is_dev() { "senseid-dev" } else { "senseid" } }
```

`sensei-dev start` → spawns `senseid-dev` on port 7745.
`sensei start` → spawns `senseid` on port 7744.
`sensei-dev remove all --purge` → cleans `~/.sensei-dev/` and `-dev` binaries.

### Desktop app (Tauri + SvelteKit)

The Tauri app uses `cfg!(debug_assertions)` — a Rust compile-time flag — to select the port:

```rust
// app/src-tauri/src/commands/assistants.rs
fn daemon_url() -> &'static str {
    if cfg!(debug_assertions) { "http://127.0.0.1:7745" } else { "http://127.0.0.1:7744" }
}
```

`bunx tauri dev` → debug build → `cfg!(debug_assertions) = true` → port 7745 (dev daemon).
`bunx tauri build` → release build → `cfg!(debug_assertions) = false` → port 7744 (release daemon).

This applies to all Tauri commands (assistants, bootstrap health checks, session APIs). SvelteKit frontend uses `import.meta.env.DEV` for Vite-level debug checks (HMR, devtools).

### MCP server (`sensei-mcp` / `sensei-mcp-dev`)

The MCP server connects to the daemon. Same binary-name rule applies:

```rust
// crates/mcp/src/main.rs  (planned, same pattern)
fn daemon_url() -> &'static str {
    if std::env::current_exe().is_dev_binary() { "http://127.0.0.1:7745" } else { "http://127.0.0.1:7744" }
}
```

---

## Port Allocation

| Mode | Daemon port | Directory | Database |
|------|-------------|-----------|----------|
| Release | 7744 | `~/.sensei/` | `sensei` |
| Dev | 7745 | `~/.sensei-dev/` | `sensei_dev` |

The ports are fixed. There is no dynamic port selection. Homebrew service definitions, Tauri commands, hook scripts, and the CLI all hardcode these two values.

Hook scripts fan-out to both ports simultaneously when the dev daemon is active (see `docs/ideas/hooks.md` for caching details).

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

# Run dev daemon from build dir (no install required)
# SENSEI_MODE=dev is required here because the binary is named 'senseid' (not 'senseid-dev')
make daemon-dev   # → SENSEI_MODE=dev target/debug/senseid start
```

`make daemon-dev` uses the `SENSEI_MODE=dev` env var override because `cargo build` always produces `target/debug/senseid` (without the `-dev` suffix). The installed binaries get the `-dev` suffix during the `cp` step in `make install-dev`.

---

## Database

### Schema separation

| Database | Owner | Used by |
|----------|-------|---------|
| `sensei` | production data | Release daemon, release CLI, release Tauri app |
| `sensei_dev` | development data | Dev daemon, dev CLI, Tauri dev build |

`DATABASE_URL` in `.env.dev` points to `sensei_dev`. The release daemon reads `DATABASE_URL` from the environment or defaults based on mode.

### DDL management

DDL lives in `database/ddl/`. Both databases share the same schema definition — they just hold different data. `dbd apply` is run against each database separately:

```bash
dbd apply -d postgresql://localhost/sensei      # production schema
dbd apply -d postgresql://localhost/sensei_dev  # dev schema
```

The `DATABASE_URL` in `.env.dev` automatically targets `sensei_dev` for local dev. Staging tables and import procedures are also applied to both databases.

---

## Homebrew Distribution (Release Only)

Homebrew distributes release binaries only. The formula installs:
- `senseid` (daemon)
- `sensei` (CLI)
- `sensei-mcp` (MCP server)

No `-dev` binaries are distributed via Homebrew. The Homebrew service definition runs `senseid` (release) on port 7744.

Dev binaries are built locally by contributors using `make install-dev`. They are never packaged for distribution.

---

## Uninstall Scope

`sensei remove all --purge` (release binary) removes:
- `~/.sensei/` directory
- Release binaries: `senseid`, `sensei`, `sensei-mcp` from install locations

`sensei-dev remove all --purge` (dev binary) removes:
- `~/.sensei-dev/` directory
- Dev binaries: `senseid-dev`, `sensei-dev`, `sensei-mcp-dev` from install locations

Each binary only removes its own scope. Running `sensei-dev remove all --purge` cannot affect the production installation and vice versa.

---

## Decision Log

**Why binary name and not `--mode` flag?**
Flags require every caller to pass them. Homebrew service definitions run `senseid` directly — adding a flag would require modifying the formula for dev use. Binary name is ambient: it's set at deployment and requires zero cooperation from callers.

**Why not a config file instead?**
A config file in `~/.sensei/config.toml` would work but adds another piece to install/configure. Binary name is self-contained and inspectable with `ls ~/.local/bin/`.

**Why not compile-time `#[cfg(feature = "dev")]`?**
Feature flags change the binary at compile time but require a recompile to switch modes. Binary name allows the same build artifact to be installed under either name (though we do build both separately for clarity).

**Why hardcoded ports (7744/7745) and not dynamic?**
Hook scripts, Tauri apps, and the CLI all need to know the port without negotiation. Dynamic port selection would require a discovery mechanism (PID file, environment variable, registry). Hardcoding two well-known ports is simpler and less error-prone.

**Why `cfg!(debug_assertions)` in Tauri and not binary name?**
Tauri compiles to a single binary (`sensei-desktop`). There's no dev vs release binary name difference — it's always `sensei-desktop`. `cfg!(debug_assertions)` is the Rust-idiomatic way to distinguish Tauri debug builds from release builds.
