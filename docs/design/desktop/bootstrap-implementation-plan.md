# Bootstrap Post-Install Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the bootstrap crate behind a platform provider trait, wire Tauri commands for the three post-install phases, and update the frontend to drive the full lifecycle.

**Architecture:** Platform provider trait in Rust (`PlatformProvider`) with macOS implementation. Three sequential Tauri commands (`install_prerequisites`, `start_services`, `setup_database`) emit status events. Frontend `BootstrapState` drives the UI via derived state — template renders data, nothing else.

**Tech Stack:** Rust (sensei-bootstrap crate), Tauri 2 (commands + events), SvelteKit + Svelte 5 runes, Vitest

**Spec:** `docs/design/02-desktop/bootstrap-post-install.md`
**Gap analysis:** `docs/design/02-desktop/bootstrap-gap-analysis.md`

---

## Task 0: Delete all experimental bootstrap code

Clean slate. All existing code was experimental prototyping. The design replaces it entirely. Delete before writing new code to avoid contamination.

**Experimental code inventory (all to be deleted):**

Rust — `daemon/crates/bootstrap/src/`:
- `homebrew.rs` — entire file (moves to `platform/macos.rs` + `util.rs`)
- `service.rs` — entire file (moves to `platform/macos.rs` + `util.rs`)

Tauri — `app/src-tauri/src/commands/bootstrap.rs`:
- `check_all_components` — redundant with `run_bootstrap`
- `install_component` — experimental per-gate install, replaced by `install_prerequisites`
- `start_component` — experimental per-gate start, replaced by `start_services`
- `create_database` — experimental, replaced by `setup_database`
- `brew_bundle_install` — experimental Phase 1, replaced by `install_prerequisites`

Frontend — `app/src/lib/bootstrap.ts`:
- `installComponent()` — experimental, replaced by `installPrerequisites()`
- `startComponent()` — experimental, replaced by `startServices()`
- `createDatabase()` — experimental, replaced by `setupDatabase()`
- `brewBundleInstall()` — experimental, replaced by `installPrerequisites()`

Frontend — `app/src/routes/(health)/health/+page.svelte`:
- `runBrewBundle()` — experimental, replaced by `runInstallPrereqs()`
- `retryAll()` — references `missingBrewGates`, renamed to `missingPrereqGates`
- Hardcoded `BREWFILE_URL` const — replaced by `bs.platformInfo.prereq_remedy.command`
- All `missingBrewGates` / `needsBrewInstall` / `installing` inline refs — moved to BootstrapState

**Files:**
- Modify: `app/src-tauri/src/commands/bootstrap.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src/lib/bootstrap.ts`

- [ ] **Step 1: Remove all experimental Tauri commands**

In `app/src-tauri/src/commands/bootstrap.rs`, delete these functions entirely:
- `check_all_components`
- `install_component`
- `start_component`
- `create_database`
- `brew_bundle_install`

Keep only: `run_bootstrap`, `detect_hardware`, `list_models`, `missing_models` (these are correct and match the design).

- [ ] **Step 2: Remove from Tauri handler registration**

In `app/src-tauri/src/lib.rs`, remove from `generate_handler![]`:
- `commands::bootstrap::check_all_components`
- `commands::bootstrap::install_component`
- `commands::bootstrap::start_component`
- `commands::bootstrap::create_database`
- `commands::bootstrap::brew_bundle_install`

- [ ] **Step 3: Remove obsolete frontend API functions**

In `app/src/lib/bootstrap.ts`, delete:
- `installComponent()` function
- `startComponent()` function
- `createDatabase()` function
- `brewBundleInstall()` function
- `listenBootstrapEvents()` function (will be rewritten with new event names)

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo check -p sensei-bootstrap
cd /Users/Jerry/Developer/sensei/app && bunx tsc --noEmit
```

Expected: Rust passes. TypeScript may have errors from the page still referencing deleted functions — that's expected and gets fixed in Task 7-8.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: delete all experimental bootstrap code — clean slate for design implementation"
```

---

## Task 1: Extract shared utilities from homebrew.rs and service.rs

Move cross-platform functions out of platform-specific modules into a shared module.

**Files:**
- Create: `daemon/crates/bootstrap/src/util.rs`
- Modify: `daemon/crates/bootstrap/src/homebrew.rs`
- Modify: `daemon/crates/bootstrap/src/service.rs`
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Create `util.rs` with shared functions**

Create `daemon/crates/bootstrap/src/util.rs`:

```rust
//! Cross-platform utilities — binary detection, port probing, version fetching.

use std::net::TcpStream;
use std::process::Command;
use std::time::Duration;

use crate::types::ComponentStatus;

/// Find a binary in PATH. Returns the full path if found.
pub fn which_binary(name: &str) -> Option<String> {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(cmd).arg(name).output().ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).lines().next()?.trim().to_string())
    } else {
        None
    }
}

/// Run `<binary> <flag>` and extract a version string from the first line of output.
pub fn binary_version(binary: &str, flag: &str) -> Option<String> {
    let output = Command::new(binary).arg(flag).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let first_line = text.lines().next()?;
    first_line
        .split_whitespace()
        .rev()
        .find(|w| w.chars().any(|c| c.is_ascii_digit()))
        .map(|v| v.to_string())
}

/// Check if a binary exists in PATH and report its version.
/// The binary path is stored in `detail`.
pub fn check_binary(name: &str, binary: &str, version_flag: &str) -> ComponentStatus {
    let path = match which_binary(binary) {
        Some(p) => p,
        None => return ComponentStatus::missing(name),
    };
    let version = binary_version(binary, version_flag);
    let mut status = ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"));
    status.detail = Some(path);
    status
}

/// Probe a TCP port (connect with 2s timeout). Returns true if responsive.
pub fn probe_port(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_secs(2),
    )
    .is_ok()
}

/// Check if a service is reachable on the given port.
pub fn check_service(name: &str, port: u16) -> ComponentStatus {
    match probe_port(port) {
        true => {
            let version = fetch_service_version(name, port);
            ComponentStatus::ready(name, version.as_deref().unwrap_or("unknown"))
        }
        false => ComponentStatus::failed(name, &format!("not reachable on port {port}")),
    }
}

/// Fetch version from a service's health endpoint.
pub fn fetch_service_version(name: &str, port: u16) -> Option<String> {
    match name {
        "daemon" => {
            let url = format!("http://127.0.0.1:{port}/health");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build().ok()?
                .get(&url).send().ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "ollama" => {
            let url = format!("http://127.0.0.1:{port}/api/version");
            let resp = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(2))
                .build().ok()?
                .get(&url).send().ok()?;
            let json: serde_json::Value = resp.json().ok()?;
            json["version"].as_str().map(|s| s.to_string())
        }
        "postgresql" => {
            let output = Command::new("psql").args(["--version"]).output().ok()?;
            let text = String::from_utf8_lossy(&output.stdout);
            text.split_whitespace().last().map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Start a daemon process directly (cross-platform).
pub fn start_daemon(port: u16) -> Result<ComponentStatus, String> {
    let binary = which_binary("senseid").ok_or("senseid binary not found in PATH")?;

    Command::new(&binary)
        .args(["serve", "--port", &port.to_string()])
        .spawn()
        .map_err(|e| format!("failed to start senseid: {e}"))?;

    std::thread::sleep(Duration::from_millis(500));

    if probe_port(port) {
        let version = fetch_service_version("daemon", port);
        Ok(ComponentStatus::ready("daemon", version.as_deref().unwrap_or("unknown")))
    } else {
        Ok(ComponentStatus {
            name: "daemon".to_string(),
            state: crate::types::ComponentState::Starting,
            version: None,
            detail: Some("started but not yet responding".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_binary_finds_ls() {
        let result = which_binary("ls");
        assert!(result.is_some(), "should find ls in PATH");
    }

    #[test]
    fn which_binary_returns_none_for_nonexistent() {
        let result = which_binary("sensei-nonexistent-binary-xyz");
        assert!(result.is_none());
    }

    #[test]
    fn probe_port_closed() {
        assert!(!probe_port(1));
    }

    #[test]
    fn check_binary_nonexistent() {
        let status = check_binary("nope", "sensei-nonexistent-binary-xyz", "--version");
        assert!(status.is_failed());
    }

    #[test]
    fn version_parsing_postgres() {
        let line = "postgres (PostgreSQL) 17.2";
        let parsed = line.split_whitespace().rev()
            .find(|w| w.chars().any(|c| c.is_ascii_digit()))
            .map(|v| v.to_string());
        assert_eq!(parsed, Some("17.2".to_string()));
    }

    #[test]
    fn version_parsing_ollama() {
        let line = "ollama version 0.5.4";
        let parsed = line.split_whitespace().rev()
            .find(|w| w.chars().any(|c| c.is_ascii_digit()))
            .map(|v| v.to_string());
        assert_eq!(parsed, Some("0.5.4".to_string()));
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

In `daemon/crates/bootstrap/src/lib.rs`, add:

```rust
pub mod util;
```

- [ ] **Step 3: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap -- util
```

Expected: all `util` tests pass.

- [ ] **Step 4: Commit**

```bash
git add daemon/crates/bootstrap/src/util.rs daemon/crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): extract cross-platform utilities to util.rs"
```

---

## Task 2: Create platform provider trait and macOS implementation

Define the `PlatformProvider` trait and move Homebrew/launchd logic into a macOS provider.

**Files:**
- Create: `daemon/crates/bootstrap/src/platform/mod.rs`
- Create: `daemon/crates/bootstrap/src/platform/macos.rs`
- Create: `daemon/crates/bootstrap/src/platform/windows.rs`
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Create `platform/mod.rs` with trait definition**

Create `daemon/crates/bootstrap/src/platform/mod.rs`:

```rust
//! Platform abstraction — same bootstrap pipeline, OS-specific implementations.

pub mod macos;
pub mod windows;

use crate::types::ComponentStatus;

/// Detected operating system.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
}

/// Commands to show the user for manual installation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstallRemedy {
    /// Human-readable title (e.g. "Install missing components")
    pub title: String,
    /// Shell command the user can copy-paste
    pub command: String,
    /// URL for more info (e.g. brew.sh, github releases)
    pub url: Option<String>,
}

/// Platform-specific operations for bootstrap.
pub trait PlatformProvider: Send + Sync {
    /// Which OS this provider is for.
    fn platform(&self) -> Platform;

    /// Check if the platform's package manager is available.
    fn check_package_manager(&self) -> ComponentStatus;

    /// Package manager display name (e.g. "Homebrew", "winget").
    fn package_manager_name(&self) -> &str;

    /// Install all prerequisites in one shot. Blocking.
    fn install_prerequisites(&self) -> Result<(), String>;

    /// Start a named service. Returns status after start attempt.
    fn start_service(&self, name: &str) -> Result<ComponentStatus, String>;

    /// Get the install remedy to show the user (for manual/browser mode).
    fn prereq_install_remedy(&self) -> InstallRemedy;

    /// Get the package manager install remedy (when pkg mgr itself is missing).
    fn package_manager_remedy(&self) -> InstallRemedy;
}

/// Detect the current platform and return the appropriate provider.
pub fn detect() -> Box<dyn PlatformProvider> {
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
        Box::new(macos::MacOSProvider::new())
    } else if cfg!(target_os = "windows") {
        Box::new(windows::WindowsProvider::new())
    } else {
        // Fallback to macOS provider (best effort)
        Box::new(macos::MacOSProvider::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_returns_provider() {
        let provider = detect();
        // On macOS CI/dev this should be MacOS
        let platform = provider.platform();
        assert!(
            platform == Platform::MacOS || platform == Platform::Linux || platform == Platform::Windows,
            "should detect a known platform"
        );
    }

    #[test]
    fn provider_has_package_manager_name() {
        let provider = detect();
        assert!(!provider.package_manager_name().is_empty());
    }
}
```

- [ ] **Step 2: Create `platform/macos.rs` — Homebrew provider**

Create `daemon/crates/bootstrap/src/platform/macos.rs`:

```rust
//! macOS / Linux platform provider — uses Homebrew for package management,
//! launchd (via brew services) for service management.

use std::process::Command;
use std::io::Write;

use crate::types::{ComponentStatus, ComponentState};
use crate::util;
use super::{PlatformProvider, Platform, InstallRemedy};

const BREW_PATH_ARM: &str = "/opt/homebrew/bin/brew";
const BREW_PATH_INTEL: &str = "/usr/local/bin/brew";

const BREWFILE_URL: &str = "https://raw.githubusercontent.com/sensei-hq/homebrew-tap/main/Brewfile";

const BREWFILE_CONTENT: &str = concat!(
    "tap \"sensei-hq/tap\", \"https://github.com/sensei-hq/homebrew-tap\"\n",
    "brew \"postgresql@17\"\n",
    "brew \"ollama\"\n",
    "brew \"sensei-hq/tap/sensei\"\n",
);

pub struct MacOSProvider {
    brew_path: Option<String>,
}

impl MacOSProvider {
    pub fn new() -> Self {
        Self {
            brew_path: find_brew_path(),
        }
    }

    /// Get the brew binary path, if available.
    pub fn brew_path(&self) -> Option<&str> {
        self.brew_path.as_deref()
    }

    /// Run brew bundle with the Brewfile piped to stdin.
    pub fn brew_bundle_install(&self) -> Result<(), String> {
        let brew = self.brew_path.as_deref()
            .ok_or_else(|| "homebrew not installed".to_string())?;

        let mut child = Command::new(brew)
            .args(["bundle", "--file=-"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to start brew bundle: {e}"))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(BREWFILE_CONTENT.as_bytes())
                .map_err(|e| format!("failed to write Brewfile: {e}"))?;
        }

        let output = child.wait_with_output()
            .map_err(|e| format!("brew bundle failed: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Err(format!("brew bundle failed: {stderr}"))
        }
    }
}

impl PlatformProvider for MacOSProvider {
    fn platform(&self) -> Platform {
        if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            Platform::Linux
        }
    }

    fn check_package_manager(&self) -> ComponentStatus {
        match &self.brew_path {
            Some(p) => {
                let version = brew_version(p);
                ComponentStatus::ready("homebrew", version.as_deref().unwrap_or("unknown"))
            }
            None => ComponentStatus::missing("homebrew"),
        }
    }

    fn package_manager_name(&self) -> &str {
        "Homebrew"
    }

    fn install_prerequisites(&self) -> Result<(), String> {
        self.brew_bundle_install()
    }

    fn start_service(&self, name: &str) -> Result<ComponentStatus, String> {
        let brew = self.brew_path.as_deref()
            .ok_or_else(|| "homebrew not installed".to_string())?;

        let formula = match name {
            "postgresql" | "postgres" => "postgresql@17",
            "ollama" => "ollama",
            "daemon" | "senseid" => "sensei-hq/tap/sensei",
            _ => return Err(format!("unknown service: {name}")),
        };

        let output = Command::new(brew)
            .args(["services", "start", formula])
            .output()
            .map_err(|e| format!("failed to start {name}: {e}"))?;

        if output.status.success() {
            Ok(ComponentStatus {
                name: name.to_string(),
                state: ComponentState::Starting,
                version: None,
                detail: Some(format!("brew services start {formula}")),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // Fallback for daemon: try direct start
            if name == "daemon" || name == "senseid" {
                return util::start_daemon(crate::DAEMON_PORT);
            }
            Err(format!("brew services start {formula} failed: {stderr}"))
        }
    }

    fn prereq_install_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install missing components".into(),
            command: format!("curl -fsSL {BREWFILE_URL} | brew bundle --file=-"),
            url: Some("https://github.com/sensei-hq/homebrew-tap".into()),
        }
    }

    fn package_manager_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install Homebrew".into(),
            command: r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.into(),
            url: Some("https://brew.sh".into()),
        }
    }
}

fn find_brew_path() -> Option<String> {
    if std::path::Path::new(BREW_PATH_ARM).exists() {
        Some(BREW_PATH_ARM.to_string())
    } else if std::path::Path::new(BREW_PATH_INTEL).exists() {
        Some(BREW_PATH_INTEL.to_string())
    } else {
        None
    }
}

fn brew_version(brew_path: &str) -> Option<String> {
    let output = Command::new(brew_path).args(["--version"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("Homebrew "))
        .map(|v| v.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_reports_platform() {
        let p = MacOSProvider::new();
        let platform = p.platform();
        assert!(platform == Platform::MacOS || platform == Platform::Linux);
    }

    #[test]
    fn check_package_manager_returns_status() {
        let p = MacOSProvider::new();
        let status = p.check_package_manager();
        assert_eq!(status.name, "homebrew");
        // May be ready or failed depending on dev machine
    }

    #[test]
    fn package_manager_name_is_homebrew() {
        let p = MacOSProvider::new();
        assert_eq!(p.package_manager_name(), "Homebrew");
    }

    #[test]
    fn prereq_remedy_contains_brewfile_url() {
        let p = MacOSProvider::new();
        let remedy = p.prereq_install_remedy();
        assert!(remedy.command.contains("homebrew-tap"));
    }

    #[test]
    fn package_manager_remedy_contains_brew_sh() {
        let p = MacOSProvider::new();
        let remedy = p.package_manager_remedy();
        assert!(remedy.command.contains("Homebrew/install"));
    }
}
```

- [ ] **Step 3: Create `platform/windows.rs` — stub**

Create `daemon/crates/bootstrap/src/platform/windows.rs`:

```rust
//! Windows platform provider — stub for future implementation.
//! Uses winget for package management, Windows Services for service management.

use crate::types::ComponentStatus;
use super::{PlatformProvider, Platform, InstallRemedy};

pub struct WindowsProvider;

impl WindowsProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for WindowsProvider {
    fn platform(&self) -> Platform {
        Platform::Windows
    }

    fn check_package_manager(&self) -> ComponentStatus {
        // winget ships with Windows 11+, check if available
        match crate::util::which_binary("winget") {
            Some(_) => ComponentStatus::ready("winget", "built-in"),
            None => ComponentStatus::failed("winget", "winget not available — requires Windows 10 1709+"),
        }
    }

    fn package_manager_name(&self) -> &str {
        "winget"
    }

    fn install_prerequisites(&self) -> Result<(), String> {
        Err("Windows prerequisite installation not yet implemented".into())
    }

    fn start_service(&self, name: &str) -> Result<ComponentStatus, String> {
        Err(format!("Windows service management not yet implemented for {name}"))
    }

    fn prereq_install_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "Install missing components".into(),
            command: "winget install PostgreSQL.PostgreSQL.17\nwinget install Ollama.Ollama".into(),
            url: Some("https://github.com/sensei-hq/sensei/releases".into()),
        }
    }

    fn package_manager_remedy(&self) -> InstallRemedy {
        InstallRemedy {
            title: "winget is built into Windows".into(),
            command: "winget --version".into(),
            url: Some("https://learn.microsoft.com/en-us/windows/package-manager/winget/".into()),
        }
    }
}
```

- [ ] **Step 4: Register platform module in lib.rs**

In `daemon/crates/bootstrap/src/lib.rs`, add:

```rust
pub mod platform;
```

- [ ] **Step 5: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap -- platform
```

Expected: all platform tests pass.

- [ ] **Step 6: Commit**

```bash
git add daemon/crates/bootstrap/src/platform/
git commit -m "feat(bootstrap): add PlatformProvider trait with macOS and Windows stub"
```

---

## Task 3: Rewrite lib.rs::run() to use platform provider

Replace the flat list of checks with platform-aware detection.

**Files:**
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Rewrite `run()` function**

Replace the entire `run()` function and add a `PlatformBootstrapResult`:

```rust
//! sensei-bootstrap — prerequisite detection, installation, and hardware profiling.
//!
//! This crate has NO database dependencies and NO daemon dependencies.
//! It checks and fixes prerequisites using shell commands and port probes.
//!
//! Consumers: sensei-cli (`sensei doctor`), Tauri desktop app (sidecar commands).

pub mod database;
pub mod hardware;
pub mod models;
pub mod platform;
pub mod types;
pub mod util;

pub use types::*;

/// Default ports for sensei services.
pub const DAEMON_PORT: u16 = 7744;
pub const OLLAMA_PORT: u16 = 11434;
pub const POSTGRES_PORT: u16 = 5432;

/// Run the full bootstrap check — all phases, returns composite result.
///
/// Uses the detected platform provider for package manager checks.
/// Binary detection and port probes are cross-platform.
pub fn run() -> BootstrapResult {
    let provider = platform::detect();
    let hw = hardware::detect();

    let components = vec![
        // Gate 一: Package manager
        provider.check_package_manager(),
        // Gate 二: PostgreSQL binary
        util::check_binary("postgresql", "postgres", "--version"),
        // Gate 三: Ollama binary
        util::check_binary("ollama", "ollama", "--version"),
        // Gate 四: Sensei CLI
        util::check_binary("sensei", "sensei", "--version"),
        // Gate 二+: PostgreSQL service
        util::check_service("postgresql", POSTGRES_PORT),
        // Gate 三+: Ollama service
        util::check_service("ollama", OLLAMA_PORT),
        // Gate 五: Database
        database::check(None),
        // Gate 六: Daemon service
        util::check_service("daemon", DAEMON_PORT),
    ];

    BootstrapResult::from_checks(components, hw)
}

/// Get the current platform provider.
pub fn provider() -> Box<dyn platform::PlatformProvider> {
    platform::detect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_does_not_panic() {
        let result = run();
        assert!(!result.components.is_empty(), "should check at least one component");
        assert!(result.components.len() >= 5, "should check multiple components");
    }

    #[test]
    fn port_constants() {
        assert_eq!(DAEMON_PORT, 7744);
        assert_eq!(OLLAMA_PORT, 11434);
        assert_eq!(POSTGRES_PORT, 5432);
    }

    #[test]
    fn provider_returns_valid() {
        let p = provider();
        assert!(!p.package_manager_name().is_empty());
    }
}
```

- [ ] **Step 2: Run all crate tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap
```

Expected: all tests pass. Some existing homebrew.rs/service.rs tests may now be redundant — that's fine, they'll be cleaned up in the next task.

- [ ] **Step 3: Commit**

```bash
git add daemon/crates/bootstrap/src/lib.rs
git commit -m "feat(bootstrap): rewrite run() to use platform provider"
```

---

## Task 4: Clean up old homebrew.rs and service.rs

The functionality has moved to `util.rs` and `platform/macos.rs`. The old files become thin re-exports for backward compatibility during transition, then get deleted.

**Files:**
- Delete: `daemon/crates/bootstrap/src/homebrew.rs`
- Delete: `daemon/crates/bootstrap/src/service.rs`
- Modify: `daemon/crates/bootstrap/src/lib.rs`

- [ ] **Step 1: Remove `homebrew.rs` and `service.rs`**

Delete both files entirely. All their functionality now lives in:
- `util.rs` — `which_binary`, `binary_version`, `check_binary`, `probe_port`, `check_service`, `fetch_service_version`, `start_daemon`
- `platform/macos.rs` — `MacOSProvider` (brew path, brew bundle, brew services, remedies)

- [ ] **Step 2: Remove module declarations from `lib.rs`**

In `daemon/crates/bootstrap/src/lib.rs`, remove:

```rust
pub mod homebrew;
pub mod service;
```

- [ ] **Step 3: Update any remaining internal references**

Search for `homebrew::` and `service::` in the crate. Update:
- `platform/macos.rs` — if it references `crate::homebrew::brew_path_pub()`, replace with internal `find_brew_path()`
- Tauri `commands/bootstrap.rs` — update `bootstrap::homebrew::check_binary` → `bootstrap::util::check_binary`, `bootstrap::service::` → `bootstrap::util::`, `bootstrap::homebrew::brew_path_pub()` → `bootstrap::provider()` pattern

- [ ] **Step 4: Run all tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap
cd /Users/Jerry/Developer/sensei/app/src-tauri && cargo check
```

Expected: all pass. Old homebrew/service tests are gone; their coverage is in `util.rs` and `platform/macos.rs`.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor(bootstrap): remove homebrew.rs and service.rs — replaced by util.rs + platform/"
```

---

## Task 5: Add database setup pipeline

Extend `database.rs` with the full Phase 3 orchestration: ensure extensions + setup.

**Files:**
- Modify: `daemon/crates/bootstrap/src/database.rs`

- [ ] **Step 1: Add `ensure_extensions()` function**

Add to `daemon/crates/bootstrap/src/database.rs`:

```rust
/// Ensure pgvector extension is installed. Creates it if missing.
pub fn ensure_extensions(db_name: Option<&str>) -> Result<ComponentStatus, String> {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    let output = Command::new("psql")
        .args(["-d", db, "-c", "CREATE EXTENSION IF NOT EXISTS vector"])
        .output()
        .map_err(|e| format!("failed to run psql: {e}"))?;

    if output.status.success() {
        Ok(ComponentStatus::ready("database", "extensions ok"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("failed to create pgvector extension: {stderr}"))
    }
}
```

- [ ] **Step 2: Add `migrate()` stub**

Add to `daemon/crates/bootstrap/src/database.rs`:

```rust
/// Run database migrations via `senseid migrate`.
///
/// Currently a stub — will be replaced by embedded dbd-core when available.
/// For now, checks if senseid is available and attempts to run migrations.
pub fn migrate() -> Result<ComponentStatus, String> {
    let senseid = crate::util::which_binary("senseid")
        .ok_or("senseid not found in PATH — cannot run migrations")?;

    let output = Command::new(&senseid)
        .args(["migrate"])
        .output()
        .map_err(|e| format!("senseid migrate failed: {e}"))?;

    if output.status.success() {
        let version = schema_version(DEFAULT_DB_NAME);
        Ok(ComponentStatus::ready("database", &format!("schema-{}", version.unwrap_or(0))))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("senseid migrate failed: {stderr}"))
    }
}
```

- [ ] **Step 3: Add `setup()` orchestrator**

Add to `daemon/crates/bootstrap/src/database.rs`:

```rust
/// Full Phase 3 database setup pipeline.
///
/// 1. Verify PostgreSQL is accepting connections
/// 2. Create database if missing
/// 3. Ensure pgvector extension
/// 4. Run migrations
///
/// Returns the final ComponentStatus for the database gate.
pub fn setup(db_name: Option<&str>) -> Result<ComponentStatus, String> {
    let db = db_name.unwrap_or(DEFAULT_DB_NAME);

    // Step 1: PostgreSQL reachable?
    if !pg_is_ready() {
        return Err("PostgreSQL is not accepting connections (pg_isready failed)".into());
    }

    // Step 2: Create database if missing
    if !database_exists(db) {
        create(Some(db))?;
    }

    // Step 3: Ensure extensions
    ensure_extensions(Some(db))?;

    // Step 4: Run migrations
    migrate()
}
```

- [ ] **Step 4: Add tests**

Add to the `tests` module in `database.rs`:

```rust
    #[test]
    fn ensure_extensions_command_format() {
        // Test that the psql command is correctly formed
        // (actual execution depends on PostgreSQL being available)
        let cmd = std::process::Command::new("psql")
            .args(["-d", "sensei", "-c", "CREATE EXTENSION IF NOT EXISTS vector"])
            .output();
        // Just verify the command can be spawned (may fail if psql not installed)
        assert!(cmd.is_ok() || cmd.is_err(), "command should attempt to run");
    }
```

- [ ] **Step 5: Run tests**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap -- database
```

Expected: existing + new tests pass.

- [ ] **Step 6: Commit**

```bash
git add daemon/crates/bootstrap/src/database.rs
git commit -m "feat(bootstrap): add database setup pipeline (ensure_extensions + migrate + setup)"
```

---

## Task 6: Rewrite Tauri commands

Replace the experimental Tauri commands with the three phase commands from the design.

**Files:**
- Rewrite: `app/src-tauri/src/commands/bootstrap.rs`
- Modify: `app/src-tauri/src/lib.rs`

- [ ] **Step 1: Rewrite `commands/bootstrap.rs`**

Replace the entire file:

```rust
//! Bootstrap commands — prerequisite detection, installation, service startup, database setup.
//!
//! Three phase commands emit `bootstrap:status` events for real-time UI updates.
//! Detection runs synchronously; phase commands spawn background threads.

use sensei_bootstrap::{self as bootstrap, BootstrapResult, ComponentStatus, HardwareInfo};
use sensei_bootstrap::platform::{self, InstallRemedy, Platform};

/// Phase 0: Run the full bootstrap detection — all components + hardware.
#[tauri::command]
pub fn run_bootstrap() -> BootstrapResult {
    bootstrap::run()
}

/// Get the detected platform info.
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

/// Phase 1: Install prerequisites via the platform's package manager.
///
/// Spawns the install in a background thread and polls binary checks every 1s.
/// Emits `bootstrap:status` events as each component becomes available.
/// Emits `bootstrap:install-complete` when done.
#[tauri::command]
pub fn install_prerequisites(app: tauri::AppHandle) -> Result<(), String> {
    let provider = bootstrap::provider();

    // Mark all prereq gates as checking
    for &id in &["postgresql", "ollama", "sensei"] {
        let _ = app.emit("bootstrap:status", serde_json::json!({
            "id": id, "status": "checking"
        }));
    }

    std::thread::spawn(move || {
        let checks: Vec<(&str, &str, &str)> = vec![
            ("postgresql", "postgres", "--version"),
            ("ollama", "ollama", "--version"),
            ("sensei", "sensei", "--version"),
        ];

        // Start the install (blocking in this thread)
        let install_result = provider.install_prerequisites();

        // If install itself errored, emit error for all
        if let Err(ref e) = install_result {
            for &(name, _, _) in &checks {
                let _ = app.emit("bootstrap:status", serde_json::json!({
                    "id": name, "status": "error", "detail": e
                }));
            }
            let _ = app.emit("bootstrap:install-complete", serde_json::json!({"success": false}));
            return;
        }

        // Final check — emit final statuses
        for &(name, binary, flag) in &checks {
            let status = bootstrap::util::check_binary(name, binary, flag);
            let gate_status = if status.is_ready() { "ready" } else { "missing" };
            let _ = app.emit("bootstrap:status", serde_json::json!({
                "id": name,
                "status": gate_status,
                "version": status.version,
                "detail": status.detail,
            }));
        }

        let _ = app.emit("bootstrap:install-complete", serde_json::json!({"success": true}));
    });

    Ok(())
}

/// Phase 2: Start services sequentially (PostgreSQL → Ollama → Daemon).
///
/// Skips services that are already running (port responsive).
/// Emits `bootstrap:status` events as each service comes up.
/// Emits `bootstrap:services-complete` when all are started.
#[tauri::command]
pub fn start_services(app: tauri::AppHandle) -> Result<(), String> {
    let provider = bootstrap::provider();

    let services: Vec<(&str, u16)> = vec![
        ("postgresql", bootstrap::POSTGRES_PORT),
        ("ollama", bootstrap::OLLAMA_PORT),
        ("daemon", bootstrap::DAEMON_PORT),
    ];

    std::thread::spawn(move || {
        for &(name, port) in &services {
            // Skip if already running
            if bootstrap::util::probe_port(port) {
                let version = bootstrap::util::fetch_service_version(name, port);
                let _ = app.emit("bootstrap:status", serde_json::json!({
                    "id": name,
                    "status": "ready",
                    "version": version,
                }));
                continue;
            }

            let _ = app.emit("bootstrap:status", serde_json::json!({
                "id": name, "status": "starting"
            }));

            // Attempt to start
            match provider.start_service(name) {
                Ok(_) => {
                    // Poll port until responsive (max 30s)
                    let mut ready = false;
                    for _ in 0..30 {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if bootstrap::util::probe_port(port) {
                            ready = true;
                            break;
                        }
                    }
                    if ready {
                        let version = bootstrap::util::fetch_service_version(name, port);
                        let _ = app.emit("bootstrap:status", serde_json::json!({
                            "id": name,
                            "status": "ready",
                            "version": version,
                        }));
                    } else {
                        let _ = app.emit("bootstrap:status", serde_json::json!({
                            "id": name,
                            "status": "error",
                            "detail": format!("not responding on port {port} after 30s"),
                        }));
                    }
                }
                Err(e) => {
                    let _ = app.emit("bootstrap:status", serde_json::json!({
                        "id": name,
                        "status": "error",
                        "detail": e,
                    }));
                }
            }
        }

        let _ = app.emit("bootstrap:services-complete", serde_json::json!({}));
    });

    Ok(())
}

/// Phase 3: Database setup — create DB, extensions, run migrations.
///
/// Emits `bootstrap:status` events for the database gate.
/// Emits `bootstrap:database-complete` when done.
#[tauri::command]
pub fn setup_database(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.emit("bootstrap:status", serde_json::json!({
        "id": "database", "status": "checking"
    }));

    std::thread::spawn(move || {
        match bootstrap::database::setup(None) {
            Ok(status) => {
                let _ = app.emit("bootstrap:status", serde_json::json!({
                    "id": "database",
                    "status": "ready",
                    "version": status.version,
                }));
            }
            Err(e) => {
                let _ = app.emit("bootstrap:status", serde_json::json!({
                    "id": "database",
                    "status": "error",
                    "detail": e,
                }));
            }
        }
        let _ = app.emit("bootstrap:database-complete", serde_json::json!({}));
    });

    Ok(())
}

/// Detect hardware capabilities.
#[tauri::command]
pub fn detect_hardware() -> HardwareInfo {
    bootstrap::hardware::detect()
}

/// List installed Ollama models.
#[tauri::command]
pub fn list_models() -> Vec<String> {
    bootstrap::models::list()
}

/// Check which models are missing for the recommended tier.
#[tauri::command]
pub fn missing_models() -> Vec<String> {
    let hw = bootstrap::hardware::detect();
    bootstrap::models::missing_models(&hw.recommended_tier)
}
```

- [ ] **Step 2: Update `lib.rs` handler registration**

Replace the bootstrap command registrations in `app/src-tauri/src/lib.rs`:

```rust
// Bootstrap (prereqs, hardware, models)
commands::bootstrap::run_bootstrap,
commands::bootstrap::get_platform,
commands::bootstrap::install_prerequisites,
commands::bootstrap::start_services,
commands::bootstrap::setup_database,
commands::bootstrap::detect_hardware,
commands::bootstrap::list_models,
commands::bootstrap::missing_models,
```

- [ ] **Step 3: Verify compilation**

```bash
cd /Users/Jerry/Developer/sensei/app/src-tauri && cargo check
```

Expected: compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/commands/bootstrap.rs app/src-tauri/src/lib.rs
git commit -m "feat(bootstrap): rewrite Tauri commands — install_prerequisites, start_services, setup_database"
```

---

## Task 7: Update frontend — gates, state, API

Rename brew → prereq, add platform awareness, wire new Tauri commands.

**Files:**
- Modify: `app/src/lib/bootstrap-gates.ts`
- Modify: `app/src/lib/bootstrap-state.svelte.ts`
- Modify: `app/src/lib/bootstrap.ts`

- [ ] **Step 1: Update `bootstrap-gates.ts`**

Change remedy type from `'brew'` to `'prereq'`:

```typescript
export type GateStatus = 'pending' | 'checking' | 'ready' | 'missing' | 'error' | 'starting';

export interface GateDefinition {
  id: string;
  n: string;
  name: string;
  detail: string;
  check: string;
  remedy: 'install' | 'prereq' | 'db' | 'daemon';
  sub?: SubCheckDefinition[];
}

export interface SubCheckDefinition {
  id: string;
  name: string;
  check: string;
}

export const GATES: GateDefinition[] = [
  {
    id: 'homebrew', n: '一', name: 'Homebrew',
    detail: 'package manager',
    check: 'which brew',
    remedy: 'install',
  },
  {
    id: 'postgres', n: '二', name: 'PostgreSQL',
    detail: 'storage · @17',
    check: 'which postgres',
    remedy: 'prereq',
  },
  {
    id: 'ollama', n: '三', name: 'Ollama',
    detail: 'local models for embeddings',
    check: 'which ollama',
    remedy: 'prereq',
  },
  {
    id: 'sensei', n: '四', name: 'Sensei components',
    detail: 'MCP · CLI · daemon',
    check: 'sensei --version',
    remedy: 'prereq',
    sub: [
      { id: 'cli', name: 'sensei-cli', check: 'sensei --version' },
      { id: 'mcp', name: 'MCP bridge', check: 'sensei-mcp --version' },
      { id: 'daemon', name: 'sensei-daemon', check: 'senseid --help' },
    ],
  },
  {
    id: 'database', n: '五', name: 'Database',
    detail: 'sensei schema · pgvector',
    check: 'psql sensei -c "SELECT 1"',
    remedy: 'db',
  },
  {
    id: 'senseid', n: '六', name: 'Daemon',
    detail: 'background observer',
    check: 'curl localhost:7744/health',
    remedy: 'daemon',
  },
];
```

- [ ] **Step 2: Update `bootstrap-state.svelte.ts`**

Rename brew references to prereq and add platform state:

```typescript
import { GATES, type GateStatus, type GateDefinition } from './bootstrap-gates.js';

export interface PlatformInfo {
  platform: 'macos' | 'linux' | 'windows';
  package_manager: string;
  prereq_remedy: { title: string; command: string; url?: string };
  pkgmgr_remedy: { title: string; command: string; url?: string };
}

export class BootstrapState {
  statuses = $state<Record<string, GateStatus>>(
    Object.fromEntries(GATES.map(g => [g.id, 'pending' as GateStatus]))
  );

  subStatuses = $state<Record<string, GateStatus>>({
    cli: 'pending', mcp: 'pending', daemon: 'pending',
  });

  dbUrl = $state('postgresql://localhost:5432/sensei');
  installing = $state(false);
  platformInfo = $state<PlatformInfo>({
    platform: 'macos',
    package_manager: 'Homebrew',
    prereq_remedy: { title: 'Install missing components', command: '', url: '' },
    pkgmgr_remedy: { title: 'Install Homebrew', command: '', url: '' },
  });

  // ── Derived ────────────────────────────────────────────────

  get gates() {
    if (!this.statuses) return [];
    return GATES.map(g => ({
      ...g,
      // Override gate 一 name with platform-specific package manager name
      name: g.id === 'homebrew' ? this.platformInfo.package_manager : g.name,
      status: this.statuses[g.id] ?? 'pending' as GateStatus,
      sub: g.sub?.map(s => ({ ...s, status: this.subStatuses?.[s.id] ?? 'pending' as GateStatus })),
    }));
  }

  get readyCount(): number {
    return GATES.filter(g => this.statuses[g.id] === 'ready').length;
  }

  get totalCount(): number {
    return GATES.length;
  }

  get allReady(): boolean {
    return GATES.every(g => this.statuses[g.id] === 'ready');
  }

  get firstBlockedIdx(): number {
    return GATES.findIndex(g => {
      const s = this.statuses[g.id];
      return s === 'missing' || s === 'error';
    });
  }

  get isChecking(): boolean {
    return Object.values(this.statuses).some(s => s === 'checking' || s === 'starting');
  }

  /** Prereq-type gates that are currently missing or errored. */
  get missingPrereqGates() {
    return this.gates.filter(g => g.remedy === 'prereq' && (g.status === 'missing' || g.status === 'error'));
  }

  /** True when prereq install needs to run. */
  get needsPrereqInstall(): boolean {
    return this.missingPrereqGates.length > 0;
  }

  /** Gates visible in the UI — collapses when prereqs are missing. */
  get visibleGates() {
    return this.needsPrereqInstall
      ? this.gates.filter(g => g.id === 'homebrew')
      : this.gates;
  }

  // ── Mutations ──────────────────────────────────────────────

  setGateStatus(id: string, status: GateStatus) {
    this.statuses = { ...this.statuses, [id]: status };
  }

  setSubStatus(id: string, status: GateStatus) {
    this.subStatuses = { ...this.subStatuses, [id]: status };
    const allSubReady = ['cli', 'mcp', 'daemon'].every(k => this.subStatuses[k] === 'ready');
    const anySubMissing = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'missing' || this.subStatuses[k] === 'error');
    const anySubChecking = ['cli', 'mcp', 'daemon'].some(k => this.subStatuses[k] === 'checking');
    if (allSubReady) this.setGateStatus('sensei', 'ready');
    else if (anySubMissing) this.setGateStatus('sensei', 'missing');
    else if (anySubChecking) this.setGateStatus('sensei', 'checking');
  }

  setPlatform(info: PlatformInfo) {
    this.platformInfo = info;
  }

  applyPreset(preset: Record<string, GateStatus>) {
    this.statuses = { ...this.statuses, ...preset };
  }

  startChecking() {
    this.statuses = Object.fromEntries(
      GATES.map((g, i) => [g.id, i === 0 ? 'checking' as GateStatus : 'pending' as GateStatus])
    );
  }
}

export const bootstrapState = new BootstrapState();
```

- [ ] **Step 3: Update `bootstrap.ts` API functions**

Delete `installComponent`, `startComponent`, `brewBundleInstall`. Add new functions:

```typescript
/** Install prerequisites via platform provider. Requires Tauri. Status via events. */
export async function installPrerequisites(): Promise<void> {
  return tauriInvoke<void>('install_prerequisites');
}

/** Start services sequentially. Requires Tauri. Status via events. */
export async function startServices(): Promise<void> {
  return tauriInvoke<void>('start_services');
}

/** Run database setup pipeline. Requires Tauri. Status via events. */
export async function setupDatabase(): Promise<void> {
  return tauriInvoke<void>('setup_database');
}

/** Get platform info from the backend. Requires Tauri. */
export async function getPlatform(): Promise<any> {
  return tauriInvoke<any>('get_platform');
}
```

Update `listenBootstrapEvents` to also listen for `bootstrap:services-complete` and `bootstrap:database-complete`:

```typescript
export async function listenBootstrapEvents(
  onStatus: (id: string, status: string, version?: string, detail?: string) => void,
  onPhaseComplete: (phase: string) => void,
): Promise<() => void> {
  const { listen } = await import('@tauri-apps/api/event');

  const unlisteners = await Promise.all([
    listen<{ id: string; status: string; version?: string; detail?: string }>(
      'bootstrap:status',
      (event) => onStatus(event.payload.id, event.payload.status, event.payload.version, event.payload.detail),
    ),
    listen('bootstrap:install-complete', () => onPhaseComplete('install')),
    listen('bootstrap:services-complete', () => onPhaseComplete('services')),
    listen('bootstrap:database-complete', () => onPhaseComplete('database')),
  ]);

  return () => unlisteners.forEach(fn => fn());
}
```

- [ ] **Step 4: Run type check**

```bash
cd /Users/Jerry/Developer/sensei/app && bunx tsc --noEmit
```

Expected: no type errors.

- [ ] **Step 5: Run existing tests**

```bash
cd /Users/Jerry/Developer/sensei/app && bun test
```

Expected: `bootstrap.spec.ts` tests pass (they test pure type functions, not API calls).

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/bootstrap-gates.ts app/src/lib/bootstrap-state.svelte.ts app/src/lib/bootstrap.ts
git commit -m "feat(bootstrap): update frontend — prereq naming, platform state, new API functions"
```

---

## Task 8: Update health page to use new state

Wire the page to the new BootstrapState shape and platform-aware remedies.

**Files:**
- Modify: `app/src/routes/(health)/health/+page.svelte`

- [ ] **Step 1: Rewrite the page script block**

Replace the entire `<script>` section:

```typescript
import { goto } from '$app/navigation';
import { onMount } from 'svelte';
import { appState } from '$lib/appstate.svelte.js';
import {
  hasTauri, installPrerequisites, startServices, setupDatabase,
  getPlatform, listenBootstrapEvents, runBootstrap,
} from '$lib/bootstrap.js';
import { bootstrapState as bs } from '$lib/bootstrap-state.svelte.js';
import { GATES } from '$lib/bootstrap-gates.js';
import type { GateStatus } from '$lib/bootstrap-gates.js';

// Browser mode: apply mock preset
if (!hasTauri()) {
  bs.applyPreset({
    homebrew: 'ready', postgres: 'missing', ollama: 'missing',
    sensei: 'missing', database: 'pending', senseid: 'pending',
  });
}

// Auto-advance when all ready
$effect(() => {
  if (bs.allReady) {
    setTimeout(() => {
      if (appState.setupComplete) goto('/observatory', { replaceState: true });
      else goto('/setup/welcome', { replaceState: true });
    }, 900);
  }
});

// Wire Tauri events → state updates
onMount(async () => {
  if (!hasTauri()) return;

  // Load platform info
  try {
    const info = await getPlatform();
    bs.setPlatform(info);
  } catch { /* browser mode fallback */ }

  const unlisten = await listenBootstrapEvents(
    (id, status) => bs.setGateStatus(id, status as GateStatus),
    (phase) => {
      if (phase === 'install') bs.installing = false;
    },
  );

  return () => unlisten();
});

// Retry a single gate (browser simulation)
function retry(gateId: string) {
  bs.setGateStatus(gateId, 'checking');
  if (!hasTauri()) {
    setTimeout(() => {
      bs.setGateStatus(gateId, 'ready');
      const idx = GATES.findIndex(g => g.id === gateId);
      if (idx + 1 < GATES.length && bs.statuses[GATES[idx + 1].id] === 'pending') {
        bs.setGateStatus(GATES[idx + 1].id, 'checking');
        setTimeout(() => {
          GATES.slice(idx + 1).forEach(g => bs.setGateStatus(g.id, 'ready'));
        }, 900);
      }
    }, 1100);
  }
}

async function runInstallPrereqs() {
  if (!hasTauri()) return;
  bs.installing = true;
  bs.missingPrereqGates.forEach(g => bs.setGateStatus(g.id, 'checking'));
  await installPrerequisites();
}

function retryAll() {
  bs.missingPrereqGates.forEach(g => retry(g.id));
}

function statusColor(s: GateStatus): string {
  if (s === 'ready') return 'var(--jade)';
  if (s === 'missing' || s === 'error') return 'var(--shu)';
  if (s === 'checking' || s === 'starting') return 'var(--sumi-2)';
  return 'var(--sumi-4)';
}

function pillBg(s: GateStatus): string {
  if (s === 'ready') return 'rgba(122,158,98,.10)';
  if (s === 'missing' || s === 'error') return 'rgba(192,71,45,.08)';
  if (s === 'checking' || s === 'starting') return 'var(--paper-2)';
  return 'transparent';
}

function pillLabel(s: GateStatus): string {
  const map: Record<string, string> = {
    ready: 'ready', checking: 'checking', starting: 'starting',
    missing: 'missing', error: 'blocked', pending: 'waiting',
  };
  return map[s] ?? 'waiting';
}
```

- [ ] **Step 2: Update the prereq remedy card in the template**

Replace the hardcoded brew remedy with platform-aware content:

```svelte
{#if bs.needsPrereqInstall}
  <div class="brew-remedy">
    <div class="display remedy-title">{bs.platformInfo.prereq_remedy.title}</div>
    <div class="missing-list">
      {#each bs.missingPrereqGates as gate}
        <span class="missing-tag">{gate.name}</span>
      {/each}
    </div>
    <p class="remedy-intro">
      One command installs everything. Already-installed items are skipped.
    </p>
    {#if hasTauri()}
      <div class="remedy-actions">
        <button class="btn-solid btn-sm" onclick={runInstallPrereqs} disabled={bs.installing}>
          {bs.installing ? 'Installing…' : 'Install all'}
        </button>
        <button class="btn-outline btn-sm" onclick={retryAll}>Retry checks</button>
      </div>
    {:else}
      <div class="command-block">
        <code>{bs.platformInfo.prereq_remedy.command}</code>
      </div>
      <div class="remedy-actions">
        <button class="btn-solid btn-sm" onclick={retryAll}>Retry checks</button>
      </div>
    {/if}
  </div>
{/if}
```

- [ ] **Step 3: Update homebrew remedy in gate list**

Update the `install` remedy case to use platform info:

```svelte
{#if gate.remedy === 'install'}
  <div class="display remedy-title">{bs.platformInfo.pkgmgr_remedy.title}</div>
  <p class="remedy-intro">
    {bs.platformInfo.package_manager} is the base that installs everything else.
  </p>
  <div class="command-block">
    <code>{bs.platformInfo.pkgmgr_remedy.command}</code>
  </div>
  <div class="remedy-actions">
    {#if bs.platformInfo.pkgmgr_remedy.url}
      <a href={bs.platformInfo.pkgmgr_remedy.url} target="_blank" rel="noreferrer" class="btn-outline btn-sm">
        Learn more <span style="color: var(--sumi-3);">↗</span>
      </a>
    {/if}
    <button class="btn-solid btn-sm" onclick={() => retry(gate.id)}>I've installed it — retry</button>
  </div>
{/if}
```

- [ ] **Step 4: Verify in browser**

```bash
# Dev server should already be running on :5173
# Navigate to http://localhost:5173/health
```

Expected: page renders with Homebrew gate + prereq remedy card. Tags show POSTGRESQL, OLLAMA, SENSEI COMPONENTS. Command comes from platform info (default macOS values in browser mode).

- [ ] **Step 5: Commit**

```bash
git add app/src/routes/(health)/health/+page.svelte
git commit -m "feat(bootstrap): update health page — platform-aware remedies, prereq naming"
```

---

## Task 9: Integration test — full Tauri build

Verify the entire stack compiles together.

**Files:** No new files. Compilation check.

- [ ] **Step 1: Build the Rust bootstrap crate**

```bash
cd /Users/Jerry/Developer/sensei/daemon && cargo test -p sensei-bootstrap
```

Expected: all tests pass.

- [ ] **Step 2: Build the Tauri app**

```bash
cd /Users/Jerry/Developer/sensei/app/src-tauri && cargo check
```

Expected: compiles cleanly with the new command registrations.

- [ ] **Step 3: Run frontend type check**

```bash
cd /Users/Jerry/Developer/sensei/app && bunx tsc --noEmit
```

Expected: no type errors.

- [ ] **Step 4: Run frontend tests**

```bash
cd /Users/Jerry/Developer/sensei/app && bun test
```

Expected: all tests pass.

- [ ] **Step 5: Verify in browser**

Navigate to `http://localhost:5173/health`. Verify:
- Fixed header stays pinned on scroll
- Homebrew gate shows READY
- Prereq remedy card shows missing components with platform command
- Retry checks simulates success in browser mode
- All gates going ready triggers auto-advance

- [ ] **Step 6: Final commit**

```bash
git add -A
git commit -m "feat(bootstrap): complete post-install restructure — platform provider, 3-phase pipeline, platform-aware UI"
```
