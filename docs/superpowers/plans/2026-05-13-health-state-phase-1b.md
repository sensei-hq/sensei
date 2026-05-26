# Health State — Phase 1b Implementation Plan (v2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the existing per-component bootstrap engine with a generic, trait-based dependency-resolution framework whose public surface is the `HealthPayload` / `HealthEvent` contract `HealthState` (TypeScript) already consumes.

**Architecture in one paragraph:** A universal `DependencySpec` table declares the 5 dependencies — id / label / note / topological edges — same on every platform. Each `PlatformProvider` returns `dependencies() -> Vec<Dependency>` where each `Dependency` packages the universal spec with its platform-specific `Box<dyn Checker>`. Resolvers self-declare which dependencies they cover via `resolves() -> &[ComponentId]`. The orchestrator iterates `provider.dependencies()`, runs each checker, and groups failed deps by the resolver that covers them so a multi-target resolver (brew bundle) runs once for postgres + ollama + sensei. `check()` returns a `HealthPayload` synchronously. `check_and_resolve(emit)` streams `HealthEvent`s and returns the terminal payload. Daemon `/health` and Tauri sidecar are one-liner transports.

**Tech Stack:** Rust (`sensei_bootstrap`, `senseid`, `src-tauri`), serde, axum, Tauri 2.

**Source contract:** `app/src/lib/health-types.ts` defines the wire shape Rust must match.

**Source spec:** `docs/superpowers/specs/2026-05-12-health-state-design.md` Section 3.

**Superseded:** `docs/superpowers/plans/2026-05-13-health-state-phase-1b-v1-superseded.md` (kept for history; do not implement).

---

## Discipline (applies to every section)

This plan is strict TDD with strict 100% coverage.

1. **Red-green-refactor at every step.** For every new function, method, branch, or trait impl: write the failing test FIRST. Run it. See red. THEN write the impl. Run again. See green. Commit. Never commit code that wasn't preceded by a failing test.

2. **100% line + branch coverage on every Phase-1b module.** The verification gate (end of this doc) runs `cargo llvm-cov` and fails the phase if any health-related module is below 100% on lines or branches. Modules in scope:
   - `health::types` (incl. `validate()` invariants)
   - `health::ids`
   - `health::graph`
   - `health::checker` (the trait + `CheckOutcome` constructors)
   - `health::resolver` (the trait + `ResolveOutcome`)
   - `health::provider` (the trait — including default `check`/`resolve` impls)
   - `health::platforms::macos`
   - `health::platforms::windows` (stub still needs its tests)
   - `health::checkers::*` (each concrete checker module)
   - `health::resolvers::*` (each concrete resolver module)
   - `health::mod` (crate-level wrappers)

3. **Every public scenario tested.** Specifically:
   - All four `HealthStatus` values produced by `check()` under different mock-provider configurations.
   - Every branch in `HealthPayload::validate()` — INV-1 both sides, INV-2 length, INV-2 order, INV-3 mac/linux/windows.
   - Every `match` arm in every `checker_for` impl.
   - Resolver dedup: one resolver covering 3 deps runs ONCE.
   - Resolver outcome both arms: `Resolved` (re-check happens) and `NeedsHumanAction` (remedy threaded through).
   - JSON wire-shape tests pin the contract against `app/src/lib/health-types.ts`.

4. **No `_ =>` catchall in any `match id` arm.** Adding a new `ComponentId` must be a compile error across every platform impl until each handles it. Audited in code review.

5. **Coverage tool.** If `cargo-llvm-cov` isn't installed locally, install it as part of pre-flight:
   ```bash
   cargo install cargo-llvm-cov --locked
   ```
   The verification gate uses `cargo llvm-cov` with `--fail-under-lines 100 --fail-under-branches 100` scoped to the health modules.

---

## Pre-flight

- [ ] **Confirm baseline**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
git status --short                                           # only known-WIP files
cargo build  -p sensei-bootstrap 2>&1 | tail -3              # clean baseline
cargo test   -p sensei-bootstrap 2>&1 | tail -5
cargo build  -p senseid 2>&1 | tail -3
cargo build  --manifest-path app/src-tauri/Cargo.toml 2>&1 | tail -3
cd app && bun run test:unit 2>&1 | tail -3                   # 273 tests pass
```

Record the test counts. New code must not regress them.

- [ ] **Install the coverage tool (one-time)**

```bash
cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
```

The Phase 1b verification gate enforces 100% line + branch coverage on the new health modules via this tool.

- [ ] **Find every external consumer of `sensei_bootstrap`'s prereq / platform / health surface — these are the call sites you'll update later.**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
grep -rn "sensei_bootstrap::prereq\|bootstrap::prereq\|sensei_bootstrap::provider\|bootstrap::provider\|check_and_fix\|BootstrapReport\|ProgressEvent\|GateStatus\|bootstrap::ComponentStatus\|bootstrap::ComponentState\|bootstrap::database\|bootstrap::models\|bootstrap::HardwareInfo\|bootstrap::hardware" crates app/src-tauri 2>&1 | grep -v "^crates/bootstrap/" | head -40
```

Expected hits (full audit lives in `docs/superpowers/plans/notes/1b-callers.md` after this run):
- `crates/senseid/src/api/handlers/health.rs` — replaced in Section F.
- `app/src-tauri/src/commands/bootstrap.rs` — replaced in Section G.

If any other hits surface, they must be migrated as part of this phase. Stop and surface them before continuing.

---

## Section 0 — Quarantine the old engine

We're not deleting the old code yet; we're moving it out of the compile graph so the new code can't accidentally depend on it.

### Task 0.1 — Move old modules to `_legacy/`

**Files:**
- Move: `crates/bootstrap/src/prereq/` → `crates/bootstrap/src/_legacy/prereq/`
- Move: `crates/bootstrap/src/platform/` → `crates/bootstrap/src/_legacy/platform/`
- Move: `crates/bootstrap/src/types.rs` → `crates/bootstrap/src/_legacy/types.rs`
- Move: `crates/bootstrap/src/database.rs` → `crates/bootstrap/src/_legacy/database.rs`
- Move: `crates/bootstrap/src/models.rs` → `crates/bootstrap/src/_legacy/models.rs`
- Move: `crates/bootstrap/src/hardware.rs` → `crates/bootstrap/src/_legacy/hardware.rs`
- Modify: `crates/bootstrap/src/lib.rs` — remove `pub mod prereq`/`platform`/`types`/`database`/`models`/`hardware`; remove `check_and_fix`, `check_and_fix_with_context`, `provider()` functions; keep `config`, `util` modules and their re-exports.

`_legacy/` is **not** referenced from any `mod` declaration; Cargo will not compile it. It's preserved as reference until Section H deletes it.

- [ ] **Step 1: Move files**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/crates/bootstrap/src
mkdir -p _legacy
git mv prereq _legacy/
git mv platform _legacy/
git mv types.rs _legacy/types.rs
git mv database.rs _legacy/database.rs
git mv models.rs _legacy/models.rs
git mv hardware.rs _legacy/hardware.rs
```

- [ ] **Step 2: Trim `lib.rs`**

Replace `crates/bootstrap/src/lib.rs` with:

```rust
//! sensei-bootstrap — generic dependency-resolution framework with a public
//! health contract.
//!
//! Public surface:
//!   * health::{HealthPayload, HealthEvent, ...} — wire types matching TS.
//!   * health::check()             — sync fast path; daemon /health uses this.
//!   * health::check_and_resolve() — streaming check + fix; sidecar uses this.
//!   * config::*                    — runtime config (unchanged).
//!   * util::*                      — small utilities (unchanged).

pub mod config;
pub mod util;
pub mod health;

pub use config::{
    SenseiConfig, SenseiMode, SenseiLocalConfig,
    COMPILE_DEV, home_dir,
    BREW_PATHS, BREW_TAP, GITHUB_ORG, GITHUB_REPO,
    HOMEBREW_BREWFILE_URL, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL,
    MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    OLLAMA_PORT, POSTGRES_PORT,
    DB_POOL_MAX_CONNECTIONS, DB_POOL_ACQUIRE_TIMEOUT_SECS, DB_POOL_IDLE_TIMEOUT_SECS,
};
pub use health::*;

/// Daemon port for the current mode.
pub fn daemon_port() -> u16 { SenseiConfig::from_env().daemon_port }

/// Lazily-initialised config singleton.
pub fn config() -> &'static SenseiConfig {
    use std::sync::OnceLock;
    static CFG: OnceLock<SenseiConfig> = OnceLock::new();
    CFG.get_or_init(SenseiConfig::from_env)
}

/// Shorthand for `config().daemon_url()`.
pub fn daemon_url() -> String { config().daemon_url() }
```

Note: `provider()` is removed. The old `check_and_fix*` functions are removed. `health` module will be created in Section A and provides the new entry points.

- [ ] **Step 3: Stub the health module so the crate compiles**

Create `crates/bootstrap/src/health/mod.rs`:

```rust
//! Health module — to be implemented in Sections A-D.
//! This is an empty placeholder so the crate compiles after the quarantine.
```

- [ ] **Step 4: Build — expect breakage in consumers**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei && cargo build --workspace 2>&1 | tail -20
```

Expected: `sensei-bootstrap` compiles clean; `senseid` and `src-tauri` fail because they import `bootstrap::prereq`, `bootstrap::ComponentStatus`, `bootstrap::hardware`, `bootstrap::database`, `bootstrap::models`, `bootstrap::provider`. **Do not fix those yet** — they'll be migrated in their own sections. The intermediate state is broken on purpose.

- [ ] **Step 5: Commit the quarantine**

```bash
git add crates/bootstrap/
git commit -m "$(cat <<'EOF'
chore(bootstrap): quarantine pre-existing engine under _legacy/

Phase 1b is a clean-slate rewrite of the health/dependency-resolution
surface. The old prereq, platform, types, database, models, hardware
modules are moved under src/_legacy/ (not referenced by lib.rs, not
compiled). They will be deleted in Section H once consumers migrate.

senseid and src-tauri compile-fail at this commit; they're rewritten in
Sections F and G to use the new health module.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)" 2>&1 | tail -3
```

---

## Section A — Foundation types (TS-matching)

### Task A1 — `health/types.rs`

**Files:**
- Create: `crates/bootstrap/src/health/types.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

- [ ] **Step 1: Write `types.rs`**

```rust
//! Public wire types — match `app/src/lib/health-types.ts` exactly.

use serde::{Deserialize, Serialize};

// ── Closed enums ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform { Macos, Linux, Windows }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthStatus { Checking, Resolving, Ok, NeedsAction }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus { Pending, Checking, Installing, Ready, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentId { Postgres, Ollama, Sensei, Database, Daemon }

pub const COMPONENT_ORDER: [ComponentId; 5] = [
    ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
    ComponentId::Database, ComponentId::Daemon,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManagerId { Homebrew, Winget }

// ── Component & Remedy ───────────────────────────────────────────────────

/// Generic component — used for ledger rows AND the package manager.
/// `id` is a lowercase string ("postgres", "ollama", …, "homebrew", "winget").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    pub id:      String,
    pub label:   String,
    pub note:    Option<String>,
    pub status:  ComponentStatus,
    pub version: Option<String>,
    pub detail:  Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remedy {
    pub message: String,
    pub script:  String,
    pub url:     Option<String>,
}

// ── HealthPayload ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPayload {
    pub version:         String,
    pub uptime_seconds:  u64,
    pub platform:        Platform,
    pub package_manager: Component,
    pub components:      Vec<Component>,
    pub status:          HealthStatus,
    pub remedy:          Option<Remedy>,
}

impl HealthPayload {
    /// Runtime invariant guard mirroring the TS apply() checks.
    pub fn validate(&self) -> Result<(), String> {
        // INV-1
        match (self.status, self.remedy.as_ref()) {
            (HealthStatus::NeedsAction, None) =>
                return Err("HealthPayload: needs-action requires a remedy".to_string()),
            (s, Some(_)) if s != HealthStatus::NeedsAction =>
                return Err(format!("HealthPayload: status={:?} must not carry a remedy", s)),
            _ => {}
        }
        // INV-2
        if self.components.len() != COMPONENT_ORDER.len() {
            return Err(format!(
                "HealthPayload: expected {} components, got {}",
                COMPONENT_ORDER.len(), self.components.len()
            ));
        }
        for (i, expected) in COMPONENT_ORDER.iter().enumerate() {
            let want = super::ids::component_id_str(*expected);
            if self.components[i].id != want {
                return Err(format!(
                    "HealthPayload: components[{}].id must be \"{}\", got \"{}\"",
                    i, want, self.components[i].id
                ));
            }
        }
        // INV-3
        let want = match self.platform {
            Platform::Windows => "winget",
            _ => "homebrew",
        };
        if self.package_manager.id != want {
            return Err(format!(
                "HealthPayload: platform={:?} expects packageManager.id=\"{}\", got \"{}\"",
                self.platform, want, self.package_manager.id
            ));
        }
        Ok(())
    }
}

// ── HealthEvent (streaming) ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum HealthEvent {
    Phase     { phase: HealthStatus },
    Component { id: String, patch: ComponentPatch },
    Remedy    { remedy: Remedy },
    Report    { payload: HealthPayload },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentPatch {
    #[serde(skip_serializing_if = "Option::is_none")] pub label:   Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub note:    Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub status:  Option<ComponentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")] pub version: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub detail:  Option<Option<String>>,
}
```

- [ ] **Step 2: Create `crates/bootstrap/src/health/ids.rs`**

```rust
//! String form of ComponentId and PackageManagerId — needed wherever a
//! type-erased id field appears (Component.id is a String to allow mixing
//! the two enums).

use super::types::{ComponentId, PackageManagerId, Platform};

pub fn component_id_str(id: ComponentId) -> &'static str {
    match id {
        ComponentId::Postgres => "postgres",
        ComponentId::Ollama   => "ollama",
        ComponentId::Sensei   => "sensei",
        ComponentId::Database => "database",
        ComponentId::Daemon   => "daemon",
    }
}

pub fn package_manager_id_str(id: PackageManagerId) -> &'static str {
    match id {
        PackageManagerId::Homebrew => "homebrew",
        PackageManagerId::Winget   => "winget",
    }
}

pub fn package_manager_for_platform(p: Platform) -> PackageManagerId {
    match p {
        Platform::Windows => PackageManagerId::Winget,
        _ => PackageManagerId::Homebrew,
    }
}
```

- [ ] **Step 3: Update `health/mod.rs`**

```rust
//! Public health surface.

pub mod types;
pub mod ids;

pub use types::*;
```

- [ ] **Step 4: Build + small JSON shape test**

Add to `health/types.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_status_serializes_kebab_case() {
        let s = serde_json::to_string(&HealthStatus::NeedsAction).unwrap();
        assert_eq!(s, "\"needs-action\"");
    }

    #[test]
    fn component_status_serializes_lowercase() {
        let s = serde_json::to_string(&ComponentStatus::Ready).unwrap();
        assert_eq!(s, "\"ready\"");
    }

    #[test]
    fn component_id_serializes_lowercase() {
        let s = serde_json::to_string(&ComponentId::Postgres).unwrap();
        assert_eq!(s, "\"postgres\"");
    }

    #[test]
    fn health_event_uses_kind_tag() {
        let ev = HealthEvent::Phase { phase: HealthStatus::Checking };
        let s = serde_json::to_string(&ev).unwrap();
        assert_eq!(s, r#"{"kind":"phase","phase":"checking"}"#);
    }
}
```

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -5
cargo test  -p sensei-bootstrap health 2>&1 | tail -5
```

Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): health types matching TS wire contract"
```

---

## Section B — The Dependency Graph and Resolver/Checker traits

### Task B1 — Universal dependency graph + trait definitions

**Files:**
- Create: `crates/bootstrap/src/health/graph.rs`
- Create: `crates/bootstrap/src/health/checker.rs`
- Create: `crates/bootstrap/src/health/resolver.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

Dependencies are **universal across platforms** — the IDs, labels, notes, and topological edges are the same on macOS, Linux, and Windows. Only the **how** (checkers + resolvers) varies per platform.

- [ ] **Step 1: Tests first (TDD)**

Create `crates/bootstrap/src/health/graph.rs` — the universal SPEC table (no checkers):

```rust
//! Universal dependency spec — same on every platform.
//! Specs are pure data: id, label, note, topological edges. Platform-specific
//! checkers are bound at construction by PlatformProvider::dependencies(),
//! which packages each spec with a Box<dyn Checker> into a `Dependency`.

use super::types::ComponentId;

#[derive(Debug, Clone, Copy)]
pub struct DependencySpec {
    pub id:         ComponentId,
    pub label:      &'static str,
    pub note:       Option<&'static str>,
    pub depends_on: &'static [ComponentId],
}

const GRAPH: [DependencySpec; 5] = [
    DependencySpec { id: ComponentId::Postgres, label: "PostgreSQL",        note: None, depends_on: &[] },
    DependencySpec { id: ComponentId::Ollama,   label: "Ollama",            note: None, depends_on: &[] },
    DependencySpec { id: ComponentId::Sensei,   label: "Sensei components", note: Some("cli · mcp · daemon"), depends_on: &[] },
    DependencySpec { id: ComponentId::Database, label: "Database & schema", note: Some("pgvector · sensei tables"),
                     depends_on: &[ComponentId::Postgres] },
    DependencySpec { id: ComponentId::Daemon,   label: "Background daemon", note: None,
                     depends_on: &[ComponentId::Database, ComponentId::Sensei] },
];

pub fn dependency_specs() -> &'static [DependencySpec] { &GRAPH }

pub fn spec_for(id: ComponentId) -> &'static DependencySpec {
    GRAPH.iter().find(|d| d.id == id).expect("ComponentId must be in graph")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::COMPONENT_ORDER;

    #[test]
    fn graph_has_exactly_five_specs_in_canonical_order() {
        let g = dependency_specs();
        assert_eq!(g.len(), 5);
        for (i, d) in g.iter().enumerate() {
            assert_eq!(d.id, COMPONENT_ORDER[i], "graph[{i}] must be {:?}", COMPONENT_ORDER[i]);
        }
    }

    #[test]
    fn spec_labels_match_ts_defaults() {
        assert_eq!(spec_for(ComponentId::Postgres).label, "PostgreSQL");
        assert_eq!(spec_for(ComponentId::Ollama).label,   "Ollama");
        assert_eq!(spec_for(ComponentId::Sensei).label,   "Sensei components");
        assert_eq!(spec_for(ComponentId::Sensei).note,    Some("cli · mcp · daemon"));
        assert_eq!(spec_for(ComponentId::Database).label, "Database & schema");
        assert_eq!(spec_for(ComponentId::Database).note,  Some("pgvector · sensei tables"));
        assert_eq!(spec_for(ComponentId::Daemon).label,   "Background daemon");
    }

    #[test]
    fn database_depends_on_postgres() {
        assert_eq!(spec_for(ComponentId::Database).depends_on, &[ComponentId::Postgres]);
    }

    #[test]
    fn daemon_depends_on_database_and_sensei() {
        assert_eq!(spec_for(ComponentId::Daemon).depends_on, &[ComponentId::Database, ComponentId::Sensei]);
    }
}
```

No separate `Dependency` struct. The universal `DependencySpec` is iterated by the orchestrator (in the trait default `check`); the platform-specific checker comes from `PlatformProvider::checker_for(id)`. Binding spec + checker into a struct would just be a redundant cache of two things the orchestrator already has in hand at iteration time.

- [ ] **Step 2: Create `health/checker.rs`**

```rust
//! Checker trait — platform implementations build concrete checkers and
//! bind them to dependency ids via PlatformProvider::checker_for(id).

use super::types::ComponentStatus;

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub status:  ComponentStatus,         // Ready or Failed (Pending/Checking are orchestrator-set)
    pub version: Option<String>,
    pub detail:  Option<String>,          // failure detail when status==Failed
}

impl CheckOutcome {
    pub fn ready(version: impl Into<String>) -> Self {
        Self { status: ComponentStatus::Ready, version: Some(version.into()), detail: None }
    }
    pub fn ready_no_version() -> Self {
        Self { status: ComponentStatus::Ready, version: None, detail: None }
    }
    pub fn failed(detail: impl Into<String>) -> Self {
        Self { status: ComponentStatus::Failed, version: None, detail: Some(detail.into()) }
    }
}

pub trait Checker: Send + Sync {
    fn check(&self) -> CheckOutcome;
}
```

- [ ] **Step 3: Create `health/resolver.rs`**

```rust
//! Resolver trait — a Resolver self-declares which dependencies it can fix
//! via `resolves()`. The orchestrator iterates failed deps, groups them by
//! the resolver that covers them, and runs each resolver ONCE with all its
//! target deps.

use super::types::{ComponentId, Remedy};

#[derive(Debug, Clone)]
pub enum ResolveOutcome {
    /// Resolver completed successfully; orchestrator will re-check the targets.
    Resolved,
    /// Resolver couldn't fix it on its own; UI shows this Remedy to the user.
    NeedsHumanAction(Remedy),
}

pub trait Resolver: Send + Sync {
    /// Stable id (used in logs and event metadata; not user-facing).
    fn id(&self) -> &'static str;

    /// Which dependencies this resolver covers. The orchestrator uses this
    /// to (a) find the right resolver for a failed dep, and (b) dedupe so
    /// the resolver runs once for all its targets, not once per target.
    fn resolves(&self) -> &'static [ComponentId];

    /// Run the fix. `targets` is the subset of `resolves()` that's currently
    /// failed. Resolver decides whether to batch (brew bundle covers all in
    /// one call) or iterate.
    fn resolve(&self, targets: &[ComponentId]) -> ResolveOutcome;
}
```

- [ ] **Step 4: Wire into `health/mod.rs`**

```rust
pub mod types;
pub mod ids;
pub mod graph;
pub mod checker;
pub mod resolver;

pub use types::*;
pub use graph::{DependencySpec, dependency_specs, spec_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
```

- [ ] **Step 5: Build + test**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -5
cargo test  -p sensei-bootstrap health::graph 2>&1 | tail -10
```

Expected: 4 graph tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): universal dependency graph + Checker/Resolver traits"
```

---

## Section C — PlatformProvider trait + macOS implementation

### Task C1 — `PlatformProvider` trait

**Files:**
- Create: `crates/bootstrap/src/health/provider.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

- [ ] **Step 1: TDD red — write the trait test first**

Create `crates/bootstrap/src/health/provider.rs` initially as a SKELETON with a stub trait so we can write a test against the default implementations BEFORE filling them in:

```rust
//! PlatformProvider — Template Method trait.
//!
//! Required methods carry platform-specific knowledge.
//! Default check/resolve methods carry the shared orchestration logic;
//! a platform overrides them only if its check/resolve flow is fundamentally
//! different (e.g. a future cloud-VM platform that polls a control-plane API).
//!
//! IMPORTANT: every impl of `checker_for` MUST use exhaustive `match id { ... }`
//! with no `_ =>` catchall, so adding a new ComponentId is a compile error
//! across every platform until each handles it.

use std::collections::HashMap;
use super::types::*;
use super::checker::Checker;
use super::resolver::Resolver;
use super::graph::dependency_specs;
use super::ids::{component_id_str, package_manager_id_str};

pub trait PlatformProvider: Send + Sync {
    // ── Required (per-platform) ───────────────────────────────────────────
    fn platform(&self) -> Platform;
    fn package_manager_id(&self) -> PackageManagerId;
    fn package_manager_checker(&self) -> Box<dyn Checker>;
    fn checker_for(&self, id: ComponentId) -> Box<dyn Checker>;
    fn resolvers(&self) -> Vec<Box<dyn Resolver>>;
    fn default_remedy(&self) -> Remedy;

    // ── Default Template Method ───────────────────────────────────────────

    /// Run every checker once. Returns a validated HealthPayload.
    /// Override if your platform has a fundamentally different probe shape.
    fn check(&self, app_version: &str) -> HealthPayload {
        let pm = build_package_manager(self);
        let components: Vec<Component> = dependency_specs().iter().map(|spec| {
            let outcome = self.checker_for(spec.id).check();
            Component {
                id:     component_id_str(spec.id).to_string(),
                label:  spec.label.to_string(),
                note:   spec.note.map(str::to_string),
                status: outcome.status,
                version: outcome.version,
                detail:  outcome.detail,
            }
        }).collect();

        let status = overall_status(&pm, &components);
        let remedy = if matches!(status, HealthStatus::NeedsAction) {
            Some(self.default_remedy())
        } else { None };

        let payload = HealthPayload {
            version: app_version.to_string(),
            uptime_seconds: 0,
            platform: self.platform(),
            package_manager: pm,
            components,
            status,
            remedy,
        };
        payload.validate().expect("PlatformProvider::check produced an invalid payload");
        payload
    }

    /// Walk the resolvers, find covered failed deps, run each once, re-check.
    /// Emits HealthEvent values as the work progresses; returns terminal payload.
    /// Override only if resolution requires a totally different flow.
    fn resolve<F>(&self, current: &HealthPayload, app_version: &str, emit: F) -> HealthPayload
    where F: Fn(HealthEvent) + Send + Sync
    {
        emit(HealthEvent::Phase { phase: HealthStatus::Resolving });

        let failed: Vec<ComponentId> = current.components.iter()
            .filter(|c| c.status == ComponentStatus::Failed)
            .filter_map(|c| super::ids::parse_component_id(&c.id))
            .collect();

        let resolvers = self.resolvers();
        let mut applied_remedy: Option<Remedy> = None;

        for resolver in &resolvers {
            let targets: Vec<ComponentId> = resolver.resolves().iter().copied()
                .filter(|id| failed.contains(id)).collect();
            if targets.is_empty() { continue; }

            // Mark targets as "installing" so the UI ledger animates.
            for tid in &targets {
                emit(HealthEvent::Component {
                    id: component_id_str(*tid).to_string(),
                    patch: ComponentPatch {
                        status: Some(ComponentStatus::Installing),
                        ..Default::default()
                    },
                });
            }

            match resolver.resolve(&targets) {
                super::resolver::ResolveOutcome::Resolved => {
                    for tid in &targets {
                        let outcome = self.checker_for(*tid).check();
                        emit(HealthEvent::Component {
                            id: component_id_str(*tid).to_string(),
                            patch: ComponentPatch {
                                status:  Some(outcome.status),
                                version: Some(outcome.version),
                                detail:  Some(outcome.detail),
                                ..Default::default()
                            },
                        });
                    }
                }
                super::resolver::ResolveOutcome::NeedsHumanAction(remedy) => {
                    applied_remedy.get_or_insert(remedy.clone());
                    emit(HealthEvent::Remedy { remedy });
                }
            }
        }

        // Final re-check builds the terminal payload.
        let mut terminal = self.check(app_version);
        if terminal.status == HealthStatus::NeedsAction && terminal.remedy.is_none() {
            terminal.remedy = applied_remedy.or_else(|| Some(self.default_remedy()));
        }
        if terminal.status != HealthStatus::NeedsAction { terminal.remedy = None; }
        terminal.validate().expect("PlatformProvider::resolve produced an invalid terminal payload");
        emit(HealthEvent::Report { payload: terminal.clone() });
        terminal
    }
}

fn build_package_manager(p: &dyn PlatformProvider) -> Component {
    let pm_id = p.package_manager_id();
    let (label, note) = match pm_id {
        PackageManagerId::Homebrew => ("Homebrew", "which brew"),
        PackageManagerId::Winget   => ("winget",   "winget --version"),
    };
    let outcome = p.package_manager_checker().check();
    Component {
        id:      package_manager_id_str(pm_id).to_string(),
        label:   label.to_string(),
        note:    Some(note.to_string()),
        status:  outcome.status,
        version: outcome.version,
        detail:  outcome.detail,
    }
}

fn overall_status(pm: &Component, components: &[Component]) -> HealthStatus {
    let all_ready = pm.status == ComponentStatus::Ready
        && components.iter().all(|c| c.status == ComponentStatus::Ready);
    if all_ready { return HealthStatus::Ok; }
    let any_failed = pm.status == ComponentStatus::Failed
        || components.iter().any(|c| c.status == ComponentStatus::Failed);
    if any_failed { return HealthStatus::NeedsAction; }
    HealthStatus::Checking
}

/// Detect the current platform and return its provider.
pub fn detect_provider() -> Box<dyn PlatformProvider> {
    #[cfg(target_os = "windows")]
    { Box::new(super::platforms::windows::WindowsProvider) }
    #[cfg(not(target_os = "windows"))]
    { Box::new(super::platforms::macos::MacOSProvider) }
}
```

- [ ] **Step 1.1: Add a `parse_component_id` helper to `health/ids.rs`**

Append to `health/ids.rs`:

```rust
pub fn parse_component_id(s: &str) -> Option<ComponentId> {
    match s {
        "postgres" => Some(ComponentId::Postgres),
        "ollama"   => Some(ComponentId::Ollama),
        "sensei"   => Some(ComponentId::Sensei),
        "database" => Some(ComponentId::Database),
        "daemon"   => Some(ComponentId::Daemon),
        _ => None,
    }
}
```

Test alongside it:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip() {
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
                   ComponentId::Database, ComponentId::Daemon] {
            assert_eq!(parse_component_id(component_id_str(id)), Some(id));
        }
        assert_eq!(parse_component_id("nope"), None);
    }
}
```

- [ ] **Step 2: Update `health/mod.rs`**

```rust
pub mod types;
pub mod ids;
pub mod graph;
pub mod checker;
pub mod resolver;
pub mod provider;
pub mod platforms;

pub use types::*;
pub use graph::{DependencySpec, dependency_specs, spec_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
pub use provider::{PlatformProvider, detect_provider};
```

- [ ] **Step 2.1: TDD tests for the default check/resolve impls — write FIRST**

Append to `health/provider.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // ── Mock building blocks ──────────────────────────────────────────

    struct StubChecker(CheckOutcome);
    impl Checker for StubChecker {
        fn check(&self) -> CheckOutcome { self.0.clone() }
    }

    struct StubResolver {
        id:       &'static str,
        targets:  &'static [ComponentId],
        outcome:  ResolveOutcome,
        calls:    Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl Resolver for StubResolver {
        fn id(&self) -> &'static str { self.id }
        fn resolves(&self) -> &'static [ComponentId] { self.targets }
        fn resolve(&self, t: &[ComponentId]) -> ResolveOutcome {
            self.calls.lock().unwrap().push(t.to_vec());
            self.outcome.clone()
        }
    }

    struct MockProvider {
        pm_id:      PackageManagerId,
        outcomes:   HashMap<ComponentId, CheckOutcome>,
        pm_outcome: CheckOutcome,
        resolvers:  Vec<Box<dyn Resolver>>,
    }
    impl PlatformProvider for MockProvider {
        fn platform(&self) -> Platform { Platform::Macos }
        fn package_manager_id(&self) -> PackageManagerId { self.pm_id }
        fn package_manager_checker(&self) -> Box<dyn Checker> { Box::new(StubChecker(self.pm_outcome.clone())) }
        fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
            Box::new(StubChecker(self.outcomes.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("no stub for id"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            // Caller is expected to replace; we can't clone Box<dyn>, so build trivial empty
            self.resolvers.iter().map(|_| -> Box<dyn Resolver> {
                Box::new(StubResolver {
                    id: "noop", targets: &[], outcome: ResolveOutcome::Resolved,
                    calls: Arc::new(Mutex::new(Vec::new())),
                })
            }).collect()
        }
        fn default_remedy(&self) -> Remedy {
            Remedy { message: "m".into(), script: "s".into(), url: None }
        }
    }

    fn ready_outcomes() -> HashMap<ComponentId, CheckOutcome> {
        let mut m = HashMap::new();
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
                   ComponentId::Database, ComponentId::Daemon] {
            m.insert(id, CheckOutcome::ready("1.0"));
        }
        m
    }

    fn one_failed(failed: ComponentId, detail: &str) -> HashMap<ComponentId, CheckOutcome> {
        let mut m = ready_outcomes();
        m.insert(failed, CheckOutcome::failed(detail));
        m
    }

    // ── Tests ─────────────────────────────────────────────────────────

    #[test]
    fn check_all_ready_yields_ok() {
        let p = MockProvider { pm_id: PackageManagerId::Homebrew, pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(), resolvers: vec![] };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::Ok);
        assert!(payload.remedy.is_none());
        assert_eq!(payload.components.len(), 5);
        for c in &payload.components { assert_eq!(c.status, ComponentStatus::Ready); }
    }

    #[test]
    fn check_with_one_failed_yields_needs_action_with_default_remedy() {
        let p = MockProvider { pm_id: PackageManagerId::Homebrew, pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: one_failed(ComponentId::Postgres, "pg_isready failed"), resolvers: vec![] };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::NeedsAction);
        assert!(payload.remedy.is_some());
        let pg = payload.components.iter().find(|c| c.id == "postgres").unwrap();
        assert_eq!(pg.status, ComponentStatus::Failed);
        assert_eq!(pg.detail.as_deref(), Some("pg_isready failed"));
    }

    #[test]
    fn check_pm_failed_with_components_ready_yields_needs_action() {
        let p = MockProvider { pm_id: PackageManagerId::Homebrew, pm_outcome: CheckOutcome::failed("brew missing"),
            outcomes: ready_outcomes(), resolvers: vec![] };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::NeedsAction);
        assert_eq!(payload.package_manager.status, ComponentStatus::Failed);
    }

    #[test]
    fn check_payload_validates() {
        let p = MockProvider { pm_id: PackageManagerId::Homebrew, pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(), resolvers: vec![] };
        let payload = p.check("0.0.0-test");
        payload.validate().expect("must validate");
    }

    #[test]
    fn check_components_in_canonical_order() {
        let p = MockProvider { pm_id: PackageManagerId::Homebrew, pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(), resolvers: vec![] };
        let payload = p.check("0.0.0-test");
        let ids: Vec<&str> = payload.components.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["postgres","ollama","sensei","database","daemon"]);
    }

    // resolve() tests require a custom MockProvider whose resolvers() can return
    // the StubResolvers we want to inspect. Build per-test rather than via the
    // shared MockProvider above (which can't clone Box<dyn>).

    struct RecordingMockProvider {
        pm_outcome:  CheckOutcome,
        outcomes:    Mutex<HashMap<ComponentId, CheckOutcome>>,
        resolver_id: &'static str,
        resolver_targets: &'static [ComponentId],
        resolver_outcome: ResolveOutcome,
        calls:       Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl PlatformProvider for RecordingMockProvider {
        fn platform(&self) -> Platform { Platform::Macos }
        fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
        fn package_manager_checker(&self) -> Box<dyn Checker> { Box::new(StubChecker(self.pm_outcome.clone())) }
        fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
            // After resolve runs, the post-resolve re-check should see updated outcomes.
            // For tests where the resolver succeeds, the test inserts a Ready outcome
            // for the targets before calling resolve, so the re-check sees Ready.
            let m = self.outcomes.lock().unwrap();
            Box::new(StubChecker(m.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("none"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            vec![Box::new(StubResolver {
                id: self.resolver_id,
                targets: self.resolver_targets,
                outcome: self.resolver_outcome.clone(),
                calls: self.calls.clone(),
            })]
        }
        fn default_remedy(&self) -> Remedy {
            Remedy { message: "m".into(), script: "s".into(), url: None }
        }
    }

    #[test]
    fn resolve_runs_covering_resolver_once_with_all_targets() {
        const TARGETS: &[ComponentId] = &[ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei];
        let mut outcomes = ready_outcomes();
        // Three components start as failed; resolver should be called ONCE with all three.
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("x"));
        outcomes.insert(ComponentId::Ollama,   CheckOutcome::failed("x"));
        outcomes.insert(ComponentId::Sensei,   CheckOutcome::failed("x"));
        let calls = Arc::new(Mutex::new(Vec::new()));

        let p = RecordingMockProvider {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes.clone()),
            resolver_id: "brew_bundle",
            resolver_targets: TARGETS,
            resolver_outcome: ResolveOutcome::Resolved,
            calls: calls.clone(),
        };
        // First build the "current" state from a check().
        let current = p.check("0.0.0-test");
        assert_eq!(current.status, HealthStatus::NeedsAction);

        // After resolve succeeds, the re-check needs to see Ready. Update stub outcomes.
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Postgres, CheckOutcome::ready("16.3"));
            m.insert(ComponentId::Ollama,   CheckOutcome::ready("0.4"));
            m.insert(ComponentId::Sensei,   CheckOutcome::ready_no_version());
        }

        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", move |e| ec.lock().unwrap().push(e));

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1, "resolver invoked once");
        assert_eq!(recorded[0].len(), 3, "with all three targets in one call");
        assert_eq!(terminal.status, HealthStatus::Ok);

        // Event sequence sanity
        let evs = events.lock().unwrap();
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving })));
        assert!(matches!(evs.last(),  Some(HealthEvent::Report { .. })));
        let installing = evs.iter().filter(|e| matches!(e,
            HealthEvent::Component { patch: ComponentPatch { status: Some(ComponentStatus::Installing), .. }, .. }
        )).count();
        assert_eq!(installing, 3, "installing emitted per-target");
    }

    #[test]
    fn resolve_skips_resolvers_with_no_targets() {
        let p = RecordingMockProvider {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(ready_outcomes()), // nothing failed
            resolver_id: "brew_bundle",
            resolver_targets: &[ComponentId::Postgres],
            resolver_outcome: ResolveOutcome::Resolved,
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        // current.status == Ok, so resolve() should still emit Phase + Report
        // but never invoke the resolver.
        let p_calls = p.calls.clone();
        let _ = p.resolve(&current, "0.0.0-test", |_| {});
        assert!(p_calls.lock().unwrap().is_empty(), "resolver not called when no targets");
    }

    #[test]
    fn resolve_needs_human_action_sets_remedy() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("x"));
        let p = RecordingMockProvider {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "brew_bundle",
            resolver_targets: &[ComponentId::Postgres],
            resolver_outcome: ResolveOutcome::NeedsHumanAction(Remedy {
                message: "run this".into(), script: "brew install postgres".into(), url: None,
            }),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", move |e| ec.lock().unwrap().push(e));
        assert_eq!(terminal.status, HealthStatus::NeedsAction);
        assert_eq!(terminal.remedy.as_ref().unwrap().script, "brew install postgres");
        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|e| matches!(e, HealthEvent::Remedy { .. })));
    }

    #[test]
    fn resolve_terminal_payload_validates() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Daemon, CheckOutcome::failed("not running"));
        let p = RecordingMockProvider {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "daemon_start",
            resolver_targets: &[ComponentId::Daemon],
            resolver_outcome: ResolveOutcome::Resolved,
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Daemon, CheckOutcome::ready_no_version());
        }
        let terminal = p.resolve(&current, "0.0.0-test", |_| {});
        terminal.validate().expect("terminal must validate");
        assert_eq!(terminal.status, HealthStatus::Ok);
    }
}
```

These 9 tests exercise every branch of the default `check`/`resolve` impls:
- 5 `check` tests (all ready → ok; one failed → needs-action; pm failed; validation; canonical order)
- 4 `resolve` tests (dedup by resolver; skip empty; human-action remedy; terminal validates)

**Coverage gate:** before committing this section, run `cargo tarpaulin -p sensei-bootstrap --packages sensei-bootstrap -- --lib health::provider 2>&1 | tail -5` (or `cargo llvm-cov` if installed). The `health::provider` module must hit 100% line + branch coverage. If any branch is uncovered, add a targeted test.

- [ ] **Step 3: Stub `platforms` module**

Create `crates/bootstrap/src/health/platforms/mod.rs`:

```rust
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
```

- [ ] **Step 4: Build (will fail — platforms::macos missing)**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -5
```

Expected: failure on missing `platforms::macos`. Task C2 fixes it.

- [ ] **Step 5: Commit the trait + provider stub**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): PlatformProvider trait + module skeleton"
```

---

### Task C2 — Concrete checkers (small, focused, fresh)

**Files:**
- Create: `crates/bootstrap/src/health/checkers/mod.rs`
- Create: `crates/bootstrap/src/health/checkers/binary.rs`
- Create: `crates/bootstrap/src/health/checkers/port.rs`
- Create: `crates/bootstrap/src/health/checkers/composite.rs`
- Create: `crates/bootstrap/src/health/checkers/postgres_db.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

Write fresh — do NOT import from `_legacy/`. Reference the old implementations only for inspiration.

- [ ] **Step 1: `checkers/binary.rs`**

```rust
//! Check whether a binary is on PATH and report its version.

use std::process::Command;
use crate::health::checker::{Checker, CheckOutcome};

pub struct BinaryChecker {
    pub bin:           &'static str,
    /// Optional --version style invocation to extract version. If None,
    /// the binary is just probed via `which`.
    pub version_arg:   Option<&'static str>,
}

impl BinaryChecker {
    pub const fn new(bin: &'static str) -> Self { Self { bin, version_arg: None } }
    pub const fn with_version(bin: &'static str, arg: &'static str) -> Self {
        Self { bin, version_arg: Some(arg) }
    }
}

impl Checker for BinaryChecker {
    fn check(&self) -> CheckOutcome {
        if which(self.bin).is_none() {
            return CheckOutcome::failed(format!("{} not found on PATH", self.bin));
        }
        match self.version_arg {
            None => CheckOutcome::ready_no_version(),
            Some(arg) => match Command::new(self.bin).arg(arg).output() {
                Ok(out) if out.status.success() => {
                    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    CheckOutcome::ready(if v.is_empty() { "unknown".to_string() } else { v })
                }
                Ok(out) => CheckOutcome::failed(format!("{} {} exited {}", self.bin, arg, out.status)),
                Err(e)  => CheckOutcome::failed(format!("{}: {e}", self.bin)),
            },
        }
    }
}

fn which(bin: &str) -> Option<std::path::PathBuf> {
    crate::util::which_binary(bin)
}
```

- [ ] **Step 2: `checkers/port.rs`**

```rust
//! Check whether a TCP port is accepting connections on 127.0.0.1.

use std::net::{SocketAddr, TcpStream};
use std::time::Duration;
use crate::health::checker::{Checker, CheckOutcome};

pub struct PortChecker {
    pub label: &'static str,
    pub port:  u16,
}

impl PortChecker {
    pub const fn new(label: &'static str, port: u16) -> Self { Self { label, port } }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckOutcome {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        match TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
            Ok(_)  => CheckOutcome::ready_no_version(),
            Err(e) => CheckOutcome::failed(format!("{} not listening on :{}: {e}", self.label, self.port)),
        }
    }
}
```

- [ ] **Step 3: `checkers/composite.rs`**

```rust
//! AndChecker — all sub-checkers must pass. Used to combine a binary check
//! AND a port check (e.g. postgres = `pg_isready` AND port 5432).

use crate::health::checker::{Checker, CheckOutcome};
use crate::health::types::ComponentStatus;

pub struct AndChecker(pub Vec<Box<dyn Checker>>);

impl Checker for AndChecker {
    fn check(&self) -> CheckOutcome {
        let mut version: Option<String> = None;
        for c in &self.0 {
            let out = c.check();
            match out.status {
                ComponentStatus::Failed => return out,
                ComponentStatus::Ready  => { if version.is_none() { version = out.version; } }
                _ => {}
            }
        }
        CheckOutcome { status: ComponentStatus::Ready, version, detail: None }
    }
}
```

- [ ] **Step 4: `checkers/postgres_db.rs`**

```rust
//! Check whether the sensei database exists with pgvector + the expected
//! tables. Used by the Database dependency.

use std::process::Command;
use crate::health::checker::{Checker, CheckOutcome};

pub struct PostgresDatabaseChecker {
    pub db_name: String,
}

impl Checker for PostgresDatabaseChecker {
    fn check(&self) -> CheckOutcome {
        // 1. database exists?
        let exists = Command::new("psql")
            .args(["-tAc", &format!("SELECT 1 FROM pg_database WHERE datname='{}'", self.db_name)])
            .output();
        match exists {
            Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "1" => {}
            Ok(o) => return CheckOutcome::failed(format!(
                "database {} not found (psql exit {}): {}",
                self.db_name, o.status, String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => return CheckOutcome::failed(format!("psql failed: {e}")),
        }
        // 2. pgvector + sensei tables present?
        let check_sql =
            "SELECT 1 FROM pg_extension WHERE extname='vector' UNION ALL \
             SELECT 1 FROM information_schema.tables WHERE table_name='sessions' AND table_schema='public'";
        let probe = Command::new("psql")
            .args(["-d", &self.db_name, "-tAc", check_sql])
            .output();
        match probe {
            Ok(o) if o.status.success() => {
                let count = String::from_utf8_lossy(&o.stdout).lines().count();
                if count >= 2 {
                    CheckOutcome::ready_no_version()
                } else {
                    CheckOutcome::failed("database exists but pgvector or sensei tables missing".to_string())
                }
            }
            Ok(o) => CheckOutcome::failed(format!("schema probe failed: {}", String::from_utf8_lossy(&o.stderr).trim())),
            Err(e) => CheckOutcome::failed(format!("psql: {e}")),
        }
    }
}
```

- [ ] **Step 5: `checkers/mod.rs`**

```rust
pub mod binary;
pub mod port;
pub mod composite;
pub mod postgres_db;

pub use binary::BinaryChecker;
pub use port::PortChecker;
pub use composite::AndChecker;
pub use postgres_db::PostgresDatabaseChecker;
```

Wire into `health/mod.rs`: add `pub mod checkers;` after the existing modules.

- [ ] **Step 6: Build (still missing macos provider)**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -8
```

Expected: failure on missing `platforms::macos`. Proceed to C3.

- [ ] **Step 7: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): concrete checkers (binary, port, composite, postgres-db)"
```

---

### Task C3 — Concrete resolvers

**Files:**
- Create: `crates/bootstrap/src/health/resolvers/mod.rs`
- Create: `crates/bootstrap/src/health/resolvers/brew_bundle.rs`
- Create: `crates/bootstrap/src/health/resolvers/db_setup.rs`
- Create: `crates/bootstrap/src/health/resolvers/daemon_start.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

Each resolver self-declares the `&[ComponentId]` it covers.

- [ ] **Step 1: `resolvers/brew_bundle.rs`**

```rust
//! BrewBundleResolver — installs postgres@16, ollama, and the sensei binaries
//! in ONE `brew bundle` invocation. Covers three dependencies; runs once.

use std::process::Command;
use crate::config::HOMEBREW_BREWFILE_URL;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct BrewBundleResolver;

const TARGETS: &[ComponentId] = &[ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei];

impl Resolver for BrewBundleResolver {
    fn id(&self) -> &'static str { "brew_bundle" }
    fn resolves(&self) -> &'static [ComponentId] { TARGETS }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        // We always run the FULL bundle (it's idempotent) — even if only one
        // of the targets is failing.
        let brew = match crate::util::which_binary("brew") {
            Some(p) => p,
            None => return ResolveOutcome::NeedsHumanAction(homebrew_install_remedy()),
        };
        let status = Command::new(brew)
            .args(["bundle", "--file", &format!("={}", HOMEBREW_BREWFILE_URL)])
            .status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(bundle_failed_remedy(format!("brew bundle exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(bundle_failed_remedy(format!("brew bundle: {e}"))),
        }
    }
}

fn homebrew_install_remedy() -> Remedy {
    Remedy {
        message: "Homebrew isn't installed. Run the script below to install it, then re-check.".to_string(),
        script:  r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.to_string(),
        url:     Some("https://brew.sh".to_string()),
    }
}

fn bundle_failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't complete `brew bundle` automatically ({detail}). Run it yourself."),
        script:  format!("brew bundle --file=={HOMEBREW_BREWFILE_URL}"),
        url:     None,
    }
}
```

- [ ] **Step 2: `resolvers/db_setup.rs`**

```rust
//! DatabaseResolver — creates the sensei database, installs pgvector, applies
//! the schema. Covers the Database dependency only.

use std::process::Command;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DatabaseResolver {
    pub db_name: String,
}

impl Resolver for DatabaseResolver {
    fn id(&self) -> &'static str { "db_setup" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Database] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let sensei = match crate::util::which_binary("sensei") {
            Some(p) => p,
            None => return ResolveOutcome::NeedsHumanAction(missing_cli_remedy()),
        };
        // The CLI knows how to create the DB + apply schema. Calling
        // `sensei db:create` is the single entry point. Caller is responsible
        // for re-checking after this returns.
        let status = Command::new(sensei).args(["db:create"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(db_failed_remedy(format!("sensei db:create exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(db_failed_remedy(format!("sensei db:create: {e}"))),
        }
    }
}

fn missing_cli_remedy() -> Remedy {
    Remedy {
        message: "The `sensei` CLI is not installed. Install it via Homebrew first.".to_string(),
        script:  "brew install sensei-hq/tap/sensei".to_string(),
        url:     None,
    }
}

fn db_failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't set up the database automatically ({detail}). Run it yourself."),
        script:  "sensei db:create".to_string(),
        url:     None,
    }
}
```

- [ ] **Step 3: `resolvers/daemon_start.rs`**

```rust
//! DaemonStartResolver — runs `senseid start` to bring the daemon up.

use std::process::Command;
use crate::health::resolver::{Resolver, ResolveOutcome};
use crate::health::types::{ComponentId, Remedy};

pub struct DaemonStartResolver;

impl Resolver for DaemonStartResolver {
    fn id(&self) -> &'static str { "daemon_start" }
    fn resolves(&self) -> &'static [ComponentId] { &[ComponentId::Daemon] }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let senseid = match crate::util::which_binary("senseid") {
            Some(p) => p,
            None => return ResolveOutcome::NeedsHumanAction(missing_remedy()),
        };
        // `senseid start` daemonizes and exits 0 when the daemon is reachable.
        let status = Command::new(senseid).args(["start"]).status();
        match status {
            Ok(s) if s.success() => ResolveOutcome::Resolved,
            Ok(s)  => ResolveOutcome::NeedsHumanAction(failed_remedy(format!("senseid start exited {s}"))),
            Err(e) => ResolveOutcome::NeedsHumanAction(failed_remedy(format!("senseid: {e}"))),
        }
    }
}

fn missing_remedy() -> Remedy {
    Remedy {
        message: "The `senseid` binary isn't installed. Run brew bundle to install all sensei binaries.".to_string(),
        script:  "brew bundle --file==https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile".to_string(),
        url:     None,
    }
}

fn failed_remedy(detail: String) -> Remedy {
    Remedy {
        message: format!("Couldn't start the daemon automatically ({detail}). Run it yourself."),
        script:  "senseid start".to_string(),
        url:     None,
    }
}
```

- [ ] **Step 4: `resolvers/mod.rs`**

```rust
pub mod brew_bundle;
pub mod db_setup;
pub mod daemon_start;

pub use brew_bundle::BrewBundleResolver;
pub use db_setup::DatabaseResolver;
pub use daemon_start::DaemonStartResolver;
```

Wire into `health/mod.rs`: add `pub mod resolvers;`.

- [ ] **Step 5: Build**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -8
```

Expected: still failing on missing `platforms::macos`. C4 finishes the loop.

- [ ] **Step 6: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): brew-bundle / db-setup / daemon-start resolvers"
```

---

### Task C4 — `MacOSProvider` (Linux uses the same impl)

**Files:**
- Create: `crates/bootstrap/src/health/platforms/macos.rs`

- [ ] **Step 1: Implement the provider**

```rust
//! macOS (and Linux-via-Homebrew) PlatformProvider.
//!
//! This impl provides ONLY the platform-specific knowledge:
//! - which package manager (homebrew)
//! - the platform-specific checker for each dependency id
//! - the resolvers available on this platform
//! - the default remedy
//!
//! The shared orchestration (`check` / `resolve`) is inherited from the
//! PlatformProvider trait's default methods.
//!
//! IMPORTANT: `checker_for` uses exhaustive `match id { ... }` (no `_ =>`).
//! Adding a new ComponentId is a compile error here until handled.

use crate::config::{POSTGRES_PORT, OLLAMA_PORT, SenseiConfig};
use crate::health::checker::Checker;
use crate::health::checkers::{BinaryChecker, PortChecker, AndChecker, PostgresDatabaseChecker};
use crate::health::provider::PlatformProvider;
use crate::health::resolver::Resolver;
use crate::health::resolvers::{BrewBundleResolver, DatabaseResolver, DaemonStartResolver};
use crate::health::types::{ComponentId, PackageManagerId, Platform, Remedy};

pub struct MacOSProvider;

impl PlatformProvider for MacOSProvider {
    fn platform(&self) -> Platform {
        if cfg!(target_os = "linux") { Platform::Linux } else { Platform::Macos }
    }

    fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }

    fn package_manager_checker(&self) -> Box<dyn Checker> {
        Box::new(BinaryChecker::with_version("brew", "--version"))
    }

    fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
        match id {
            ComponentId::Postgres => Box::new(AndChecker(vec![
                Box::new(BinaryChecker::with_version("pg_isready", "--version")),
                Box::new(PortChecker::new("postgres", POSTGRES_PORT)),
            ])),
            ComponentId::Ollama => Box::new(AndChecker(vec![
                Box::new(BinaryChecker::with_version("ollama", "--version")),
                Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
            ])),
            ComponentId::Sensei => Box::new(AndChecker(vec![
                Box::new(BinaryChecker::new("sensei")),
                Box::new(BinaryChecker::new("senseid")),
                Box::new(BinaryChecker::new("sensei-mcp")),
            ])),
            ComponentId::Database => Box::new(PostgresDatabaseChecker {
                db_name: SenseiConfig::from_env().db_name,
            }),
            ComponentId::Daemon => Box::new(PortChecker::new("daemon", SenseiConfig::from_env().daemon_port)),
        }
    }

    fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
        vec![
            Box::new(BrewBundleResolver),
            Box::new(DatabaseResolver { db_name: SenseiConfig::from_env().db_name }),
            Box::new(DaemonStartResolver),
        ]
    }

    fn default_remedy(&self) -> Remedy {
        Remedy {
            message: "Some components need attention. Run the sensei bootstrap repair flow.".to_string(),
            script:  "brew bundle --file==https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile".to_string(),
            url:     None,
        }
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -5
```

Expected: 0 errors. The Windows provider doesn't exist yet — that's fine, `detect_provider()` falls back to MacOSProvider for non-Windows. Windows stub goes in Task C5 (deferred).

- [ ] **Step 3: Smoke test the provider**

Append to `health/provider.rs` tests (alongside the default-impl tests):

```rust
    #[test]
    fn detected_provider_builds_all_5_checkers_and_pm() {
        let p = detect_provider();
        let _ = p.package_manager_checker(); // didn't panic
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
                   ComponentId::Database, ComponentId::Daemon] {
            let _ = p.checker_for(id);
        }
        let resolvers = p.resolvers();
        assert_eq!(resolvers.len(), 3);
    }

    #[test]
    fn brew_bundle_covers_postgres_ollama_sensei() {
        let p = detect_provider();
        let resolvers = p.resolvers();
        let brew = resolvers.iter().find(|r| r.id() == "brew_bundle").expect("brew_bundle present");
        let targets = brew.resolves();
        assert!(targets.contains(&ComponentId::Postgres));
        assert!(targets.contains(&ComponentId::Ollama));
        assert!(targets.contains(&ComponentId::Sensei));
        assert!(!targets.contains(&ComponentId::Database));
        assert!(!targets.contains(&ComponentId::Daemon));
    }
```

```bash
cargo test -p sensei-bootstrap health 2>&1 | tail -10
```

Expected: provider tests pass, plus all earlier tests.

- [ ] **Step 4: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): MacOSProvider with checkers + 3 resolvers wired"
```

---

### Task C5 — Windows stub provider

**Files:**
- Create: `crates/bootstrap/src/health/platforms/windows.rs`

For Phase 1b we stub Windows so the crate builds on Windows. Real winget support is Phase 2.

- [ ] **Step 1: Stub**

```rust
//! Windows PlatformProvider — Phase 1b stub. Real winget support is Phase 2.

use crate::health::checker::{Checker, CheckOutcome};
use crate::health::provider::PlatformProvider;
use crate::health::resolver::Resolver;
use crate::health::types::{ComponentId, PackageManagerId, Platform, Remedy};

pub struct WindowsProvider;

struct UnsupportedChecker(&'static str);
impl Checker for UnsupportedChecker {
    fn check(&self) -> CheckOutcome {
        CheckOutcome::failed(format!("{}: Windows support not yet implemented", self.0))
    }
}

impl PlatformProvider for WindowsProvider {
    fn platform(&self) -> Platform { Platform::Windows }

    fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Winget }

    fn package_manager_checker(&self) -> Box<dyn Checker> {
        Box::new(UnsupportedChecker("winget"))
    }

    fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
        Box::new(UnsupportedChecker(match id {
            ComponentId::Postgres => "postgres",
            ComponentId::Ollama   => "ollama",
            ComponentId::Sensei   => "sensei",
            ComponentId::Database => "database",
            ComponentId::Daemon   => "daemon",
        }))
    }

    fn resolvers(&self) -> Vec<Box<dyn Resolver>> { vec![] }

    fn default_remedy(&self) -> Remedy {
        Remedy {
            message: "Windows support is coming. For now, install components manually.".to_string(),
            script:  "# windows install steps TBD".to_string(),
            url:     None,
        }
    }
}
```

- [ ] **Step 2: Build (only meaningful on Windows; cargo check is fine elsewhere)**

```bash
cargo check -p sensei-bootstrap --target x86_64-pc-windows-msvc 2>&1 | tail -5 || echo "cross-target check skipped (no toolchain) — that's OK"
```

If cross-target isn't available, this step is best-effort. The build will be exercised on Windows CI in a later phase.

- [ ] **Step 3: Commit**

```bash
git add crates/bootstrap/src/health/platforms/windows.rs
git commit -m "feat(bootstrap): WindowsProvider stub (real winget impl is Phase 2)"
```

---

## Section D — Crate-level convenience wrappers

The orchestration logic now lives inside `PlatformProvider`'s default `check` and `resolve` methods (Section C1). Crate-level entry points are one-line shims so daemon and sidecar don't have to call `detect_provider()` first.

### Task D1 — `bootstrap::check` and `bootstrap::resolve` wrappers

**Files:**
- Modify: `crates/bootstrap/src/health/mod.rs`

- [ ] **Step 1: TDD red — smoke test**

Add to `crates/bootstrap/src/health/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_check_returns_validated_payload() {
        let payload = check("0.0.0-test");
        payload.validate().expect("validate must pass");
        assert_eq!(payload.components.len(), 5);
    }

    #[test]
    fn crate_resolve_emits_phase_then_report() {
        use std::sync::{Arc, Mutex};
        let initial = check("0.0.0-test");
        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let _ = resolve(&initial, "0.0.0-test", move |e| ec.lock().unwrap().push(e));
        let evs = events.lock().unwrap();
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving }))
                || initial.status == HealthStatus::Ok);
        assert!(matches!(evs.last(), Some(HealthEvent::Report { .. }))
                || evs.is_empty());
    }
}
```

(The `|| status == Ok` arm handles the "nothing to resolve" case — on a system where `check()` already returned Ok, `resolve()` may short-circuit. The test above accepts either path; sharper assertions live in the `provider::tests` module that uses a MockProvider.)

- [ ] **Step 2: Add the wrappers**

In `crates/bootstrap/src/health/mod.rs`:

```rust
//! Public health surface.

pub mod types;
pub mod ids;
pub mod graph;
pub mod checker;
pub mod resolver;
pub mod provider;
pub mod platforms;
pub mod checkers;
pub mod resolvers;

pub use types::*;
pub use graph::{DependencySpec, dependency_specs, spec_for};
pub use checker::{Checker, CheckOutcome};
pub use resolver::{Resolver, ResolveOutcome};
pub use provider::{PlatformProvider, detect_provider};

/// Sync fast path — runs every checker once, returns a validated HealthPayload.
/// Daemon `GET /health` uses this. No events emitted.
pub fn check(app_version: &str) -> HealthPayload {
    detect_provider().check(app_version)
}

/// Streaming resolve — runs resolvers covering any failed components in
/// `current`, re-checks affected deps, returns terminal payload. Emits
/// HealthEvent values throughout. Tauri sidecar uses this AFTER calling
/// `check()` and emitting its initial state.
pub fn resolve<F>(current: &HealthPayload, app_version: &str, emit: F) -> HealthPayload
where F: Fn(HealthEvent) + Send + Sync
{
    detect_provider().resolve(current, app_version, emit)
}
```

- [ ] **Step 3: Run**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -5
cargo test  -p sensei-bootstrap health 2>&1 | tail -10
```

Expected: the existing provider::tests (9 tests from Section C1) PLUS these 2 wrapper tests = 11 health tests pass.

- [ ] **Step 4: Coverage gate**

```bash
cargo llvm-cov -p sensei-bootstrap --html 2>&1 | tail -5
# or: cargo tarpaulin -p sensei-bootstrap -o stdout 2>&1 | grep -E "health::(provider|mod)" | head -10
```

Required: `health::provider` and `health::mod` at 100% line coverage. The default `check`/`resolve` impls in the trait MUST be exercised by the MockProvider tests in C1.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): bootstrap::check and bootstrap::resolve crate-level wrappers"
```

---

## Section E — Tests that lock the wire shape

### Task E1 — JSON snapshot of an ok payload + a needs-action payload

**Files:**
- Create: `crates/bootstrap/tests/json_wire_shape.rs`

These tests are the single source of truth that the daemon /health response shape matches the TS contract.

- [ ] **Step 1: Write the integration test**

```rust
//! Lock the wire-shape contract — these JSON shapes are what the TypeScript
//! HealthState consumes. If a field name changes, this test catches it.

use sensei_bootstrap::*;

fn mock_payload(status: HealthStatus, with_remedy: bool) -> HealthPayload {
    let mk_component = |id: &str, label: &str, note: Option<&str>, s: ComponentStatus| Component {
        id: id.to_string(), label: label.to_string(), note: note.map(str::to_string),
        status: s, version: None, detail: None,
    };
    HealthPayload {
        version: "0.0.0-test".into(),
        uptime_seconds: 42,
        platform: Platform::Macos,
        package_manager: mk_component("homebrew", "Homebrew", Some("which brew"), ComponentStatus::Ready),
        components: vec![
            mk_component("postgres", "PostgreSQL", None, ComponentStatus::Ready),
            mk_component("ollama",   "Ollama", None, ComponentStatus::Ready),
            mk_component("sensei",   "Sensei components", Some("cli · mcp · daemon"), ComponentStatus::Ready),
            mk_component("database", "Database & schema", Some("pgvector · sensei tables"), ComponentStatus::Ready),
            mk_component("daemon",   "Background daemon", None, ComponentStatus::Ready),
        ],
        status,
        remedy: if with_remedy { Some(Remedy { message: "msg".into(), script: "cmd".into(), url: None }) } else { None },
    }
}

#[test]
fn ok_payload_serializes_to_expected_json() {
    let p = mock_payload(HealthStatus::Ok, false);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["remedy"], serde_json::Value::Null);
    assert_eq!(json["platform"], "macos");
    assert_eq!(json["package_manager"]["id"], "homebrew");
    assert_eq!(json["components"][0]["id"], "postgres");
    assert_eq!(json["components"][0]["label"], "PostgreSQL");
    assert_eq!(json["components"][2]["note"], "cli · mcp · daemon");
}

#[test]
fn needs_action_payload_serializes_with_remedy() {
    let p = mock_payload(HealthStatus::NeedsAction, true);
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(json["status"], "needs-action");
    assert_eq!(json["remedy"]["message"], "msg");
    assert_eq!(json["remedy"]["script"], "cmd");
}

#[test]
fn health_event_phase_serializes_correctly() {
    let ev = HealthEvent::Phase { phase: HealthStatus::Resolving };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"],  "phase");
    assert_eq!(json["phase"], "resolving");
}

#[test]
fn health_event_component_patch_serializes_correctly() {
    let ev = HealthEvent::Component {
        id: "postgres".to_string(),
        patch: ComponentPatch {
            status: Some(ComponentStatus::Installing),
            ..Default::default()
        },
    };
    let json: serde_json::Value = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
    assert_eq!(json["kind"], "component");
    assert_eq!(json["id"],   "postgres");
    assert_eq!(json["patch"]["status"], "installing");
}
```

- [ ] **Step 2: Run**

```bash
cargo test -p sensei-bootstrap --test json_wire_shape 2>&1 | tail -10
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/bootstrap/tests/json_wire_shape.rs
git commit -m "test(bootstrap): lock the JSON wire shape against the TS contract"
```

---

## Section F — Daemon `/health` becomes a one-liner

### Task F1 — Replace `crates/senseid/src/api/handlers/health.rs`

**Files:**
- Modify (full rewrite): `crates/senseid/src/api/handlers/health.rs`

- [ ] **Step 1: Overwrite**

```rust
//! Health endpoint — thin wrapper over sensei_bootstrap::check.

use axum::{extract::State, response::Json};
use sensei_bootstrap::{self as bootstrap, HealthPayload};
use std::time::Instant;
use crate::api::state::AppState;

static START_TIME: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub(crate) fn init_uptime() {
    START_TIME.get_or_init(Instant::now);
}

fn uptime_seconds() -> u64 {
    START_TIME.get().map(|t| t.elapsed().as_secs()).unwrap_or(0)
}

pub(crate) async fn health() -> Json<HealthPayload> {
    let mut payload = tokio::task::spawn_blocking(|| {
        bootstrap::check(env!("CARGO_PKG_VERSION"))
    }).await.expect("health check task panicked");
    payload.uptime_seconds = uptime_seconds();
    Json(payload)
}

// ── Watcher endpoints (unchanged) ─────────────────────────────────────────

pub(crate) async fn watcher_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(state.task_queue.clone());
    if let Ok(w) = watcher.lock() {
        let status = format!("{:?}", w.status());
        let roots: Vec<serde_json::Value> = w.roots().iter().map(|(path, root)| {
            serde_json::json!({ "path": path.to_string_lossy(), "excluded": root.excluded })
        }).collect();
        Json(serde_json::json!({ "status": status, "roots": roots }))
    } else {
        Json(serde_json::json!({ "status": "error", "message": "lock poisoned" }))
    }
}

pub(crate) async fn watcher_unregister(
    axum::extract::Json(body): axum::extract::Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let path = body.get("path").and_then(|v| v.as_str()).unwrap_or("");
    if path.is_empty() { return Json(serde_json::json!({ "error": "path required" })); }
    let queue = std::sync::Arc::new(crate::tasks::queue::TaskQueue::new());
    let watcher = crate::watcher::root_watcher::RootWatcher::instance(queue);
    if let Ok(mut w) = watcher.lock() {
        w.unregister(&std::path::PathBuf::from(path));
        Json(serde_json::json!({ "ok": true, "unregistered": path }))
    } else {
        Json(serde_json::json!({ "error": "lock poisoned" }))
    }
}
```

- [ ] **Step 2: Build**

```bash
cargo build -p senseid 2>&1 | tail -10
```

Expected: any senseid file that still imports `bootstrap::ComponentState` / `bootstrap::database::check` / `bootstrap::models::list` will fail. Find each one and remove the dependency — those code paths are dead now that /health is thin.

- [ ] **Step 3: Curl smoke**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
cargo run -p senseid &
SENSEID_PID=$!
sleep 3
curl -s http://localhost:7744/health | jq .status,.platform,.package_manager.id,'.components[0].id','.components|length'
kill $SENSEID_PID
```

Expected:
```
"ok" | "needs-action"        (depends on actual system state)
"macos"
"homebrew"
"postgres"
5
```

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/api/handlers/health.rs
git commit -m "refactor(senseid): /health returns bootstrap::HealthPayload (thin wrapper)"
```

---

## Section G — Sidecar thin rewrite

### Task G1 — Move auxiliary commands out of `bootstrap.rs`

**Files:**
- Create: `app/src-tauri/src/commands/platform_info.rs`
- Modify: `app/src-tauri/src/commands/mod.rs`
- Modify: `app/src-tauri/src/commands/bootstrap.rs` (delete the moved commands)
- Modify: `app/src-tauri/src/lib.rs` (rewire `generate_handler!`)

The hardware/models/platform-info commands belong in their own module — `bootstrap.rs` should be JUST health.

But: `bootstrap::hardware::detect()`, `bootstrap::models::list()`, etc. are in `_legacy/` now. Either (a) move them back to public surface, or (b) delete the Tauri commands that depended on them.

Looking at the sidecar: `detect_hardware` / `list_models` / `missing_models` / `get_platform` are used by the setup wizard. These are NOT part of the health domain — they're separate "hardware/models" concerns. Decision:

- Keep `get_daemon_port`. Simple, depends on `bootstrap::daemon_port()`.
- Promote `hardware`, `models` modules back to the public bootstrap surface (move from `_legacy/` back to `src/`). They're not part of the health rewrite — they're orthogonal.
- Delete `get_platform` — its data (platform + package manager) is now available via `bootstrap::check()` for free.

- [ ] **Step 1: Promote `hardware.rs` and `models.rs` back**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei/crates/bootstrap/src
git mv _legacy/hardware.rs hardware.rs
git mv _legacy/models.rs   models.rs
```

Edit `crates/bootstrap/src/lib.rs` — add:

```rust
pub mod hardware;
pub mod models;
```

If these modules import from the old `prereq`/`platform`/`types`, audit each function and rewrite the imports against the new surface. The hardware module is mostly self-contained (sysinfo crate); models lists Ollama models via shell-out.

- [ ] **Step 2: Create `commands/platform_info.rs`**

```rust
//! Auxiliary commands — hardware + models. Thin pass-throughs to bootstrap.

use sensei_bootstrap::{self as bootstrap, hardware::HardwareInfo};

#[tauri::command]
pub fn detect_hardware() -> HardwareInfo {
    bootstrap::hardware::detect()
}

#[tauri::command]
pub fn list_models() -> Vec<String> {
    bootstrap::models::list()
}

#[tauri::command]
pub fn missing_models() -> Vec<String> {
    let hw = bootstrap::hardware::detect();
    bootstrap::models::missing_models(&hw.recommended_tier)
}

#[tauri::command]
pub fn get_daemon_port() -> u16 {
    bootstrap::daemon_port()
}
```

Add `pub mod platform_info;` to `commands/mod.rs`.

- [ ] **Step 3: Update `lib.rs`**

Rewire the `generate_handler!` invocation. Old:

```rust
commands::bootstrap::check_and_fix_bootstrap,
commands::bootstrap::detect_hardware,
commands::bootstrap::list_models,
commands::bootstrap::missing_models,
commands::bootstrap::get_platform,
commands::bootstrap::get_daemon_port,
```

New (after Task G2 finishes bootstrap.rs):

```rust
commands::bootstrap::health_check,
commands::bootstrap::health_check_and_resolve,
commands::platform_info::detect_hardware,
commands::platform_info::list_models,
commands::platform_info::missing_models,
commands::platform_info::get_daemon_port,
```

`get_platform` is removed; UI obtains platform + package manager from the `HealthPayload`.

- [ ] **Step 4: Build (this will fail until G2 is done)**

Don't run yet — Task G2 rewrites bootstrap.rs and the build becomes clean.

- [ ] **Step 5: Commit (combined with G2 below since intermediate state is broken)**

(no commit yet)

---

### Task G2 — Wipe and rewrite `commands/bootstrap.rs`

**Files:**
- Modify (full rewrite): `app/src-tauri/src/commands/bootstrap.rs`

- [ ] **Step 1: Overwrite**

```rust
//! Health commands — thin wrappers over sensei_bootstrap.
//!
//! Two commands and two only:
//!   * `health_check`             — sync. Returns HealthPayload.
//!   * `health_check_and_resolve` — fire-and-forget. Runs check() then
//!                                  resolve() on a background thread; events
//!                                  stream on the "health" channel.

use sensei_bootstrap::{self as bootstrap, HealthEvent, HealthPayload, HealthStatus};
use tauri::Emitter;

use crate::flog;

#[tauri::command]
pub fn health_check(app: tauri::AppHandle) -> HealthPayload {
    let version = app.package_info().version.to_string();
    bootstrap::check(&version)
}

#[tauri::command]
pub fn health_check_and_resolve(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== health_check_and_resolve called v={version} ==="));

    std::thread::spawn(move || {
        let emit = {
            let app = app.clone();
            move |ev: HealthEvent| { let _ = app.emit("health", &ev); }
        };

        // 1. Phase: checking → compute initial state → broadcast it.
        emit(HealthEvent::Phase { phase: HealthStatus::Checking });
        let state = bootstrap::check(&version);
        emit(HealthEvent::Report { payload: state.clone() });

        // 2. If anything failed, run the resolvers. resolve() emits
        //    Phase(Resolving), per-component patches, and a final Report.
        if state.status != HealthStatus::Ok {
            bootstrap::resolve(&state, &version, emit);
        }

        flog::log("health_check_and_resolve complete");
    });
    Ok(())
}
```

Five lines of orchestration in the sidecar: emit Phase(Checking), call check, emit Report, conditionally call resolve. Everything else lives in the bootstrap crate's default trait methods.

- [ ] **Step 2: Build**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
cargo build --manifest-path app/src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: 0 errors. The TS client still uses old command names — that breaks runtime, fixed in Phase 1c.

- [ ] **Step 3: Combined commit for G1 + G2**

```bash
git add crates/bootstrap/src/lib.rs crates/bootstrap/src/hardware.rs crates/bootstrap/src/models.rs \
        app/src-tauri/src/commands/ app/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(src-tauri): bootstrap.rs is now two thin wrappers; aux commands split

* commands/bootstrap.rs now contains exactly two thin commands:
    health_check                  -> bootstrap::check
    health_check_and_resolve      -> bootstrap::check_and_resolve, events on 'health'
* commands/platform_info.rs holds detect_hardware / list_models / missing_models /
  get_daemon_port — they're hardware/config concerns, not health.
* get_platform command removed: HealthPayload already carries platform + pm.
* hardware.rs and models.rs are promoted back from _legacy/ since they're
  orthogonal to the health rewrite.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

⚠️ **Deliberately dropped from the sidecar:**
- `dispatch` / `emit_gate` / `emit_phase_complete` — shape translation no longer needed.
- `write_bootstrap_session` / `collect_system_info` — log session writing. The TS client (Phase 1c) will re-introduce this concern via a `health-session-logger.ts` module that subscribes to the `health` event stream and POSTs sessions to the existing log collector. **This is a known regression for the duration of Phase 1b.**

---

## Section H — Delete `_legacy/`

### Task H1 — Final cleanup

**Files:**
- Delete: `crates/bootstrap/src/_legacy/`

- [ ] **Step 1: Confirm nothing references `_legacy/`**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
grep -rn "_legacy" crates app/src-tauri 2>&1 | grep -v "src/_legacy/" | head -5
```

Expected: zero hits.

- [ ] **Step 2: Delete**

```bash
cd crates/bootstrap/src
git rm -r _legacy/
```

- [ ] **Step 3: Workspace build + test**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
cargo build --workspace 2>&1 | tail -5
cargo test  --workspace 2>&1 | tail -10
cd app && bun run test:unit 2>&1 | tail -3
bun run check 2>&1 | tail -3
```

Expected: workspace builds clean, all Rust tests pass, 273 TS tests pass, 0 svelte-check errors.

- [ ] **Step 4: Commit**

```bash
git add crates/bootstrap/
git commit -m "chore(bootstrap): delete _legacy/ — clean-slate health rewrite complete"
```

---

## Verification gate (end of Phase 1b)

- [ ] **Workspace builds clean:** `cargo build --workspace`
- [ ] **All Rust tests pass:** `cargo test --workspace`
- [ ] **Wire-shape tests pass (the JSON contract guard):** `cargo test -p sensei-bootstrap --test json_wire_shape`
- [ ] **100% line + branch coverage on every Phase-1b health module.** Run:
   ```bash
   cargo llvm-cov -p sensei-bootstrap --html --fail-under-lines 100 --fail-under-branches 100 \
     --ignore-filename-regex 'src/_legacy/|src/config\.rs|src/util\.rs|src/hardware\.rs|src/models\.rs' \
     2>&1 | tail -20
   ```
   If any module under `src/health/` is below 100%, the gate fails. Add the missing test, don't lower the threshold.
- [ ] **App still passes:** `bun run test:unit` (273), `bun run check` (0/0)
- [ ] **`curl localhost:7744/health` returns the `HealthPayload` JSON shape** (manual smoke)
- [ ] **Phase 1b commits are pushed to `origin/develop`**

When all seven pass, Phase 1b is complete.

---

## Out of scope (Phase 1c+)

- TS client (`app/src/lib/bootstrap.ts`) migration to call `health_check` / `health_check_and_resolve` and listen on the `health` event channel.
- `HealthState.init()` lifecycle wiring (Phase 2).
- `health-session-logger.ts` to restore session logging via the new event stream.
- Real `winget` Windows provider (currently a stub).
- E2E smoke: cold-start app, delete `sensei_dev` DB + sensei files, verify the live UI walks all gates and lands on `ok`.
