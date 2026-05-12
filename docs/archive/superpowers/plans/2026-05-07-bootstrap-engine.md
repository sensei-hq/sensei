# Bootstrap Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the three-phase factory+runner model with a single `check_and_fix_bootstrap` Tauri command backed by a dependency-aware `BootstrapEngine` that checks, fixes, and streams progress for all 10 bootstrap components.

**Architecture:** `BootstrapEngine::check_and_fix()` runs a parallel check phase, builds a dependency-aware fix plan, executes fixes sequentially (batching brew installs into one `BrewBundleFixer` call), and returns a `BootstrapReport`. Test injection via `BootstrapContext::with_checker()` / `with_fixer()` enables deterministic integration tests without any real binaries.

**Tech Stack:** Rust, reqwest::blocking (already in Cargo.toml), existing dbd-core, existing Checker/Fixer/PlatformProvider traits, Tauri event system.

**DRY note on `run_bootstrap`:** The engine's Phase A already runs all checks. A call where everything passes returns `BootstrapReport { all_ok: true }` immediately — identical to the old `run_bootstrap` read-only check. Remove `run_bootstrap` as a Tauri command; the frontend uses only `check_and_fix_bootstrap`.

---

## File Map

| Action | File | Responsibility |
|---|---|---|
| Modify | `crates/bootstrap/src/config.rs` | Add `HOMEBREW_BREWFILE_URL`, `HOMEBREW_TAP_REPO` constants; add `sensei_binary()`, `senseid_binary()`, `sensei_mcp_binary()` methods |
| Modify | `crates/bootstrap/src/prereq/mod.rs` | Add `HumanAction`, `GateReport`, `BootstrapReport` types; expose `registry` and `engine` modules |
| Modify | `crates/bootstrap/src/prereq/fixer.rs` | Add `human_action()` default method to `Fixer` trait; add `BrewBundleFixer`, `HumanActionFixer` |
| Create | `crates/bootstrap/src/prereq/registry.rs` | `BuildContext`, `ComponentSpec`, `COMPONENTS` static (10 components) |
| Create | `crates/bootstrap/src/prereq/engine.rs` | `BootstrapContext` (with test injection), `BootstrapEngine::check_and_fix()` |
| Modify | `crates/bootstrap/src/lib.rs` | Add `check_and_fix()` convenience wrapper; re-export new public types |
| Modify | `crates/bootstrap/src/platform/macos.rs` | Remove embedded `BREWFILE` const |
| Delete | `crates/bootstrap/src/prereq/factory.rs` | Replaced by registry + engine |
| Modify | `app/src-tauri/src/commands/bootstrap.rs` | Remove `run_bootstrap`, `install_prerequisites`, `start_services`, `setup_database`; add `check_and_fix_bootstrap` |
| Modify | `app/src-tauri/src/lib.rs` | Update `invoke_handler` registrations |

---

## Task 1: Config additions

**Files:**
- Modify: `crates/bootstrap/src/config.rs`

- [ ] **Step 1.1: Write failing tests**

Add at the bottom of `config.rs` (inside existing `#[cfg(test)]` block):

```rust
#[test]
fn homebrew_brewfile_url_is_github_raw() {
    assert!(HOMEBREW_BREWFILE_URL.starts_with("https://raw.githubusercontent.com/"));
    assert!(HOMEBREW_BREWFILE_URL.contains("homebrew-tap"));
    assert!(HOMEBREW_BREWFILE_URL.ends_with("Brewfile"));
}

#[test]
fn binary_names_prod() {
    // Temporarily force prod mode by ensuring env var absent or set to prod
    let cfg = SenseiConfig {
        mode: SenseiMode::Prod,
        daemon_port: 7744,
        db_name: "sensei".to_string(),
        db_url: "postgresql://localhost/sensei".to_string(),
        dir_suffix: ".sensei",
    };
    assert_eq!(cfg.sensei_binary(), "sensei");
    assert_eq!(cfg.senseid_binary(), "senseid");
    assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp");
}

#[test]
fn binary_names_dev() {
    let cfg = SenseiConfig {
        mode: SenseiMode::Dev,
        daemon_port: 7745,
        db_name: "sensei_dev".to_string(),
        db_url: "postgresql://localhost/sensei_dev".to_string(),
        dir_suffix: ".sensei-dev",
    };
    assert_eq!(cfg.sensei_binary(), "sensei-dev");
    assert_eq!(cfg.senseid_binary(), "senseid-dev");
    assert_eq!(cfg.sensei_mcp_binary(), "sensei-mcp-dev");
}
```

- [ ] **Step 1.2: Run to verify tests fail**

```bash
cargo test -p sensei-bootstrap config 2>&1 | tail -20
```
Expected: `homebrew_brewfile_url_is_github_raw`, `binary_names_prod`, `binary_names_dev` FAIL (symbols not found).

- [ ] **Step 1.3: Implement constants and methods**

Add after line 29 (after existing constants) in `config.rs`:

```rust
/// Raw GitHub URL for the homebrew tap Brewfile (authoritative install source).
pub const HOMEBREW_BREWFILE_URL: &str =
    "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile";

/// Homebrew tap repository slug (for reference/logging).
pub const HOMEBREW_TAP_REPO: &str = "sensei-hq/homebrew-tap";
```

Add to the `impl SenseiConfig` block (after `pid_path`):

```rust
/// Returns the sensei CLI binary name for the current mode.
pub fn sensei_binary(&self) -> &'static str {
    if self.is_dev() { "sensei-dev" } else { "sensei" }
}

/// Returns the senseid daemon binary name for the current mode.
pub fn senseid_binary(&self) -> &'static str {
    if self.is_dev() { "senseid-dev" } else { "senseid" }
}

/// Returns the sensei-mcp binary name for the current mode.
pub fn sensei_mcp_binary(&self) -> &'static str {
    if self.is_dev() { "sensei-mcp-dev" } else { "sensei-mcp" }
}
```

Note: `dir_suffix` field is currently private. For the tests to construct `SenseiConfig` directly, change `dir_suffix` to `pub`:
```rust
pub dir_suffix: &'static str,
```

- [ ] **Step 1.4: Run tests to verify pass**

```bash
cargo test -p sensei-bootstrap config 2>&1 | tail -20
```
Expected: all config tests PASS.

- [ ] **Step 1.5: Commit**

```bash
git add crates/bootstrap/src/config.rs
git commit -m "feat(bootstrap): add HOMEBREW_BREWFILE_URL and binary name helpers to SenseiConfig"
```

---

## Task 2: New types in prereq/mod.rs

**Files:**
- Modify: `crates/bootstrap/src/prereq/mod.rs`

- [ ] **Step 2.1: Write failing tests**

Add inside the existing `#[cfg(test)]` block in `prereq/mod.rs`:

```rust
#[test]
fn human_action_stores_fields() {
    let a = HumanAction {
        component_id: "homebrew",
        title: "Install Homebrew".to_string(),
        command: "/bin/bash -c install.sh".to_string(),
        url: Some("https://brew.sh".to_string()),
    };
    assert_eq!(a.component_id, "homebrew");
    assert_eq!(a.command, "/bin/bash -c install.sh");
    assert!(a.url.is_some());
}

#[test]
fn gate_report_fix_attempted_flag() {
    let r = GateReport {
        id: "postgresql",
        status: GateStatus::Ready { version: Some("17.2".into()), detail: None },
        fix_attempted: true,
        fix_detail: Some("brew bundle upgraded".into()),
    };
    assert!(r.fix_attempted);
    assert_eq!(r.fix_detail.as_deref(), Some("brew bundle upgraded"));
}

#[test]
fn bootstrap_report_all_ok_false_when_any_failed() {
    let report = BootstrapReport {
        gates: vec![
            GateReport { id: "homebrew", status: GateStatus::Ready { version: None, detail: None }, fix_attempted: false, fix_detail: None },
            GateReport { id: "postgresql", status: GateStatus::Failed { error: "not found".into() }, fix_attempted: false, fix_detail: None },
        ],
        all_ok: false,
        blocked_on: None,
    };
    assert!(!report.all_ok);
    assert!(report.blocked_on.is_none());
}

#[test]
fn bootstrap_report_serializes() {
    let report = BootstrapReport {
        gates: vec![],
        all_ok: true,
        blocked_on: None,
    };
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("\"all_ok\":true"));
}
```

- [ ] **Step 2.2: Run to verify tests fail**

```bash
cargo test -p sensei-bootstrap prereq 2>&1 | tail -20
```
Expected: FAIL (HumanAction, GateReport, BootstrapReport not defined).

- [ ] **Step 2.3: Add types to prereq/mod.rs**

Add before the `pub mod checker;` line in `prereq/mod.rs`:

```rust
/// A human action required before bootstrap can continue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAction {
    pub component_id: &'static str,
    pub title:        String,
    pub command:      String,
    pub url:          Option<String>,
}

/// Final result for a single component gate after check_and_fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReport {
    pub id:            &'static str,
    pub status:        GateStatus,
    pub fix_attempted: bool,
    pub fix_detail:    Option<String>,
}

/// Returned by BootstrapEngine::check_and_fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapReport {
    pub gates:      Vec<GateReport>,
    pub all_ok:     bool,
    pub blocked_on: Option<HumanAction>,
}
```

- [ ] **Step 2.4: Run tests to verify pass**

```bash
cargo test -p sensei-bootstrap prereq 2>&1 | tail -20
```
Expected: all prereq/mod tests PASS.

- [ ] **Step 2.5: Commit**

```bash
git add crates/bootstrap/src/prereq/mod.rs
git commit -m "feat(bootstrap): add HumanAction, GateReport, BootstrapReport types"
```

---

## Task 3: Fixer trait + BrewBundleFixer + HumanActionFixer

**Files:**
- Modify: `crates/bootstrap/src/prereq/fixer.rs`

- [ ] **Step 3.1: Write failing tests**

Add inside the existing `#[cfg(test)]` block in `fixer.rs`:

```rust
#[test]
fn brew_bundle_fixer_stores_fields() {
    let f = BrewBundleFixer::new("/opt/homebrew/bin/brew", "https://example.com/Brewfile");
    assert_eq!(f.brew_path, "/opt/homebrew/bin/brew");
    assert_eq!(f.brewfile_url, "https://example.com/Brewfile");
}

#[test]
fn brew_bundle_fixer_nonexistent_brew_returns_err() {
    let f = BrewBundleFixer::new("/nonexistent/brew", "https://example.com/Brewfile");
    // reqwest will fail first (no URL), but if URL fails we still get an error
    let result = f.fix();
    assert!(result.is_err(), "should fail with invalid brew path");
}

#[test]
fn human_action_fixer_returns_none_from_fix() {
    let f = HumanActionFixer {
        component_id: "homebrew",
        title:        "Install Homebrew",
        command:      "/bin/bash install.sh",
        url:          Some("https://brew.sh"),
    };
    assert!(f.fix().is_err(), "HumanActionFixer fix() always returns Err");
}

#[test]
fn human_action_fixer_human_action_returns_some() {
    let f = HumanActionFixer {
        component_id: "homebrew",
        title:        "Install Homebrew",
        command:      "/bin/bash install.sh",
        url:          Some("https://brew.sh"),
    };
    let action = f.human_action();
    assert!(action.is_some());
    let a = action.unwrap();
    assert_eq!(a.component_id, "homebrew");
    assert_eq!(a.command, "/bin/bash install.sh");
    assert_eq!(a.url.as_deref(), Some("https://brew.sh"));
}

#[test]
fn noop_fixer_human_action_returns_none() {
    let f = NoopFixer::new("reason");
    assert!(f.human_action().is_none());
}
```

- [ ] **Step 3.2: Run to verify tests fail**

```bash
cargo test -p sensei-bootstrap fixer 2>&1 | tail -20
```
Expected: FAIL (`BrewBundleFixer`, `HumanActionFixer`, `human_action` not found).

- [ ] **Step 3.3: Add `human_action()` to Fixer trait**

Replace the `Fixer` trait definition in `fixer.rs`:

```rust
use super::HumanAction;

/// Pluggable fix strategy.
pub trait Fixer: Send + Sync {
    fn fix(&self) -> Result<FixResult, String>;
    /// If this fixer requires human action before it can proceed, return Some.
    /// The engine stops fix execution and surfaces the action to the user.
    fn human_action(&self) -> Option<HumanAction> { None }
}
```

- [ ] **Step 3.4: Add BrewBundleFixer**

Add after `BrewUpgradeFixer` in `fixer.rs`:

```rust
/// Fetches the Brewfile from a URL and pipes it to `brew bundle --upgrade --file=-`.
/// This is the single fixer for all binary components — handles both first installs
/// and upgrades in one pass. Idempotent: brew skips already-current formulae.
pub struct BrewBundleFixer {
    pub brew_path:    String,
    pub brewfile_url: String,
}

impl BrewBundleFixer {
    pub fn new(brew_path: impl Into<String>, brewfile_url: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), brewfile_url: brewfile_url.into() }
    }
}

impl Fixer for BrewBundleFixer {
    fn fix(&self) -> Result<FixResult, String> {
        // Fetch the Brewfile content from the tap
        let brewfile = reqwest::blocking::get(&self.brewfile_url)
            .map_err(|e| format!("failed to fetch Brewfile from {}: {e}", self.brewfile_url))?
            .error_for_status()
            .map_err(|e| format!("Brewfile URL returned error: {e}"))?
            .text()
            .map_err(|e| format!("failed to read Brewfile response: {e}"))?;

        // Pipe the Brewfile to `brew bundle --upgrade --file=-`
        let mut child = std::process::Command::new(&self.brew_path)
            .args(["bundle", "--upgrade", "--file=-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn brew bundle: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            stdin.write_all(brewfile.as_bytes())
                .map_err(|e| format!("failed to write Brewfile to stdin: {e}"))?;
        }

        let output = child.wait_with_output()
            .map_err(|e| format!("failed to wait for brew bundle: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new("brew bundle --upgrade completed"))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("brew bundle --upgrade failed: {stderr}"))
        }
    }
}
```

- [ ] **Step 3.5: Add HumanActionFixer**

Add after `BrewBundleFixer` in `fixer.rs`:

```rust
/// Fixer that cannot auto-fix — surfaces a structured human action instead.
/// Used for: Homebrew missing (cannot auto-install), dev-mode binaries (run `make install-dev`).
pub struct HumanActionFixer {
    pub component_id: &'static str,
    pub title:        &'static str,
    pub command:      &'static str,
    pub url:          Option<&'static str>,
}

impl Fixer for HumanActionFixer {
    fn fix(&self) -> Result<FixResult, String> {
        Err(format!("human action required: {}", self.title))
    }

    fn human_action(&self) -> Option<HumanAction> {
        Some(HumanAction {
            component_id: self.component_id,
            title:        self.title.to_string(),
            command:      self.command.to_string(),
            url:          self.url.map(|s| s.to_string()),
        })
    }
}
```

Also add `use reqwest;` at the top of `fixer.rs`.

- [ ] **Step 3.6: Run tests to verify pass**

```bash
cargo test -p sensei-bootstrap fixer 2>&1 | tail -30
```
Expected: all fixer tests PASS.

- [ ] **Step 3.7: Commit**

```bash
git add crates/bootstrap/src/prereq/fixer.rs
git commit -m "feat(bootstrap): add Fixer::human_action, BrewBundleFixer, HumanActionFixer"
```

---

## Task 4: Component registry (prereq/registry.rs)

**Files:**
- Create: `crates/bootstrap/src/prereq/registry.rs`

- [ ] **Step 4.1: Write failing test file**

Create `registry.rs` with test stubs only:

```rust
//! Component registry — defines all 10 bootstrap components and their dependency graph.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_count_is_ten() {
        assert_eq!(COMPONENTS.len(), 10);
    }

    #[test]
    fn component_ids_are_unique() {
        let mut ids: std::collections::HashSet<&str> = Default::default();
        for c in COMPONENTS {
            assert!(ids.insert(c.id), "duplicate component id: {}", c.id);
        }
    }

    #[test]
    fn depends_on_ids_are_valid() {
        let valid_ids: std::collections::HashSet<&str> = COMPONENTS.iter().map(|c| c.id).collect();
        for c in COMPONENTS {
            for dep in c.depends_on {
                assert!(valid_ids.contains(dep), "component '{}' depends_on '{}' which does not exist", c.id, dep);
            }
        }
    }

    #[test]
    fn post_fix_trigger_ids_are_valid() {
        let valid_ids: std::collections::HashSet<&str> = COMPONENTS.iter().map(|c| c.id).collect();
        for c in COMPONENTS {
            for tid in c.post_fix_trigger {
                assert!(valid_ids.contains(tid), "component '{}' post_fix_trigger '{}' is not a valid id", c.id, tid);
            }
        }
    }

    #[test]
    fn homebrew_has_no_dependencies() {
        let homebrew = COMPONENTS.iter().find(|c| c.id == "homebrew").unwrap();
        assert!(homebrew.depends_on.is_empty());
        assert!(homebrew.fix_group.is_none());
    }

    #[test]
    fn senseid_triggers_database() {
        let senseid = COMPONENTS.iter().find(|c| c.id == "senseid").unwrap();
        assert!(senseid.post_fix_trigger.contains(&"database"), "senseid must trigger database");
    }

    #[test]
    fn build_context_constructs_checkers_and_fixers() {
        use std::sync::Arc;
        use crate::platform;
        use crate::config::SenseiConfig;
        let ctx = BuildContext {
            platform: &Arc::from(platform::detect()),
            config: &SenseiConfig::from_env(),
            app_version: "0.1.0",
        };
        for spec in COMPONENTS {
            // Must not panic
            let _checker = (spec.checker_fn)(&ctx);
            let _fixer   = (spec.fixer_fn)(&ctx);
        }
    }
}
```

- [ ] **Step 4.2: Run to verify tests fail (symbol not found)**

```bash
cargo test -p sensei-bootstrap registry 2>&1 | tail -10
```
Expected: FAIL (module not found — we haven't added it to mod.rs yet).

Add to `prereq/mod.rs`:
```rust
pub mod registry;
```

Re-run — should fail with "COMPONENTS not found" etc.

- [ ] **Step 4.3: Implement BuildContext and ComponentSpec**

Fill in `registry.rs`:

```rust
//! Component registry — defines all 10 bootstrap components and their dependency graph.

use std::sync::Arc;
use crate::platform::PlatformProvider;
use crate::config::SenseiConfig;
use super::checker::Checker;
use super::fixer::Fixer;
use super::GateKind;

/// Lightweight context passed to checker_fn / fixer_fn when building components.
/// Does NOT contain test-injection maps — those live in BootstrapContext (engine.rs).
pub struct BuildContext<'a> {
    pub platform:    &'a Arc<dyn PlatformProvider>,
    pub config:      &'a SenseiConfig,
    pub app_version: &'a str,
}

/// A single bootstrap component — id, dependency edges, and factory functions.
pub struct ComponentSpec {
    pub id:               &'static str,
    pub label:            &'static str,
    pub depends_on:       &'static [&'static str],
    /// If Some, component participates in a BrewBundleFixer batch with others sharing the same group.
    pub fix_group:        Option<&'static str>,
    /// After this component is successfully fixed, force these ids into the fix plan.
    pub post_fix_trigger: &'static [&'static str],
    pub gate_kind:        GateKind,
    pub checker_fn:       fn(&BuildContext) -> Box<dyn Checker>,
    pub fixer_fn:         fn(&BuildContext) -> Box<dyn Fixer>,
}
```

- [ ] **Step 4.4: Implement COMPONENTS — homebrew and bundle components**

Add to `registry.rs`:

```rust
use crate::POSTGRES_PORT;
use crate::OLLAMA_PORT;
use super::checker::{BinaryChecker, PortChecker, VersionedBinaryChecker, DatabaseChecker};
use super::fixer::{HumanActionFixer, BrewBundleFixer, ServiceStartFixer, DatabaseSetupFixer, NoopFixer};
use crate::config::HOMEBREW_BREWFILE_URL;

fn detect_brew_path() -> Option<String> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
}

pub static COMPONENTS: &[ComponentSpec] = &[
    // ── Homebrew ─────────────────────────────────────────────────────────────
    ComponentSpec {
        id: "homebrew",
        label: "Homebrew",
        depends_on: &[],
        fix_group: None,
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |ctx| Box::new({
            // Delegate to PlatformProvider::check_package_manager — already handles brew detection
            struct HomebrewChecker(Arc<dyn PlatformProvider>);
            use super::checker::Checker;
            use super::CheckResult;
            impl Checker for HomebrewChecker {
                fn check(&self) -> CheckResult {
                    let status = self.0.check_package_manager();
                    if status.is_ready() {
                        CheckResult::ok(status.version.as_deref().unwrap_or("unknown"))
                    } else {
                        CheckResult::fail(match &status.state {
                            crate::types::ComponentState::Failed { error } => error.clone(),
                            _ => "homebrew not found".to_string(),
                        })
                    }
                }
            }
            HomebrewChecker(Arc::clone(ctx.platform))
        }),
        fixer_fn: |_ctx| Box::new(HumanActionFixer {
            component_id: "homebrew",
            title:        "Install Homebrew",
            command:      "/bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"",
            url:          Some("https://brew.sh"),
        }),
    },
    // ── PostgreSQL (binary) ──────────────────────────────────────────────────
    ComponentSpec {
        id: "postgresql",
        label: "PostgreSQL",
        depends_on: &["homebrew"],
        fix_group: Some("bundle"),
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |_ctx| Box::new(BinaryChecker::new("postgres", "--version")),
        fixer_fn:   |_ctx| {
            match detect_brew_path() {
                Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
                None       => Box::new(NoopFixer::new("Homebrew not found")),
            }
        },
    },
    // ── Ollama (binary) ──────────────────────────────────────────────────────
    ComponentSpec {
        id: "ollama",
        label: "Ollama",
        depends_on: &["homebrew"],
        fix_group: Some("bundle"),
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |_ctx| Box::new(BinaryChecker::new("ollama", "--version")),
        fixer_fn:   |_ctx| {
            match detect_brew_path() {
                Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
                None       => Box::new(NoopFixer::new("Homebrew not found")),
            }
        },
    },
    // ── Sensei CLI ───────────────────────────────────────────────────────────
    ComponentSpec {
        id: "sensei",
        label: "Sensei CLI",
        depends_on: &["homebrew"],
        fix_group: Some("bundle"),    // overridden to None in dev mode at engine time
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.sensei_binary();
            Box::new(VersionedBinaryChecker::new(bin, "--version", ctx.app_version))
        },
        fixer_fn: |ctx| {
            if ctx.config.is_dev() {
                Box::new(HumanActionFixer {
                    component_id: "sensei",
                    title:        "Build dev binaries",
                    command:      "make install-dev",
                    url:          None,
                })
            } else {
                match detect_brew_path() {
                    Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
                    None       => Box::new(NoopFixer::new("Homebrew not found")),
                }
            }
        },
    },
    // ── Sensei Daemon (binary) ───────────────────────────────────────────────
    ComponentSpec {
        id: "senseid",
        label: "Sensei Daemon",
        depends_on: &["homebrew"],
        fix_group: Some("bundle"),
        post_fix_trigger: &["database"],
        gate_kind: GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.senseid_binary();
            Box::new(VersionedBinaryChecker::new(bin, "--version", ctx.app_version))
        },
        fixer_fn: |ctx| {
            if ctx.config.is_dev() {
                Box::new(HumanActionFixer {
                    component_id: "senseid",
                    title:        "Build dev binaries",
                    command:      "make install-dev",
                    url:          None,
                })
            } else {
                match detect_brew_path() {
                    Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
                    None       => Box::new(NoopFixer::new("Homebrew not found")),
                }
            }
        },
    },
    // ── Sensei MCP (binary) ──────────────────────────────────────────────────
    ComponentSpec {
        id: "sensei_mcp",
        label: "Sensei MCP",
        depends_on: &["homebrew"],
        fix_group: Some("bundle"),
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |ctx| {
            let bin = ctx.config.sensei_mcp_binary();
            Box::new(BinaryChecker::new(bin, "--version"))
        },
        fixer_fn: |ctx| {
            if ctx.config.is_dev() {
                Box::new(HumanActionFixer {
                    component_id: "sensei_mcp",
                    title:        "Build dev binaries",
                    command:      "make install-dev",
                    url:          None,
                })
            } else {
                match detect_brew_path() {
                    Some(brew) => Box::new(BrewBundleFixer::new(brew, HOMEBREW_BREWFILE_URL)),
                    None       => Box::new(NoopFixer::new("Homebrew not found")),
                }
            }
        },
    },
    // ── PostgreSQL Service ───────────────────────────────────────────────────
    ComponentSpec {
        id: "postgresql_service",
        label: "PostgreSQL Service",
        depends_on: &["postgresql"],
        fix_group: None,
        post_fix_trigger: &[],
        gate_kind: GateKind::Service,
        checker_fn: |_ctx| Box::new(PortChecker::new("postgresql", POSTGRES_PORT)),
        fixer_fn:   |ctx| Box::new(ServiceStartFixer::new(Arc::clone(ctx.platform), "postgresql", POSTGRES_PORT)),
    },
    // ── Ollama Service ───────────────────────────────────────────────────────
    ComponentSpec {
        id: "ollama_service",
        label: "Ollama Service",
        depends_on: &["ollama"],
        fix_group: None,
        post_fix_trigger: &[],
        gate_kind: GateKind::Service,
        checker_fn: |_ctx| Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
        fixer_fn:   |ctx| Box::new(ServiceStartFixer::new(Arc::clone(ctx.platform), "ollama", OLLAMA_PORT)),
    },
    // ── Sensei Database ──────────────────────────────────────────────────────
    ComponentSpec {
        id: "database",
        label: "Sensei Database",
        depends_on: &["postgresql_service", "senseid"],
        fix_group: None,
        post_fix_trigger: &[],
        gate_kind: GateKind::Install,
        checker_fn: |_ctx| Box::new(DatabaseChecker),
        fixer_fn:   |ctx| Box::new(DatabaseSetupFixer::new(ctx.app_version)),
    },
    // ── Sensei Daemon Service ────────────────────────────────────────────────
    ComponentSpec {
        id: "daemon",
        label: "Sensei Daemon Service",
        depends_on: &["senseid", "database"],
        fix_group: None,
        post_fix_trigger: &[],
        gate_kind: GateKind::Service,
        checker_fn: |ctx| Box::new(PortChecker::new("daemon", ctx.config.daemon_port)),
        fixer_fn:   |ctx| Box::new(ServiceStartFixer::new(Arc::clone(ctx.platform), "daemon", ctx.config.daemon_port)),
    },
];
```

Note: `sensei`, `senseid`, `sensei_mcp` have `fix_group: Some("bundle")` set statically, but the engine skips adding them to the bundle batch in dev mode (they use `HumanActionFixer` which immediately stops execution via `human_action()`). The engine checks `fixer.human_action()` before grouping into a batch.

- [ ] **Step 4.5: Run tests to verify pass**

```bash
cargo test -p sensei-bootstrap registry 2>&1 | tail -20
```
Expected: all 7 registry tests PASS.

- [ ] **Step 4.6: Commit**

```bash
git add crates/bootstrap/src/prereq/registry.rs crates/bootstrap/src/prereq/mod.rs
git commit -m "feat(bootstrap): add ComponentSpec registry with 10 components and dependency graph"
```

---

## Task 5: BootstrapContext + BootstrapEngine skeleton

**Files:**
- Create: `crates/bootstrap/src/prereq/engine.rs`

- [ ] **Step 5.1: Write failing tests**

Create `engine.rs` with only the test block:

```rust
//! Bootstrap engine — dependency-aware check-and-fix orchestrator.

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx() -> BootstrapContext {
        use std::sync::Arc;
        use crate::platform;
        use crate::config::SenseiConfig;
        BootstrapContext::new(
            Arc::from(platform::detect()),
            SenseiConfig::from_env(),
            "0.1.0".to_string(),
        )
    }

    #[test]
    fn bootstrap_context_constructs() {
        let ctx = make_ctx();
        assert_eq!(ctx.app_version, "0.1.0");
    }

    #[test]
    fn with_checker_stores_override() {
        use std::sync::Arc;
        use crate::prereq::CheckResult;
        use crate::prereq::checker::Checker;

        struct AlwaysOk;
        impl Checker for AlwaysOk {
            fn check(&self) -> CheckResult { CheckResult::ok("mock") }
        }

        let ctx = make_ctx().with_checker("postgresql", Arc::new(AlwaysOk));
        assert!(ctx.checker_overrides.contains_key("postgresql"));
    }

    #[test]
    fn with_fixer_stores_override() {
        use std::sync::Arc;
        use crate::prereq::{FixResult, fixer::Fixer};

        struct AlwaysFix;
        impl Fixer for AlwaysFix {
            fn fix(&self) -> Result<FixResult, String> { Ok(FixResult::new("mock")) }
        }

        let ctx = make_ctx().with_fixer("postgresql", Arc::new(AlwaysFix));
        assert!(ctx.fixer_overrides.contains_key("postgresql"));
    }
}
```

- [ ] **Step 5.2: Run to verify tests fail**

Add to `prereq/mod.rs`:
```rust
pub mod engine;
```

```bash
cargo test -p sensei-bootstrap engine 2>&1 | tail -10
```
Expected: FAIL (BootstrapContext not defined).

- [ ] **Step 5.3: Implement BootstrapContext**

```rust
//! Bootstrap engine — dependency-aware check-and-fix orchestrator.

use std::collections::HashMap;
use std::sync::Arc;
use crate::platform::PlatformProvider;
use crate::config::SenseiConfig;
use super::checker::Checker;
use super::fixer::Fixer;
use super::{BootstrapReport, GateReport, GateStatus, HumanAction, ProgressEvent};
use super::registry::{BuildContext, COMPONENTS};

/// Full bootstrap context — owns platform, config, app_version, and test-injection maps.
pub struct BootstrapContext {
    pub platform:    Arc<dyn PlatformProvider>,
    pub config:      SenseiConfig,
    pub app_version: String,
    /// Test-only checker overrides indexed by component id.
    pub(crate) checker_overrides: HashMap<&'static str, Arc<dyn Checker>>,
    /// Test-only fixer overrides indexed by component id.
    pub(crate) fixer_overrides:   HashMap<&'static str, Arc<dyn Fixer>>,
}

impl BootstrapContext {
    pub fn new(platform: Arc<dyn PlatformProvider>, config: SenseiConfig, app_version: String) -> Self {
        Self {
            platform,
            config,
            app_version,
            checker_overrides: HashMap::new(),
            fixer_overrides:   HashMap::new(),
        }
    }

    /// Inject a mock checker for a component id (used in tests).
    pub fn with_checker(mut self, id: &'static str, checker: Arc<dyn Checker>) -> Self {
        self.checker_overrides.insert(id, checker);
        self
    }

    /// Inject a mock fixer for a component id (used in tests).
    pub fn with_fixer(mut self, id: &'static str, fixer: Arc<dyn Fixer>) -> Self {
        self.fixer_overrides.insert(id, fixer);
        self
    }

    /// Build a BuildContext borrowing from this context.
    pub(crate) fn build_context(&self) -> BuildContext<'_> {
        BuildContext {
            platform:    &self.platform,
            config:      &self.config,
            app_version: &self.app_version,
        }
    }
}

/// The bootstrap engine — runs check_and_fix against the component registry.
pub struct BootstrapEngine {
    ctx: Arc<BootstrapContext>,
}

impl BootstrapEngine {
    pub fn new(ctx: Arc<BootstrapContext>) -> Self {
        Self { ctx }
    }

    /// Check all components, fix what's broken, return a full report.
    /// Emits ProgressEvents via callback throughout.
    pub fn check_and_fix(&self, _callback: impl Fn(ProgressEvent) + Send) -> BootstrapReport {
        // Skeleton — Phase A only (all checked, no fixing yet)
        BootstrapReport {
            gates: vec![],
            all_ok: false,
            blocked_on: None,
        }
    }
}
```

- [ ] **Step 5.4: Run tests to verify pass**

```bash
cargo test -p sensei-bootstrap engine 2>&1 | tail -20
```
Expected: `bootstrap_context_constructs`, `with_checker_stores_override`, `with_fixer_stores_override` PASS.

- [ ] **Step 5.5: Commit**

```bash
git add crates/bootstrap/src/prereq/engine.rs crates/bootstrap/src/prereq/mod.rs
git commit -m "feat(bootstrap): add BootstrapContext and BootstrapEngine skeleton"
```

---

## Task 6: Engine Phase A (parallel check) + Phase B (fix plan)

**Files:**
- Modify: `crates/bootstrap/src/prereq/engine.rs`

- [ ] **Step 6.1: Write failing tests**

Add to the `#[cfg(test)]` block in `engine.rs`:

```rust
fn make_mock_checker(ok: bool) -> Arc<dyn Checker> {
    use crate::prereq::CheckResult;
    struct M(bool);
    impl Checker for M {
        fn check(&self) -> CheckResult {
            if self.0 { CheckResult::ok("mock") } else { CheckResult::fail("mock fail") }
        }
    }
    Arc::new(M(ok))
}

fn make_mock_fixer(succeed: bool) -> Arc<dyn Fixer> {
    use crate::prereq::FixResult;
    struct M(bool);
    impl Fixer for M {
        fn fix(&self) -> Result<FixResult, String> {
            if self.0 { Ok(FixResult::new("mock fixed")) } else { Err("mock fix failed".into()) }
        }
    }
    Arc::new(M(succeed))
}

#[test]
fn all_ready_returns_all_ok_true() {
    use std::sync::Arc;
    use crate::platform;
    use crate::config::SenseiConfig;
    // Inject ok checkers for all 10 components
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()),
        SenseiConfig::from_env(),
        "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        ctx = ctx.with_checker(spec.id, make_mock_checker(true));
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    assert!(report.all_ok, "all ready mocks should produce all_ok=true");
    assert!(report.blocked_on.is_none());
    assert_eq!(report.gates.len(), 10);
}

#[test]
fn all_ready_emits_checking_then_ready_events() {
    use std::sync::{Arc, Mutex};
    use crate::platform;
    use crate::config::SenseiConfig;
    let events: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    let events_cb = Arc::clone(&events);
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()),
        SenseiConfig::from_env(),
        "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        ctx = ctx.with_checker(spec.id, make_mock_checker(true));
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    engine.check_and_fix(move |e| {
        if let ProgressEvent::Gate { id, status } = &e {
            let tag = match status {
                GateStatus::Checking => format!("{id}:checking"),
                GateStatus::Ready { .. } => format!("{id}:ready"),
                _ => format!("{id}:other"),
            };
            events_cb.lock().unwrap().push(tag);
        }
    });
    let ev = events.lock().unwrap();
    // Every component should have emitted both checking and ready
    for spec in super::registry::COMPONENTS {
        assert!(ev.contains(&format!("{}:checking", spec.id)), "missing checking for {}", spec.id);
        assert!(ev.contains(&format!("{}:ready", spec.id)), "missing ready for {}", spec.id);
    }
}
```

- [ ] **Step 6.2: Run to verify tests fail**

```bash
cargo test -p sensei-bootstrap engine::tests::all_ready 2>&1 | tail -20
```
Expected: FAIL (check_and_fix still returns empty skeleton).

- [ ] **Step 6.3: Implement Phase A (parallel check)**

Replace the `check_and_fix` body in `engine.rs`:

```rust
pub fn check_and_fix(&self, callback: impl Fn(ProgressEvent) + Send + 'static) -> BootstrapReport {
    let callback = Arc::new(callback);
    let ctx = Arc::clone(&self.ctx);

    // ── Phase A: Parallel check ───────────────────────────────────────────
    // Spawn one thread per component, collect (id, CheckResult)
    let handles: Vec<_> = COMPONENTS.iter().map(|spec| {
        let ctx = Arc::clone(&ctx);
        let cb = Arc::clone(&callback);
        let id = spec.id;
        let check_fn: Box<dyn FnOnce() -> (&&'static str, super::CheckResult) + Send> = {
            let checker: Arc<dyn Checker> = if let Some(c) = ctx.checker_overrides.get(id) {
                Arc::clone(c)
            } else {
                let bctx = ctx.build_context();
                Arc::from((spec.checker_fn)(&bctx))
            };
            Box::new(move || {
                cb(ProgressEvent::Gate { id: id.to_string(), status: GateStatus::Checking });
                (&id, checker.check())
            })
        };
        // We can't move a non-'static closure into a thread directly.
        // Use a channel to collect results.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = checker.check();
            cb(ProgressEvent::Gate { id: id.to_string(), status: GateStatus::Checking });
            let _ = tx.send((id, result));
        });
        rx
    }).collect();
```

Wait — the threading approach needs to be cleaner. Let me rewrite Phase A properly:

```rust
pub fn check_and_fix<F>(&self, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let callback = Arc::new(callback);
    let ctx = Arc::clone(&self.ctx);

    // ── Phase A: Parallel check ───────────────────────────────────────────
    use std::sync::mpsc;
    let (tx, rx) = mpsc::channel::<(&'static str, super::CheckResult)>();

    for spec in COMPONENTS {
        let tx = tx.clone();
        let cb = Arc::clone(&callback);
        let checker: Arc<dyn Checker> = ctx.checker_overrides
            .get(spec.id)
            .map(Arc::clone)
            .unwrap_or_else(|| {
                let bctx = ctx.build_context();
                Arc::from((spec.checker_fn)(&bctx))
            });
        let id = spec.id;
        std::thread::spawn(move || {
            cb(ProgressEvent::Gate { id: id.to_string(), status: GateStatus::Checking });
            let result = checker.check();
            let _ = tx.send((id, result));
        });
    }
    drop(tx); // close the sending end so rx knows when all senders are done

    // Collect results
    let mut check_results: HashMap<&'static str, super::CheckResult> = HashMap::new();
    for (id, result) in rx {
        if result.ok {
            callback(ProgressEvent::Gate {
                id: id.to_string(),
                status: GateStatus::Ready { version: result.version.clone(), detail: result.detail.clone() },
            });
        }
        check_results.insert(id, result);
    }

    // Separate ready vs pending
    let ready: std::collections::HashSet<&'static str> = check_results.iter()
        .filter(|(_, r)| r.ok)
        .map(|(id, _)| *id)
        .collect();
    let pending: Vec<&'static str> = COMPONENTS.iter()
        .map(|s| s.id)
        .filter(|id| !ready.contains(id))
        .collect();

    // If all ready, build report and return early
    if pending.is_empty() {
        let gates = COMPONENTS.iter().map(|spec| GateReport {
            id: spec.id,
            status: GateStatus::Ready {
                version: check_results[spec.id].version.clone(),
                detail:  check_results[spec.id].detail.clone(),
            },
            fix_attempted: false,
            fix_detail: None,
        }).collect();
        return BootstrapReport { gates, all_ok: true, blocked_on: None };
    }

    // Phase B + C follow in next task ...
    BootstrapReport { gates: vec![], all_ok: false, blocked_on: None }
}
```

- [ ] **Step 6.4: Run Phase A tests**

```bash
cargo test -p sensei-bootstrap engine::tests::all_ready 2>&1 | tail -20
```
Expected: both `all_ready_*` tests PASS.

- [ ] **Step 6.5: Implement Phase B (fix plan: topo-sort + dep-blocked + batch grouping)**

Add a helper method `build_fix_plan` to `BootstrapEngine`:

```rust
/// Represents one step in the sequential fix plan.
enum PlanStep {
    /// A single component fix (not batched).
    Individual(&'static str),
    /// A batch of bundle components fixed together via BrewBundleFixer.
    Batch(Vec<&'static str>),
}

impl BootstrapEngine {
    /// Topo-sort `pending` by depends_on edges, mark dep-blocked components,
    /// and group consecutive fix_group="bundle" non-dep-blocked components.
    fn build_fix_plan(
        pending: &[&'static str],
        ready: &std::collections::HashSet<&'static str>,
    ) -> (Vec<PlanStep>, std::collections::HashSet<&'static str>) {
        let pending_set: std::collections::HashSet<&'static str> = pending.iter().copied().collect();

        // Mark dep-blocked: any pending component whose depends_on has a pending (failed) member
        let mut dep_blocked: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
        for spec in COMPONENTS {
            if !pending_set.contains(spec.id) { continue; }
            let blocked = spec.depends_on.iter().any(|dep| pending_set.contains(dep));
            if blocked { dep_blocked.insert(spec.id); }
        }

        // Topo-sort: iterate COMPONENTS in definition order (already a valid topo order
        // as long as depends_on only reference earlier entries — enforced by registry definition).
        // Components not in pending are skipped.
        let ordered: Vec<&'static str> = COMPONENTS.iter()
            .map(|s| s.id)
            .filter(|id| pending_set.contains(id))
            .collect();

        // Group consecutive fix_group="bundle" non-dep-blocked components into Batch steps.
        let mut plan: Vec<PlanStep> = Vec::new();
        let mut bundle_batch: Vec<&'static str> = Vec::new();

        let flush_bundle = |batch: &mut Vec<&'static str>, plan: &mut Vec<PlanStep>| {
            if batch.len() == 1 {
                plan.push(PlanStep::Individual(batch[0]));
            } else if !batch.is_empty() {
                plan.push(PlanStep::Batch(batch.clone()));
            }
            batch.clear();
        };

        for &id in &ordered {
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            if dep_blocked.contains(id) {
                flush_bundle(&mut bundle_batch, &mut plan);
                plan.push(PlanStep::Individual(id)); // dep-blocked individual (will be skipped in Phase C)
                continue;
            }
            if spec.fix_group == Some("bundle") {
                bundle_batch.push(id);
            } else {
                flush_bundle(&mut bundle_batch, &mut plan);
                plan.push(PlanStep::Individual(id));
            }
        }
        flush_bundle(&mut bundle_batch, &mut plan);

        (plan, dep_blocked)
    }
}
```

- [ ] **Step 6.6: Write Phase B tests**

Add to `#[cfg(test)]`:

```rust
#[test]
fn build_fix_plan_dep_blocked_when_homebrew_fails() {
    use std::collections::HashSet;
    let pending = vec!["homebrew", "postgresql", "ollama"];
    let ready: HashSet<&'static str> = HashSet::new();
    let (plan, dep_blocked) = BootstrapEngine::build_fix_plan(&pending, &ready);
    assert!(dep_blocked.contains("postgresql"), "postgresql should be dep-blocked by homebrew");
    assert!(dep_blocked.contains("ollama"), "ollama should be dep-blocked by homebrew");
    assert!(!dep_blocked.contains("homebrew"), "homebrew itself is not dep-blocked");
    // homebrew should be an Individual step
    assert!(plan.iter().any(|s| matches!(s, PlanStep::Individual("homebrew"))));
}

#[test]
fn build_fix_plan_bundles_postgresql_and_ollama() {
    use std::collections::HashSet;
    let pending = vec!["postgresql", "ollama"];
    let mut ready: HashSet<&'static str> = HashSet::new();
    ready.insert("homebrew"); // homebrew ready — so postgresql/ollama are not dep-blocked
    let (plan, dep_blocked) = BootstrapEngine::build_fix_plan(&pending, &ready);
    assert!(dep_blocked.is_empty(), "no dep-blocked when homebrew is ready");
    // postgresql and ollama should be in a Batch together
    assert!(plan.iter().any(|s| matches!(s, PlanStep::Batch(ids) if ids.contains(&"postgresql") && ids.contains(&"ollama"))),
        "postgresql and ollama should be batched together");
}
```

- [ ] **Step 6.7: Run Phase B tests**

```bash
cargo test -p sensei-bootstrap engine::tests::build_fix_plan 2>&1 | tail -20
```
Expected: both PASS.

- [ ] **Step 6.8: Commit**

```bash
git add crates/bootstrap/src/prereq/engine.rs
git commit -m "feat(bootstrap): engine Phase A (parallel check) and Phase B (fix plan)"
```

---

## Task 7: Engine Phase C (sequential fix) + Phase D (return report)

**Files:**
- Modify: `crates/bootstrap/src/prereq/engine.rs`

- [ ] **Step 7.1: Write failing integration test scaffolding**

Add to `#[cfg(test)]`:

```rust
// Sequenced mock checker: returns values from a VecDeque (first=initial, second=after fix)
struct SeqChecker(std::sync::Mutex<std::collections::VecDeque<bool>>);
impl SeqChecker {
    fn new(seq: &[bool]) -> Arc<dyn Checker> {
        use std::collections::VecDeque;
        Arc::new(Self(std::sync::Mutex::new(VecDeque::from(seq.to_vec()))))
    }
}
impl Checker for SeqChecker {
    fn check(&self) -> super::CheckResult {
        let val = self.0.lock().unwrap().pop_front().unwrap_or(false);
        if val { super::CheckResult::ok("mock") } else { super::CheckResult::fail("mock fail") }
    }
}

#[test]
fn failing_postgresql_fixed_becomes_ready() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    // postgresql: fail on check, pass on recheck
    // all others: always pass
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        if spec.id == "postgresql" {
            ctx = ctx.with_checker(spec.id, SeqChecker::new(&[false, true]));
            ctx = ctx.with_fixer(spec.id, make_mock_fixer(true));
        } else {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
    assert!(matches!(pg.status, GateStatus::Ready { .. }), "postgresql should be Ready after fix");
    assert!(pg.fix_attempted, "fix_attempted should be true");
}

#[test]
fn homebrew_missing_returns_blocked_on() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    use super::fixer::HumanActionFixer;
    struct HomebrewHumanFixer;
    impl Fixer for HomebrewHumanFixer {
        fn fix(&self) -> Result<super::FixResult, String> { Err("human action required".into()) }
        fn human_action(&self) -> Option<HumanAction> {
            Some(HumanAction {
                component_id: "homebrew",
                title: "Install Homebrew".to_string(),
                command: "/bin/bash install.sh".to_string(),
                url: None,
            })
        }
    }
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(false));
    ctx = ctx.with_fixer("homebrew", Arc::new(HomebrewHumanFixer));
    for spec in super::registry::COMPONENTS {
        if spec.id != "homebrew" {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    assert!(!report.all_ok);
    assert!(report.blocked_on.is_some());
    assert_eq!(report.blocked_on.unwrap().component_id, "homebrew");
}
```

- [ ] **Step 7.2: Run to verify tests fail**

```bash
cargo test -p sensei-bootstrap engine::tests::failing_postgresql 2>&1 | tail -10
cargo test -p sensei-bootstrap engine::tests::homebrew_missing 2>&1 | tail -10
```
Expected: FAIL (Phase C not implemented).

- [ ] **Step 7.3: Implement Phase C in check_and_fix**

Replace the stub at the end of `check_and_fix` (after Phase B call) with:

```rust
    // ── Phase B: Build fix plan ───────────────────────────────────────────
    let (mut plan, dep_blocked) = Self::build_fix_plan(&pending, &ready);

    // ── Phase C: Sequential fix execution ────────────────────────────────
    // Gate reports accumulate here. Initialize with already-ready gates.
    let mut gate_reports: HashMap<&'static str, GateReport> = HashMap::new();
    for spec in COMPONENTS {
        if ready.contains(spec.id) {
            gate_reports.insert(spec.id, GateReport {
                id: spec.id,
                status: GateStatus::Ready {
                    version: check_results[spec.id].version.clone(),
                    detail:  check_results[spec.id].detail.clone(),
                },
                fix_attempted: false,
                fix_detail: None,
            });
        }
    }

    let mut blocked_on: Option<HumanAction> = None;
    let mut post_trigger_ids: Vec<&'static str> = Vec::new();

    // Helper: run checker for an id (uses override or spec fn)
    let run_check = |id: &'static str| -> super::CheckResult {
        if let Some(c) = ctx.checker_overrides.get(id) {
            c.check()
        } else {
            let bctx = ctx.build_context();
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            (spec.checker_fn)(&bctx).check()
        }
    };

    // Helper: get fixer for an id (uses override or spec fn)
    let get_fixer = |id: &'static str| -> Arc<dyn Fixer> {
        if let Some(f) = ctx.fixer_overrides.get(id) {
            Arc::clone(f)
        } else {
            let bctx = ctx.build_context();
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            Arc::from((spec.fixer_fn)(&bctx))
        }
    };

    for step in &plan {
        if blocked_on.is_some() { break; }

        match step {
            PlanStep::Individual(id) => {
                let id = *id;
                let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();

                if dep_blocked.contains(id) {
                    callback(ProgressEvent::Gate {
                        id: id.to_string(),
                        status: GateStatus::Failed {
                            error: format!("dep-blocked: {:?} not ready", spec.depends_on),
                        },
                    });
                    gate_reports.insert(id, GateReport {
                        id,
                        status: GateStatus::Failed { error: format!("dep: {:?} not ready", spec.depends_on) },
                        fix_attempted: false,
                        fix_detail: None,
                    });
                    continue;
                }

                let fixer = get_fixer(id);

                // Check for human action before attempting fix
                if let Some(action) = fixer.human_action() {
                    callback(ProgressEvent::Gate {
                        id: id.to_string(),
                        status: GateStatus::Failed { error: "human action required".to_string() },
                    });
                    gate_reports.insert(id, GateReport {
                        id,
                        status: GateStatus::Failed { error: "human action required".to_string() },
                        fix_attempted: false,
                        fix_detail: None,
                    });
                    blocked_on = Some(action);
                    break;
                }

                // Emit in-progress status
                let in_progress = match spec.gate_kind {
                    GateKind::Install => GateStatus::Installing,
                    GateKind::Service => GateStatus::Starting,
                };
                callback(ProgressEvent::Gate { id: id.to_string(), status: in_progress });

                // Run fix
                let fix_result = fixer.fix();
                let recheck = run_check(id);

                let final_status = if recheck.ok {
                    GateStatus::Ready { version: recheck.version, detail: recheck.detail }
                } else {
                    let err = fix_result.err().unwrap_or_else(|| "recheck failed".to_string());
                    GateStatus::Failed { error: err }
                };
                let fix_detail = fix_result.ok().map(|r| r.approach);
                callback(ProgressEvent::Gate { id: id.to_string(), status: final_status.clone() });

                gate_reports.insert(id, GateReport {
                    id,
                    status: final_status,
                    fix_attempted: true,
                    fix_detail,
                });
            }

            PlanStep::Batch(ids) => {
                // Emit Installing for all ids in the batch
                for &id in ids {
                    callback(ProgressEvent::Gate { id: id.to_string(), status: GateStatus::Installing });
                }

                // Use BrewBundleFixer from the first non-dep-blocked id's fixer
                // (all bundle components use the same BrewBundleFixer)
                let representative_id = ids[0];
                let fixer = get_fixer(representative_id);
                let fix_result = fixer.fix();

                match fix_result {
                    Err(e) => {
                        for &id in ids {
                            let status = GateStatus::Failed { error: e.clone() };
                            callback(ProgressEvent::Gate { id: id.to_string(), status: status.clone() });
                            gate_reports.insert(id, GateReport {
                                id, status, fix_attempted: true, fix_detail: None,
                            });
                        }
                    }
                    Ok(fix_res) => {
                        for &id in ids {
                            let recheck = run_check(id);
                            let final_status = if recheck.ok {
                                GateStatus::Ready { version: recheck.version, detail: recheck.detail }
                            } else {
                                GateStatus::Failed { error: "recheck failed after brew bundle".to_string() }
                            };
                            callback(ProgressEvent::Gate { id: id.to_string(), status: final_status.clone() });
                            gate_reports.insert(id, GateReport {
                                id, status: final_status.clone(),
                                fix_attempted: true,
                                fix_detail: Some(fix_res.approach.clone()),
                            });

                            // post_fix_trigger: if this id ended Ready, add triggered ids
                            if matches!(final_status, GateStatus::Ready { .. }) {
                                let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
                                for &tid in spec.post_fix_trigger {
                                    if !post_trigger_ids.contains(&tid) {
                                        post_trigger_ids.push(tid);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Execute post_fix_trigger steps (e.g. database after senseid upgrade)
    for &tid in &post_trigger_ids {
        if blocked_on.is_some() { break; }
        let spec = COMPONENTS.iter().find(|s| s.id == tid).unwrap();
        let fixer = get_fixer(tid);

        let in_progress = match spec.gate_kind {
            GateKind::Install => GateStatus::Installing,
            GateKind::Service => GateStatus::Starting,
        };
        callback(ProgressEvent::Gate { id: tid.to_string(), status: in_progress });

        let fix_result = fixer.fix();
        let recheck = run_check(tid);

        let final_status = if recheck.ok {
            GateStatus::Ready { version: recheck.version, detail: recheck.detail }
        } else {
            GateStatus::Failed { error: fix_result.err().unwrap_or_else(|| "recheck failed".to_string()) }
        };
        let fix_detail = fix_result.ok().map(|r| r.approach);
        callback(ProgressEvent::Gate { id: tid.to_string(), status: final_status.clone() });
        gate_reports.insert(tid, GateReport {
            id: tid, status: final_status, fix_attempted: true, fix_detail,
        });
    }

    // ── Phase D: Return ───────────────────────────────────────────────────
    let gates: Vec<GateReport> = COMPONENTS.iter()
        .filter_map(|spec| gate_reports.remove(spec.id))
        .collect();
    let all_ok = blocked_on.is_none() && gates.iter().all(|g| matches!(g.status, GateStatus::Ready { .. }));

    BootstrapReport { gates, all_ok, blocked_on }
```

- [ ] **Step 7.4: Run Phase C tests**

```bash
cargo test -p sensei-bootstrap engine::tests::failing_postgresql 2>&1 | tail -10
cargo test -p sensei-bootstrap engine::tests::homebrew_missing 2>&1 | tail -10
```
Expected: both PASS.

- [ ] **Step 7.5: Commit**

```bash
git add crates/bootstrap/src/prereq/engine.rs
git commit -m "feat(bootstrap): engine Phase C (sequential fix) and Phase D (report)"
```

---

## Task 8: Full integration test suite (all 13 scenarios)

**Files:**
- Modify: `crates/bootstrap/src/prereq/engine.rs` (add to `#[cfg(test)]`)

- [ ] **Step 8.1: Add all 13 integration tests**

Add the following tests to the `#[cfg(test)]` block. Each test uses only mock checkers and fixers — no real binaries.

```rust
// ── Shared helpers ────────────────────────────────────────────────────────

fn all_ok_ctx() -> BootstrapContext {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        ctx = ctx.with_checker(spec.id, make_mock_checker(true));
    }
    ctx
}

struct HumanFixer { id: &'static str }
impl Fixer for HumanFixer {
    fn fix(&self) -> Result<super::FixResult, String> { Err("human action".into()) }
    fn human_action(&self) -> Option<HumanAction> {
        Some(HumanAction {
            component_id: self.id,
            title: format!("Fix {}", self.id),
            command: "manual".to_string(),
            url: None,
        })
    }
}

// ── Test 1: All ready → all_ok=true, 0 fix calls ──────────────────────────
// (already covered by all_ready_returns_all_ok_true above)

// ── Test 2: Homebrew missing → blocked_on.component_id = homebrew ─────────
// (already covered by homebrew_missing_returns_blocked_on above)

// ── Test 3: Homebrew blocks bundle components ──────────────────────────────
#[test]
fn homebrew_missing_blocks_postgresql_fix() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(false));
    ctx = ctx.with_fixer("homebrew", Arc::new(HumanFixer { id: "homebrew" }));
    ctx = ctx.with_checker("postgresql", make_mock_checker(false));
    ctx = ctx.with_fixer("postgresql", make_mock_fixer(true)); // never called
    for spec in super::registry::COMPONENTS {
        if spec.id != "homebrew" && spec.id != "postgresql" {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    assert!(report.blocked_on.is_some(), "should be blocked on homebrew");
    assert_eq!(report.blocked_on.unwrap().component_id, "homebrew");
    // postgresql fix should NOT have been attempted
    if let Some(pg) = report.gates.iter().find(|g| g.id == "postgresql") {
        assert!(!pg.fix_attempted, "postgresql fix should not have been attempted");
    }
}

// ── Test 4: postgresql + ollama missing → BrewBundleFixer called once ─────
#[test]
fn postgresql_and_ollama_fixed_via_bundle() {
    use std::sync::{Arc, Mutex};
    use crate::{platform, config::SenseiConfig};

    let fix_call_count = Arc::new(Mutex::new(0u32));
    let fix_call_count_clone = Arc::clone(&fix_call_count);

    struct CountingBundleFixer(Arc<Mutex<u32>>);
    impl Fixer for CountingBundleFixer {
        fn fix(&self) -> Result<super::FixResult, String> {
            *self.0.lock().unwrap() += 1;
            Ok(super::FixResult::new("bundle ran"))
        }
    }

    let bundle_fixer: Arc<dyn Fixer> = Arc::new(CountingBundleFixer(fix_call_count_clone));

    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    ctx = ctx.with_checker("postgresql", SeqChecker::new(&[false, true]));
    ctx = ctx.with_checker("ollama", SeqChecker::new(&[false, true]));
    ctx = ctx.with_fixer("postgresql", Arc::clone(&bundle_fixer));
    ctx = ctx.with_fixer("ollama", Arc::clone(&bundle_fixer));
    for spec in super::registry::COMPONENTS {
        if !["homebrew","postgresql","ollama"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});

    // BrewBundleFixer called once for the batch
    assert_eq!(*fix_call_count.lock().unwrap(), 1, "BrewBundleFixer should be called exactly once");
    let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
    let ol = report.gates.iter().find(|g| g.id == "ollama").unwrap();
    assert!(matches!(pg.status, GateStatus::Ready { .. }));
    assert!(matches!(ol.status, GateStatus::Ready { .. }));
}

// ── Test 5: senseid outdated → bundle fixes it ────────────────────────────
#[test]
fn senseid_outdated_fixed_via_bundle() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    ctx = ctx.with_checker("senseid", SeqChecker::new(&[false, true]));
    ctx = ctx.with_fixer("senseid", make_mock_fixer(true));
    // database always passes so post_fix_trigger doesn't trigger a failing db
    for spec in super::registry::COMPONENTS {
        if !["homebrew","senseid"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let sd = report.gates.iter().find(|g| g.id == "senseid").unwrap();
    assert!(matches!(sd.status, GateStatus::Ready { .. }), "senseid should be Ready after bundle fix");
    assert!(sd.fix_attempted);
}

// ── Test 6: DB schema not deployed → DatabaseSetupFixer called ───────────
#[test]
fn db_schema_not_deployed_gets_fixed() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        if spec.id == "database" {
            ctx = ctx.with_checker("database", SeqChecker::new(&[false, true]));
            ctx = ctx.with_fixer("database", make_mock_fixer(true));
        } else {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let db = report.gates.iter().find(|g| g.id == "database").unwrap();
    assert!(matches!(db.status, GateStatus::Ready { .. }), "database should be Ready after deploy");
    assert!(db.fix_attempted);
    assert!(db.fix_detail.is_some(), "fix_detail should be populated");
}

// ── Test 7: Fix succeeds but recheck fails → gate=Failed ─────────────────
#[test]
fn fix_succeeds_but_recheck_fails_gate_is_failed() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    // postgresql: fail on check, STILL fail on recheck (fixer runs but doesn't actually fix)
    ctx = ctx.with_checker("postgresql", make_mock_checker(false));
    ctx = ctx.with_fixer("postgresql", make_mock_fixer(true)); // fixer says ok but recheck still fails
    for spec in super::registry::COMPONENTS {
        if !["homebrew","postgresql"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
    assert!(matches!(pg.status, GateStatus::Failed { .. }), "gate should be Failed when recheck fails");
    assert!(pg.fix_attempted, "fix_attempted should be true");
}

// ── Test 8: Resume after human action ─────────────────────────────────────
#[test]
fn resume_after_human_action() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};

    // First call: homebrew fails with HumanActionFixer
    let mut ctx1 = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx1 = ctx1.with_checker("homebrew", make_mock_checker(false));
    ctx1 = ctx1.with_fixer("homebrew", Arc::new(HumanFixer { id: "homebrew" }));
    for spec in super::registry::COMPONENTS {
        if spec.id != "homebrew" {
            ctx1 = ctx1.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let report1 = BootstrapEngine::new(Arc::new(ctx1)).check_and_fix(|_| {});
    assert!(report1.blocked_on.is_some());

    // Second call (after human action): homebrew now passes
    let mut ctx2 = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        ctx2 = ctx2.with_checker(spec.id, make_mock_checker(true));
    }
    let report2 = BootstrapEngine::new(Arc::new(ctx2)).check_and_fix(|_| {});
    assert!(report2.all_ok, "second call should be all_ok after homebrew fixed");
}

// ── Test 9: senseid upgrade triggers db deploy ────────────────────────────
#[test]
fn senseid_upgrade_triggers_database_deploy() {
    use std::sync::{Arc, Mutex};
    use crate::{platform, config::SenseiConfig};

    let db_fixer_called = Arc::new(Mutex::new(false));
    let db_fixer_called_clone = Arc::clone(&db_fixer_called);
    struct DbFixer(Arc<Mutex<bool>>);
    impl Fixer for DbFixer {
        fn fix(&self) -> Result<super::FixResult, String> {
            *self.0.lock().unwrap() = true;
            Ok(super::FixResult::new("schema deployed"))
        }
    }

    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    ctx = ctx.with_checker("senseid", SeqChecker::new(&[false, true]));
    ctx = ctx.with_fixer("senseid", make_mock_fixer(true));
    // database INITIALLY passes — post_fix_trigger forces it anyway
    ctx = ctx.with_checker("database", SeqChecker::new(&[true, true]));
    ctx = ctx.with_fixer("database", Arc::new(DbFixer(db_fixer_called_clone)));
    for spec in super::registry::COMPONENTS {
        if !["homebrew","senseid","database"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    assert!(*db_fixer_called.lock().unwrap(), "database fixer should be called via post_fix_trigger");
    let db = report.gates.iter().find(|g| g.id == "database").unwrap();
    assert!(matches!(db.status, GateStatus::Ready { .. }));
    assert!(db.fix_detail.as_deref() == Some("schema deployed"));
}

// ── Test 10: senseid upgrade, db deploy fails ─────────────────────────────
#[test]
fn senseid_upgrade_db_deploy_fails() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    ctx = ctx.with_checker("senseid", SeqChecker::new(&[false, true]));
    ctx = ctx.with_fixer("senseid", make_mock_fixer(true));
    // db deploy fails; recheck also fails
    ctx = ctx.with_checker("database", make_mock_checker(false));
    ctx = ctx.with_fixer("database", make_mock_fixer(false));
    for spec in super::registry::COMPONENTS {
        if !["homebrew","senseid","database"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let db = report.gates.iter().find(|g| g.id == "database").unwrap();
    assert!(matches!(db.status, GateStatus::Failed { .. }), "db gate should be Failed");
    assert!(db.fix_detail.is_none(), "no fix_detail when fixer fails");
}

// ── Test 11: senseid upgrade, db already current → Ready, fix_detail populated
#[test]
fn senseid_upgrade_db_already_current() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    ctx = ctx.with_checker("homebrew", make_mock_checker(true));
    ctx = ctx.with_checker("senseid", SeqChecker::new(&[false, true]));
    ctx = ctx.with_fixer("senseid", make_mock_fixer(true));
    // db: initially passes, fixer is a no-op that succeeds
    ctx = ctx.with_checker("database", SeqChecker::new(&[true, true]));
    ctx = ctx.with_fixer("database", make_mock_fixer(true)); // represents dbd::deploy no-op
    for spec in super::registry::COMPONENTS {
        if !["homebrew","senseid","database"].contains(&spec.id) {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let db = report.gates.iter().find(|g| g.id == "database").unwrap();
    assert!(matches!(db.status, GateStatus::Ready { .. }));
    assert!(db.fix_detail.is_some());
}

// ── Test 12: db deploy success populates fix_detail ───────────────────────
#[test]
fn db_deploy_success_populates_fix_detail() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    struct DetailFixer;
    impl Fixer for DetailFixer {
        fn fix(&self) -> Result<super::FixResult, String> {
            Ok(super::FixResult::new("3 migrations applied"))
        }
    }
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    for spec in super::registry::COMPONENTS {
        if spec.id == "database" {
            ctx = ctx.with_checker("database", SeqChecker::new(&[false, true]));
            ctx = ctx.with_fixer("database", Arc::new(DetailFixer));
        } else {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let db = report.gates.iter().find(|g| g.id == "database").unwrap();
    assert_eq!(db.fix_detail.as_deref(), Some("3 migrations applied"));
}

// ── Test 13: Daemon blocked by database failure ───────────────────────────
#[test]
fn daemon_blocked_when_database_fails() {
    use std::sync::Arc;
    use crate::{platform, config::SenseiConfig};
    let mut ctx = BootstrapContext::new(
        Arc::from(platform::detect()), SenseiConfig::from_env(), "0.1.0".to_string(),
    );
    // postgresql_service fails → database is dep-blocked → daemon is dep-blocked
    for spec in super::registry::COMPONENTS {
        if spec.id == "postgresql_service" {
            ctx = ctx.with_checker(spec.id, make_mock_checker(false));
            ctx = ctx.with_fixer(spec.id, make_mock_fixer(false));
        } else {
            ctx = ctx.with_checker(spec.id, make_mock_checker(true));
        }
    }
    let engine = BootstrapEngine::new(Arc::new(ctx));
    let report = engine.check_and_fix(|_| {});
    let daemon = report.gates.iter().find(|g| g.id == "daemon").unwrap();
    assert!(matches!(daemon.status, GateStatus::Failed { .. }), "daemon should be Failed (dep-blocked)");
    assert!(!daemon.fix_attempted, "daemon fix should not be attempted when dep-blocked");
}
```

- [ ] **Step 8.2: Run all integration tests**

```bash
cargo test -p sensei-bootstrap engine 2>&1 | tail -40
```
Expected: all 13 (plus the 2 from earlier) tests PASS. Fix any failures before proceeding.

- [ ] **Step 8.3: Commit**

```bash
git add crates/bootstrap/src/prereq/engine.rs
git commit -m "test(bootstrap): full integration test suite — all 13 engine scenarios"
```

---

## Task 9: lib.rs check_and_fix() + module exports

**Files:**
- Modify: `crates/bootstrap/src/lib.rs`
- Modify: `crates/bootstrap/src/prereq/mod.rs`

- [ ] **Step 9.1: Write failing test**

Add to `lib.rs` `#[cfg(test)]` block:

```rust
#[test]
fn check_and_fix_all_ready_returns_all_ok() {
    use std::sync::Arc;
    use crate::prereq::engine::BootstrapContext;
    use crate::prereq::registry::COMPONENTS;
    use crate::prereq::checker::Checker;
    use crate::prereq::CheckResult;

    struct OkChecker;
    impl Checker for OkChecker { fn check(&self) -> CheckResult { CheckResult::ok("mock") } }

    let mut ctx = BootstrapContext::new(
        Arc::from(provider()),
        SenseiConfig::from_env(),
        "0.1.0".to_string(),
    );
    for spec in COMPONENTS {
        ctx = ctx.with_checker(spec.id, Arc::new(OkChecker));
    }
    let report = check_and_fix_with_context(ctx, |_| {});
    assert!(report.all_ok);
}
```

- [ ] **Step 9.2: Run to verify test fails**

```bash
cargo test -p sensei-bootstrap lib::tests::check_and_fix 2>&1 | tail -10
```
Expected: FAIL (`check_and_fix_with_context` not found).

- [ ] **Step 9.3: Add check_and_fix_with_context to lib.rs**

Add to `lib.rs`:

```rust
pub use prereq::{BootstrapReport, GateReport, GateStatus, HumanAction, ProgressEvent};
pub use prereq::engine::{BootstrapContext, BootstrapEngine};

/// Run check_and_fix with a pre-built BootstrapContext (supports test injection).
pub fn check_and_fix_with_context<F>(ctx: BootstrapContext, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let engine = BootstrapEngine::new(std::sync::Arc::new(ctx));
    engine.check_and_fix(callback)
}

/// Convenience: run check_and_fix from environment config (no test injection).
pub fn check_and_fix<F>(app_version: &str, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let ctx = BootstrapContext::new(
        std::sync::Arc::from(provider()),
        SenseiConfig::from_env(),
        app_version.to_string(),
    );
    check_and_fix_with_context(ctx, callback)
}
```

- [ ] **Step 9.4: Run test to verify pass**

```bash
cargo test -p sensei-bootstrap 2>&1 | tail -20
```
Expected: ALL tests PASS.

- [ ] **Step 9.5: Commit**

```bash
git add crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): expose check_and_fix / check_and_fix_with_context in lib.rs"
```

---

## Task 10: Cleanup — remove factory.rs, remove BREWFILE from macos.rs

**Files:**
- Modify: `crates/bootstrap/src/prereq/mod.rs` (remove `factory` module)
- Delete: `crates/bootstrap/src/prereq/factory.rs`
- Modify: `crates/bootstrap/src/platform/macos.rs` (remove `BREWFILE` const)

- [ ] **Step 10.1: Remove factory from mod.rs**

In `prereq/mod.rs`, remove the line:
```rust
pub mod factory;
```

- [ ] **Step 10.2: Delete factory.rs**

```bash
rm crates/bootstrap/src/prereq/factory.rs
```

- [ ] **Step 10.3: Remove BREWFILE const from macos.rs**

In `platform/macos.rs`, remove the `const BREWFILE` block and update `install_prerequisites` to use the URL-fetching approach (or mark it for future removal — the engine no longer calls `PlatformProvider::install_prerequisites`).

Replace the `BREWFILE` const and the `install_prerequisites` implementation with a stub that documents it's superseded:

```rust
// Note: install_prerequisites via PlatformProvider is superseded by BrewBundleFixer
// in the engine. This implementation is kept for API compatibility only.
fn install_prerequisites(&self) -> Result<(), String> {
    let brew = self.brew_path.as_deref()
        .ok_or_else(|| "Homebrew is not installed".to_string())?;

    let brewfile = reqwest::blocking::get(crate::config::HOMEBREW_BREWFILE_URL)
        .map_err(|e| format!("failed to fetch Brewfile: {e}"))?
        .text()
        .map_err(|e| format!("failed to read Brewfile: {e}"))?;

    let mut child = std::process::Command::new(brew)
        .args(["bundle", "--upgrade", "--file=-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn brew bundle: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(brewfile.as_bytes())
            .map_err(|e| format!("failed to write Brewfile to stdin: {e}"))?;
    }

    let output = child.wait_with_output()
        .map_err(|e| format!("failed to wait for brew bundle: {e}"))?;

    if output.status.success() { Ok(()) }
    else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("brew bundle failed: {stderr}"))
    }
}
```

Update the test `prereq_remedy_contains_brewfile_url` to reference the constant instead of a hardcoded string:

```rust
#[test]
fn prereq_remedy_contains_brewfile_url() {
    let provider = MacOSProvider::new();
    let remedy = provider.prereq_install_remedy();
    assert!(remedy.command.contains("homebrew-tap"));
    assert!(remedy.url.as_deref().unwrap().contains("homebrew-tap"));
}
```

- [ ] **Step 10.4: Run full tests**

```bash
cargo test -p sensei-bootstrap 2>&1 | tail -20
```
Expected: ALL tests PASS, no references to factory.

- [ ] **Step 10.5: Commit**

```bash
git add -A crates/bootstrap/
git commit -m "refactor(bootstrap): remove factory.rs and embedded BREWFILE const"
```

---

## Task 11: Tauri command update

**Files:**
- Modify: `app/src-tauri/src/commands/bootstrap.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 11.1: Read current lib.rs registrations**

Read `app/src-tauri/src/lib.rs` to find the `invoke_handler` line.

- [ ] **Step 11.2: Replace bootstrap.rs command implementations**

Replace the three phase commands and `run_bootstrap` in `bootstrap.rs`. Keep `detect_hardware`, `list_models`, `missing_models`, `get_platform`, `get_daemon_port` unchanged.

Remove imports and functions:
- `use sensei_bootstrap::{..., prereq::{GateStatus, ProgressEvent, factory, runner}}`
- `fn install_prerequisites`
- `fn start_services`
- `fn setup_database`
- `fn run_bootstrap` (subsumed by check_and_fix_bootstrap)
- `fn write_bootstrap_session` (move log writing into check_and_fix_bootstrap)
- `fn collect_system_info` (keep — used by new log writing)

Update imports to:
```rust
use sensei_bootstrap::{
    self as bootstrap,
    BootstrapReport, BootstrapTrace, HardwareInfo,
    prereq::{GateStatus, HumanAction, ProgressEvent, BootstrapReport as PrereqReport},
    BootstrapContext, BootstrapEngine,
    POSTGRES_PORT, OLLAMA_PORT,
};
```

Add `check_and_fix_bootstrap`:
```rust
/// Check all bootstrap components and fix what's broken.
/// Streams progress via "bootstrap" Tauri events, then emits "bootstrap:complete".
/// This is the single entry point — replaces run_bootstrap + install_prerequisites
/// + start_services + setup_database. Calling it again is the resume mechanism.
#[tauri::command]
pub fn check_and_fix_bootstrap(app: tauri::AppHandle) -> Result<(), String> {
    let version = app.package_info().version.to_string();
    flog::log(&format!("=== check_and_fix_bootstrap called v={version} ==="));
    std::thread::spawn(move || {
        let ctx = bootstrap::BootstrapContext::new(
            std::sync::Arc::from(bootstrap::provider()),
            sensei_bootstrap::SenseiConfig::from_env(),
            version.clone(),
        );
        let app_cb = app.clone();
        let report = bootstrap::check_and_fix_with_context(ctx, move |e| dispatch(&app_cb, e));
        flog::log(&format!(
            "check_and_fix_bootstrap done: all_ok={} blocked_on={}",
            report.all_ok,
            report.blocked_on.as_ref().map(|a| a.component_id).unwrap_or("none"),
        ));
        // Write session log (best-effort)
        write_bootstrap_session_from_report(&app, &report, &version);
        // Emit completion event
        let _ = app.emit("bootstrap:complete", &report);
    });
    Ok(())
}

fn write_bootstrap_session_from_report(
    app: &tauri::AppHandle,
    report: &sensei_bootstrap::BootstrapReport,
    _app_version: &str,
) {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let session_id = format!(
        "sess-bs-{}-{:04x}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ"),
        COUNTER.fetch_add(1, Ordering::Relaxed) & 0xFFFF,
    );

    let hw = bootstrap::hardware::detect();
    let system_info = collect_system_info(&hw);
    let outcome = if report.all_ok { "success" } else { "failed" };

    let session = crate::log_collector::LogSession {
        id:          session_id,
        module:      "bootstrap".to_string(),
        started_at:  chrono::Utc::now().to_rfc3339(),
        app_version: app.package_info().version.to_string(),
        system_info,
        outcome:     outcome.to_string(),
        duration_ms: 0,
        traces:      vec![],
    };
    let collector = app.state::<crate::log_collector::LogCollector>();
    collector.write_session(&session);
}
```

- [ ] **Step 11.3: Update invoke_handler in lib.rs**

Remove `run_bootstrap`, `install_prerequisites`, `start_services`, `setup_database` from the handler list. Add `check_and_fix_bootstrap`.

- [ ] **Step 11.4: Build the Tauri crate to verify it compiles**

```bash
cd app && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -30
```
Expected: compiles without errors.

- [ ] **Step 11.5: Commit**

```bash
git add app/src-tauri/src/commands/bootstrap.rs app/src-tauri/src/lib.rs
git commit -m "feat(app): replace 3-phase Tauri commands with single check_and_fix_bootstrap; drop run_bootstrap"
```

---

## Task 12: Full test run and verification

- [ ] **Step 12.1: Run all bootstrap unit tests**

```bash
cargo test -p sensei-bootstrap 2>&1
```
Expected: ALL tests PASS, 0 failures.

- [ ] **Step 12.2: Run full workspace tests**

```bash
make test-fast 2>&1 | tail -30
```
Expected: PASS.

- [ ] **Step 12.3: Verify zero-errors policy**

```bash
cargo clippy -p sensei-bootstrap -- -D warnings 2>&1 | tail -20
cargo clippy --manifest-path app/src-tauri/Cargo.toml -- -D warnings 2>&1 | tail -20
```
Fix any warnings before proceeding.

- [ ] **Step 12.4: Final commit and merge to main when stable**

```bash
git add -A
git commit -m "chore: zero-errors-policy pass — bootstrap engine complete"
```

Once all tests pass and clippy is clean, merge develop to main.

---

## Quick-reference: component dependency graph

```
homebrew ──────────────────────────────────────────────────┐
  ├── postgresql ──> postgresql_service ──> database ──> daemon
  ├── ollama ──────> ollama_service
  ├── sensei
  ├── senseid ─────────────────────────────> database (post_fix_trigger)
  └── sensei_mcp
```

**Bundle group:** postgresql, ollama, sensei (prod), senseid (prod), sensei_mcp (prod)  
**HumanActionFixer:** homebrew (always), sensei/senseid/sensei_mcp (dev mode only)
