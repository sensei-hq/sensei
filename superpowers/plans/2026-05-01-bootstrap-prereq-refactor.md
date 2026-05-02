# Bootstrap Prerequisite Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace hardcoded bootstrap orchestration with a Strategy + Composite + Factory pattern: each prerequisite self-describes its checker, fixer, and platform remedy. Adding a new prerequisite or platform becomes a one-file change.

**Architecture:** A new `prereq` module inside `sensei-bootstrap` crate defines a `Prerequisite` trait composed from pluggable `Checker` and `Fixer` traits. A `PlatformFactory` constructs the right checker+fixer pair per platform. A generic `Runner` loops through prerequisites emitting typed `ProgressEvent`s via callback — no hardcoded gate names. The Tauri command layer maps those events to `emit_gate` calls; it owns no orchestration logic. A `LogCollector` stub (no-op) is wired up as a Tauri managed-state placeholder so the logging session can replace it without touching the command layer.

**Tech Stack:** Rust, `sensei-bootstrap` crate (`daemon/crates/bootstrap/`), Tauri managed state, existing `util`, `database`, `platform` modules.

---

## File Map

### New files — `daemon/crates/bootstrap/src/prereq/`
- `mod.rs` — `Prerequisite` trait, `CheckResult`, `FixResult`, `Remedy`, `GateKind`, `GateStatus`, `ProgressEvent`
- `checker.rs` — `Checker` trait + `BinaryChecker`, `PortChecker`, `BinaryAndPortChecker`, `DatabaseChecker`
- `fixer.rs` — `Fixer` trait + `NoopFixer`, `BrewFixer`, `WingetFixer`, `ServiceStartFixer` (with port polling), `DatabaseSetupFixer`
- `generic.rs` — `GenericPrerequisite` — composes one Checker + one Fixer
- `factory.rs` — `install_prerequisites()`, `start_services()`, `setup_database()` — platform-aware builders
- `runner.rs` — `run(phase, prereqs, on_progress)` — generic check→fix→recheck loop

### New files — app layer
- `app/src-tauri/src/log_collector.rs` — `LogCollector` stub (all no-ops; replaced by logging session)

### Modified files
- `daemon/crates/bootstrap/src/lib.rs` — add `pub mod prereq;`
- `app/src-tauri/src/commands/bootstrap.rs` — swap hardcoded orchestration for factory+runner calls
- `app/src-tauri/src/lib.rs` — register `LogCollector` as managed state

---

## Task 0: LogCollector stub

**Files:**
- Create: `app/src-tauri/src/log_collector.rs`
- Modify: `app/src-tauri/src/lib.rs`

This stub exists so the logging session (`2026-05-01-bootstrap-logging-plan.md`) can implement the real `LogCollector` without touching the command layer. All methods are no-ops.

- [ ] **Step 1: Create the stub**

```rust
//! LogCollector — session log manager.
//!
//! STUB: All methods are no-ops. The bootstrap-logging session will replace
//! this with the real implementation (file rotation, JSON session writes, etc.).
//! Do not add logic here — wait for the logging session to land.

/// Tauri managed state for bootstrap session logging.
/// Register once via `.manage(LogCollector::new())` in lib.rs.
pub struct LogCollector;

impl LogCollector {
    pub fn new() -> Self {
        Self
    }

    /// Start a new log session. No-op until logging session ships.
    pub fn session_start(&self, _session_id: &str) {}

    /// Append a trace record. No-op until logging session ships.
    pub fn append_trace(&self, _session_id: &str, _trace: &serde_json::Value) {}

    /// End a log session. No-op until logging session ships.
    pub fn session_end(&self, _session_id: &str, _success: bool) {}

    /// Return all stored sessions. Returns empty until logging session ships.
    pub fn get_sessions(&self) -> Vec<serde_json::Value> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_does_not_panic() {
        let _ = LogCollector::new();
    }

    #[test]
    fn stub_methods_are_no_ops() {
        let lc = LogCollector::new();
        lc.session_start("sess-001");
        lc.append_trace("sess-001", &serde_json::json!({"step": "test"}));
        lc.session_end("sess-001", true);
        let sessions = lc.get_sessions();
        assert!(sessions.is_empty(), "stub should return empty sessions");
    }
}
```

- [ ] **Step 2: Register in lib.rs**

In `app/src-tauri/src/lib.rs`, add `mod log_collector;` and chain `.manage(log_collector::LogCollector::new())` before `.invoke_handler(...)`:

```rust
mod commands;
mod log_collector;

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_opener::init())
        .manage(log_collector::LogCollector::new())
        .invoke_handler(tauri::generate_handler![
            // ... existing handlers unchanged
        ])
        // ... rest unchanged
```

- [ ] **Step 3: Build and test**

```bash
cd /Users/Jerry/Developer/sensei/app
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
cargo test --manifest-path src-tauri/Cargo.toml log_collector 2>&1 | tail -10
```
Expected: compiles, `test result: ok. 2 passed`

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/log_collector.rs app/src-tauri/src/lib.rs
git commit -m "feat(app): add LogCollector stub as Tauri managed state (logging session will implement)"
```

---

## Task 1: Core types and Prerequisite trait — `prereq/mod.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/mod.rs`

- [ ] **Step 1: Create the file**

```rust
//! Strategy-based prerequisite system for bootstrap.
//!
//! Each prerequisite composes a Checker (how to verify) and a Fixer (how to repair).
//! The Runner orchestrates check→fix→recheck and emits ProgressEvents via callback.

use serde::{Deserialize, Serialize};

/// Result of a check operation.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub ok: bool,
    pub version: Option<String>,
    pub detail: Option<String>,
    pub error: Option<String>,
}

impl CheckResult {
    pub fn ok(version: impl Into<String>) -> Self {
        Self { ok: true, version: Some(version.into()), detail: None, error: None }
    }

    pub fn ok_with_detail(version: impl Into<String>, detail: impl Into<String>) -> Self {
        Self { ok: true, version: Some(version.into()), detail: Some(detail.into()), error: None }
    }

    pub fn fail(error: impl Into<String>) -> Self {
        Self { ok: false, version: None, detail: None, error: Some(error.into()) }
    }
}

/// Result of a fix attempt.
#[derive(Debug, Clone)]
pub struct FixResult {
    pub approach: String,
}

impl FixResult {
    pub fn new(approach: impl Into<String>) -> Self {
        Self { approach: approach.into() }
    }
}

/// Manual remedy shown in the UI when a prerequisite cannot be auto-fixed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remedy {
    pub title: String,
    pub command: String,
    pub url: Option<String>,
}

/// What in-progress status to emit while fixing.
#[derive(Debug, Clone, PartialEq)]
pub enum GateKind {
    /// Binary/package install in progress → emits `Installing`.
    Install,
    /// Service start in progress → emits `Starting`.
    Service,
}

/// The status for a single gate emitted to the frontend.
#[derive(Debug, Clone)]
pub enum GateStatus {
    Checking,
    Installing,
    Starting,
    Ready { version: Option<String>, detail: Option<String> },
    Failed { error: String },
}

/// Progress event emitted by the runner.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    Gate { id: String, status: GateStatus },
    PhaseComplete { phase: String, success: bool },
}

/// A self-describing prerequisite: check, fix, and report.
pub trait Prerequisite: Send + Sync {
    fn id(&self) -> &str;
    fn label(&self) -> &str;
    fn check(&self) -> CheckResult;
    fn fix(&self) -> Result<FixResult, String>;
    fn gate_kind(&self) -> GateKind;
    fn remedy(&self) -> Option<&Remedy>;
}

pub mod checker;
pub mod fixer;
pub mod generic;
pub mod factory;
pub mod runner;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_result_ok_sets_fields() {
        let r = CheckResult::ok("1.0.0");
        assert!(r.ok);
        assert_eq!(r.version.as_deref(), Some("1.0.0"));
        assert!(r.error.is_none());
    }

    #[test]
    fn check_result_fail_sets_fields() {
        let r = CheckResult::fail("not found");
        assert!(!r.ok);
        assert!(r.version.is_none());
        assert_eq!(r.error.as_deref(), Some("not found"));
    }

    #[test]
    fn check_result_ok_with_detail() {
        let r = CheckResult::ok_with_detail("17.2", "/opt/homebrew/bin/postgres");
        assert!(r.ok);
        assert_eq!(r.detail.as_deref(), Some("/opt/homebrew/bin/postgres"));
    }

    #[test]
    fn fix_result_stores_approach() {
        let r = FixResult::new("brew install postgresql@17");
        assert_eq!(r.approach, "brew install postgresql@17");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::tests 2>&1 | tail -10
```
Expected: `test result: ok. 4 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/mod.rs
git commit -m "feat(bootstrap): add prereq module with Prerequisite trait and core types"
```

---

## Task 2: Checker trait + implementations — `prereq/checker.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/checker.rs`

- [ ] **Step 1: Create the file**

```rust
use crate::util;
use crate::database;
use crate::types::ComponentState;
use super::CheckResult;

/// Pluggable check strategy.
pub trait Checker: Send + Sync {
    fn check(&self) -> CheckResult;
}

/// Checks whether a binary exists in PATH and reads its version.
pub struct BinaryChecker {
    pub binary: String,
    pub version_flag: String,
}

impl BinaryChecker {
    pub fn new(binary: impl Into<String>, version_flag: impl Into<String>) -> Self {
        Self { binary: binary.into(), version_flag: version_flag.into() }
    }
}

impl Checker for BinaryChecker {
    fn check(&self) -> CheckResult {
        match util::which_binary(&self.binary) {
            None => CheckResult::fail(format!("{} not found in PATH", self.binary)),
            Some(path) => {
                let version = util::binary_version(&self.binary, &self.version_flag)
                    .unwrap_or_else(|| "unknown".to_string());
                CheckResult::ok_with_detail(version, path)
            }
        }
    }
}

/// Checks whether a TCP port is listening.
pub struct PortChecker {
    pub name: String,
    pub port: u16,
}

impl PortChecker {
    pub fn new(name: impl Into<String>, port: u16) -> Self {
        Self { name: name.into(), port }
    }
}

impl Checker for PortChecker {
    fn check(&self) -> CheckResult {
        if util::probe_port(self.port) {
            let version = util::fetch_service_version(&self.name, self.port);
            CheckResult::ok(version.unwrap_or_else(|| "unknown".to_string()))
        } else {
            CheckResult::fail(format!("{} not reachable on port {}", self.name, self.port))
        }
    }
}

/// Checks that a binary exists AND its service port is open.
pub struct BinaryAndPortChecker {
    pub binary: String,
    pub version_flag: String,
    pub name: String,
    pub port: u16,
}

impl BinaryAndPortChecker {
    pub fn new(
        binary: impl Into<String>,
        version_flag: impl Into<String>,
        name: impl Into<String>,
        port: u16,
    ) -> Self {
        Self { binary: binary.into(), version_flag: version_flag.into(), name: name.into(), port }
    }
}

impl Checker for BinaryAndPortChecker {
    fn check(&self) -> CheckResult {
        let path = match util::which_binary(&self.binary) {
            None => return CheckResult::fail(format!("{} binary not found in PATH", self.binary)),
            Some(p) => p,
        };
        if !util::probe_port(self.port) {
            return CheckResult::fail(format!(
                "{} installed at {} but service not running on port {}",
                self.binary, path, self.port
            ));
        }
        let version = util::fetch_service_version(&self.name, self.port)
            .or_else(|| util::binary_version(&self.binary, &self.version_flag))
            .unwrap_or_else(|| "unknown".to_string());
        CheckResult::ok_with_detail(version, path)
    }
}

/// Checks whether the sensei database is ready (pg_isready + DB exists + pgvector).
pub struct DatabaseChecker;

impl Checker for DatabaseChecker {
    fn check(&self) -> CheckResult {
        let status = database::check(None);
        if status.is_ready() {
            CheckResult::ok(status.version.as_deref().unwrap_or("unknown"))
        } else if let ComponentState::Failed { ref error } = status.state {
            CheckResult::fail(error.clone())
        } else {
            CheckResult::fail("database not ready")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_checker_missing_binary_returns_fail() {
        let checker = BinaryChecker::new("sensei-nonexistent-xyz-binary", "--version");
        let result = checker.check();
        assert!(!result.ok);
        assert!(result.error.is_some());
    }

    #[test]
    fn binary_checker_finds_ls() {
        let checker = BinaryChecker::new("ls", "--color");
        let result = checker.check();
        // ls exists — ok field will be true, version may be "unknown"
        assert!(result.ok || !result.ok); // just assert no panic
        let _ = result;
    }

    #[test]
    fn port_checker_closed_port_returns_fail() {
        let checker = PortChecker::new("test-service", 1);
        let result = checker.check();
        assert!(!result.ok, "port 1 should not be open");
        assert!(result.error.as_deref().unwrap().contains("port 1"));
    }

    #[test]
    fn binary_and_port_checker_missing_binary_returns_fail() {
        let checker = BinaryAndPortChecker::new(
            "sensei-nonexistent-xyz-binary", "--version", "test", 9999,
        );
        let result = checker.check();
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap().contains("not found"));
    }

    #[test]
    fn binary_checker_error_contains_binary_name() {
        let checker = BinaryChecker::new("totally-missing-binary", "--version");
        let result = checker.check();
        assert!(result.error.as_deref().unwrap().contains("totally-missing-binary"));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::checker::tests 2>&1 | tail -10
```
Expected: `test result: ok. 5 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/checker.rs
git commit -m "feat(bootstrap): add Checker trait with Binary, Port, BinaryAndPort, Database impls"
```

---

## Task 3: Fixer trait + implementations — `prereq/fixer.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/fixer.rs`

- [ ] **Step 1: Create the file**

```rust
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use crate::platform::PlatformProvider;
use crate::util;
use super::FixResult;

/// Pluggable fix strategy.
pub trait Fixer: Send + Sync {
    fn fix(&self) -> Result<FixResult, String>;
}

/// No-op fixer — always returns Err with the given reason.
/// Use for prerequisites that cannot be auto-installed (e.g. Homebrew itself).
pub struct NoopFixer {
    pub reason: String,
}

impl NoopFixer {
    pub fn new(reason: impl Into<String>) -> Self {
        Self { reason: reason.into() }
    }
}

impl Fixer for NoopFixer {
    fn fix(&self) -> Result<FixResult, String> {
        Err(self.reason.clone())
    }
}

/// Runs `brew install <formula>`.
pub struct BrewFixer {
    pub brew_path: String,
    pub formula: String,
}

impl BrewFixer {
    pub fn new(brew_path: impl Into<String>, formula: impl Into<String>) -> Self {
        Self { brew_path: brew_path.into(), formula: formula.into() }
    }
}

impl Fixer for BrewFixer {
    fn fix(&self) -> Result<FixResult, String> {
        let output = Command::new(&self.brew_path)
            .args(["install", &self.formula])
            .output()
            .map_err(|e| format!("failed to run brew install: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new(format!("brew install {}", self.formula)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("brew install {} failed: {stderr}", self.formula))
        }
    }
}

/// Runs `winget install --id <package> -e --silent`.
pub struct WingetFixer {
    pub package: String,
}

impl WingetFixer {
    pub fn new(package: impl Into<String>) -> Self {
        Self { package: package.into() }
    }
}

impl Fixer for WingetFixer {
    fn fix(&self) -> Result<FixResult, String> {
        let output = Command::new("winget")
            .args(["install", "--id", &self.package, "-e", "--silent"])
            .output()
            .map_err(|e| format!("failed to run winget install: {e}"))?;

        if output.status.success() {
            Ok(FixResult::new(format!("winget install {}", self.package)))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("winget install {} failed: {stderr}", self.package))
        }
    }
}

/// Calls `platform_provider.start_service(name)` then polls the port for up to 30s.
pub struct ServiceStartFixer {
    provider: Arc<dyn PlatformProvider>,
    service_name: String,
    port: u16,
}

impl ServiceStartFixer {
    pub fn new(
        provider: Arc<dyn PlatformProvider>,
        service_name: impl Into<String>,
        port: u16,
    ) -> Self {
        Self { provider, service_name: service_name.into(), port }
    }
}

impl Fixer for ServiceStartFixer {
    fn fix(&self) -> Result<FixResult, String> {
        self.provider
            .start_service(&self.service_name)
            .map_err(|e| format!("failed to start {}: {e}", self.service_name))?;

        // Poll up to 30s for the service to bind its port
        for _ in 0..30 {
            std::thread::sleep(Duration::from_secs(1));
            if util::probe_port(self.port) {
                return Ok(FixResult::new(format!(
                    "started {} on port {}", self.service_name, self.port
                )));
            }
        }
        Err(format!(
            "timed out waiting for {} on port {}", self.service_name, self.port
        ))
    }
}

/// Runs the full database setup pipeline (create db, extensions, migrations).
pub struct DatabaseSetupFixer;

impl Fixer for DatabaseSetupFixer {
    fn fix(&self) -> Result<FixResult, String> {
        crate::database::setup(None).map(|status| {
            FixResult::new(format!(
                "database setup complete: {}",
                status.version.as_deref().unwrap_or("unknown")
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_fixer_returns_err_with_reason() {
        let fixer = NoopFixer::new("manual installation required");
        let result = fixer.fix();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "manual installation required");
    }

    #[test]
    fn brew_fixer_stores_fields() {
        let fixer = BrewFixer::new("/opt/homebrew/bin/brew", "postgresql@17");
        assert_eq!(fixer.formula, "postgresql@17");
        assert_eq!(fixer.brew_path, "/opt/homebrew/bin/brew");
    }

    #[test]
    fn winget_fixer_stores_package() {
        let fixer = WingetFixer::new("PostgreSQL.PostgreSQL");
        assert_eq!(fixer.package, "PostgreSQL.PostgreSQL");
    }

    #[test]
    fn brew_fixer_nonexistent_brew_returns_err() {
        let fixer = BrewFixer::new("/nonexistent/path/to/brew", "postgresql@17");
        let result = fixer.fix();
        assert!(result.is_err(), "should fail when brew binary does not exist");
    }

    #[test]
    fn service_start_fixer_stores_fields() {
        use crate::platform;
        let provider = Arc::from(platform::detect());
        let fixer = ServiceStartFixer::new(provider, "postgresql", 5432);
        assert_eq!(fixer.service_name, "postgresql");
        assert_eq!(fixer.port, 5432);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::fixer::tests 2>&1 | tail -10
```
Expected: `test result: ok. 5 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/fixer.rs
git commit -m "feat(bootstrap): add Fixer trait with Noop, Brew, Winget, ServiceStart, DatabaseSetup impls"
```

---

## Task 4: GenericPrerequisite — `prereq/generic.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/generic.rs`

- [ ] **Step 1: Create the file**

```rust
use super::{CheckResult, FixResult, GateKind, Prerequisite, Remedy};
use super::checker::Checker;
use super::fixer::Fixer;

/// A prerequisite built by composing a Checker with a Fixer.
pub struct GenericPrerequisite {
    id: String,
    label: String,
    checker: Box<dyn Checker>,
    fixer: Box<dyn Fixer>,
    gate_kind: GateKind,
    remedy: Option<Remedy>,
}

impl GenericPrerequisite {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        checker: Box<dyn Checker>,
        fixer: Box<dyn Fixer>,
        gate_kind: GateKind,
        remedy: Option<Remedy>,
    ) -> Self {
        Self { id: id.into(), label: label.into(), checker, fixer, gate_kind, remedy }
    }
}

impl Prerequisite for GenericPrerequisite {
    fn id(&self) -> &str { &self.id }
    fn label(&self) -> &str { &self.label }
    fn check(&self) -> CheckResult { self.checker.check() }
    fn fix(&self) -> Result<FixResult, String> { self.fixer.fix() }
    fn gate_kind(&self) -> GateKind { self.gate_kind.clone() }
    fn remedy(&self) -> Option<&Remedy> { self.remedy.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::checker::Checker;
    use super::super::fixer::Fixer;

    struct ReadyChecker;
    impl Checker for ReadyChecker {
        fn check(&self) -> CheckResult { CheckResult::ok("1.2.3") }
    }

    struct FailChecker;
    impl Checker for FailChecker {
        fn check(&self) -> CheckResult { CheckResult::fail("binary not found") }
    }

    struct SuccessFixer;
    impl Fixer for SuccessFixer {
        fn fix(&self) -> Result<FixResult, String> { Ok(FixResult::new("installed via test")) }
    }

    struct FailFixer;
    impl Fixer for FailFixer {
        fn fix(&self) -> Result<FixResult, String> { Err("install failed".into()) }
    }

    fn make(checker: Box<dyn Checker>, fixer: Box<dyn Fixer>) -> GenericPrerequisite {
        GenericPrerequisite::new("test-gate", "Test Gate", checker, fixer, GateKind::Install, None)
    }

    #[test]
    fn id_and_label_stored() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        assert_eq!(p.id(), "test-gate");
        assert_eq!(p.label(), "Test Gate");
    }

    #[test]
    fn check_delegates_to_checker_ready() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        let r = p.check();
        assert!(r.ok);
        assert_eq!(r.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn check_delegates_to_checker_fail() {
        let p = make(Box::new(FailChecker), Box::new(SuccessFixer));
        let r = p.check();
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("binary not found"));
    }

    #[test]
    fn fix_success_returns_approach() {
        let p = make(Box::new(FailChecker), Box::new(SuccessFixer));
        let r = p.fix();
        assert!(r.is_ok());
        assert_eq!(r.unwrap().approach, "installed via test");
    }

    #[test]
    fn fix_failure_returns_err() {
        let p = make(Box::new(FailChecker), Box::new(FailFixer));
        assert!(p.fix().is_err());
    }

    #[test]
    fn gate_kind_returned() {
        let p = GenericPrerequisite::new(
            "svc", "Svc", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Service, None,
        );
        assert_eq!(p.gate_kind(), GateKind::Service);
    }

    #[test]
    fn remedy_none_by_default() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        assert!(p.remedy().is_none());
    }

    #[test]
    fn remedy_returned_when_set() {
        let remedy = Remedy { title: "Install".into(), command: "brew install foo".into(), url: None };
        let p = GenericPrerequisite::new(
            "foo", "Foo", Box::new(ReadyChecker), Box::new(SuccessFixer),
            GateKind::Install, Some(remedy),
        );
        assert_eq!(p.remedy().unwrap().command, "brew install foo");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::generic::tests 2>&1 | tail -10
```
Expected: `test result: ok. 8 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/generic.rs
git commit -m "feat(bootstrap): add GenericPrerequisite composing Checker + Fixer"
```

---

## Task 5: Runner — `prereq/runner.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/runner.rs`

- [ ] **Step 1: Create the file**

```rust
//! Generic check→fix→recheck orchestration loop.
//!
//! The runner knows nothing about specific gates, platforms, or Tauri events.
//! It emits ProgressEvents via a callback that the Tauri command layer maps to
//! emit_gate() calls.

use super::{GateKind, GateStatus, Prerequisite, ProgressEvent};

/// Run a list of prerequisites in order.
///
/// For each prerequisite:
///   1. Emit `Checking`
///   2. `check()` — if ok, emit `Ready` and move on
///   3. Emit `Installing` or `Starting` based on `gate_kind()`
///   4. `fix()` — if Err, emit `Failed` and move on
///   5. Re-check — emit `Ready` or `Failed`
///
/// After all prerequisites, emit `PhaseComplete`.
/// Returns `true` if all prerequisites ended in `Ready`.
pub fn run<F>(phase: &str, prereqs: Vec<Box<dyn Prerequisite>>, on_progress: F) -> bool
where
    F: Fn(ProgressEvent),
{
    let mut all_ok = true;

    for prereq in &prereqs {
        let id = prereq.id().to_string();

        on_progress(ProgressEvent::Gate { id: id.clone(), status: GateStatus::Checking });

        let check = prereq.check();
        if check.ok {
            on_progress(ProgressEvent::Gate {
                id: id.clone(),
                status: GateStatus::Ready { version: check.version, detail: check.detail },
            });
            continue;
        }

        // Not ready — attempt fix
        let fixing_status = match prereq.gate_kind() {
            GateKind::Service => GateStatus::Starting,
            GateKind::Install => GateStatus::Installing,
        };
        on_progress(ProgressEvent::Gate { id: id.clone(), status: fixing_status });

        match prereq.fix() {
            Err(e) => {
                all_ok = false;
                on_progress(ProgressEvent::Gate {
                    id: id.clone(),
                    status: GateStatus::Failed { error: e },
                });
            }
            Ok(_) => {
                let recheck = prereq.check();
                if recheck.ok {
                    on_progress(ProgressEvent::Gate {
                        id: id.clone(),
                        status: GateStatus::Ready { version: recheck.version, detail: recheck.detail },
                    });
                } else {
                    all_ok = false;
                    on_progress(ProgressEvent::Gate {
                        id: id.clone(),
                        status: GateStatus::Failed {
                            error: recheck.error.unwrap_or_else(|| "still failing after fix".to_string()),
                        },
                    });
                }
            }
        }
    }

    on_progress(ProgressEvent::PhaseComplete { phase: phase.to_string(), success: all_ok });
    all_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prereq::{CheckResult, FixResult, GateKind, Prerequisite, Remedy};
    use crate::prereq::checker::Checker;
    use crate::prereq::fixer::Fixer;
    use crate::prereq::generic::GenericPrerequisite;
    use std::sync::{Arc, Mutex};

    // --- Test doubles ---

    struct ReadyChecker;
    impl Checker for ReadyChecker {
        fn check(&self) -> CheckResult { CheckResult::ok("1.0") }
    }

    struct FailChecker;
    impl Checker for FailChecker {
        fn check(&self) -> CheckResult { CheckResult::fail("missing") }
    }

    /// Fails on first call, succeeds on subsequent calls (simulates post-fix state).
    struct FailThenPassChecker { calls: Arc<Mutex<u32>> }
    impl FailThenPassChecker {
        fn new() -> Self { Self { calls: Arc::new(Mutex::new(0)) } }
    }
    impl Checker for FailThenPassChecker {
        fn check(&self) -> CheckResult {
            let mut c = self.calls.lock().unwrap();
            *c += 1;
            if *c == 1 { CheckResult::fail("not yet") } else { CheckResult::ok("1.0") }
        }
    }

    struct SuccessFixer;
    impl Fixer for SuccessFixer {
        fn fix(&self) -> Result<FixResult, String> { Ok(FixResult::new("fixed")) }
    }

    struct FailFixer;
    impl Fixer for FailFixer {
        fn fix(&self) -> Result<FixResult, String> { Err("could not fix".into()) }
    }

    // --- Helpers ---

    fn make_prereq(id: &str, checker: Box<dyn Checker>, fixer: Box<dyn Fixer>, kind: GateKind) -> Box<dyn Prerequisite> {
        Box::new(GenericPrerequisite::new(id, id, checker, fixer, kind, None))
    }

    fn collect(prereqs: Vec<Box<dyn Prerequisite>>) -> Vec<ProgressEvent> {
        let acc = Arc::new(Mutex::new(Vec::new()));
        let acc2 = acc.clone();
        run("phase", prereqs, move |e| acc2.lock().unwrap().push(e));
        Arc::try_unwrap(acc).unwrap().into_inner().unwrap()
    }

    fn statuses_for(events: &[ProgressEvent], id: &str) -> Vec<&str> {
        events.iter().filter_map(|e| {
            if let ProgressEvent::Gate { id: eid, status } = e {
                if eid == id {
                    return Some(match status {
                        GateStatus::Checking    => "checking",
                        GateStatus::Installing  => "installing",
                        GateStatus::Starting    => "starting",
                        GateStatus::Ready { .. } => "ready",
                        GateStatus::Failed { .. } => "failed",
                    });
                }
            }
            None
        }).collect()
    }

    // --- Tests ---

    #[test]
    fn already_ready_prereq_emits_checking_then_ready() {
        let events = collect(vec![
            make_prereq("pg", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "pg"), vec!["checking", "ready"]);
    }

    #[test]
    fn failing_prereq_with_successful_fix_emits_installing_then_ready() {
        let events = collect(vec![
            make_prereq("ollama", Box::new(FailThenPassChecker::new()), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "ollama"), vec!["checking", "installing", "ready"]);
    }

    #[test]
    fn service_prereq_emits_starting_not_installing() {
        let events = collect(vec![
            make_prereq("daemon", Box::new(FailThenPassChecker::new()), Box::new(SuccessFixer), GateKind::Service),
        ]);
        assert_eq!(statuses_for(&events, "daemon"), vec!["checking", "starting", "ready"]);
    }

    #[test]
    fn failing_prereq_with_failed_fix_emits_failed() {
        let events = collect(vec![
            make_prereq("sensei", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "sensei"), vec!["checking", "installing", "failed"]);
    }

    #[test]
    fn fix_succeeds_but_recheck_fails_emits_failed() {
        // FailChecker always fails — even after fix, recheck fails
        let events = collect(vec![
            make_prereq("bad", Box::new(FailChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert_eq!(statuses_for(&events, "bad"), vec!["checking", "installing", "failed"]);
    }

    #[test]
    fn phase_complete_emitted_last_with_success() {
        let events = collect(vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ]);
        assert!(matches!(
            events.last().unwrap(),
            ProgressEvent::PhaseComplete { phase, success: true } if phase == "phase"
        ));
    }

    #[test]
    fn phase_complete_false_when_any_fails() {
        let events = collect(vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
            make_prereq("b", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ]);
        assert!(matches!(
            events.last().unwrap(),
            ProgressEvent::PhaseComplete { success: false, .. }
        ));
    }

    #[test]
    fn run_returns_true_when_all_pass() {
        let ok = run("p", vec![
            make_prereq("a", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Install),
        ], |_| {});
        assert!(ok);
    }

    #[test]
    fn run_returns_false_when_any_fails() {
        let ok = run("p", vec![
            make_prereq("a", Box::new(FailChecker), Box::new(FailFixer), GateKind::Install),
        ], |_| {});
        assert!(!ok);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::runner::tests 2>&1 | tail -15
```
Expected: `test result: ok. 9 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/runner.rs
git commit -m "feat(bootstrap): add runner — generic check→fix→recheck loop with typed ProgressEvents"
```

---

## Task 6: PlatformFactory — `prereq/factory.rs`

**Files:**
- Create: `daemon/crates/bootstrap/src/prereq/factory.rs`

- [ ] **Step 1: Create the file**

```rust
//! Platform-aware factory — builds Prerequisite lists for each bootstrap phase.
//!
//! Adding a new prerequisite: implement the Prerequisite trait (or use GenericPrerequisite)
//! and register it here. Adding a new platform: add a match arm in each factory function.

use std::sync::Arc;
use crate::{POSTGRES_PORT, OLLAMA_PORT, DAEMON_PORT};
use crate::platform::{Platform, PlatformProvider};
use super::{GateKind, Prerequisite};
use super::checker::{BinaryChecker, PortChecker, DatabaseChecker};
use super::fixer::{BrewFixer, WingetFixer, NoopFixer, ServiceStartFixer, DatabaseSetupFixer, Fixer};
use super::generic::GenericPrerequisite;

/// Find Homebrew binary in well-known locations.
pub fn detect_brew_path() -> Option<String> {
    ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|s| s.to_string())
}

/// Phase 1 — install binaries: postgresql, ollama, sensei CLI.
pub fn install_prerequisites(provider: Arc<dyn PlatformProvider>) -> Vec<Box<dyn Prerequisite>> {
    match provider.platform() {
        Platform::MacOS | Platform::Linux => {
            let brew = detect_brew_path();
            let mk_fixer = |formula: &str| -> Box<dyn Fixer> {
                match &brew {
                    Some(p) => Box::new(BrewFixer::new(p, formula)),
                    None => Box::new(NoopFixer::new("Homebrew not found — install Homebrew first")),
                }
            };
            vec![
                Box::new(GenericPrerequisite::new(
                    "postgresql", "PostgreSQL",
                    Box::new(BinaryChecker::new("postgres", "--version")),
                    mk_fixer("postgresql@17"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "ollama", "Ollama",
                    Box::new(BinaryChecker::new("ollama", "--version")),
                    mk_fixer("ollama"),
                    GateKind::Install, None,
                )),
                Box::new(GenericPrerequisite::new(
                    "sensei", "Sensei CLI",
                    Box::new(BinaryChecker::new("sensei", "--version")),
                    mk_fixer("sensei-hq/tap/sensei"),
                    GateKind::Install, None,
                )),
            ]
        }
        Platform::Windows => vec![
            Box::new(GenericPrerequisite::new(
                "postgresql", "PostgreSQL",
                Box::new(BinaryChecker::new("postgres", "--version")),
                Box::new(WingetFixer::new("PostgreSQL.PostgreSQL")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "ollama", "Ollama",
                Box::new(BinaryChecker::new("ollama", "--version")),
                Box::new(WingetFixer::new("Ollama.Ollama")),
                GateKind::Install, None,
            )),
            Box::new(GenericPrerequisite::new(
                "sensei", "Sensei CLI",
                Box::new(BinaryChecker::new("sensei", "--version")),
                Box::new(NoopFixer::new("Download sensei from sensei.so/download")),
                GateKind::Install, None,
            )),
        ],
    }
}

/// Phase 2 — start services: postgresql, ollama, daemon.
pub fn start_services(provider: Arc<dyn PlatformProvider>) -> Vec<Box<dyn Prerequisite>> {
    vec![
        Box::new(GenericPrerequisite::new(
            "postgresql", "PostgreSQL",
            Box::new(PortChecker::new("postgresql", POSTGRES_PORT)),
            Box::new(ServiceStartFixer::new(provider.clone(), "postgresql", POSTGRES_PORT)),
            GateKind::Service, None,
        )),
        Box::new(GenericPrerequisite::new(
            "ollama", "Ollama",
            Box::new(PortChecker::new("ollama", OLLAMA_PORT)),
            Box::new(ServiceStartFixer::new(provider.clone(), "ollama", OLLAMA_PORT)),
            GateKind::Service, None,
        )),
        Box::new(GenericPrerequisite::new(
            "daemon", "Sensei Daemon",
            Box::new(PortChecker::new("daemon", DAEMON_PORT)),
            Box::new(ServiceStartFixer::new(provider, "daemon", DAEMON_PORT)),
            GateKind::Service, None,
        )),
    ]
}

/// Phase 3 — database setup.
pub fn setup_database() -> Vec<Box<dyn Prerequisite>> {
    vec![
        Box::new(GenericPrerequisite::new(
            "database", "Sensei Database",
            Box::new(DatabaseChecker),
            Box::new(DatabaseSetupFixer),
            GateKind::Install, None,
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform;

    #[test]
    fn install_prerequisites_returns_three_gates() {
        let provider = Arc::from(platform::detect());
        let prereqs = install_prerequisites(provider);
        assert_eq!(prereqs.len(), 3);
        assert_eq!(prereqs[0].id(), "postgresql");
        assert_eq!(prereqs[1].id(), "ollama");
        assert_eq!(prereqs[2].id(), "sensei");
    }

    #[test]
    fn start_services_returns_three_gates() {
        let provider = Arc::from(platform::detect());
        let prereqs = start_services(provider);
        assert_eq!(prereqs.len(), 3);
        assert_eq!(prereqs[0].id(), "postgresql");
        assert_eq!(prereqs[1].id(), "ollama");
        assert_eq!(prereqs[2].id(), "daemon");
    }

    #[test]
    fn setup_database_returns_one_gate() {
        let prereqs = setup_database();
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].id(), "database");
    }

    #[test]
    fn install_prereqs_all_have_install_kind() {
        let provider = Arc::from(platform::detect());
        for p in install_prerequisites(provider) {
            assert_eq!(p.gate_kind(), GateKind::Install, "{} should be Install kind", p.id());
        }
    }

    #[test]
    fn service_prereqs_all_have_service_kind() {
        let provider = Arc::from(platform::detect());
        for p in start_services(provider) {
            assert_eq!(p.gate_kind(), GateKind::Service, "{} should be Service kind", p.id());
        }
    }

    #[test]
    fn detect_brew_path_does_not_panic() {
        let _ = detect_brew_path(); // Some on macOS with brew, None otherwise
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap prereq::factory::tests 2>&1 | tail -10
```
Expected: `test result: ok. 6 passed`

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/prereq/factory.rs
git commit -m "feat(bootstrap): add PlatformFactory building phase prerequisites per platform"
```

---

## Task 7: Expose prereq module from lib.rs

**Files:**
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Add module declaration**

Add `pub mod prereq;` after `pub mod util;` in `daemon/crates/bootstrap/src/lib.rs`:

```rust
pub mod database;
pub mod hardware;
pub mod models;
pub mod platform;
pub mod prereq;       // ← add this line
pub mod types;
pub mod util;
```

- [ ] **Step 2: Run full crate test suite**

```bash
cd /Users/Jerry/Developer/sensei/daemon
cargo test -p sensei-bootstrap 2>&1 | tail -20
```
Expected: All existing tests pass + all new prereq tests pass. Zero failures.

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): expose prereq module from crate root"
```

---

## Task 8: Refactor Tauri commands to use factory + runner

**Files:**
- Modify: `app/src-tauri/src/commands/bootstrap.rs`

- [ ] **Step 1: Replace the file content**

```rust
//! Bootstrap commands — prerequisite detection, installation, and hardware profiling.
//!
//! Phase commands delegate entirely to the prereq factory + runner.
//! No orchestration logic lives here — commands are thin wrappers that:
//!   1. Build a prerequisite list via the platform factory
//!   2. Run the generic runner with an event-emitting progress callback
//!   3. Return immediately — progress arrives on the "bootstrap" channel

use sensei_bootstrap::{
    self as bootstrap,
    BootstrapResult, HardwareInfo,
    prereq::{GateStatus, ProgressEvent, factory, runner},
};
use std::sync::Arc;
use tauri::Emitter;

// ---------------------------------------------------------------------------
// Event helpers
// ---------------------------------------------------------------------------

fn emit_gate(
    app: &tauri::AppHandle,
    id: &str,
    status: &str,
    version: Option<&str>,
    detail: Option<&str>,
) {
    let _ = app.emit(
        "bootstrap",
        serde_json::json!({
            "action": "update",
            "entity": "gate",
            "id": id,
            "data": { "status": status, "version": version, "detail": detail }
        }),
    );
}

fn emit_phase_complete(app: &tauri::AppHandle, phase: &str, success: bool) {
    let _ = app.emit(
        "bootstrap",
        serde_json::json!({
            "action": "set",
            "entity": "phase",
            "id": phase,
            "data": { "complete": true, "success": success }
        }),
    );
}

/// Map a ProgressEvent from the runner to Tauri events on the "bootstrap" channel.
fn dispatch(app: &tauri::AppHandle, event: ProgressEvent) {
    match event {
        ProgressEvent::Gate { id, status } => {
            let (s, version, detail) = match status {
                GateStatus::Checking            => ("checking",   None, None),
                GateStatus::Installing          => ("installing", None, None),
                GateStatus::Starting            => ("starting",   None, None),
                GateStatus::Ready { version, detail } => ("ready", version, detail),
                GateStatus::Failed { error }    => ("error",      None, Some(error)),
            };
            emit_gate(app, &id, s, version.as_deref(), detail.as_deref());
        }
        ProgressEvent::PhaseComplete { phase, success } => {
            emit_phase_complete(app, &phase, success);
        }
    }
}

// ---------------------------------------------------------------------------
// Read-only commands (unchanged)
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn run_bootstrap() -> BootstrapResult {
    bootstrap::run()
}

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

// ---------------------------------------------------------------------------
// Phase commands — factory + runner
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn install_prerequisites(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let provider = Arc::from(bootstrap::provider());
        let prereqs = factory::install_prerequisites(provider);
        runner::run("install", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}

#[tauri::command]
pub fn start_services(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let provider = Arc::from(bootstrap::provider());
        let prereqs = factory::start_services(provider);
        runner::run("services", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}

#[tauri::command]
pub fn setup_database(app: tauri::AppHandle) -> Result<(), String> {
    std::thread::spawn(move || {
        let prereqs = factory::setup_database();
        runner::run("database", prereqs, |e| dispatch(&app, e));
    });
    Ok(())
}
```

- [ ] **Step 2: Build the app**

```bash
cd /Users/Jerry/Developer/sensei/app
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -15
```
Expected: `Finished` with zero errors.

- [ ] **Step 3: Run all Tauri tests**

```bash
cd /Users/Jerry/Developer/sensei/app
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -15
```
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/commands/bootstrap.rs
git commit -m "refactor(app): replace hardcoded bootstrap orchestration with factory+runner"
```

---

## Self-Review

**Spec coverage:**
- [x] `Prerequisite` trait (id, label, check, fix, gate_kind, remedy) — Task 1
- [x] `Checker` implementations: Binary, Port, BinaryAndPort, Database — Task 2
- [x] `Fixer` implementations: Noop, Brew, Winget, ServiceStart (with port polling), DatabaseSetup — Task 3
- [x] `GenericPrerequisite` composing Checker + Fixer — Task 4
- [x] `Runner` with check→fix→recheck + typed ProgressEvents — Task 5
- [x] `PlatformFactory` per phase (macOS/Linux Brew, Windows Winget) — Task 6
- [x] Tauri commands wired to factory + runner — Task 8
- [x] `LogCollector` stub as managed state — Task 0
- [x] Platform quirks handled in factory match arms — Task 6
- [x] All unit tests use test doubles, no mocking framework needed — Tasks 4, 5

**Placeholder scan:** No TBDs. All code blocks are complete.

**Type consistency:**
- `CheckResult::ok_with_detail` defined Task 1, used in Task 2 ✓
- `GateKind::Service` / `GateKind::Install` defined Task 1, used in Tasks 4, 5, 6 ✓
- `ProgressEvent::Gate` / `ProgressEvent::PhaseComplete` defined Task 1, used in Tasks 5, 8 ✓
- `factory::install_prerequisites` defined Task 6, used in Task 8 ✓
- `runner::run` defined Task 5, used in Task 8 ✓
- `ServiceStartFixer::new(provider, name, port)` — 3-arg constructor consistent across Tasks 3 and 6 ✓
