//! Strategy-based prerequisite system for bootstrap.
//!
//! Each prerequisite composes a Checker (how to verify) and a Fixer (how to repair).
//! The Runner orchestrates check→fix→recheck and emits ProgressEvents via callback.

use serde::{Deserialize, Serialize};

/// Result of a check operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub ok: bool,
    pub version: Option<String>,
    pub detail: Option<String>,
    pub error: Option<String>,
}

impl CheckResult {
    pub fn ok(version: impl Into<String>) -> Self {
        Self {
            ok: true,
            version: Some(version.into()),
            detail: None,
            error: None,
        }
    }

    pub fn ok_with_detail(version: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            ok: true,
            version: Some(version.into()),
            detail: Some(detail.into()),
            error: None,
        }
    }

    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            version: None,
            detail: None,
            error: Some(error.into()),
        }
    }
}

/// Result of a fix attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixResult {
    pub approach: String,
}

impl FixResult {
    pub fn new(approach: impl Into<String>) -> Self {
        Self {
            approach: approach.into(),
        }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateStatus {
    Checking,
    Installing,
    Starting,
    Ready {
        version: Option<String>,
        detail: Option<String>,
    },
    Failed {
        error: String,
    },
}

/// Progress event emitted by the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProgressEvent {
    Gate {
        id: String,
        status: GateStatus,
    },
    PhaseComplete {
        phase: String,
        success: bool,
    },
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

/// A human action required before bootstrap can continue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanAction {
    pub component_id: String,
    pub title:        String,
    pub command:      String,
    pub url:          Option<String>,
}

/// Final result for a single component gate after check_and_fix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateReport {
    pub id:            String,
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

    #[test]
    fn human_action_stores_fields() {
        let a = HumanAction {
            component_id: "homebrew".to_string(),
            title:        "Install Homebrew".to_string(),
            command:      "/bin/bash -c install.sh".to_string(),
            url:          Some("https://brew.sh".to_string()),
        };
        assert_eq!(a.component_id, "homebrew");
        assert_eq!(a.command, "/bin/bash -c install.sh");
        assert!(a.url.is_some());
    }

    #[test]
    fn gate_report_fix_attempted_flag() {
        let r = GateReport {
            id:            "postgresql".to_string(),
            status:        GateStatus::Ready { version: Some("17.2".into()), detail: None },
            fix_attempted: true,
            fix_detail:    Some("brew bundle upgraded".into()),
        };
        assert!(r.fix_attempted);
        assert_eq!(r.fix_detail.as_deref(), Some("brew bundle upgraded"));
    }

    #[test]
    fn bootstrap_report_all_ok_false_when_any_failed() {
        let report = BootstrapReport {
            gates: vec![
                GateReport { id: "homebrew".to_string(), status: GateStatus::Ready { version: None, detail: None }, fix_attempted: false, fix_detail: None },
                GateReport { id: "postgresql".to_string(), status: GateStatus::Failed { error: "not found".into() }, fix_attempted: false, fix_detail: None },
            ],
            all_ok:     false,
            blocked_on: None,
        };
        assert!(!report.all_ok);
        assert!(report.blocked_on.is_none());
    }

    #[test]
    fn bootstrap_report_serializes() {
        let report = BootstrapReport {
            gates:      vec![],
            all_ok:     true,
            blocked_on: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"all_ok\":true"));
    }
}
