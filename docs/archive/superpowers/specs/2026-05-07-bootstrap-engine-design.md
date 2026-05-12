# Bootstrap Engine — Design Spec
**Date:** 2026-05-07  
**Status:** Approved for implementation  
**Replaces:** `crates/bootstrap/src/prereq/factory.rs` (three-phase factory model)

---

## Problem statement

The existing bootstrap crate has three problems:

1. **Duplicate prerequisite definitions** — `factory.rs` defines the same four components twice, once per platform, differing only in fixer type.
2. **No dependency graph** — if Homebrew fails, the runner still attempts `brew install` for every subsequent component. There is no skip-on-dep-failure logic.
3. **Three-phase Tauri commands** — the frontend drives `install_prerequisites → start_services → setup_database` as sequential Tauri calls. The crate has no single entry point that resolves the full bootstrap lifecycle.
4. **Missing `sensei-mcp`** — the MCP binary is not checked or installed.
5. **Hardcoded binary names** — dev builds use `-dev` suffix (`sensei-dev`, `senseid-dev`, `sensei-mcp-dev`) but the factory does not read this from config.
6. **Embedded Brewfile** — `macos.rs` duplicates the Brewfile as a Rust const instead of referencing the authoritative GitHub tap.

---

## Goals

- Single `check_and_fix(callback) -> BootstrapReport` entry point: checks everything, fixes what's broken, streams progress events.
- Idempotent: calling it again is the resume mechanism after human intervention.
- Dependency-aware: skip fixes for components whose dependencies failed.
- `BrewBundleFixer` as the unified install + upgrade strategy via the homebrew tap Brewfile.
- `sensei-mcp` added as a first-class component.
- Binary names derived from `SenseiConfig` (dev/prod suffix).
- Schema deployment driven by `dbd::deploy("sensei-hq/sensei/database@v{app_version}")` — no manual version comparison.
- Full integration test coverage via injectable mock checkers and fixers.

---

## Architecture

### Module changes

```
crates/bootstrap/src/
  config.rs           ← add binary_name helpers + HOMEBREW_BREWFILE_URL constant
  types.rs            ← add HumanAction, GateReport, BootstrapReport structs
  prereq/
    checker.rs        ← unchanged (existing checker types stay)
    fixer.rs          ← add BrewBundleFixer, HumanActionFixer
    generic.rs        ← unchanged
    runner.rs         ← unchanged (kept; not called by engine)
    registry.rs       ← NEW: ComponentSpec + COMPONENTS static array
    engine.rs         ← NEW: BootstrapContext, BootstrapEngine
    factory.rs        ← REMOVED (replaced by registry + engine)
    mod.rs            ← expose new modules; keep existing public types
  lib.rs              ← add check_and_fix(); keep run() / run_with_traces()
```

---

## Config additions (`config.rs`)

```rust
/// Raw GitHub URL for the homebrew tap Brewfile (authoritative install source).
pub const HOMEBREW_BREWFILE_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile";

/// Homebrew tap repository slug (for reference/logging).
pub const HOMEBREW_TAP_REPO: &str = "sensei-hq/homebrew-tap";
```

Binary name helpers on `SenseiConfig` — returns the correct name for the current mode:

```rust
impl SenseiConfig {
    pub fn sensei_binary(&self) -> &'static str {
        if self.is_dev() { "sensei-dev" } else { "sensei" }
    }
    pub fn senseid_binary(&self) -> &'static str {
        if self.is_dev() { "senseid-dev" } else { "senseid" }
    }
    pub fn sensei_mcp_binary(&self) -> &'static str {
        if self.is_dev() { "sensei-mcp-dev" } else { "sensei-mcp" }
    }
}
```

---

## New types (`types.rs` additions)

```rust
/// A human action required before bootstrap can continue.
pub struct HumanAction {
    pub component_id: &'static str,
    pub title:        String,
    pub command:      String,
    pub url:          Option<String>,
}

/// Result for a single component gate after check_and_fix.
pub struct GateReport {
    pub id:            &'static str,
    pub status:        GateStatus,     // existing type
    pub fix_attempted: bool,
    pub fix_detail:    Option<String>, // on_complete info from dbd, brew output, etc.
}

/// Returned by check_and_fix.
pub struct BootstrapReport {
    pub gates:      Vec<GateReport>,
    pub all_ok:     bool,
    pub blocked_on: Option<HumanAction>, // first component needing human action
}
```

---

## New fixers (`fixer.rs` additions)

### `BrewBundleFixer`

Fetches the Brewfile from `HOMEBREW_BREWFILE_URL` at fix time (via `reqwest::blocking`, already a dep) and pipes it to `brew bundle --upgrade --file=-`.

- `--upgrade` ensures both missing installs AND outdated versions are resolved.
- This is the single fixer for all binary components (postgresql, ollama, sensei, senseid, sensei-mcp).
- Idempotent: brew skips already-current formulae.
- Replaces `BrewFixer`, `BrewUpgradeFixer` for the install use-case. Those types are kept for any standalone use but are no longer registered in the component registry.

```rust
pub struct BrewBundleFixer {
    brew_path:    String,
    brewfile_url: String,   // HOMEBREW_BREWFILE_URL
}
```

### `HumanActionFixer`

Returns a structured `HumanAction` instead of `Err(String)`. Used for:
- Homebrew missing (cannot be auto-installed)
- Sensei/senseid/sensei-mcp in dev mode (must run `make install-dev`)

```rust
pub struct HumanActionFixer {
    pub component_id: &'static str,
    pub title:        &'static str,
    pub command:      &'static str,
    pub url:          Option<&'static str>,
}
```

The `Fixer` trait gains a new method:
```rust
pub trait Fixer: Send + Sync {
    fn fix(&self) -> Result<FixResult, String>;
    fn human_action(&self) -> Option<HumanAction> { None }  // default: None
}
```
`HumanActionFixer` overrides `human_action()` to return `Some(...)` and returns `Err` from `fix()`.

---

## Component registry (`prereq/registry.rs`)

### `ComponentSpec`

```rust
pub struct ComponentSpec {
    pub id:               &'static str,
    pub label:            &'static str,
    pub depends_on:       &'static [&'static str],
    /// If Some, this component is batched with others sharing the same group
    /// for a single BrewBundleFixer call.
    pub fix_group:        Option<&'static str>,
    /// After this component is successfully fixed, force these component ids
    /// into the fix plan even if their checker currently passes.
    /// Used by senseid to trigger database deploy after a bundle upgrade.
    pub post_fix_trigger: &'static [&'static str],
    pub checker_fn:       fn(&BootstrapContext) -> Box<dyn Checker>,
    pub fixer_fn:         fn(&BootstrapContext) -> Box<dyn Fixer>,
}
```

### `COMPONENTS` static

Ten components covering the full bootstrap lifecycle:

| id | label | depends_on | fix_group | post_fix_trigger |
|---|---|---|---|---|
| `homebrew` | Homebrew | `[]` | — | — |
| `postgresql` | PostgreSQL | `[homebrew]` | `bundle` | — |
| `ollama` | Ollama | `[homebrew]` | `bundle` | — |
| `sensei` | Sensei CLI | `[homebrew]` | `bundle` (prod) / — (dev) | — |
| `senseid` | Sensei Daemon | `[homebrew]` | `bundle` (prod) / — (dev) | `[database]` |
| `sensei_mcp` | Sensei MCP | `[homebrew]` | `bundle` (prod) / — (dev) | — |
| `postgresql_service` | PostgreSQL Service | `[postgresql]` | — | — |
| `ollama_service` | Ollama Service | `[ollama]` | — | — |
| `database` | Sensei Database | `[postgresql_service, senseid]` | — | — |
| `daemon` | Sensei Daemon Service | `[senseid, database]` | — | — |

**Dev mode notes:**
- `sensei`, `senseid`, `sensei_mcp` checkers use `ctx.config.sensei_binary()` etc. for the `-dev` suffix.
- `sensei`, `senseid`, `sensei_mcp` fixers use `HumanActionFixer` in dev mode: `"Run: make install-dev"`.
- `homebrew` checker reuses `PlatformProvider::check_package_manager()`.
- `postgresql_service` / `ollama_service` use `PortChecker` (existing).
- `daemon` uses `PortChecker(ctx.config.daemon_port)` (existing).

**Database checker logic:**
1. `pg_isready` reachable
2. Database named `ctx.config.db_name` exists
3. `pgvector` extension installed
4. `sensei` schema present in `_dbd_meta` (schema deployed indicator)

The checker does not compare versions. If the schema is absent → fail. If basic health fails → fail. Schema version currency is handled by `dbd::deploy`.

**Database fixer logic:**
Calls `database::setup(app_version)`:
```
db_schema_source(app_version) = "sensei-hq/sensei/database@v{app_version}"
```
`dbd::deploy` resolves this source, reads `design.yaml`, and applies pending migrations.

Error and success flow:
```rust
match design.deploy(&adapter, false, |step| { /* progress */ }).await {
    Ok(())  => /* on_complete already fired inside deploy — all steps succeeded */
    Err(e)  => /* first failed step; on_complete was NOT called */
}
```

- **On error:** `DatabaseSetupFixer::fix()` returns `Err(e.to_string())`. The engine emits `Gate{database, Failed(e)}` and records no `fix_detail`.
- **On success:** `on_complete` has already fired inside `deploy`. `DatabaseSetupFixer::fix()` returns `Ok(FixResult { approach: "<on_complete summary>" })`. The engine re-checks (passes) and records the summary in `GateReport.fix_detail`.

**Bundle → deploy coupling rule:**
When brew bundle installs or upgrades `senseid`, `dbd::deploy` must always run for the same version. This is enforced via `senseid.post_fix_trigger: [database]`:
- After `senseid` is successfully fixed (bundle batch), the engine adds `database` to the fix plan unconditionally.
- The `DatabaseSetupFixer` calls `dbd::deploy("sensei-hq/sensei/database@v{app_version}")` — same version that drove the bundle.
- If the schema is already current, `dbd::deploy` is a no-op. If migrations are pending, they are applied.

---

## Bootstrap engine (`prereq/engine.rs`)

### `BootstrapContext`

```rust
pub struct BootstrapContext {
    pub platform:          Arc<dyn PlatformProvider>,
    pub config:            SenseiConfig,
    pub app_version:       String,
    // Test injection: overrides ComponentSpec checker_fn / fixer_fn by component id
    checker_overrides:     HashMap<&'static str, Box<dyn Checker>>,
    fixer_overrides:       HashMap<&'static str, Box<dyn Fixer>>,
}

impl BootstrapContext {
    pub fn new(platform, config, app_version) -> Self { ... }
    pub fn from_app(app: &tauri::AppHandle) -> Self { ... }  // in Tauri crate

    // Builder helpers for test injection
    pub fn with_checker(mut self, id, checker) -> Self { ... }
    pub fn with_fixer(mut self, id, fixer) -> Self { ... }
}
```

### `BootstrapEngine::check_and_fix`

```
fn check_and_fix(callback: impl Fn(ProgressEvent) + Send) -> BootstrapReport

PHASE A — Parallel check
  For each component in COMPONENTS (spawn one thread per):
    emit Gate{id, Checking}
    run checker (from override map or spec checker_fn)
    if ok → emit Gate{id, Ready}; record as ready
    if fail → record as pending

PHASE B — Build fix plan
  Topo-sort pending components by depends_on edges.
  Mark component as "dep-blocked" if any depends_on is in pending set.
  Group consecutive fix_group="bundle" non-dep-blocked components as BatchStep.
  Result: Vec<PlanStep>  where PlanStep is Individual(id) or Batch(Vec<id>)

PHASE C — Sequential fix execution
  For each PlanStep:

    Individual(id):
      if dep-blocked → emit Gate{id, Failed("dep: {dep_id} not ready")} → skip
      emit Gate{id, Installing or Starting}  (from gate_kind)
      check fixer.human_action() → if Some(action) →
          emit Gate{id, Failed("human action required")}
          return BootstrapReport { blocked_on: Some(action), ... }   ← STOP
      run fixer.fix()
        Err(e) → emit Gate{id, Failed(e)} → continue
        Ok(result) →
          re-run checker → if ok: emit Gate{id, Ready}; if fail: emit Gate{id, Failed}

    Batch(ids):          // all fix_group="bundle", none dep-blocked
      emit Gate{id, Installing} for ALL ids
      run BrewBundleFixer once (fetch HOMEBREW_BREWFILE_URL → brew bundle --upgrade)
        Err(e) → emit Gate{id, Failed(e)} for ALL ids → continue
        Ok(_)  →
          re-check each id individually → emit Ready or Failed per id
          for each id that ended Ready AND has post_fix_trigger ids:
            add triggered ids to the pending fix list (force-fix regardless of checker)

  Post-trigger steps:
    Execute any ids added by post_fix_trigger as Individual steps in dep order.
    (Example: senseid fixed → database added → DatabaseSetupFixer runs dbd::deploy)

PHASE D — Return
  BootstrapReport {
    gates:      per-component GateReport (id, final status, fix_attempted, fix_detail)
    all_ok:     all gates ended Ready
    blocked_on: first HumanAction encountered (or None)
  }
```

### Resume semantics

`check_and_fix` is stateless. Calling it again after human action re-runs Phase A from scratch. Already-healthy components pass immediately. Only remaining failures are included in the new fix plan.

---

## Integration tests

All in `prereq/engine.rs` `#[cfg(test)]` block. Zero real binaries required.

### Mock types

```rust
// In tests module
struct MockChecker { sequence: Vec<bool> }   // per-call sequence of ok/fail
struct MockFixer { result: FixerResult }
enum FixerResult { Succeed(String), Fail(String), HumanAction(HumanAction) }
```

### Test scenarios

| Test | Checkers | Fixers | Expected |
|---|---|---|---|
| All ready | all ok | — | `all_ok=true`, 0 fix calls |
| Homebrew missing | homebrew=fail, others=ok | homebrew=HumanAction | `blocked_on.component_id = homebrew` |
| Homebrew blocks bundle | homebrew=fail, postgresql=fail | homebrew=HumanAction | postgresql fix skipped (dep-blocked) |
| postgresql + ollama missing | both fail, then pass | bundle=succeed | BrewBundleFixer called once, both Ready |
| Version mismatch (senseid outdated) | senseid=fail (outdated), then pass | bundle=succeed | lands in bundle batch, Ready after upgrade |
| DB schema not deployed | database=fail, then pass | db=succeed | DatabaseSetupFixer called, fix_detail populated |
| Fix succeeds but recheck fails | postgresql fixer=ok, recheck=fail | — | gate=Failed (not Ready) |
| Resume after human action | 1st call: homebrew=HumanAction; mock updated; 2nd call: homebrew=ok | — | 2nd call: all_ok |
| senseid upgrade triggers db deploy | senseid=fail-then-pass, database initially passes | bundle=succeed | post_fix_trigger forces db fixer, deploy runs |
| senseid upgrade, db deploy fails | senseid fixed, dbd::deploy returns Err(e) | db fixer returns Err | gate=Failed, no fix_detail |
| senseid upgrade, db already current | senseid fixed, dbd::deploy is no-op (Ok(())) | db fixer=succeed | gate=Ready, fix_detail="schema up to date" |
| db deploy success populates fix_detail | db fixer runs, deploy Ok(()) | on_complete fires, summary string | GateReport.fix_detail = on_complete summary |
| Daemon blocked by database failure | postgresql_service=fail, database=dep-blocked, daemon=dep-blocked | none | daemon gets dep-blocked status |

---

## Tauri changes (`app/src-tauri/src/commands/bootstrap.rs`)

**Removed:** `install_prerequisites`, `start_services`, `setup_database`

**Kept:** `run_bootstrap` (read-only health check, no fix)

**Added:**
```rust
#[tauri::command]
pub fn check_and_fix_bootstrap(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let ctx = BootstrapContext::from_app(&app);
        let engine = BootstrapEngine::new(ctx);
        let report = engine.check_and_fix(|e| dispatch(&app, e));
        dispatch_report(&app, report);  // emits "bootstrap:complete" with BootstrapReport
    });
    Ok(())
}
```

Frontend flow:
1. Call `run_bootstrap` → get health snapshot → if not all ready, show "Fix" button
2. Click Fix → call `check_and_fix_bootstrap` → receives gate events + final `bootstrap:complete`
3. If `blocked_on` is set → show instruction card + "Continue" button
4. Click Continue → call `check_and_fix_bootstrap` again (resume)

---

## Files removed

- `crates/bootstrap/src/prereq/factory.rs`
- Embedded `BREWFILE` const from `crates/bootstrap/src/platform/macos.rs`
- `install_prerequisites`, `start_services`, `setup_database` Tauri commands

---

## Out of scope

- Windows winget path (stub `NoopFixer` for all install components on Windows — existing behaviour unchanged)
- Parallel fix execution (fixes are sequential by design — dependency ordering requires it)
- Streaming brew output line-by-line (brew bundle output is captured; final status emitted when complete)
