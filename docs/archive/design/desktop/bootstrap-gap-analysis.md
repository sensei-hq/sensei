---
name: Bootstrap Gap Analysis
description: Existing code vs design — what stays, what gets rewritten, what gets deleted
date: 2026-04-29
type: analysis
traces:
  - design/02-desktop/bootstrap-post-install.md
---

# Bootstrap Gap Analysis

Audit of existing code against the post-install design. The design is the spec. Code that doesn't match gets rewritten.

## Legend

- **KEEP** — aligns with design, no changes needed
- **REFACTOR** — right idea, wrong shape — needs restructuring
- **REWRITE** — exists but doesn't match design, replace entirely
- **DELETE** — no longer needed, remove
- **NEW** — doesn't exist yet, must be created

---

## Rust: `daemon/crates/bootstrap/src/`

### `types.rs` — KEEP

Types are solid and match the design exactly:
- `ComponentState` enum with all required states
- `ComponentStatus` with name/state/version/detail
- `HardwareInfo` with model tier
- `BootstrapResult` with from_checks

No changes needed.

### `homebrew.rs` — REFACTOR → `platform/macos.rs`

**What's here:** Homebrew-specific detection (`check`, `check_formula`, `check_binary`, `install_formula`, `upgrade_formula`) plus private helpers (`brew_path`, `which_binary`, `binary_version`).

**What the design says:** This becomes the internals of a macOS platform provider. The public API should go through a `PlatformProvider` trait.

**Action:**
- Move to `platform/macos.rs`
- `check()` → `MacOSProvider::check_package_manager()`
- `check_binary()` → stays as utility (used by all platforms)
- `check_formula()` → internal to macOS provider (not used in the design's detection flow — we use `check_binary` now)
- `install_formula()` / `upgrade_formula()` → internal to macOS provider
- `brew_path()` / `brew_path_pub()` → internal to macOS provider
- `which_binary()` / `binary_version()` → move to shared utility (cross-platform)

### `service.rs` — REFACTOR → `platform/macos.rs` (partial)

**What's here:** Port probes (`check`, `probe_port`), brew service management (`start_brew_service`), daemon start (`start_daemon`), version fetching (`fetch_version`), `which()`.

**What the design says:** Port probing and version fetching are platform-agnostic. Service start/stop goes through the platform provider.

**Action:**
- `probe_port()` → shared utility (used by all platforms)
- `check()` (port-based) → shared utility
- `fetch_version()` → shared utility
- `start_brew_service()` → `MacOSProvider::start_service()`
- `start_daemon()` → shared (direct binary start works on all platforms)
- `which()` → consolidate with `which_binary()` from homebrew.rs

### `database.rs` — REFACTOR

**What's here:** `check()` (pg_isready + DB exists + pgvector + schema version), `create()` (createdb).

**What the design says:** Same steps, but Phase 3 also needs: CREATE EXTENSION vector, and `senseid migrate`. The check/create logic is correct but incomplete.

**Action:**
- `check()` → KEEP as-is
- `create()` → KEEP as-is
- **NEW**: `ensure_extensions()` — `CREATE EXTENSION IF NOT EXISTS vector`
- **NEW**: `migrate()` — calls `senseid migrate` (or embeds dbd-core when available)
- **NEW**: `setup()` — orchestrates: check → create → ensure_extensions → migrate (the Phase 3 pipeline)

### `hardware.rs` — KEEP

Detects RAM, CPU, GPU, Metal support, model tier. Platform-aware (macOS sysctl, Linux lspci). Matches design.

### `models.rs` — KEEP

Lists models, checks missing, returns required per tier. Matches design.

### `lib.rs` — REWRITE

**What's here:** `run()` calls `homebrew::check()`, `homebrew::check_binary()` x3, `service::check()` x3, `database::check()` in a flat list.

**What the design says:** Detection should go through the platform provider. The `run()` function should use `PlatformProvider::check_binary()` and shared port probes.

**Action:**
- Rewrite `run()` to:
  1. Detect platform
  2. Get provider
  3. Call provider.check_package_manager()
  4. Call shared check_binary() for postgres, ollama, sensei
  5. Call shared port probes for services
  6. Call database::check()
- Add `pub mod platform;` module

### NEW: `platform/mod.rs`

Platform trait definition + detection:

```rust
pub trait PlatformProvider {
    fn detect_os(&self) -> Platform;
    fn check_package_manager(&self) -> ComponentStatus;
    fn install_prerequisites(&self) -> Result<(), String>;
    fn start_service(&self, name: &str) -> Result<ComponentStatus, String>;
    fn stop_service(&self, name: &str) -> Result<(), String>;
    fn install_remedy(&self) -> InstallRemedy;
}

pub fn detect() -> Box<dyn PlatformProvider>;
```

### NEW: `platform/macos.rs`

MacOS provider using Homebrew. Consumes current `homebrew.rs` and `service.rs` internals.

### NEW: `platform/windows.rs`

Stub for future Windows support. Returns "not yet supported" errors.

### NEW: `shared.rs` (or keep utilities inline)

Cross-platform utilities:
- `which_binary()` / `binary_version()` — binary detection
- `probe_port()` — TCP port check
- `fetch_version()` — service version from health endpoints

---

## Tauri: `app/src-tauri/src/commands/bootstrap.rs`

### Current commands — REFACTOR

| Command | Status | Action |
|---------|--------|--------|
| `run_bootstrap` | Exists | KEEP — delegates to bootstrap::run() |
| `check_all_components` | Exists | DELETE — redundant with run_bootstrap |
| `install_component` | Exists | DELETE — replaced by install_prerequisites |
| `start_component` | Exists | DELETE — replaced by start_services |
| `detect_hardware` | Exists | KEEP |
| `list_models` | Exists | KEEP |
| `missing_models` | Exists | KEEP |
| `create_database` | Exists | REFACTOR → part of setup_database |
| `brew_bundle_install` | Exists | REFACTOR → called by install_prerequisites |

### NEW commands (from design)

| Command | Purpose |
|---------|---------|
| `install_prerequisites` | Platform-aware prereq install (calls brew_bundle on macOS) + poll + events |
| `start_services` | Sequential service start (PostgreSQL → Ollama → daemon) + poll + events |
| `setup_database` | createdb + extensions + migrate + events |

### Hardcoded Brewfile in `brew_bundle_install`

The current `brew_bundle_install` has the Brewfile content hardcoded as a Rust string. This is fine for now (macOS only), but should move into the macOS platform provider.

---

## Frontend: `app/src/lib/`

### `bootstrap-gates.ts` — REFACTOR

**What's here:** Static GATES array with hardcoded gate definitions.

**What the design says:** Gate 一 should be platform-aware ("Homebrew" on macOS, "Package Manager" on Windows). Remedy type should be `prereq` not `brew`.

**Action:**
- Rename remedy type: `'brew'` → `'prereq'` (semantic — it's not always brew)
- Gate 一 ID: `'homebrew'` → `'pkgmgr'` (or keep `homebrew` and let state override the label)
- Add `platform` field to gate definitions, or make gates a function of platform
- Sub-checks on sensei gate: KEEP

**Decision needed:** Do we rename the gate ID now (breaking change in state) or keep `homebrew` and override display label from platform? Recommend: keep ID as `homebrew` for now, override display name from platform info. Rename is a Phase B concern.

### `bootstrap-state.svelte.ts` — REFACTOR

**What's here:** `BootstrapState` class with statuses, derived getters, mutations.

**What the design says:** Add `platform` state. Rename brew-specific getters to prereq-generic.

**Action:**
- Add: `platform = $state<'macos' | 'windows' | 'linux'>('macos')`
- Rename: `missingBrewGates` → `missingPrereqGates`
- Rename: `needsBrewInstall` → `needsPrereqInstall`
- Gate 一 display name derived from platform
- The rest (visibleGates, allReady, readyCount, etc.) — KEEP logic, update references

### `bootstrap.ts` — REFACTOR

**What's here:** Types, Tauri invoke wrappers, event listeners.

**What the design says:** Types are correct. API functions need to match new Tauri commands.

**Action:**
- Types: KEEP
- DELETE: `installComponent()`, `startComponent()` — replaced by new commands
- KEEP: `runBootstrap()`, `detectHardware()`, `listModels()`, `missingModels()`, `createDatabase()`, `hasTauri()`, `stateLabel()`, `isReady()`, `isFailed()`, `errorMessage()`
- RENAME: `brewBundleInstall()` → `installPrerequisites()`
- NEW: `startServices()` — invokes start_services Tauri command
- NEW: `setupDatabase()` — invokes setup_database Tauri command
- REFACTOR: `listenBootstrapEvents()` — KEEP shape, works with all three phase commands

### `bootstrap.spec.ts` — KEEP + EXTEND

Tests are pure function tests on types. All valid. Add tests for new functions as they're created.

### `+page.svelte` (health page) — REFACTOR

**What's here:** Experimental UI with brew-specific remedy, retry simulation, manual event wiring.

**What the design says:** Template renders `bs.visibleGates`. Consolidated prereq remedy card. Tauri event wiring via onMount.

**Action:**
- Replace `missingBrewGates` references → `bs.missingPrereqGates`
- Replace `needsBrewInstall` references → `bs.needsPrereqInstall`
- Replace `runBrewBundle()` → `runInstallPrereqs()` (calls installPrerequisites)
- Remove inline derived state (already moved to BootstrapState, but page still has `BREWFILE_URL` const)
- KEEP: fixed header + scroll area layout
- KEEP: gate rendering loop (uses bs.visibleGates)
- KEEP: per-gate remedy for install/db/daemon types
- KEEP: consolidated prereq remedy card
- REFACTOR: remedy card text should be platform-aware (not hardcoded curl command)

### `+layout.svelte` (health layout) — KEEP

Simple shell wrapper. No changes needed.

---

## Summary: Work items

### Delete (clean up before implementation)

1. `check_all_components` Tauri command — redundant
2. `install_component` Tauri command — replaced by install_prerequisites
3. `start_component` Tauri command — replaced by start_services
4. `installComponent()` / `startComponent()` frontend functions — same

### Restructure (Rust crate)

5. Create `platform/` module with trait + macos + windows stub
6. Move homebrew.rs internals → platform/macos.rs
7. Move service.rs brew-specific code → platform/macos.rs
8. Extract shared utilities (which_binary, probe_port, fetch_version)
9. Rewrite lib.rs::run() to use platform provider
10. Add database::setup() orchestrator (create + extensions + migrate)

### Restructure (Tauri)

11. Add `install_prerequisites` command (wraps platform provider)
12. Add `start_services` command (sequential start + events)
13. Add `setup_database` command (create + extensions + migrate + events)
14. Keep `brew_bundle_install` as internal (called by macOS provider)

### Restructure (Frontend)

15. Rename brew → prereq in state and gates
16. Add platform to BootstrapState
17. Update page to use prereq naming
18. Make remedy card platform-aware
19. Add `installPrerequisites()`, `startServices()`, `setupDatabase()` API functions

### New code

20. `platform/mod.rs` — trait definition
21. `platform/macos.rs` — Homebrew/launchd provider
22. `platform/windows.rs` — winget/services stub
23. `database::setup()` — Phase 3 pipeline
24. `database::ensure_extensions()` — pgvector check + create
25. `senseid migrate` integration (blocked on dbd-core)
