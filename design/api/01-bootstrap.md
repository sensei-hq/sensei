---
name: "Screen 0: Bootstrap"
description: Dependency detection, installation, and daemon startup — runs every app launch
date: 2026-04-27
type: design
traces:
  - ideas/26-bootstrap-and-dependencies.md
  - design/api/00-api-surface-overview.md
---

# Screen 0: Bootstrap

## What the user sees

On every app launch, the bootstrap screen checks all dependencies. If everything is healthy (< 2s), it auto-advances to the observatory. If anything needs attention, it stays visible and guides the user through resolution.

### Component list

| Component | Check | Fix |
|-----------|-------|-----|
| Homebrew | `/opt/homebrew/bin/brew` exists | Show terminal command (requires sudo) |
| sensei formula | `brew list sensei` | `brew install sensei` |
| PostgreSQL | `brew list postgresql@17` + `pg_isready` | `brew install postgresql@17` + `brew services start` |
| Ollama | `brew list ollama` + HTTP check `:11434` | `brew install ollama` + `brew services start` |
| Database | `sensei` DB exists + schema version | `createdb sensei` + daemon runs migrations |
| Local models | `ollama list` for required models | `ollama pull <model>` (skippable) |
| Daemon | HTTP check `:9823/health` | `senseid serve` |

### Component states

Each component displays one of: `detecting`, `installing`, `pulling`, `starting`, `upgrading`, `ready`, `failed`, `skipped`.

### Flow

```
App opens
  │
  ├─ Daemon reachable? (:9823/health)
  │   ├─ Yes → daemon returns full health status → fast path
  │   └─ No  → sidecar mode (bootstrap crate via Tauri)
  │
  ├─ Phase 1: Homebrew
  ├─ Phase 2: Core components (sensei, postgresql, ollama)
  ├─ Phase 3: Database (exists, schema, pgvector)
  ├─ Phase 4: Local models (hardware-aware)
  ├─ Phase 5: Daemon startup
  │
  └─ All ready → advance to setup wizard (first run) or observatory (subsequent)
```

---

## Bootstrap crate API (`sensei-bootstrap`)

### Types

```rust
pub enum ComponentState {
    Detecting,
    Installing,
    Starting,
    Upgrading,
    Pulling { progress_pct: u8, size_mb: u32 },
    Ready,
    Failed { error: String },
    Skipped,
}

pub struct ComponentStatus {
    pub name: String,
    pub state: ComponentState,
    pub version: Option<String>,
    pub detail: Option<String>,
}

pub struct HardwareInfo {
    pub ram_gb: u32,
    pub cpu_cores: u32,
    pub gpu: Option<String>,       // "Apple M2 Pro", "NVIDIA RTX 4090", etc.
    pub metal_support: bool,
    pub recommended_tier: ModelTier,
}

pub enum ModelTier {
    Minimum,      // 8GB  — gemma3:12b
    Recommended,  // 16GB — gemma3:27b + qwen3:14b
    Full,         // 32GB — full MOE panel
}

pub struct BootstrapResult {
    pub components: Vec<ComponentStatus>,
    pub hardware: HardwareInfo,
    pub ready: bool,  // all non-skipped components are Ready
}
```

### Functions

```rust
// Phase 1
pub fn check_homebrew() -> ComponentStatus;

// Phase 2
pub fn check_formula(name: &str) -> ComponentStatus;
pub fn install_formula(name: &str) -> Result<ComponentStatus>;
pub fn upgrade_formula(name: &str) -> Result<ComponentStatus>;

// Phase 2 (services)
pub fn check_service(name: &str, port: u16) -> ComponentStatus;
pub fn start_service(name: &str) -> Result<ComponentStatus>;

// Phase 3
pub fn check_database(db_name: &str) -> ComponentStatus;
pub fn create_database(db_name: &str) -> Result<ComponentStatus>;
pub fn check_schema_version() -> ComponentStatus;

// Phase 4
pub fn detect_hardware() -> HardwareInfo;
pub fn list_models() -> Vec<String>;
pub fn pull_model(name: &str) -> Result<impl Stream<Item = PullProgress>>;

// Phase 5
pub fn start_daemon(port: u16) -> Result<ComponentStatus>;
pub fn check_daemon(port: u16) -> ComponentStatus;

// All-in-one
pub fn run_bootstrap() -> BootstrapResult;
```

---

## Tauri commands (app repo)

These wrap the bootstrap crate functions as Tauri invoke commands:

```rust
// src-tauri/src/commands/bootstrap.rs

#[tauri::command]
async fn check_homebrew() -> Result<ComponentStatus, String>;

#[tauri::command]
async fn check_all_components() -> Result<Vec<ComponentStatus>, String>;

#[tauri::command]
async fn install_component(name: String) -> Result<ComponentStatus, String>;

#[tauri::command]
async fn start_component(name: String) -> Result<ComponentStatus, String>;

#[tauri::command]
async fn detect_hardware() -> Result<HardwareInfo, String>;

#[tauri::command]
async fn pull_model(name: String) -> Result<(), String>;
// Model pull progress via Tauri event emission (not return value)

#[tauri::command]
async fn run_bootstrap() -> Result<BootstrapResult, String>;
```

---

## Daemon health endpoint (already exists, extend)

When the daemon IS running, the bootstrap screen uses it instead of the sidecar:

### `GET /health`

Current: returns basic status.

**Extended response:**

```json
{
  "status": "healthy",
  "version": "0.9.4",
  "uptime_seconds": 3421,
  "components": {
    "postgresql": { "state": "ready", "version": "17.2" },
    "ollama": { "state": "ready", "version": "0.6.2" },
    "database": { "state": "ready", "schema_version": 42 },
    "models": ["gemma3:27b", "qwen3:14b"]
  }
}
```

### `GET /api/health/components`

Current: returns component status.

**Align with bootstrap types:**

```json
{
  "data": [
    { "name": "homebrew", "state": "ready", "version": "4.4.2" },
    { "name": "sensei", "state": "ready", "version": "0.9.4" },
    { "name": "postgresql", "state": "ready", "version": "17.2" },
    { "name": "ollama", "state": "ready", "version": "0.6.2" },
    { "name": "database", "state": "ready", "version": "schema-42" },
    { "name": "daemon", "state": "ready", "version": "0.9.4" }
  ],
  "hardware": {
    "ram_gb": 32,
    "cpu_cores": 12,
    "gpu": "Apple M2 Pro",
    "metal_support": true,
    "recommended_tier": "full"
  },
  "ready": true
}
```

---

## SvelteKit frontend (app repo)

### Route: `/bootstrap` (or root gate)

The bootstrap screen is the app's entry gate. The layout checks health on mount:

1. Call `invoke('run_bootstrap')` (Tauri) or `fetch('/health')` (daemon)
2. If all ready → redirect to `/observatory` (or `/setup` if first run)
3. If issues → render bootstrap screen with component cards
4. Poll/re-check as user resolves issues

### Data flow

```
+layout.svelte (app gate)
  │
  ├─ fetch('http://localhost:9823/health')
  │   ├─ OK + all healthy → /observatory
  │   ├─ OK + issues → /bootstrap (show daemon-reported issues)
  │   └─ FAIL → invoke('run_bootstrap') via Tauri
  │       ├─ All ready → /observatory
  │       └─ Issues → /bootstrap (show sidecar-detected issues)
  │
  /bootstrap page
    ├─ Renders component cards with state indicators
    ├─ "Fix" buttons call invoke('install_component', { name })
    ├─ Model pull shows progress bar (Tauri event stream)
    └─ Auto-advances when all components reach ready/skipped
```

---

## Verification scenarios

### Scenario 1: Clean install (nothing installed)

1. App opens, daemon unreachable
2. Sidecar detects: no Homebrew
3. UI shows Homebrew instruction, polls for `/opt/homebrew/bin/brew`
4. User installs Homebrew, UI detects it
5. UI auto-runs: `brew install sensei postgresql@17 ollama`
6. UI starts services: postgresql, ollama
7. UI creates database, starts daemon
8. Daemon runs migrations
9. Hardware detection → recommend models → user pulls or skips
10. All ready → advance to setup wizard

### Scenario 2: Healthy system (daily launch)

1. App opens, `GET /health` succeeds, all components healthy
2. Bootstrap screen flashes briefly (or not at all) → observatory

### Scenario 3: Daemon crashed

1. App opens, `GET /health` fails
2. Sidecar checks: Homebrew OK, formulae OK, PostgreSQL running, Ollama running
3. Sidecar starts daemon: `senseid serve`
4. Polls `:9823/health` until responsive
5. Auto-advances to observatory

### Scenario 4: Version mismatch after app update

1. App opens, `GET /health` succeeds
2. Daemon version 0.9.2, app version 0.9.4
3. UI shows: "sensei is v0.9.2, desktop is v0.9.4 — [Upgrade] [Skip]"
4. User clicks Upgrade → `brew upgrade sensei` → restart daemon
5. Verify versions match → advance

### Scenario 5: Ollama model missing

1. App opens, daemon healthy, but `gemma3:27b` not in model list
2. Bootstrap shows Ollama as ready but models section shows "recommended model missing"
3. User clicks Pull → progress bar → completes
4. Or user clicks Skip → state becomes `skipped`, continues to observatory

---

## Issues to create

### Daemon repo: `sensei-hq/daemon`

**Issue 1: Create `bootstrap` crate**
- Add `crates/bootstrap/` with prereq check modules
- Rename `crates/sensei-cli/` → `crates/cli/`
- Rename `crates/sensei-mcp/` → `crates/mcp/`
- Update workspace Cargo.toml
- Wire `cli` to use bootstrap for `sensei doctor`
- Types: ComponentStatus, ComponentState, HardwareInfo, ModelTier, BootstrapResult
- Modules: homebrew, service, database, hardware, models
- Tests: mock each check with fixture responses

**Issue 2: Extend `/health` and `/api/health/components`**
- Return component-level status aligned with bootstrap types
- Include hardware info
- Include model list from Ollama
- Include schema version

### App repo: `sensei-hq/app`

**Issue 3: Bootstrap UI screen**
- Route: health gate in root layout
- Component cards with state indicators (detecting → ready animation)
- Fix/Install/Upgrade/Skip buttons per component
- Model pull progress via Tauri events
- Auto-advance on all-ready
- Two-mode: daemon health API (fast) vs Tauri sidecar (cold)

**Issue 4: Integrate bootstrap crate into Tauri**
- Add `sensei-bootstrap` as dependency in `src-tauri/Cargo.toml`
- Tauri commands: check_all_components, install_component, start_component, detect_hardware, pull_model, run_bootstrap
- Replace existing `check_indexer`/`start_indexer` with bootstrap equivalents
- Event emission for model pull progress
