---
name: Bootstrap Post-Install Flow
description: Service startup, database creation, migration, and daemon launch — cross-platform
date: 2026-04-29
type: design
traces:
  - design/api/01-bootstrap.md
  - journeys/01-install-bootstrap.md
  - ../mockups/lib/bootstrap.jsx
---

# Bootstrap Post-Install Flow

After prerequisites are installed, the bootstrap screen runs three sequential phases to bring the system to a fully operational state. The phases are platform-agnostic — a platform provider trait supplies the OS-specific commands.

## Platform abstraction

The bootstrap pipeline is the same on every OS. Only the *how* changes.

### Platform provider trait (Rust)

```rust
pub trait PlatformProvider {
    fn detect_os(&self) -> Platform;               // macOS, Linux, Windows
    fn check_binary(&self, name: &str) -> Option<String>;  // which/where
    fn install_prerequisites(&self) -> Result<()>;  // brew bundle / winget / etc.
    fn start_service(&self, name: &str) -> Result<()>;
    fn stop_service(&self, name: &str) -> Result<()>;
    fn service_running(&self, name: &str, port: u16) -> bool;
    fn install_remedy(&self) -> InstallRemedy;      // commands to show the user
}
```

### Per-platform implementations

| Concern | macOS / Linux (Homebrew) | Windows (native) |
|---------|--------------------------|-------------------|
| Package manager | Homebrew | winget (built into Windows 11+) |
| Gate 一 | `which brew` | `where winget` (pre-installed) |
| Install prereqs | `brew bundle --file=-` | `winget install` per component |
| PostgreSQL | `brew install postgresql@17` | `winget install PostgreSQL.PostgreSQL.17` |
| Ollama | `brew install ollama` | `winget install Ollama.Ollama` |
| sensei CLI | `brew install sensei-hq/tap/sensei` | GitHub release binary → `%LOCALAPPDATA%\sensei\` |
| Service start | `brew services start <formula>` | `pg_ctl start` / Ollama tray app / `senseid serve` |
| Service mgmt | launchd (via brew services) | Windows Services or background process |
| Config path | `~/.sensei/` | `%APPDATA%\sensei\` |

### Implementation priority

**Ship now:** macOS (Homebrew provider) — the current implementation.
**Design now, build later:** Windows (winget provider). The platform trait ensures macOS code doesn't hardcode `brew` in places that Windows needs to reach.

### Where the abstraction lives

`sensei-bootstrap` crate:
- `src/platform/mod.rs` — trait definition + `detect()` → returns the right provider
- `src/platform/macos.rs` — Homebrew-based implementation (current code, refactored)
- `src/platform/windows.rs` — winget-based implementation (stub, future)

The existing `homebrew.rs` and `service.rs` modules become the internals of `macos.rs`. Public API calls go through the trait.

---

## Lifecycle

```
App opens
  │
  Phase 0 — Detection
  │   Detect platform (macOS / Windows)
  │   Check all 6 gates in parallel using sidecar
  │   All ready → skip to observatory (subsequent) or setup wizard (first run)
  │   If package manager missing → remedy card
  │     macOS: Homebrew install command
  │     Windows: winget ships with OS (edge case: older Windows 10)
  │   Tauri: poll for package manager, on success proceed to Phase 1
  │   Browser: show install command
  │
  Phase 1 — Install prerequisites (if prereq-type gates missing)
  │   UI: collapsed view — only package manager gate + remedy card
  │   macOS: spawn brew bundle, poll binary checks every 1s
  │   Windows: spawn winget install sequence, poll binary checks every 1s
  │   Browser: show platform-appropriate commands
  │   Exit condition: all prereq binaries found in PATH
  │
  Phase 2 — Service startup
  │   Sequential: PostgreSQL → Ollama → senseid
  │   Start via: platform provider start_service()
  │   Verify via: TCP port probe (5432, 11434, 7744)
  │   UI: gates expand, each flips checking → ready
  │   Timeout: 30s per service, then show error with manual command
  │
  Phase 3 — Database setup
  │   pg_isready?
  │   → createdb sensei (skip if exists)
  │   → CREATE EXTENSION IF NOT EXISTS vector
  │   → senseid migrate (via dbd-core, source: sensei-hq/sensei/daemon/database on GitHub)
  │   Success: database gate → ready
  │   Failure: show manual commands, wait for user retry
  │
  All ready → auto-advance
      First run → /setup/welcome
      Subsequent → /observatory
```

### Phase dependencies

Phases are strictly sequential — each depends on the previous:

- Phase 1 provides binaries (postgres, ollama, sensei, senseid)
- Phase 2 starts services that Phase 3 connects to
- Phase 3 creates the database that the daemon reads from

Within Phase 0 (detection), all checks run in parallel.

### Subsequent launches

Phase 0 detects everything is already running. All gates go green in <2s. Auto-advance. Phases 1-3 are skipped entirely.

---

## Gate mapping

| Gate | ID | Phase | Detection | Fix (macOS) | Fix (Windows) |
|------|----|-------|-----------|-------------|---------------|
| 一 Package manager | `pkgmgr` | Pre-install | `which brew` / `where winget` | Show install command | Pre-installed (winget) |
| 二 PostgreSQL | `postgres` | 1 → 2 | `which postgres` → port 5432 | Brew bundle | winget install |
| 三 Ollama | `ollama` | 1 → 2 | `which ollama` → port 11434 | Brew bundle | winget install |
| 四 Sensei | `sensei` | 1 | `which sensei` | Brew bundle | GitHub release |
| 五 Database | `database` | 3 | pg_isready + DB exists + schema version | createdb → extensions → migrate | Same |
| 六 Daemon | `daemon` | 2 | port 7744 + /health endpoint | brew services start | senseid serve |

Note: Gate 一 label changes per platform ("Homebrew" on macOS, "Package Manager" on Windows). Gate definitions become platform-aware via the provider.

---

## Phase 1: Install prerequisites

### Trigger

One or more prereq-type gates (postgres, ollama, sensei) are missing after Phase 0 detection.

### UI behavior

When `bs.needsPrereqInstall` is true:
- `bs.visibleGates` collapses to only the package manager gate
- A consolidated remedy card appears showing missing component tags and the install command

### Execution — macOS (Tauri)

1. Invoke `install_prerequisites` Tauri command
2. Sidecar spawns `brew bundle --file=-` with Brewfile content piped to stdin
3. Background thread polls `check_binary` for each component every 1s
4. Emits `bootstrap:status` events: `{ id, status, version, detail }`
5. When brew bundle exits or all binaries found, emits `bootstrap:install-complete`

### Execution — Windows (Tauri, future)

1. Invoke `install_prerequisites` Tauri command
2. Sidecar runs `winget install` for each missing component sequentially
3. Same polling and event pattern as macOS

### Execution (Browser)

Show the platform-appropriate command for the user to run manually.

macOS:
```sh
curl -fsSL https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile | brew bundle --file=-
```

Windows:
```powershell
winget install PostgreSQL.PostgreSQL.17
winget install Ollama.Ollama
# Download sensei CLI from GitHub releases
```

User clicks "Retry checks" when done.

### Brewfile contents (macOS)

```ruby
tap "sensei-hq/tap", "https://github.com/sensei-hq/homebrew-tap"
brew "postgresql@17"
brew "ollama"
brew "sensei-hq/tap/sensei"
cask "sensei-hq/tap/sensei-app"  # optional
```

`brew bundle` skips already-installed items, so it's safe to run regardless of current state.

### Exit condition

All prereq-type gates report `ready`. `bs.needsPrereqInstall` becomes false, `bs.visibleGates` expands to show all gates, and Phase 2 begins.

---

## Phase 2: Service startup

### Sequence

Services start sequentially to avoid resource contention:

1. **PostgreSQL** — start service → poll port 5432
2. **Ollama** — start service → poll port 11434
3. **Daemon** — start service → poll port 7744 + `/health`

### Platform-specific start commands

| Service | macOS | Windows |
|---------|-------|---------|
| PostgreSQL | `brew services start postgresql@17` | `pg_ctl -D "%PGDATA%" start` |
| Ollama | `brew services start ollama` | Start Ollama from install path |
| Daemon | `brew services start sensei-hq/tap/sensei` | `senseid serve --port 7744` |
| Daemon fallback | `senseid serve --port 7744` | Same |

### Port verification

TCP connect with 2s timeout, retried every 1s. Overall timeout: 30s per service.

### Failure handling

If a service doesn't respond within 30s:
- Gate shows `error` status with the error message
- Remedy panel shows the manual start command (platform-appropriate)
- User can retry

### Tauri implementation

A single `start_services` command that:
1. Gets the platform provider
2. Iterates the service list
3. Calls `provider.start_service()` for each
4. Polls the port
5. Emits `bootstrap:status` events as each service comes up

### Already-running services

If a port is already responding during Phase 0, the service is marked `ready` and skipped in Phase 2.

---

## Phase 3: Database setup

### Trigger

PostgreSQL is running (port 5432 responsive).

### Steps

1. **pg_isready** — Verify PostgreSQL is accepting connections
2. **Check database exists** — `psql -lqt` and look for `sensei`
3. **Create database** (if missing) — `createdb sensei`
4. **Create extensions** (if missing) — `psql sensei -c 'CREATE EXTENSION IF NOT EXISTS vector'`
5. **Run migrations** — `senseid migrate`

These steps are platform-agnostic — `psql`, `createdb`, and `senseid` are in PATH on all platforms after Phase 1.

### DATABASE_URL resolution

No configuration prompt during bootstrap. The default is sufficient:

| Priority | Source | Value |
|----------|--------|-------|
| 1 | CLI flag | `--database-url` |
| 2 | Environment | `SENSEI_DATABASE_URL` |
| 3 | Config file | `~/.sensei/config.json` → `database_url` |
| 4 | Default | `postgres://localhost/sensei` |

First install uses the default. Custom URLs are configured later in Settings.

### Migration execution

`senseid migrate` embeds `dbd-core` (Rust library) and applies schema + seed data:

- Source: `sensei-hq/sensei/daemon/database` on GitHub (fetched by dbd-core at runtime)
- dbd-core diffs current schema against desired state and applies changes
- Handles both fresh installs (full schema) and upgrades (incremental changes)
- Tracks applied state — idempotent, safe to run on every launch

No external `dbd` binary needed. No local `database/` folder needed.

### Failure handling

**createdb fails:**
- Tauri: show the error, display manual command, wait for user to confirm
- Browser: show the command from the start

Manual commands shown to user:

```sh
createdb sensei
psql sensei -c 'CREATE EXTENSION IF NOT EXISTS vector;'
```

**senseid migrate fails:**
- Show error output
- Suggest: check PostgreSQL logs, verify pgvector extension, retry

### Success

Database gate → `ready` with `schema-<version>` as the version string.

---

## State management

All UI state is driven by `BootstrapState`:

```typescript
class BootstrapState {
  // Core state
  statuses: Record<string, GateStatus>      // per-gate status
  installing: boolean                        // prereq install in progress
  platform: 'macos' | 'windows' | 'linux'   // detected platform

  // Derived (drive the template)
  get gates()                // gate definitions + current status (platform-aware labels)
  get visibleGates()         // collapsed when needsPrereqInstall, full otherwise
  get missingPrereqGates()   // prereq-type gates that are missing/error
  get needsPrereqInstall()   // true when prereq gates are missing
  get allReady()             // all gates ready → auto-advance
  get readyCount()           // for progress rail
  get isChecking()           // any gate in checking/starting state
}
```

Template renders `bs.visibleGates` — nothing else. Visibility logic lives entirely in the state layer.

### Event flow (Tauri)

```
Tauri backend                    Frontend
─────────────                    ────────
install_prerequisites()  →       bs.installing = true
  poll check_binary()
  emit bootstrap:status  →       bs.setGateStatus(id, status)
  emit install-complete  →       bs.installing = false

start_services()         →       bs.setGateStatus(id, 'checking')
  provider.start_service()
  poll port
  emit bootstrap:status  →       bs.setGateStatus(id, 'ready')

setup_database()         →       bs.setGateStatus('database', 'checking')
  createdb + migrate
  emit bootstrap:status  →       bs.setGateStatus('database', 'ready')
```

---

## Error states and recovery

| Failure | What the user sees | Recovery |
|---------|-------------------|----------|
| Package manager missing | Platform-appropriate install command | Manual install, then retry |
| Prereq install fails | Error message + manual install commands | Run commands, retry checks |
| Service won't start | Error + platform-appropriate start command | Run command, retry |
| Port timeout (30s) | "Not responding on port XXXX" | Check logs, retry |
| createdb fails | Error + `createdb sensei` command | Run command, confirm |
| pgvector missing | Error + `CREATE EXTENSION` command | Run command, retry |
| Migration fails | Error output from senseid migrate | Check logs, retry |

All errors are recoverable through retry. The UI never dead-ends.

---

## Tauri commands

In `src-tauri/src/commands/bootstrap.rs`:

| Command | Purpose |
|---------|---------|
| `run_bootstrap` | Full detection — check all gates |
| `install_prerequisites` | Platform-aware prereq install + poll status → events |
| `start_services` | Start PostgreSQL, Ollama, daemon sequentially → events |
| `setup_database` | createdb + extensions + migrate → events |
| `brew_bundle_install` | macOS-specific: spawn brew bundle (called by install_prerequisites) |

All commands that run phases emit `bootstrap:status` events with `{ id, status, version, detail }`.

---

## Implementation plan

### Phase A: macOS (ship now)

Current code already covers most of this:
- Detection (Phase 0): `check_binary`, `check` (port probe) — done
- Brew bundle (Phase 1): `brew_bundle_install` command — done
- Service startup (Phase 2): `start_brew_service` — exists, needs event wiring
- Database setup (Phase 3): `database::check`, `database::create` — exists, needs migrate step
- Frontend state: `BootstrapState` with `visibleGates`, `needsBrewInstall` — done

Remaining:
1. Wire `start_services` Tauri command with sequential start + events
2. Wire `setup_database` Tauri command with createdb + migrate + events
3. Add `senseid migrate` (depends on dbd-core)
4. Refactor `homebrew.rs` and `service.rs` behind platform trait (prep for Windows)

### Phase B: Windows (build later)

1. Implement `WindowsProvider` behind the platform trait
2. winget-based prereq install
3. Windows service management (pg_ctl, background processes)
4. GitHub release download for sensei CLI
5. Test with native Windows PostgreSQL installer
