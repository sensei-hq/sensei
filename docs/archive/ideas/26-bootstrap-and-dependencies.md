---
name: Bootstrap & Dependency Resolution
description: Startup screen that detects, installs, and resolves all dependencies (Homebrew, PostgreSQL, Ollama, sensei components) — runs every launch, not just first setup
date: 2026-04-23
status: idea
related: 24-desktop-observatory.md, 20-local-inference.md, 25-playground-and-insights.md
---

# Bootstrap & Dependency Resolution

## Problem

The current setup wizard (Step 2: Components) only handles three items: sensei-cli, MCP bridge, sensei-daemon. It runs once during first setup and never again. This misses:

- **System dependencies** — PostgreSQL and Ollama are prerequisites that the wizard doesn't install
- **Homebrew** — the canonical way to install everything on macOS, but users may not have it
- **Version drift** — desktop app v1.1 ships but CLI is still v1.0 from a previous brew install
- **Model availability** — local inference needs models pulled into Ollama, which depends on hardware
- **Database setup** — PostgreSQL needs a live database created and migrated
- **Startup health** — the app should verify everything is healthy on every launch, not just first run

## Solution: Bootstrap screen replaces Step 2

The Components step becomes a **bootstrap screen** that runs on every app launch (not just during setup). It detects, resolves, and reports the health of all dependencies.

First launch: full installation flow. Subsequent launches: fast health check (< 2 seconds if everything is healthy).

---

## Dependency tree

```
Homebrew (package manager)
├── sensei (formula — installs cli + daemon + mcp)
│   └── sensei-cli, senseid, sensei-mcp (Rust binaries)
├── postgresql@17 (database)
│   └── sensei database (created by senseid on first run)
└── ollama (local inference runtime)
    └── models (pulled based on hardware)
        ├── gemma3:12b (minimum — 8GB RAM)
        ├── gemma3:27b (recommended — 16GB RAM)
        └── qwen3:14b (MOE panel member — 16GB+ RAM)
```

### Brewfile

```ruby
# Sensei dependencies
brew "sensei"          # CLI + daemon + MCP server
brew "postgresql@17"   # database
brew "ollama"          # local inference runtime
```

This can be referenced from sensei-homebrew git repo. One shared place

---

## Bootstrap phases

The bootstrap screen runs through these phases in order. Each phase has a **detect → resolve → verify** cycle.

### Phase 1: Package manager

| Check | Pass | Fail action |
|-------|------|-------------|
| Homebrew installed | `/opt/homebrew/bin/brew` exists | Show install command, offer to run it. Homebrew install is interactive (requires password), so desktop shows the command and waits for user to confirm in terminal. |
| Homebrew up to date | `brew --version` ≥ minimum | `brew update` (can run automatically) |

**If no Homebrew:** The bootstrap screen shows:
```
Sensei uses Homebrew to manage its dependencies.
Run this in your terminal:

  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

Then restart Sensei.
```

The desktop cannot run the Homebrew installer directly (it needs sudo and terminal interaction). It polls for `/opt/homebrew/bin/brew` existence and auto-advances when detected.

### Phase 2: Core components

| Check | Pass | Fail action |
|-------|------|-------------|
| sensei formula installed | `brew list sensei` succeeds | `brew install sensei` |
| Version matches desktop | CLI version == desktop version | `brew upgrade sensei` |
| PostgreSQL installed | `brew list postgresql@17` succeeds | `brew install postgresql@17` |
| PostgreSQL running | `pg_isready` succeeds | `brew services start postgresql@17` |
| Ollama installed | `brew list ollama` succeeds | `brew install ollama` |
| Ollama running | `curl localhost:11434/api/version` | `brew services start ollama` |

**Automatic resolution:** The desktop can run `brew install` and `brew services start` commands directly (no sudo needed for Homebrew services). Progress shows per-component with animated phase transitions (detecting → installing → starting → ready).

**Version mismatch handling:**
```
sensei-cli is v0.9.2 but desktop is v0.9.4.

  [Run brew upgrade sensei]   [Skip (may cause issues)]
```

After upgrade, desktop prompts restart.

### Phase 3: Database

| Check | Pass | Fail action |
|-------|------|-------------|
| `sensei` database exists | `psql -lqt \| grep sensei` | `createdb sensei` |
| Schema is current | Migration version matches | `senseid migrate` (daemon runs migrations on start) |
| pgvector extension | `SELECT * FROM pg_extension WHERE extname = 'vector'` | `CREATE EXTENSION vector` |

**Database creation** is a one-time operation. The daemon handles schema migrations automatically on startup. The bootstrap screen just verifies the database is reachable and the schema is current.

### Phase 4: Local models

| Check | Pass | Fail action |
|-------|------|-------------|
| Hardware detection | Read system RAM + GPU info | Determine recommended model tier |
| Minimum model available | `ollama list \| grep gemma3:12b` | `ollama pull gemma3:12b` |
| Recommended models | Based on hardware tier | Offer to pull additional models |

**Hardware-aware recommendations:**

| RAM | GPU | Recommended | Optional (MOE panel) |
|-----|-----|-------------|---------------------|
| 8GB | None | gemma3:12b | — |
| 16GB | None | gemma3:27b | qwen3:14b |
| 16GB+ | Metal/CUDA | gemma3:27b | qwen3:14b + llama4-scout:17b |
| 32GB+ | Metal/CUDA | gemma3:27b | Full MOE panel (3 models) |

**Model pull is slow** (minutes to hours). The bootstrap screen:
1. Shows progress with download speed and ETA
2. Allows skipping ("Continue without local inference — can pull later from Settings")
3. Does not block the rest of setup

### Phase 5: Daemon startup

| Check | Pass | Fail action |
|-------|------|-------------|
| Daemon running | `curl localhost:9823/health` | Start daemon (`senseid serve`) |
| Daemon version matches | Version from health endpoint | Restart daemon with new binary |

---

## Startup flow (every launch)

```
App opens
    │
    ▼
Bootstrap screen (health check)
    │
    ├── All healthy? (< 2s check)
    │   └── Yes → Observatory (skip bootstrap screen)
    │
    ├── Minor issues? (version mismatch, daemon stopped)
    │   └── Show bootstrap screen, auto-resolve, advance
    │
    ├── Major issues? (PostgreSQL down, no Homebrew)
    │   └── Show bootstrap screen, guide user through resolution
    │
    └── First launch? (nothing installed)
        └── Full bootstrap flow → then continue to setup wizard step 3+
```

**Fast path:** On a healthy system, the bootstrap check takes < 2 seconds. The user sees a brief flash of the bootstrap screen (or nothing at all — direct to observatory) with green checkmarks.

**Slow path:** On first install or after issues, the bootstrap screen stays visible with animated progress per component.

---

## Bootstrap screen UI

```
┌──────────────────────────────────────────────────────┐
│  先  Sensei · startup                                 │
├──────────────────────────────────────────────────────┤
│                                                       │
│  二  Components                                       │
│      Checking dependencies. No input needed.          │
│                                                       │
│  ┌─────────────────────────────────────────────────┐ │
│  │  ⬡  Homebrew        4.4.2 · ready            ● │ │
│  ├─────────────────────────────────────────────────┤ │
│  │  $  sensei-cli      0.9.4 · ready             ● │ │
│  ├─────────────────────────────────────────────────┤ │
│  │  ◇  sensei-daemon   0.9.4 · starting…        ◉ │ │
│  │     ████████░░░░░░░░░░░░                        │ │
│  ├─────────────────────────────────────────────────┤ │
│  │  ⟷  MCP bridge      0.9.4 · ready             ● │ │
│  ├─────────────────────────────────────────────────┤ │
│  │  🐘 PostgreSQL      17.2 · ready               ● │ │
│  ├─────────────────────────────────────────────────┤ │
│  │  🦙 Ollama          0.6.2 · ready              ● │ │
│  │     gemma3:27b · qwen3:14b                      │ │
│  └─────────────────────────────────────────────────┘ │
│                                                       │
│  Nothing leaves localhost:9823.                       │
│                                                       │
└──────────────────────────────────────────────────────┘
```

**Component states:**

| State | Visual | Meaning |
|-------|--------|---------|
| detecting | Pulsing amber dot | Checking if installed/running |
| installing | Progress bar + "installing · 12.4 MB" | Brew install in progress |
| pulling | Progress bar + "pulling gemma3:27b · 4.2 GB" | Ollama model download |
| starting | Pulsing amber dot + "starting…" | Service starting up |
| upgrading | Progress bar + "upgrading 0.9.2 → 0.9.4" | Brew upgrade in progress |
| ready | Solid jade dot + version | Healthy |
| failed | Solid amber dot + error | Needs manual intervention |
| skipped | Grey dot + "skipped" | User chose to skip (e.g. Ollama) |

---

## External model providers (future)

Phase 1 relies on Ollama for local inference. Future phases:

### Phase 2: ollama.cpp (Rust-native)

Replace the Ollama HTTP dependency with a Rust-native inference runtime. This eliminates the Ollama process dependency and gives the daemon direct control over model loading, memory management, and scheduling.

```
senseid (daemon)
├── inference
│   ├── OllamaAdapter (Phase 1 — HTTP to localhost:11434)
│   ├── NativeAdapter (Phase 2 — ollama.cpp linked as Rust library)
│   └── ProxyAdapter (Phase 3 — API key to external providers)
└── model_manager
    ├── pull / delete / list models
    ├── hardware detection + model recommendations
    └── GGUF model loading
```

### Phase 3: External provider proxy

For users who want to use their own API keys for more powerful models:

```
sensei settings → inference → providers
├── Local models (Ollama / native)     [default, always available]
├── Anthropic (Claude)                 [API key required]
├── OpenAI                             [API key required]
└── Custom OpenAI-compatible endpoint  [URL + API key]
```

The inference adapter routes tasks to the appropriate provider based on:
- **Task complexity** — simple classification → local model; nuanced reasoning → external provider
- **Cost** — local is free; external has per-token cost
- **Availability** — if Ollama is down, fall back to external (if configured)
- **User preference** — "always local" / "always external" / "auto"

This enables the MOE consensus panel to mix local and external models:
```
Reasoning panel:
  Model A: gemma3:27b (local)         — proposes root cause
  Model B: qwen3:14b (local)          — challenges/refines
  Model C: claude-haiku (external)    — synthesizes action
```

---

## Configuration (stored in `services` table)

The Ollama instance and any external providers are registered in the `services` table:

```json
// services row for Ollama
{
  "name": "ollama-local",
  "protocol": "ollama",
  "kind": "inference",
  "config": {
    "url": "http://localhost:11434",
    "models": ["gemma3:27b", "qwen3:14b"],
    "default_model": "gemma3:27b"
  },
  "installed": true
}

// services row for external provider (future)
{
  "name": "anthropic",
  "protocol": "anthropic",
  "kind": "inference",
  "config": {
    "api_key_env": "ANTHROPIC_API_KEY",
    "default_model": "claude-haiku-4-5"
  },
  "installed": true
}
```

The `services` table already supports arbitrary `protocol` and `kind` values. Adding `ollama` and `anthropic` protocols + `inference` kind extends it naturally.

---

## Brewfile management

### For desktop users

The Brewfile is bundled inside the app. On first launch:
1. Desktop checks for Homebrew
2. If present, runs `brew bundle --file=<bundled Brewfile>`
3. This installs all missing dependencies in one shot

### For CLI-only users

The Brewfile lives in the repo root. Users run:
```bash
brew bundle          # installs sensei + postgresql + ollama
sensei init          # set up a project
```

### Version pinning

The Brewfile doesn't pin versions — Homebrew handles that via its formula versions. The desktop app checks version compatibility on startup and prompts upgrade if needed.

---

## Data model additions

### `services` table update

Add `ollama` and `anthropic` (future) to the protocol check constraint:

```sql
check (protocol in ('mcp', 'ollama', 'anthropic', 'openai'))
```

Add `inference` to the `service_kind` enum:

```sql
create type service_kind as enum ('data', 'api', 'devtool', 'service', 'inference');
```

### `system_health` (new, lightweight)

Captures the latest health check result for each component. Used by the bootstrap screen and the observatory footer ("daemon · running · last heartbeat 2s ago").

```
system_health
├── component          text PK        -- homebrew, sensei, postgresql, ollama, daemon
├── status             text           -- healthy | degraded | missing | failed
├── version            text
├── detail             jsonb          -- {models:[], db_size, uptime, ...}
├── checked_at         timestamptz
```

---

## Tauri sidecar for app-only installs

### Problem

The bootstrap flow above assumes the user installed sensei via Homebrew (`brew install sensei`), which gives them the CLI, daemon, and MCP bridge as standalone binaries. But there's a second distribution path: **downloading the desktop app directly from the sensei website**.

In this case:
- The user has the `.app` bundle but no `senseid`, `sensei`, or `sensei-mcp` in PATH
- Homebrew may or may not be installed
- The daemon API is unreachable — there's nothing to `curl localhost:9823/health`
- The health page currently shows "daemon not reachable, retry" with no way forward

The desktop app needs to be self-sufficient for health checks and user guidance even when the daemon doesn't exist.

### Solution: bundled Tauri sidecar

Bundle a lightweight binary as a [Tauri sidecar](https://v2.tauri.app/develop/sidecar/) inside the `.app`. This sidecar handles bootstrap checks that don't require the daemon:

```
sensei-desktop.app/
└── Contents/
    └── Resources/
        └── sidecar/
            └── sensei-bootstrap   ← Rust binary, ships with the app
```

**What the sidecar does (daemon absent):**
- Detect Homebrew presence (`/opt/homebrew/bin/brew`)
- Check installed formulae (`brew list --versions sensei postgresql ollama`)
- Detect system hardware (RAM, GPU type, core count) for model recommendations
- Check PostgreSQL reachability (`pg_isready`)
- Check Ollama reachability (`curl localhost:11434/api/version`)
- Run `brew install` / `brew services start` commands on user's behalf
- Report component versions and status back to the UI

**What the sidecar does NOT do:**
- Run SQL queries (that's the daemon's job)
- Index code
- Serve MCP tools
- Anything that requires the database

### Two-mode bootstrap

```
App opens
    │
    ├── Try daemon API (localhost:9823/health)
    │   ├── Responds → use daemon for all health checks (fast path)
    │   └── No response → fall back to sidecar
    │
    └── Sidecar mode
        ├── Check Homebrew → guide install if missing
        ├── Check brew formulae → `brew install sensei` if missing
        ├── Check PostgreSQL → `brew services start` if stopped
        ├── Check Ollama → `brew services start` if stopped
        ├── Start daemon → `senseid serve`
        ├── Wait for daemon health endpoint
        └── Hand off to daemon for remaining checks (DB, models, schema)
```

Once the daemon is running, the sidecar's job is done. All subsequent health checks (including future app launches where daemon is already running) use the daemon API directly. The sidecar is the bootstrap-the-bootstrapper.

### Sidecar scope

The sidecar should be minimal — a Rust binary compiled alongside the desktop app, no database dependencies, no external crates beyond `sysinfo` for hardware detection and `std::process::Command` for running brew/ollama commands.

```rust
// sensei-bootstrap capabilities
pub fn check_homebrew() -> ComponentStatus;
pub fn check_formula(name: &str) -> ComponentStatus;
pub fn check_service(name: &str, port: u16) -> ComponentStatus;
pub fn detect_hardware() -> HardwareInfo;
pub fn install_formula(name: &str) -> Result<()>;
pub fn start_service(name: &str) -> Result<()>;
pub fn start_daemon(port: u16) -> Result<()>;
```

### Tauri configuration

```json
// tauri.conf.json
{
  "bundle": {
    "externalBin": ["sidecar/sensei-bootstrap"]
  }
}
```

The sidecar communicates with the frontend via Tauri's `invoke` mechanism (same as the existing `check_indexer` and `start_indexer` commands in `src-tauri/src/lib.rs`). These existing commands are a partial implementation of this pattern — they already check TCP connectivity and start the daemon. The sidecar extends this to cover the full dependency tree.

### Relationship to existing code

`src/lib/setup/daemon.ts` already has the shared health check logic (`checkComponents`, `getInitialComponents`) with Tauri detection and fallback. The sidecar would be the Rust backing for the Tauri invoke path, replacing the current `run_command` invocations with dedicated, typed commands:

| Current (daemon.ts via Tauri) | Proposed (sidecar) |
|------|------|
| `invoke('run_command', { program: 'brew', args: ['list', 'sensei'] })` | `invoke('check_formula', { name: 'sensei' })` |
| `invoke('run_command', { program: 'which', args: ['senseid'] })` | `invoke('check_homebrew')` + `invoke('check_formula', { name: 'sensei' })` |
| `invoke('check_indexer', { port })` | `invoke('check_service', { name: 'daemon', port })` |
| `invoke('start_indexer', { port })` | `invoke('start_daemon', { port })` |

### Upgrade flow integration

The sidecar also supports the upgrade flow. When the desktop app updates (auto-update or manual download), the sidecar's version matches the desktop. On launch:

1. Sidecar checks installed `sensei` formula version
2. If formula version < desktop version, prompt `brew upgrade sensei`
3. After upgrade, start daemon with new binary
4. Daemon checks `assistants.configured_version` and re-pushes extensions to stale assistants

This ensures the full chain: desktop app → sidecar → brew formula → daemon → assistants all stay in sync.

---

## Open questions

| # | Question |
|---|----------|
| 1 | Should the desktop manage Ollama model lifecycle (pull/delete) or just detect what's available? |
| 2 | Homebrew install requires terminal interaction (sudo). Should the desktop open a terminal window, or show copy-paste instructions? |
| 3 | How do we handle Linux? Homebrew works on Linux but is less common. Should we support apt/dnf as well? |
| 4 | Should the Brewfile include PostgreSQL, or should we support an existing PostgreSQL installation (e.g. Postgres.app)? |
| 5 | For the Rust-native inference runtime (Phase 2), which library? llama.cpp bindings, candle, mistral.rs? |
| 6 | Should the fast-path health check (< 2s) be truly invisible, or always show a brief branded splash? |
| 7 | Version compatibility matrix: how strict? Require exact match between desktop and CLI, or allow minor version drift? |
| 8 | Should the sidecar binary be a separate Rust crate, or compiled into the Tauri app binary as additional commands? The latter is simpler (no separate process) but couples bootstrap logic to the desktop build. |
| 9 | For app-only installs on systems without Homebrew, should the sidecar offer to install Homebrew automatically, or only show copy-paste instructions? |
