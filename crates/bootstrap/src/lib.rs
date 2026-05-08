//! sensei-bootstrap — prerequisite detection, installation, and hardware profiling.
//!
//! This crate has NO database dependencies and NO daemon dependencies.
//! It checks and fixes prerequisites using shell commands and port probes.
//!
//! Consumers: sensei-cli (`sensei doctor`), Tauri desktop app (sidecar commands).

pub mod config;
pub mod database;
pub mod hardware;
pub mod models;
pub mod platform;
pub mod prereq;
pub mod types;
pub mod util;

pub use config::{
    SenseiConfig, SenseiMode,
    BREW_PATHS, BREW_TAP, GITHUB_ORG, GITHUB_REPO,
    HOMEBREW_BREWFILE_URL, HOMEBREW_TAP_REPO, HOMEBREW_TAP_URL,
    MARKETPLACE_RAW_URL, MARKETPLACE_REPO,
    OLLAMA_PORT, POSTGRES_PORT,
};
pub use prereq::{BootstrapReport, GateReport, GateStatus, HumanAction, ProgressEvent};
pub use prereq::engine::{BootstrapContext, BootstrapEngine};
pub use types::*;
pub use types::BootstrapTrace;

/// Return the daemon port for the current mode.
///
/// Delegates to [`SenseiConfig::from_env`] — single source of truth.
pub fn daemon_port() -> u16 {
    SenseiConfig::from_env().daemon_port
}

/// Return the platform provider for the current OS.
pub fn provider() -> Box<dyn platform::PlatformProvider> {
    platform::detect()
}

/// Run check_and_fix with a pre-built BootstrapContext (supports test injection).
pub fn check_and_fix_with_context<F>(ctx: BootstrapContext, callback: F) -> BootstrapReport
where
    F: Fn(ProgressEvent) + Send + Sync + 'static,
{
    let engine = BootstrapEngine::new(std::sync::Arc::new(ctx));
    engine.check_and_fix(callback)
}

/// Convenience: run check_and_fix using environment config (no test injection).
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

/// Run all bootstrap checks in parallel and return both the result and diagnostic traces.
///
/// Each gate runs on its own thread. Results are collated in gate order.
/// The returned data is pure — the caller decides what to do with the traces.
pub fn run_with_traces() -> (BootstrapResult, Vec<BootstrapTrace>) {
    let hw = hardware::detect();

    // Spawn all 6 gates concurrently
    let h0 = std::thread::spawn(|| {
        let prov = platform::detect();
        let status = prov.check_package_manager();
        let ok = status.is_ready();
        let trace = BootstrapTrace {
            id:            util::next_trace_id(),
            ts:            util::now_iso(),
            action_type:   types::TraceAction::Check,
            step:          "package_manager".to_string(),
            desc:          "Check system package manager".to_string(),
            cmd:           "package_manager_check".to_string(),
            exit:          Some(if ok { 0 } else { 1 }),
            out:           status.version.clone().unwrap_or_default(),
            err:           match &status.state {
                               types::ComponentState::Failed { error } => error.clone(),
                               _ => String::new(),
                           },
            ms:            0,
            ok,
            fix_attempted: false,
            fix_approach:  None,
            fix_ok:        None,
        };
        (status, vec![trace])
    });

    let h1 = std::thread::spawn(|| {
        util::check_binary_and_service_traced("postgresql", "postgres", "--version", POSTGRES_PORT)
    });
    let h2 = std::thread::spawn(|| {
        util::check_binary_and_service_traced("ollama", "ollama", "--version", OLLAMA_PORT)
    });
    let h3 = std::thread::spawn(|| {
        util::check_binary_traced("sensei", "sensei", "--version")
    });
    let h4 = std::thread::spawn(database::check_traced);
    let h5 = std::thread::spawn(|| util::check_service_traced("daemon", daemon_port()));

    // Collate in gate order
    let (s0, t0) = h0.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("package_manager", "thread panicked"), vec![])
    });
    let (s1, t1) = h1.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("postgresql", "thread panicked"), vec![])
    });
    let (s2, t2) = h2.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("ollama", "thread panicked"), vec![])
    });
    let (s3, t3) = h3.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("sensei", "thread panicked"), vec![])
    });
    let (s4, t4) = h4.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("database", "thread panicked"), vec![])
    });
    let (s5, t5) = h5.join().unwrap_or_else(|_| {
        (ComponentStatus::failed("daemon", "thread panicked"), vec![])
    });

    let components = vec![s0, s1, s2, s3, s4, s5];
    let mut all_traces = Vec::new();
    for t in [t0, t1, t2, t3, t4, t5] {
        all_traces.extend(t);
    }

    (BootstrapResult::from_checks(components, hw), all_traces)
}

/// Run the full bootstrap check — all phases, returns composite result.
pub fn run() -> BootstrapResult {
    let hw = hardware::detect();
    let prov = platform::detect();

    let components = vec![
        // Gate 一: package manager
        prov.check_package_manager(),
        // Gate 二: PostgreSQL (binary + service combined)
        util::check_binary_and_service("postgresql", "postgres", "--version", POSTGRES_PORT),
        // Gate 三: Ollama (binary + service combined)
        util::check_binary_and_service("ollama", "ollama", "--version", OLLAMA_PORT),
        // Gate 四: Sensei CLI (binary only — no service)
        util::check_binary("sensei", "sensei", "--version"),
        // Gate 五: Database
        database::check(),
        // Gate 六: Daemon (service only — binary checked via sensei gate)
        util::check_service("daemon", daemon_port()),
    ];

    BootstrapResult::from_checks(components, hw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_with_traces_returns_traces() {
        let (result, traces) = run_with_traces();
        // Should have at least one trace per gate (6 gates minimum)
        assert!(
            traces.len() >= 6,
            "expected at least 6 traces, got {}",
            traces.len()
        );
        assert!(!result.components.is_empty());
        // All traces have non-empty step and id
        for t in &traces {
            assert!(!t.step.is_empty(), "trace step should not be empty");
            assert!(!t.id.is_empty(), "trace id should not be empty");
        }
    }

    #[test]
    fn run_does_not_panic() {
        // The full bootstrap check should not panic regardless of system state.
        // Individual components may be failed/missing, but the run should complete.
        let result = run();
        assert!(!result.components.is_empty(), "should check at least one component");
        assert!(result.components.len() >= 5, "should check multiple components");
    }

    #[test]
    fn port_constants() {
        assert_eq!(config::DAEMON_PORT, 7744);
        assert_eq!(OLLAMA_PORT, 11434);
        assert_eq!(POSTGRES_PORT, 5432);
    }

    #[test]
    fn provider_returns_valid_platform() {
        let prov = provider();
        let p = prov.platform();
        assert!(
            matches!(p, platform::Platform::MacOS | platform::Platform::Linux | platform::Platform::Windows),
            "provider() should return a known platform"
        );
    }

    #[test]
    fn check_and_fix_with_context_all_ready_returns_all_ok() {
        use std::sync::Arc;
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
}
