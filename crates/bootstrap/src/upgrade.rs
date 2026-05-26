//! Post-restart upgrade flow.
//!
//! Runs after the Tauri self-updater has installed a new binary and restarted
//! the app. Two steps, each emitted to the caller as `UpgradeEvent` values
//! so the UI can stream progress:
//!
//!   1. `brew upgrade sensei-hq/tap/sensei[-dev]` — refresh the homebrew
//!      formula so the CLI / daemon / MCP binaries match the new app.
//!   2. `database::deploy(app_version)` — apply any schema migrations for the
//!      version we just upgraded to.
//!
//! The deploy step shares its implementation with `DatabaseResolver` (initial
//! install), so both code paths go through the same `database::deploy` entry
//! point — the user's "extra resolver, tied to db resolver + upgrade
//! resolver" insight.

use std::process::Command;

use crate::config::SenseiConfig;
use crate::database;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One progress tick from the upgrade flow. The UI maps `step` to the static
/// step ledger and `status` to the per-step state badge.
#[derive(Debug, Clone)]
pub struct UpgradeEvent {
    pub step: &'static str,
    pub status: &'static str,
    pub error: Option<String>,
}

impl UpgradeEvent {
    fn running(step: &'static str) -> Self {
        Self { step, status: "running", error: None }
    }
    fn done(step: &'static str) -> Self {
        Self { step, status: "done", error: None }
    }
    fn failed(step: &'static str, error: String) -> Self {
        Self { step, status: "failed", error: Some(error) }
    }
    fn complete(any_failed: bool) -> Self {
        Self {
            step: "complete",
            status: if any_failed { "partial" } else { "ok" },
            error: None,
        }
    }
}

/// Run the full upgrade flow, emitting one `UpgradeEvent` per state
/// transition. Returns `true` if every step succeeded.
///
/// Steps run in order: prereqs (brew upgrade), then db_deploy. A failure in
/// one step does not skip subsequent steps — the schema may still need to
/// land even if brew couldn't update the bottle.
pub fn run<F: Fn(UpgradeEvent)>(emit: F) -> bool {
    let mut any_failed = false;

    // ── Step 1: brew upgrade sensei ───────────────────────────────────────
    emit(UpgradeEvent::running("prereqs"));
    match brew_upgrade_sensei() {
        Ok(()) => emit(UpgradeEvent::done("prereqs")),
        Err(e) => {
            any_failed = true;
            emit(UpgradeEvent::failed("prereqs", e));
        }
    }

    // ── Step 2: dbd deploy ────────────────────────────────────────────────
    emit(UpgradeEvent::running("db_deploy"));
    let cfg = SenseiConfig::from_env();
    match database::deploy(&cfg.db_name, APP_VERSION) {
        Ok(()) => emit(UpgradeEvent::done("db_deploy")),
        Err(e) => {
            any_failed = true;
            emit(UpgradeEvent::failed("db_deploy", e));
        }
    }

    emit(UpgradeEvent::complete(any_failed));
    !any_failed
}

fn brew_upgrade_sensei() -> Result<(), String> {
    let (formula, _install_args) = SenseiConfig::from_env().sensei_tap_install_args();
    let output = Command::new("brew")
        .args(["upgrade", formula])
        .output()
        .map_err(|e| format!("brew upgrade failed to spawn: {e}"))?;
    if output.status.success() {
        return Ok(());
    }
    // "already up to date" / "no available formula upgrades" should not count
    // as failure — the user already has the latest bottle.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("up-to-date")
        || stderr.contains("already installed and up-to-date")
        || stderr.contains("no available formula")
    {
        return Ok(());
    }
    Err(format!(
        "brew upgrade {formula} exited {}: {}",
        output.status,
        stderr.trim(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn upgrade_event_helpers_set_expected_fields() {
        let r = UpgradeEvent::running("prereqs");
        assert_eq!(r.step, "prereqs");
        assert_eq!(r.status, "running");
        assert!(r.error.is_none());

        let d = UpgradeEvent::done("db_deploy");
        assert_eq!(d.status, "done");

        let f = UpgradeEvent::failed("prereqs", "boom".into());
        assert_eq!(f.status, "failed");
        assert_eq!(f.error.as_deref(), Some("boom"));

        let c_ok = UpgradeEvent::complete(false);
        assert_eq!(c_ok.step, "complete");
        assert_eq!(c_ok.status, "ok");
        let c_partial = UpgradeEvent::complete(true);
        assert_eq!(c_partial.status, "partial");
    }

    #[test]
    fn run_emits_in_canonical_order() {
        // We run against a system that almost certainly fails both steps
        // (CI without brew formula + without postgres), so the assertion
        // focuses on the *shape* of the event stream, not the outcomes.
        let events: Mutex<Vec<UpgradeEvent>> = Mutex::new(Vec::new());
        let _ = run(|e| events.lock().unwrap().push(e));
        let events = events.into_inner().unwrap();

        // Must emit at least: prereqs running → prereqs (done|failed)
        //                  → db_deploy running → db_deploy (done|failed)
        //                  → complete
        let steps: Vec<&str> = events.iter().map(|e| e.step).collect();
        assert_eq!(steps.first(), Some(&"prereqs"));
        assert_eq!(steps.last(),  Some(&"complete"));
        assert!(steps.contains(&"db_deploy"), "must emit db_deploy step");
        // Ordering: prereqs events all precede db_deploy events; db_deploy
        // events all precede complete.
        let first_db = steps.iter().position(|s| *s == "db_deploy").unwrap();
        let last_prereq = steps.iter().rposition(|s| *s == "prereqs").unwrap();
        let complete = steps.iter().position(|s| *s == "complete").unwrap();
        assert!(last_prereq < first_db, "all prereqs events before db_deploy");
        assert!(first_db < complete,    "all db_deploy events before complete");
    }
}
