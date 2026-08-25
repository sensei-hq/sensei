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
        Self { step: "complete", status: if any_failed { "partial" } else { "ok" }, error: None }
    }
}

/// Run the full upgrade flow, emitting one `UpgradeEvent` per state
/// transition. Returns `true` if every step succeeded.
///
/// Steps run in order: prereqs (brew upgrade), then db_deploy. A failure in
/// one step does not skip subsequent steps — the schema may still need to
/// land even if brew couldn't update the bottle.
pub fn run<F: Fn(UpgradeEvent)>(emit: F) -> bool {
    run_with(emit, brew_upgrade_sensei, || {
        database::deploy(&SenseiConfig::from_env().db_name, APP_VERSION)
    })
}

/// Orchestration core: emit the canonical event stream around two injected
/// steps — prereqs, then db_deploy. Split out from [`run`] so the event
/// ordering is unit-testable without spawning `brew` (network) or opening a
/// Postgres connection: those real calls used to hang the pre-commit hook.
fn run_with<F, P, D>(emit: F, prereqs: P, db_deploy: D) -> bool
where
    F: Fn(UpgradeEvent),
    P: FnOnce() -> Result<(), String>,
    D: FnOnce() -> Result<(), String>,
{
    let mut any_failed = false;

    // ── Step 1: brew upgrade sensei ───────────────────────────────────────
    emit(UpgradeEvent::running("prereqs"));
    match prereqs() {
        Ok(()) => emit(UpgradeEvent::done("prereqs")),
        Err(e) => {
            any_failed = true;
            emit(UpgradeEvent::failed("prereqs", e));
        }
    }

    // ── Step 2: dbd deploy ────────────────────────────────────────────────
    emit(UpgradeEvent::running("db_deploy"));
    match db_deploy() {
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
    let formula = SenseiConfig::from_env().sensei_tap_formula();
    // Silence the network-hitting "brew update" auto-refresh so a real upgrade
    // can't hang for minutes on a slow homebrew CDN. The command's actual
    // upgrade path still runs.
    let output = Command::new("brew")
        .args(["upgrade", formula])
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_INSTALL_UPGRADE", "1")
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
    Err(format!("brew upgrade {formula} exited {}: {}", output.status, stderr.trim(),))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Collect the event stream from a `run_with` invocation with injected
    /// step outcomes — no `brew`, no Postgres, so it can never hang.
    fn steps_for(prereqs: Result<(), String>, db: Result<(), String>) -> (bool, Vec<&'static str>) {
        let events: Mutex<Vec<UpgradeEvent>> = Mutex::new(Vec::new());
        let ok = run_with(|e| events.lock().unwrap().push(e), move || prereqs, move || db);
        let steps = events.into_inner().unwrap().iter().map(|e| e.step).collect();
        (ok, steps)
    }

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
        // Inject step outcomes so the ordering assertion needs no real `brew`
        // (network) or Postgres — the flaky/slow calls that used to hang the
        // pre-commit hook. We assert the *shape* of the stream: all prereqs
        // events precede all db_deploy events, which precede complete.
        let (_ok, steps) = steps_for(Ok(()), Err("no database in test".into()));
        assert_eq!(steps.first(), Some(&"prereqs"));
        assert_eq!(steps.last(), Some(&"complete"));
        assert!(steps.contains(&"db_deploy"), "must emit db_deploy step");
        let first_db = steps.iter().position(|s| *s == "db_deploy").unwrap();
        let last_prereq = steps.iter().rposition(|s| *s == "prereqs").unwrap();
        let complete = steps.iter().position(|s| *s == "complete").unwrap();
        assert!(last_prereq < first_db, "all prereqs events before db_deploy");
        assert!(first_db < complete, "all db_deploy events before complete");
    }

    #[test]
    fn run_all_steps_succeed_returns_true() {
        let (ok, steps) = steps_for(Ok(()), Ok(()));
        assert!(ok, "both steps Ok → run returns true");
        // Success path emits done (not failed) for each step, ending in complete.
        assert_eq!(steps, vec!["prereqs", "prereqs", "db_deploy", "db_deploy", "complete"]);
    }

    #[test]
    fn run_prereqs_failure_does_not_skip_db_deploy() {
        // A brew failure must NOT skip the schema deploy — the migration may
        // still need to land even if the bottle couldn't update.
        let (ok, steps) = steps_for(Err("brew boom".into()), Ok(()));
        assert!(!ok, "any failure → run returns false");
        assert!(steps.contains(&"db_deploy"), "db_deploy runs despite prereqs failure");
    }
}
