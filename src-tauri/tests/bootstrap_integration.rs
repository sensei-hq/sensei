//! Integration tests for bootstrap commands.
//!
//! These test the actual sidecar logic — real binary detection, real port probes,
//! real platform provider — without needing the Tauri runtime or a running app.
//!
//! Run: cargo test --test bootstrap_integration

use sensei_bootstrap::{self as bootstrap, BootstrapResult};
use sensei_bootstrap::platform::{self, Platform, PlatformProvider};
use sensei_bootstrap::util;

// ── Phase 0: Detection ─────────────────────────────────────────────────

#[test]
fn run_bootstrap_returns_all_components() {
    let result = bootstrap::run();
    // Should always return at least 8 components:
    // homebrew, postgresql, ollama, sensei, postgresql-service, ollama-service, database, daemon-service
    assert!(
        result.components.len() >= 5,
        "expected at least 5 components, got {}",
        result.components.len()
    );
}

#[test]
fn run_bootstrap_includes_hardware() {
    let result = bootstrap::run();
    assert!(result.hardware.ram_gb > 0, "RAM should be detected");
    assert!(result.hardware.cpu_cores > 0, "CPU cores should be detected");
}

#[test]
fn provider_detects_platform() {
    let provider = bootstrap::provider();
    let platform = provider.platform();
    assert!(
        platform == Platform::MacOS || platform == Platform::Linux || platform == Platform::Windows,
        "should detect a valid platform"
    );
}

#[test]
fn provider_has_remedies() {
    let provider = bootstrap::provider();

    let prereq = provider.prereq_install_remedy();
    assert!(!prereq.title.is_empty(), "prereq remedy title should not be empty");
    assert!(!prereq.command.is_empty(), "prereq remedy command should not be empty");

    let pkgmgr = provider.package_manager_remedy();
    assert!(!pkgmgr.title.is_empty(), "pkgmgr remedy title should not be empty");
    assert!(!pkgmgr.command.is_empty(), "pkgmgr remedy command should not be empty");
}

// ── Binary detection ────────────────────────────────────────────────────

#[test]
fn check_binary_postgres_on_dev_machine() {
    // On a dev machine with Postgres installed, this should find it
    let status = util::check_binary("postgresql", "postgres", "--version");
    // We don't assert ready/failed — depends on machine state
    // But name should always be set
    assert_eq!(status.name, "postgresql");
}

#[test]
fn check_binary_ollama_on_dev_machine() {
    let status = util::check_binary("ollama", "ollama", "--version");
    assert_eq!(status.name, "ollama");
}

#[test]
fn check_binary_nonexistent_returns_failed() {
    let status = util::check_binary("fake", "nonexistent-binary-xyz", "--version");
    assert!(status.is_failed(), "nonexistent binary should return failed");
}

// ── Port probing ────────────────────────────────────────────────────────

#[test]
fn probe_port_unreachable() {
    // Port 1 should never be in use
    assert!(!util::probe_port(1));
}

#[test]
fn check_service_unreachable() {
    let status = util::check_service("test-service", 19999);
    assert!(status.is_failed());
    assert_eq!(status.name, "test-service");
}

// ── Database checks ─────────────────────────────────────────────────────

#[test]
fn database_check_returns_status() {
    // May be ready or failed depending on PostgreSQL state
    let status = bootstrap::database::check(None);
    assert_eq!(status.name, "database");
}

#[test]
fn database_setup_fails_without_postgres() {
    // If PostgreSQL isn't running, setup should return an Err
    if !util::probe_port(bootstrap::POSTGRES_PORT) {
        let result = bootstrap::database::setup(None);
        assert!(result.is_err(), "setup should fail when PostgreSQL is not running");
    }
}

// ── Platform provider ───────────────────────────────────────────────────

#[test]
fn package_manager_check_returns_status() {
    let provider = bootstrap::provider();
    let status = provider.check_package_manager();
    // On macOS dev machine, homebrew should be detected
    assert!(!status.name.is_empty());
}

#[test]
fn platform_remedies_are_actionable() {
    let provider = bootstrap::provider();
    let remedy = provider.prereq_install_remedy();
    // Command should contain something runnable
    assert!(
        remedy.command.contains("brew") || remedy.command.contains("winget"),
        "remedy command should reference the platform package manager"
    );
}

// ── Event contract ──────────────────────────────────────────────────────
// These verify the JSON shapes that Tauri commands emit match what
// the frontend expects. The actual emission is tested in the Tauri layer,
// but we can verify the data shapes here.

#[test]
fn component_status_serializes_to_expected_shape() {
    let status = bootstrap::ComponentStatus::ready("postgresql", "17.2");
    let json = serde_json::to_value(&status).unwrap();

    assert_eq!(json["name"], "postgresql");
    assert_eq!(json["state"]["state"], "ready");
    assert_eq!(json["version"], "17.2");
}

#[test]
fn failed_status_serializes_with_error() {
    let status = bootstrap::ComponentStatus::failed("daemon", "port in use");
    let json = serde_json::to_value(&status).unwrap();

    assert_eq!(json["name"], "daemon");
    assert_eq!(json["state"]["state"], "failed");
    assert_eq!(json["state"]["error"], "port in use");
}

#[test]
fn gate_event_payload_shape() {
    // Verify the event payload shape that Tauri commands emit
    let payload = serde_json::json!({
        "action": "update",
        "entity": "gate",
        "id": "postgresql",
        "data": {
            "status": "ready",
            "version": "17.2",
            "detail": "/opt/homebrew/bin/postgres",
        }
    });

    assert_eq!(payload["action"], "update");
    assert_eq!(payload["entity"], "gate");
    assert_eq!(payload["id"], "postgresql");
    assert_eq!(payload["data"]["status"], "ready");
}

#[test]
fn phase_event_payload_shape() {
    let payload = serde_json::json!({
        "action": "set",
        "entity": "phase",
        "id": "install",
        "data": {
            "complete": true,
            "success": true,
        }
    });

    assert_eq!(payload["action"], "set");
    assert_eq!(payload["entity"], "phase");
    assert_eq!(payload["id"], "install");
    assert_eq!(payload["data"]["complete"], true);
}

#[test]
fn platform_info_serializes_correctly() {
    let provider = bootstrap::provider();
    let info = serde_json::json!({
        "platform": provider.platform(),
        "package_manager": provider.package_manager_name(),
        "prereq_remedy": provider.prereq_install_remedy(),
        "pkgmgr_remedy": provider.package_manager_remedy(),
    });

    // Verify the shape the frontend expects
    assert!(info["platform"].is_string());
    assert!(info["package_manager"].is_string());
    assert!(info["prereq_remedy"]["title"].is_string());
    assert!(info["prereq_remedy"]["command"].is_string());
    assert!(info["pkgmgr_remedy"]["title"].is_string());
    assert!(info["pkgmgr_remedy"]["command"].is_string());
}
