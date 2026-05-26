# Health State — Phase 1b Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Rust `sensei_bootstrap` crate the single source of truth for the data shape `HealthState` (TypeScript) expects. Daemon `/health` and Tauri sidecar become thin transport-only wrappers that don't reshape data.

**Architecture:** The TS `HealthPayload` / `HealthEvent` types defined in `app/src/lib/health-types.ts` are the contract. Bootstrap exposes two functions matching the TS transport contract:
- `check() -> HealthPayload` — sync, no fixes. Called by daemon `GET /health`.
- `check_and_resolve<F: Fn(HealthEvent)>(emit) -> HealthPayload` — runs full pipeline, emits events, returns terminal payload. Called by Tauri sidecar.

The crate's 10 internal gates aggregate into the 5 public components (postgres = postgresql + service; ollama = ollama + service; sensei = sensei + senseid + sensei_mcp; database; daemon). UI never sees the granular gates.

**Tech Stack:** Rust (sensei_bootstrap, senseid, src-tauri), serde, axum, Tauri 2.

**Source spec:** `docs/superpowers/specs/2026-05-12-health-state-design.md` Section 3 (types) — the TS contract Rust must match.

**Source TS:** `app/src/lib/health-types.ts`, `app/src/lib/health-state.svelte.ts`.

---

## Pre-flight

- [ ] **Confirm baseline**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
git rev-parse --abbrev-ref HEAD              # develop
git status --short                           # clean (or only WIP files unrelated to bootstrap)
cargo build -p sensei-bootstrap 2>&1 | tail -3
cargo test  -p sensei-bootstrap 2>&1 | tail -5
cargo build -p senseid 2>&1 | tail -3
```

Expected: bootstrap crate builds clean, all tests pass. Note the test count — this is the baseline.

- [ ] **Confirm app baseline**

```bash
cd app && bun run test:unit 2>&1 | tail -3
```

Expected: 273 tests pass (unchanged from Phase 1a).

---

## Section A — Bootstrap crate type alignment

These tasks introduce the new public surface alongside the existing one. After Section B and C migrate consumers, Section D removes the old surface.

### Task A1 — Add the `health` module with TS-matching types

**Files:**
- Create: `crates/bootstrap/src/health/mod.rs`
- Create: `crates/bootstrap/src/health/types.rs`
- Modify: `crates/bootstrap/src/lib.rs` (add `pub mod health;`)

- [ ] **Step 1: Create `crates/bootstrap/src/health/types.rs`**

```rust
//! Public types that match the TypeScript `HealthState` contract exactly.
//! See: app/src/lib/health-types.ts
//!
//! Every type here is the wire-level shape — daemon /health serializes it
//! directly, and Tauri sidecar emits `HealthEvent` values verbatim.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform { Macos, Linux, Windows }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthStatus { Checking, Resolving, Ok, NeedsAction }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus { Pending, Checking, Installing, Ready, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ComponentId { Postgres, Ollama, Sensei, Database, Daemon }

pub const COMPONENT_ORDER: [ComponentId; 5] = [
    ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
    ComponentId::Database, ComponentId::Daemon,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManagerId { Homebrew, Winget }

/// Generic component shape — used for ledger rows AND the package manager.
/// `id` is either a ComponentId or a PackageManagerId, serialized as a flat
/// lowercase string. We model it as `String` here because serde-tagged enums
/// over a union of two enums is awkward; runtime validation lives in the
/// constructors.
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

/// The top-level payload returned by `check()` / `check_and_resolve()`.
///
/// Field-for-field equivalent to the TS `HealthPayload` discriminated union.
/// The TS side enforces `status === 'needs-action' ⇔ remedy !== null`; we
/// enforce the same invariant via a `validate()` method (Section A4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPayload {
    pub version:         String,
    pub uptime_seconds:  u64,
    pub platform:        Platform,
    pub package_manager: Component,
    pub components:      Vec<Component>,   // exactly 5, ordered per COMPONENT_ORDER
    pub status:          HealthStatus,
    pub remedy:          Option<Remedy>,
}

/// Streaming events emitted by `check_and_resolve()` between the entry and
/// terminal report. Mirrors the TS `HealthEvent` discriminated union.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum HealthEvent {
    Phase   { phase: HealthStatus },     // only 'checking' or 'resolving' will appear at runtime
    Component { id: String, patch: ComponentPatch },
    Remedy  { remedy: Remedy },
    Report  { payload: HealthPayload },  // terminal
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentPatch {
    #[serde(skip_serializing_if = "Option::is_none")] pub label:   Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub note:    Option<Option<String>>, // double-Option lets us pass null
    #[serde(skip_serializing_if = "Option::is_none")] pub status:  Option<ComponentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")] pub version: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub detail:  Option<Option<String>>,
}
```

Note: `ComponentPatch` uses `Option<Option<T>>` so the deserialized JSON `{"detail": null}` is distinguishable from `{}` — matches TS's `Partial<Component>` semantics where a field can be explicitly set to null.

- [ ] **Step 2: Create `crates/bootstrap/src/health/mod.rs`**

```rust
//! Public health surface — TS-matching types, defaults, aggregation, and
//! the two entry points consumed by daemon and sidecar.

pub mod types;

pub use types::*;
```

- [ ] **Step 3: Wire it into `crates/bootstrap/src/lib.rs`**

After the existing `pub mod ...;` lines, add:

```rust
pub mod health;
```

Re-export the new types at the crate root (so consumers can `use sensei_bootstrap::HealthPayload;`):

After the existing `pub use types::*;` line, add:

```rust
pub use health::*;
```

- [ ] **Step 4: Build check**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei && cargo build -p sensei-bootstrap 2>&1 | tail -5
```

Expected: 0 errors, 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/ crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): add health module with TS-matching types"
```

---

### Task A2 — Component display defaults

**Files:**
- Create: `crates/bootstrap/src/health/defaults.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

- [ ] **Step 1: Write the failing test (TDD)**

Append to `crates/bootstrap/src/health/types.rs` inside a `#[cfg(test)] mod tests { ... }` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::defaults::{component_default, package_manager_default};

    #[test]
    fn component_defaults_match_ts_labels() {
        let cases = [
            (ComponentId::Postgres, "PostgreSQL @16",     None),
            (ComponentId::Ollama,   "Ollama",             None),
            (ComponentId::Sensei,   "Sensei components",  Some("cli · mcp · daemon")),
            (ComponentId::Database, "Database & schema",  Some("pgvector · sensei tables")),
            (ComponentId::Daemon,   "Background daemon",  None),
        ];
        for (id, label, note) in cases {
            let c = component_default(id);
            assert_eq!(c.label, label, "label for {:?}", id);
            assert_eq!(c.note.as_deref(), note, "note for {:?}", id);
            assert!(matches!(c.status, ComponentStatus::Pending));
        }
    }

    #[test]
    fn package_manager_default_per_platform() {
        let mac   = package_manager_default(Platform::Macos);
        let linux = package_manager_default(Platform::Linux);
        let win   = package_manager_default(Platform::Windows);
        assert_eq!(mac.id,   "homebrew");
        assert_eq!(mac.label, "Homebrew");
        assert_eq!(mac.note.as_deref(), Some("which brew"));
        assert_eq!(linux.id, "homebrew");
        assert_eq!(win.id,   "winget");
        assert_eq!(win.label, "winget");
        assert_eq!(win.note.as_deref(), Some("winget --version"));
    }
}
```

- [ ] **Step 2: Run test, expect RED**

```bash
cargo test -p sensei-bootstrap health::types::tests 2>&1 | tail -10
```

Expected: `defaults` module not found.

- [ ] **Step 3: Implement `crates/bootstrap/src/health/defaults.rs`**

```rust
//! Display defaults for the public component surface. The UI never has to
//! invent its own labels — the bootstrap crate owns the canonical strings.

use super::types::{Component, ComponentId, ComponentStatus, Platform};

pub fn component_default(id: ComponentId) -> Component {
    let (label, note): (&str, Option<&str>) = match id {
        ComponentId::Postgres => ("PostgreSQL @16",     None),
        ComponentId::Ollama   => ("Ollama",             None),
        ComponentId::Sensei   => ("Sensei components",  Some("cli · mcp · daemon")),
        ComponentId::Database => ("Database & schema",  Some("pgvector · sensei tables")),
        ComponentId::Daemon   => ("Background daemon",  None),
    };
    Component {
        id:      id_str(id).to_string(),
        label:   label.to_string(),
        note:    note.map(str::to_string),
        status:  ComponentStatus::Pending,
        version: None,
        detail:  None,
    }
}

pub fn package_manager_default(platform: Platform) -> Component {
    let (id, label, note) = match platform {
        Platform::Macos | Platform::Linux => ("homebrew", "Homebrew", "which brew"),
        Platform::Windows                 => ("winget",   "winget",   "winget --version"),
    };
    Component {
        id:      id.to_string(),
        label:   label.to_string(),
        note:    Some(note.to_string()),
        status:  ComponentStatus::Pending,
        version: None,
        detail:  None,
    }
}

pub fn id_str(id: ComponentId) -> &'static str {
    match id {
        ComponentId::Postgres => "postgres",
        ComponentId::Ollama   => "ollama",
        ComponentId::Sensei   => "sensei",
        ComponentId::Database => "database",
        ComponentId::Daemon   => "daemon",
    }
}
```

Update `crates/bootstrap/src/health/mod.rs`:

```rust
pub mod types;
pub mod defaults;

pub use types::*;
pub use defaults::{component_default, package_manager_default};
```

- [ ] **Step 4: Run test, expect GREEN**

```bash
cargo test -p sensei-bootstrap health 2>&1 | tail -10
```

Expected: 2 new tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): add component + package-manager display defaults"
```

---

### Task A3 — Internal → public component aggregation

**Files:**
- Create: `crates/bootstrap/src/health/aggregate.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

The crate's internal gates (`postgresql`, `postgresql_service`, `ollama`, `ollama_service`, `sensei`, `senseid`, `sensei_mcp`, `database`, `daemon`, `homebrew`) collapse into the 5 public components + 1 package manager. Rules:

- `homebrew` (or `winget` on Windows) → `package_manager`. Status = its own status.
- `postgresql` + `postgresql_service` → `postgres`. Status = `failed` if either is failed; `ready` only if both ready; otherwise the most-in-progress status (`installing` > `checking` > `pending`).
- `ollama` + `ollama_service` → `ollama`. Same rule.
- `sensei` + `senseid` + `sensei_mcp` → `sensei`. Same rule across three sub-gates.
- `database` → `database`. 1:1.
- `daemon` → `daemon`. 1:1.

`detail` and `version` for aggregated components: pick the first failed sub-gate's `detail`; pick any ready sub-gate's `version` (they should agree).

- [ ] **Step 1: Failing tests**

Create `crates/bootstrap/src/health/aggregate.rs` with skeleton + tests at the bottom:

```rust
//! Aggregate the internal 10-gate map into the 5 public components.

use std::collections::HashMap;
use super::types::{Component, ComponentId, ComponentStatus};
use super::defaults::{component_default, id_str};

/// One internal gate's current state, as understood by the engine.
#[derive(Debug, Clone)]
pub struct InternalGate {
    pub id:      String,
    pub status:  ComponentStatus,
    pub version: Option<String>,
    pub detail:  Option<String>,
}

pub type GateMap = HashMap<String, InternalGate>;

pub fn aggregate(id: ComponentId, gates: &GateMap) -> Component {
    // implementation in step 3
    let _ = (id, gates);
    component_default(id)  // stub
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(id: &str, status: ComponentStatus) -> InternalGate {
        InternalGate { id: id.to_string(), status, version: None, detail: None }
    }
    fn map(gates: Vec<InternalGate>) -> GateMap {
        gates.into_iter().map(|g| (g.id.clone(), g)).collect()
    }

    #[test]
    fn postgres_ready_when_both_subs_ready() {
        let gates = map(vec![
            gate("postgresql",         ComponentStatus::Ready),
            gate("postgresql_service", ComponentStatus::Ready),
        ]);
        let c = aggregate(ComponentId::Postgres, &gates);
        assert!(matches!(c.status, ComponentStatus::Ready));
        assert_eq!(c.id, "postgres");
    }

    #[test]
    fn postgres_failed_when_either_sub_failed() {
        let gates = map(vec![
            gate("postgresql",         ComponentStatus::Ready),
            gate("postgresql_service", ComponentStatus::Failed),
        ]);
        let c = aggregate(ComponentId::Postgres, &gates);
        assert!(matches!(c.status, ComponentStatus::Failed));
    }

    #[test]
    fn postgres_in_flight_when_neither_ready_nor_failed() {
        let gates = map(vec![
            gate("postgresql",         ComponentStatus::Ready),
            gate("postgresql_service", ComponentStatus::Installing),
        ]);
        let c = aggregate(ComponentId::Postgres, &gates);
        assert!(matches!(c.status, ComponentStatus::Installing));
    }

    #[test]
    fn sensei_aggregates_three_sub_gates() {
        let gates = map(vec![
            gate("sensei",     ComponentStatus::Ready),
            gate("senseid",    ComponentStatus::Ready),
            gate("sensei_mcp", ComponentStatus::Ready),
        ]);
        let c = aggregate(ComponentId::Sensei, &gates);
        assert!(matches!(c.status, ComponentStatus::Ready));
    }

    #[test]
    fn missing_sub_gate_yields_pending() {
        let gates = map(vec![]);
        let c = aggregate(ComponentId::Database, &gates);
        assert!(matches!(c.status, ComponentStatus::Pending));
    }

    #[test]
    fn failed_propagates_detail_from_first_failed_sub() {
        let gates = map(vec![
            InternalGate { id: "postgresql".to_string(), status: ComponentStatus::Failed,
                version: None, detail: Some("pg_isready returned 1".to_string()) },
            gate("postgresql_service", ComponentStatus::Ready),
        ]);
        let c = aggregate(ComponentId::Postgres, &gates);
        assert_eq!(c.detail.as_deref(), Some("pg_isready returned 1"));
    }
}
```

Wire into `health/mod.rs`:

```rust
pub mod types;
pub mod defaults;
pub mod aggregate;

pub use types::*;
pub use defaults::{component_default, package_manager_default};
pub use aggregate::{aggregate, InternalGate, GateMap};
```

- [ ] **Step 2: Run, expect RED**

```bash
cargo test -p sensei-bootstrap health::aggregate 2>&1 | tail -15
```

Expected: 6 new tests fail (stub returns default `pending` for everything).

- [ ] **Step 3: Implement `aggregate`**

Replace the stub body in `aggregate.rs`:

```rust
const SUBS: [(ComponentId, &[&str]); 5] = [
    (ComponentId::Postgres, &["postgresql", "postgresql_service"]),
    (ComponentId::Ollama,   &["ollama", "ollama_service"]),
    (ComponentId::Sensei,   &["sensei", "senseid", "sensei_mcp"]),
    (ComponentId::Database, &["database"]),
    (ComponentId::Daemon,   &["daemon"]),
];

fn subs_for(id: ComponentId) -> &'static [&'static str] {
    SUBS.iter().find(|(i, _)| *i == id).map(|(_, s)| *s).unwrap_or(&[])
}

fn merge_status(subs: &[&InternalGate]) -> ComponentStatus {
    // Priority: any Failed -> Failed; all Ready -> Ready;
    //          any Installing -> Installing; any Checking -> Checking;
    //          else Pending.
    if subs.iter().any(|g| matches!(g.status, ComponentStatus::Failed))    { return ComponentStatus::Failed; }
    if !subs.is_empty() && subs.iter().all(|g| matches!(g.status, ComponentStatus::Ready)) { return ComponentStatus::Ready; }
    if subs.iter().any(|g| matches!(g.status, ComponentStatus::Installing)) { return ComponentStatus::Installing; }
    if subs.iter().any(|g| matches!(g.status, ComponentStatus::Checking))   { return ComponentStatus::Checking; }
    ComponentStatus::Pending
}

pub fn aggregate(id: ComponentId, gates: &GateMap) -> Component {
    let mut c = component_default(id);
    let subs_ids = subs_for(id);
    let subs: Vec<&InternalGate> = subs_ids.iter().filter_map(|s| gates.get(*s)).collect();
    if subs.is_empty() {
        return c;
    }
    c.status  = merge_status(&subs);
    c.detail  = subs.iter().find(|g| matches!(g.status, ComponentStatus::Failed)).and_then(|g| g.detail.clone());
    c.version = subs.iter().find_map(|g| g.version.clone());
    c.id      = id_str(id).to_string();
    c
}
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cargo test -p sensei-bootstrap health::aggregate 2>&1 | tail -15
```

Expected: 6/6 pass.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): aggregate internal gates into 5 public components"
```

---

### Task A4 — `HealthPayload::validate()` + `check()`

**Files:**
- Modify: `crates/bootstrap/src/health/types.rs` (add `impl HealthPayload`)
- Create: `crates/bootstrap/src/health/check.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

- [ ] **Step 1: Failing tests for `validate()`**

Append to `health/types.rs` tests:

```rust
    #[test]
    fn validate_needs_action_requires_remedy() {
        let mut p = mock_ok_payload();
        p.status = HealthStatus::NeedsAction;
        p.remedy = None;
        let err = p.validate().unwrap_err();
        assert!(err.contains("needs-action requires a remedy"));
    }

    #[test]
    fn validate_non_needs_action_rejects_remedy() {
        let mut p = mock_ok_payload();
        p.remedy = Some(Remedy { message: "x".into(), script: "y".into(), url: None });
        let err = p.validate().unwrap_err();
        assert!(err.contains("must not carry a remedy"));
    }

    #[test]
    fn validate_components_length_must_be_five() {
        let mut p = mock_ok_payload();
        p.components.pop();
        let err = p.validate().unwrap_err();
        assert!(err.contains("expected 5 components"));
    }

    #[test]
    fn validate_ok_payload_passes() {
        let p = mock_ok_payload();
        assert!(p.validate().is_ok());
    }

    fn mock_ok_payload() -> HealthPayload {
        use crate::health::defaults::{component_default, package_manager_default};
        let mut components: Vec<Component> = COMPONENT_ORDER.iter().map(|id| {
            let mut c = component_default(*id);
            c.status = ComponentStatus::Ready;
            c
        }).collect();
        let _ = &mut components;
        HealthPayload {
            version: "0.2.14".into(),
            uptime_seconds: 0,
            platform: Platform::Macos,
            package_manager: package_manager_default(Platform::Macos),
            components,
            status: HealthStatus::Ok,
            remedy: None,
        }
    }
```

- [ ] **Step 2: Add `impl HealthPayload` with `validate()`**

In `health/types.rs`:

```rust
impl HealthPayload {
    /// Runtime invariant guard mirroring the TS `apply()` checks (INV-1 / INV-2 / INV-3).
    /// Called by `check()` and `check_and_resolve()` before returning.
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
            let expected_id = crate::health::defaults::id_str(*expected);
            if self.components[i].id != expected_id {
                return Err(format!(
                    "HealthPayload: components[{}].id must be \"{}\", got \"{}\"",
                    i, expected_id, self.components[i].id
                ));
            }
        }
        // INV-3
        let expected_pm = match self.platform {
            Platform::Windows => "winget",
            _ => "homebrew",
        };
        if self.package_manager.id != expected_pm {
            return Err(format!(
                "HealthPayload: platform={:?} expects packageManager.id=\"{}\", got \"{}\"",
                self.platform, expected_pm, self.package_manager.id
            ));
        }
        Ok(())
    }
}
```

- [ ] **Step 3: Run, expect GREEN**

```bash
cargo test -p sensei-bootstrap health 2>&1 | tail -10
```

Expected: 4 new tests pass.

- [ ] **Step 4: Skeleton `check()` (still TDD)**

Create `crates/bootstrap/src/health/check.rs`:

```rust
//! Sync fast-path check — runs every internal checker, aggregates into the
//! public component shape, returns a validated HealthPayload.

use std::collections::HashMap;
use super::types::*;
use super::defaults::{package_manager_default};
use super::aggregate::{aggregate, GateMap, InternalGate};
use crate::prereq::registry::COMPONENTS;
use crate::prereq::engine::BootstrapContext;

/// Run every internal checker once, aggregate, return a HealthPayload.
/// Sync — no events, no fixers. Used by daemon `GET /health`.
pub fn check(app_version: &str) -> HealthPayload {
    let ctx = BootstrapContext::new(
        std::sync::Arc::from(crate::provider()),
        crate::SenseiConfig::from_env(),
        app_version.to_string(),
    );
    check_with_context(&ctx)
}

pub fn check_with_context(ctx: &BootstrapContext) -> HealthPayload {
    let gates = run_checkers(ctx);
    let platform = map_platform(ctx.platform_provider().platform());
    let pm = build_package_manager(platform, &gates);
    let components: Vec<Component> = COMPONENT_ORDER.iter().map(|id| aggregate(*id, &gates)).collect();
    let status = overall_status(&pm, &components);
    let payload = HealthPayload {
        version: ctx.app_version().to_string(),
        uptime_seconds: 0, // daemon overrides this; sidecar leaves 0
        platform,
        package_manager: pm,
        components,
        status,
        remedy: None, // no remedy from a pure check; resolve() may add one
    };
    payload.validate().expect("bootstrap::check produced an invalid payload");
    payload
}

fn run_checkers(ctx: &BootstrapContext) -> GateMap {
    let mut map = HashMap::new();
    for spec in COMPONENTS {
        let result = ctx.checker(spec.id).check();
        let status = if result.ok { ComponentStatus::Ready } else { ComponentStatus::Failed };
        map.insert(spec.id.to_string(), InternalGate {
            id: spec.id.to_string(),
            status,
            version: result.version.clone(),
            detail:  if result.ok { None } else { Some(result.error.unwrap_or_default()) },
        });
    }
    map
}

fn map_platform(p: crate::platform::Platform) -> Platform {
    match p {
        crate::platform::Platform::MacOS   => Platform::Macos,
        crate::platform::Platform::Linux   => Platform::Linux,
        crate::platform::Platform::Windows => Platform::Windows,
    }
}

fn build_package_manager(platform: Platform, gates: &GateMap) -> Component {
    let mut pm = package_manager_default(platform);
    let internal_id = match platform {
        Platform::Windows => "winget", // TODO: a winget internal gate doesn't exist yet — Phase 2 task
        _ => "homebrew",
    };
    if let Some(g) = gates.get(internal_id) {
        pm.status  = g.status;
        pm.version = g.version.clone();
        pm.detail  = g.detail.clone();
    }
    pm
}

fn overall_status(pm: &Component, components: &[Component]) -> HealthStatus {
    let all_ready = pm.status == ComponentStatus::Ready &&
                    components.iter().all(|c| c.status == ComponentStatus::Ready);
    if all_ready { return HealthStatus::Ok; }
    let any_failed = pm.status == ComponentStatus::Failed ||
                     components.iter().any(|c| c.status == ComponentStatus::Failed);
    if any_failed { return HealthStatus::NeedsAction; } // check() alone returns NeedsAction with no remedy — caller (sidecar) supplies it via resolve()
    HealthStatus::Checking
}
```

Note: `overall_status` returning `NeedsAction` without a remedy will fail `validate()`. That's intentional during this transition: `check()` is the "report current state" function. The daemon's `/health` may legitimately return `NeedsAction` and the UI shows a generic "run the script" without a script string. For now, the validation `expect()` would panic — that's a known gap we close in Task A5 by introducing a `default_remedy` helper that supplies a generic remedy when the checker has no fixer attached. **Leave the panic in; Task A5 fixes it.**

⚠️ **Open question for the agent:** The `ctx.checker(id)` and `ctx.platform_provider()` and `ctx.app_version()` accessor signatures referenced above assume the current `BootstrapContext` exposes them publicly. Open `crates/bootstrap/src/prereq/engine.rs` and verify. If the accessors don't exist, add them as a minimal pre-cursor before continuing.

- [ ] **Step 5: Wire into `health/mod.rs`**

```rust
pub mod check;
pub use check::{check, check_with_context};
```

- [ ] **Step 6: Build only — don't run `check()` yet**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -10
```

Expected: 0 errors. The `expect` panic on invalid-payload is reachable but no test invokes `check()` yet.

- [ ] **Step 7: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): add HealthPayload::validate and check() skeleton"
```

---

### Task A5 — Default remedy for `check()`-only failures

**Files:**
- Modify: `crates/bootstrap/src/health/check.rs`
- Modify: `crates/bootstrap/src/health/defaults.rs`

When `check()` returns `NeedsAction` (any component failed), we MUST supply a remedy to satisfy INV-1. The remedy here is generic ("Open the Sensei app and run the bootstrap repair flow") because `check()` doesn't run fixers.

- [ ] **Step 1: Test**

Append to `health/types.rs` tests:

```rust
    #[test]
    fn check_returns_validated_payload_on_clean_system() {
        // Smoke: invoking check() must produce a payload that validate() accepts.
        let payload = crate::health::check::check("0.2.14");
        payload.validate().expect("check() must always return a valid payload");
    }
```

- [ ] **Step 2: Run, expect PANIC or FAIL**

```bash
cargo test -p sensei-bootstrap health::types::tests::check_returns_validated 2>&1 | tail -10
```

Expected: either the test fails the validate() assertion OR panics inside `check()` because `NeedsAction` + no remedy.

- [ ] **Step 3: Add `default_remedy()` and wire it**

In `health/defaults.rs`:

```rust
use super::types::Remedy;

pub fn default_remedy() -> Remedy {
    Remedy {
        message: "Some components need attention. Run the Sensei bootstrap repair flow.".to_string(),
        script:  "open sensei://repair".to_string(),
        url:     None,
    }
}
```

Export from `health/mod.rs`:

```rust
pub use defaults::{component_default, package_manager_default, default_remedy};
```

In `health/check.rs` at the end of `check_with_context`, replace the `remedy: None` with:

```rust
        remedy: if status == HealthStatus::NeedsAction { Some(super::defaults::default_remedy()) } else { None },
```

- [ ] **Step 4: Run, expect GREEN**

```bash
cargo test -p sensei-bootstrap health 2>&1 | tail -10
```

Expected: all health tests pass including the new smoke test.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): default remedy for check()-only NeedsAction"
```

---

### Task A6 — `check_and_resolve` (streaming + fixers)

**Files:**
- Create: `crates/bootstrap/src/health/resolve.rs`
- Modify: `crates/bootstrap/src/health/mod.rs`

This wraps the existing `BootstrapEngine::check_and_fix`, translates its `ProgressEvent`s into `HealthEvent`s, and returns the terminal `HealthPayload`.

- [ ] **Step 1: Test (smoke)**

Append to `health/types.rs` tests:

```rust
    #[test]
    fn check_and_resolve_emits_phase_then_report() {
        use std::sync::{Arc, Mutex};
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev_clone = events.clone();
        let report = crate::health::resolve::check_and_resolve("0.2.14", move |ev| {
            ev_clone.lock().unwrap().push(ev);
        });
        let captured = events.lock().unwrap();
        assert!(!captured.is_empty(), "check_and_resolve must emit at least one event");
        assert!(matches!(captured.first(), Some(HealthEvent::Phase { phase: HealthStatus::Checking })),
            "first event must be Phase(Checking)");
        assert!(matches!(captured.last(), Some(HealthEvent::Report { .. })),
            "last event must be Report");
        report.validate().expect("terminal payload must validate");
    }
```

- [ ] **Step 2: Run, expect RED (module missing)**

- [ ] **Step 3: Implement `health/resolve.rs`**

```rust
//! Streaming check+resolve — runs the BootstrapEngine and re-emits events in
//! the public HealthEvent shape. Returns the terminal HealthPayload.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use super::types::*;
use super::aggregate::{aggregate, GateMap, InternalGate};
use super::check::{check_with_context};
use crate::prereq::{ProgressEvent, GateStatus, BootstrapEngine};
use crate::prereq::engine::BootstrapContext;

pub fn check_and_resolve<F>(app_version: &str, emit: F) -> HealthPayload
where F: Fn(HealthEvent) + Send + Sync + 'static
{
    let ctx = BootstrapContext::new(
        std::sync::Arc::from(crate::provider()),
        crate::SenseiConfig::from_env(),
        app_version.to_string(),
    );
    check_and_resolve_with_context(ctx, emit)
}

pub fn check_and_resolve_with_context<F>(ctx: BootstrapContext, emit: F) -> HealthPayload
where F: Fn(HealthEvent) + Send + Sync + 'static
{
    emit(HealthEvent::Phase { phase: HealthStatus::Checking });

    // Track internal-gate state as the engine reports it so we can aggregate live.
    let gates: Arc<Mutex<GateMap>> = Arc::new(Mutex::new(HashMap::new()));
    let gates_for_cb = gates.clone();
    let emit_arc = Arc::new(emit);
    let emit_for_cb = emit_arc.clone();

    let engine = BootstrapEngine::new(Arc::new(ctx));
    let internal_report = engine.check_and_fix(move |progress: ProgressEvent| {
        match progress {
            ProgressEvent::Gate { id, status } => {
                let new_status = match &status {
                    GateStatus::Checking            => ComponentStatus::Checking,
                    GateStatus::Installing          => ComponentStatus::Installing,
                    GateStatus::Starting            => ComponentStatus::Installing, // collapse: 'starting' is a kind of installing
                    GateStatus::Ready { .. }        => ComponentStatus::Ready,
                    GateStatus::Failed { .. }       => ComponentStatus::Failed,
                };
                let (version, detail) = match &status {
                    GateStatus::Ready { version, detail } => (version.clone(), detail.clone()),
                    GateStatus::Failed { error } => (None, Some(error.clone())),
                    _ => (None, None),
                };
                {
                    let mut m = gates_for_cb.lock().unwrap();
                    m.insert(id.clone(), InternalGate { id: id.clone(), status: new_status, version: version.clone(), detail: detail.clone() });
                }
                // Re-emit as a public Component event, mapped through aggregation.
                let public_id = public_id_for(&id);
                if let Some(pid) = public_id {
                    let aggregated = aggregate(pid, &gates_for_cb.lock().unwrap());
                    emit_for_cb(HealthEvent::Component {
                        id: aggregated.id.clone(),
                        patch: ComponentPatch {
                            label:   None,
                            note:    None,
                            status:  Some(aggregated.status),
                            version: Some(aggregated.version.clone()),
                            detail:  Some(aggregated.detail.clone()),
                        },
                    });
                } else if id == "homebrew" || id == "winget" {
                    // package manager update
                    emit_for_cb(HealthEvent::Component {
                        id: id.clone(),
                        patch: ComponentPatch {
                            label:   None,
                            note:    None,
                            status:  Some(new_status),
                            version: Some(version),
                            detail:  Some(detail),
                        },
                    });
                }
            }
            ProgressEvent::PhaseComplete { phase, success: _ } => {
                if phase == "fix" {
                    emit_for_cb(HealthEvent::Phase { phase: HealthStatus::Resolving });
                }
            }
        }
    });

    let _ = internal_report; // we re-derive the payload from gates state below
    let payload = check_with_context(&BootstrapContext::new(
        std::sync::Arc::from(crate::provider()),
        crate::SenseiConfig::from_env(),
        ctx.app_version().to_string(),
    ));
    emit_arc(HealthEvent::Report { payload: payload.clone() });
    payload
}

/// Map an internal gate id to its public ComponentId (or None for the package manager).
fn public_id_for(internal: &str) -> Option<ComponentId> {
    match internal {
        "postgresql" | "postgresql_service" => Some(ComponentId::Postgres),
        "ollama" | "ollama_service"         => Some(ComponentId::Ollama),
        "sensei" | "senseid" | "sensei_mcp" => Some(ComponentId::Sensei),
        "database"                          => Some(ComponentId::Database),
        "daemon"                            => Some(ComponentId::Daemon),
        _                                   => None,
    }
}
```

Wire into `mod.rs`:

```rust
pub mod resolve;
pub use resolve::{check_and_resolve, check_and_resolve_with_context};
```

⚠️ **Note for the agent:** This task touches the existing `BootstrapEngine::check_and_fix` callback contract. If the engine's callback signature uses a non-`Fn(Send+Sync+'static)` closure, the wrapping above will fail to compile. In that case, simplify: call the engine inline (not in a moved closure) and capture events in a `Mutex<Vec<HealthEvent>>` that gets flushed per-event via a sync channel. Implementation detail — solve at compile time.

- [ ] **Step 4: Build + test**

```bash
cargo build -p sensei-bootstrap 2>&1 | tail -10
cargo test  -p sensei-bootstrap health 2>&1 | tail -15
```

Expected: build clean, new resolve test passes.

- [ ] **Step 5: Commit**

```bash
git add crates/bootstrap/src/health/
git commit -m "feat(bootstrap): check_and_resolve streaming with public events"
```

---

## Section B — Daemon `/health` becomes a one-liner

### Task B1 — Replace `HealthResponse` with `bootstrap::HealthPayload`

**Files:**
- Modify (full rewrite): `crates/senseid/src/api/handlers/health.rs`

- [ ] **Step 1: Overwrite the file**

```rust
//! Health endpoint — thin wrapper over `sensei_bootstrap::check`.
//!
//! The daemon does NOT reshape the response. Every consumer (Tauri app,
//! external clients) gets the same `HealthPayload` shape.

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
    let result = tokio::task::spawn_blocking(|| bootstrap::check(env!("CARGO_PKG_VERSION"))).await;
    let mut payload = result.unwrap_or_else(|e| {
        tracing::error!("health check task failed: {e}");
        // Fall back to a syntactically valid payload that reports NeedsAction.
        // The bootstrap crate's default_remedy is included via check_with_context;
        // we mirror it here using check() on a fresh thread isn't possible from
        // this fallback path, so build a minimal valid payload by hand.
        bootstrap::check(env!("CARGO_PKG_VERSION"))
    });
    payload.uptime_seconds = uptime_seconds();
    Json(payload)
}

// ── Watcher endpoints (unchanged — they don't belong here architecturally but
// removing them is out of scope for Phase 1b) ─────────────────────────────────

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

- [ ] **Step 2: Build + test**

```bash
cargo build -p senseid 2>&1 | tail -5
cargo test  -p senseid api::handlers::health 2>&1 | tail -5
```

Expected: 0 errors. Existing tests for the old `HealthResponse` shape will fail — that's expected; we're replacing the contract. Update or delete them.

- [ ] **Step 3: Smoke test the wire shape**

Run the daemon and curl:

```bash
cargo run -p senseid &
sleep 3
curl -s http://localhost:7744/health | jq .
```

Expected: JSON matching the TS `HealthPayload` shape — `version`, `uptime_seconds`, `platform`, `package_manager: {...}`, `components: [...]` (5 entries), `status`, `remedy` (null or object).

Kill the daemon when verified.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/api/handlers/health.rs
git commit -m "refactor(senseid): /health returns bootstrap::HealthPayload directly"
```

---

## Section C — Sidecar thin rewrite

### Task C1 — Split auxiliary commands out of `bootstrap.rs`

**Files:**
- Create: `app/src-tauri/src/commands/platform.rs` (move `detect_hardware`, `list_models`, `missing_models`, `get_platform`, `get_daemon_port`)
- Modify: `app/src-tauri/src/commands/mod.rs` (add `pub mod platform;`)
- Modify: `app/src-tauri/src/lib.rs` (update command registrations)

Move the read-only / config commands to their own module so `bootstrap.rs` is purely about the health flow.

- [ ] **Step 1: Move the 5 commands verbatim into `commands/platform.rs`**

```rust
//! Auxiliary read-only commands — hardware, models, platform info, port.
//! Each delegates to `sensei_bootstrap` and adds zero logic.

use sensei_bootstrap::{self as bootstrap, HardwareInfo};

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
pub fn get_platform() -> serde_json::Value {
    let provider = bootstrap::provider();
    serde_json::json!({
        "platform": provider.platform(),
        "package_manager": provider.package_manager_name(),
        "prereq_remedy": provider.prereq_install_remedy(),
        "pkgmgr_remedy": provider.package_manager_remedy(),
    })
}

#[tauri::command]
pub fn get_daemon_port() -> u16 {
    sensei_bootstrap::daemon_port()
}
```

- [ ] **Step 2: Update `commands/mod.rs`**

Add `pub mod platform;` and confirm `pub mod bootstrap;` is still there (we'll empty it next).

- [ ] **Step 3: Update `app/src-tauri/src/lib.rs`**

Find the `tauri::generate_handler!` invocation. Replace the bootstrap-prefixed entries with the new paths:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::check_and_fix_bootstrap,
            commands::bootstrap::detect_hardware,
            commands::bootstrap::list_models,
            commands::bootstrap::missing_models,
            commands::bootstrap::get_platform,
            commands::bootstrap::get_daemon_port,
            // …
        ])
```

becomes:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap::check_and_fix_bootstrap,
            commands::platform::detect_hardware,
            commands::platform::list_models,
            commands::platform::missing_models,
            commands::platform::get_platform,
            commands::platform::get_daemon_port,
            // …
        ])
```

- [ ] **Step 4: Build**

```bash
cd app/src-tauri && cargo build 2>&1 | tail -5
```

Expected: 0 errors. The `bootstrap.rs` file still has those 5 commands; we'll remove them in step 5.

- [ ] **Step 5: Delete the moved commands from `bootstrap.rs`**

In `app/src-tauri/src/commands/bootstrap.rs`, delete:
- `detect_hardware`
- `list_models`
- `missing_models`
- `get_platform`
- `get_daemon_port`

Keep only `check_and_fix_bootstrap` (for now — Task C2 wipes it).

- [ ] **Step 6: Build**

```bash
cargo build 2>&1 | tail -5
```

Expected: 0 errors.

- [ ] **Step 7: Commit**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
git add app/src-tauri/src/commands/ app/src-tauri/src/lib.rs
git commit -m "refactor(src-tauri): split auxiliary commands out of bootstrap.rs"
```

---

### Task C2 — Wipe and rewrite `commands/bootstrap.rs` as a thin wrapper

**Files:**
- Modify (full rewrite): `app/src-tauri/src/commands/bootstrap.rs`
- Modify: `app/src-tauri/src/lib.rs` (rename command in `generate_handler!`)

- [ ] **Step 1: Overwrite `bootstrap.rs` with the thin version**

```rust
//! Health commands — thin wrappers over `sensei_bootstrap`.
//!
//! Two commands and two only:
//!   * `health_check`             — sync fast path. Returns HealthPayload.
//!   * `health_check_and_resolve` — kicks off the resolve pipeline; events
//!                                  arrive on the "health" channel; this
//!                                  command itself returns immediately.

use sensei_bootstrap::{self as bootstrap, HealthEvent, HealthPayload};
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
        let app_for_events = app.clone();
        let _final_payload = bootstrap::check_and_resolve(&version, move |ev: HealthEvent| {
            let _ = app_for_events.emit("health", &ev);
        });
        flog::log("health_check_and_resolve complete");
    });
    Ok(())
}
```

- [ ] **Step 2: Update `lib.rs` command registration**

Replace:

```rust
            commands::bootstrap::check_and_fix_bootstrap,
```

with:

```rust
            commands::bootstrap::health_check,
            commands::bootstrap::health_check_and_resolve,
```

- [ ] **Step 3: Build**

```bash
cd app/src-tauri && cargo build 2>&1 | tail -10
```

Expected: 0 errors. The TS client (`app/src/lib/bootstrap.ts`) still calls the old command name and will break in dev — but the Rust side is now clean. TS migration is Phase 1c.

- [ ] **Step 4: Commit**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
git add app/src-tauri/src/commands/bootstrap.rs app/src-tauri/src/lib.rs
git commit -m "refactor(src-tauri): bootstrap.rs is now two thin wrappers over bootstrap crate"
```

⚠️ **What was deleted on purpose:**
- `emit_gate`, `emit_phase_complete`, `dispatch` — shape translation no longer needed (bootstrap crate emits the public shape directly).
- `bootstrap-report` event — collapsed into the terminal `Report` variant of the `health` event stream.
- `write_bootstrap_session` + `collect_system_info` — log session writing. This is a known regression. It will be restored in Section D2 by moving the concern into a dedicated `log_session` module (sidecar side) that subscribes to the `health` event stream and emits a session record on `Report`. Out of scope for the airtight thin-wrapper milestone.

---

## Section D — Cleanup and integration prep

### Task D1 — Retire the old bootstrap public surface

Now that no consumer uses `check_and_fix`, `BootstrapReport`, `ProgressEvent`, etc., remove them.

**Files:**
- Modify: `crates/bootstrap/src/lib.rs` (drop `pub use prereq::{...}`)
- Optionally rename / restructure `prereq/mod.rs` to make those types `pub(crate)`

- [ ] **Step 1: Drop the old re-exports**

In `crates/bootstrap/src/lib.rs`, remove:

```rust
pub use prereq::{BootstrapReport, GateReport, GateStatus, HumanAction, ProgressEvent};
```

Keep `pub use prereq::engine::{BootstrapContext, BootstrapEngine};` only if internal tests still need them at the crate root; otherwise drop too.

Also remove the convenience functions:

```rust
pub fn check_and_fix_with_context<F>(...) -> BootstrapReport { ... }
pub fn check_and_fix<F>(...) -> BootstrapReport { ... }
```

(Or keep `check_and_fix_with_context` if `check_and_resolve` doesn't yet accept a `BootstrapContext` — it does, see Task A6.)

- [ ] **Step 2: Demote internal types to `pub(crate)`**

In `crates/bootstrap/src/prereq/mod.rs`, change:

```rust
pub enum ProgressEvent { ... }
pub enum GateStatus { ... }
pub struct GateReport { ... }
pub struct BootstrapReport { ... }
pub struct HumanAction { ... }
```

to `pub(crate) enum/struct ...` — they're now implementation details of the engine, only consumed by the `health::resolve` module.

- [ ] **Step 3: Build**

```bash
cargo build --workspace 2>&1 | tail -10
```

Expected: 0 errors across the whole workspace. If anything still imports the old types, fix it.

- [ ] **Step 4: Commit**

```bash
git add crates/bootstrap/
git commit -m "refactor(bootstrap): retire BootstrapReport/ProgressEvent in favor of HealthPayload/HealthEvent"
```

---

### Task D2 — Wait. TS client + log session restoration are Phase 1c.

This plan deliberately STOPS here. The TS client (`app/src/lib/bootstrap.ts`) still calls `check_and_fix_bootstrap` and listens on the `bootstrap` event — so the live app is broken from Tauri integration until Phase 1c migrates it. That migration is one focused commit:

1. Rewrite `app/src/lib/bootstrap.ts` to call `health_check` and `health_check_and_resolve`, and listen on the `health` event.
2. Restore session logging via a small `health-session-logger.ts` module that subscribes to `health` events and POSTs sessions to the existing log_collector endpoint.
3. Wire `HealthState.init()` (already designed in spec Section 9) to consume the new transport.

That is Phase 1c, written as a separate plan after Phase 1b lands.

---

## Verification gate (end of Phase 1b)

- [ ] **All Rust tests pass**
  ```bash
  cargo test --workspace 2>&1 | tail -5
  ```
- [ ] **`/health` returns the new shape**
  Run the daemon and curl `/health` — verify a 200 with the `HealthPayload` shape.
- [ ] **`sensei_bootstrap::check_and_resolve` smoke**
  In a scratch `tests/integration_resolve.rs`:
  ```rust
  use sensei_bootstrap::{check_and_resolve, HealthEvent};
  #[test]
  fn smoke_check_and_resolve_emits_terminal_report() {
      let mut got_report = false;
      let _ = check_and_resolve("0.0.0", |ev| { if matches!(ev, HealthEvent::Report { .. }) { got_report = true; } });
      assert!(got_report);
  }
  ```
- [ ] **app/ test suite still passes**
  ```bash
  cd app && bun run test:unit 2>&1 | tail -3
  ```
  Expected: 273/273. The TS state class doesn't know about the transport rewiring yet, so its tests are unaffected.

- [ ] **app/ check still passes**
  ```bash
  bun run check 2>&1 | tail -3
  ```

When all four pass, Phase 1b is complete. Commit a final `chore: phase 1b complete` tag if desired, then move to Phase 1c.

---

## Out of scope (Phase 1c+)

- TS `bootstrap.ts` migration to the new command names + event channel.
- `HealthState.init()` lifecycle wiring (Phase 2 lifecycle methods).
- `health-session-logger.ts` to restore session logging via the new event stream.
- E2E smoke: cold-start app, delete `sensei_dev` DB + sensei files, verify the bootstrap walks all gates and lands on `ok`.
- Winget gate (real Windows package manager support) — currently stubbed as "winget" in the public PackageManager id but no actual internal `winget` gate exists.
