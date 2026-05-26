//! Generic "service-style dependency" cascade.
//!
//! A service-style dependency (postgres, ollama, redis, …) is one that:
//!   * is installable via a brew formula,
//!   * runs as a long-lived service managed by launchd via `brew services`,
//!   * is "ready" when its TCP port responds.
//!
//! When such a service is failing, the *checker* has already established
//! that the port is closed. From there, the resolver tries to bring it up
//! using progressively heavier-weight stages — only escalating when the
//! cheaper stage fails. Each stage emits a structured tracing event so the
//! cascade is visible in `sensei doctor`.
//!
//! Stage 1 — `brew services start <service>`
//!   Cheapest. Works when the formula is already installed and the service
//!   is registered with brew, just stopped. ~1 second.
//!
//! Stage 2 — direct binary launch (optional per dependency)
//!   When `brew services` doesn't recognize the service (formula not
//!   installed, or installed but service not registered), try invoking the
//!   binary directly with whatever command starts the daemon. Skipped for
//!   deps that need complex args (e.g. postgres needs a data dir + user).
//!
//! Stage 3 — `brew install <formula> && brew services start <service>`
//!   Heaviest. Used when neither earlier stage worked. Delegates to the
//!   existing `brew_install_and_start_to_outcome` which already handles
//!   the various brew failure modes (link conflict, tap missing, etc.).

use std::process::Command;

use crate::health::resolver::ResolveOutcome;
use crate::health::resolvers::brew_helpers::{
    brew_install_and_start_to_outcome, brew_services_start, BrewError,
};

/// Declarative specification for a service-style dependency. One instance
/// per dep — keep them `const` where possible.
pub struct ServiceCascadeSpec {
    /// Brew formula name, e.g. `"postgresql@17"`.
    pub formula:        &'static str,
    /// Brew service name (often identical to formula).
    pub service:        &'static str,
    /// Extra args for `brew install`, e.g. `&["--HEAD"]`.
    pub install_args:   &'static [&'static str],
    /// Optional stage-2 launcher. Returns a `Command` configured to
    /// daemonize the service. `None` skips stage 2 entirely (right for
    /// services that need complex args we can't reliably supply).
    pub direct_launcher: Option<fn() -> Command>,
}

/// Run the staged cascade for a service-style dependency.
pub fn resolve_service_cascade(spec: &ServiceCascadeSpec) -> ResolveOutcome {
    // ── Stage 1 ── brew services start (formula already present case) ────
    tracing::info!(
        cascade_stage = 1,
        service = spec.service,
        "stage 1: brew services start"
    );
    match brew_services_start(spec.service) {
        Ok(()) => {
            tracing::info!(cascade_stage = 1, service = spec.service, "stage 1: ok");
            return ResolveOutcome::Resolved;
        }
        // Brew is missing entirely — no later stage can recover. Surface
        // the homebrew-install remedy that brew_helpers already builds.
        Err(BrewError::BrewNotFound) => {
            tracing::warn!(cascade_stage = 1, "brew not on PATH — escalating to install path");
            // Fall through to stage 3 which handles the BrewNotFound case
            // via brew_install_to_outcome's BrewError::BrewNotFound branch.
        }
        Err(e) => {
            tracing::info!(
                cascade_stage = 1,
                service = spec.service,
                error = ?e,
                "stage 1 failed — service likely not registered with brew yet"
            );
        }
    }

    // ── Stage 2 ── direct binary launch ──────────────────────────────────
    if let Some(make_cmd) = spec.direct_launcher {
        let mut cmd = make_cmd();
        let program = cmd.get_program().to_string_lossy().to_string();
        tracing::info!(cascade_stage = 2, command = %program, "stage 2: direct launch");
        // `spawn` returns immediately; the child becomes our orphaned
        // descendant. We do NOT wait — these daemons are meant to outlive
        // the resolver. The orchestrator's post-resolve re-check is what
        // confirms the port actually binds.
        match cmd.spawn() {
            Ok(child) => {
                tracing::info!(cascade_stage = 2, pid = child.id(), "stage 2: spawned");
                return ResolveOutcome::Resolved;
            }
            Err(e) => {
                tracing::warn!(cascade_stage = 2, error = %e, "stage 2: spawn failed");
            }
        }
    } else {
        tracing::debug!(cascade_stage = 2, "stage 2: skipped (no direct launcher)");
    }

    // ── Stage 3 ── brew install + start (heaviest) ───────────────────────
    tracing::info!(cascade_stage = 3, formula = spec.formula, "stage 3: brew install + start");
    brew_install_and_start_to_outcome(spec.formula, spec.install_args, spec.service)
}

#[cfg(test)]
mod tests {
    use super::*;

    // We can't unit-test the cascade end-to-end without shelling out to
    // brew (which test environments don't have). These tests cover the
    // spec construction shape and stage 2's optional-ness.

    #[test]
    fn spec_can_be_const_constructed() {
        const _S: ServiceCascadeSpec = ServiceCascadeSpec {
            formula: "ollama",
            service: "ollama",
            install_args: &[],
            direct_launcher: None,
        };
    }

    #[test]
    fn stage_2_is_optional() {
        let with: ServiceCascadeSpec = ServiceCascadeSpec {
            formula: "ollama",
            service: "ollama",
            install_args: &[],
            direct_launcher: Some(|| Command::new("ollama")),
        };
        let without: ServiceCascadeSpec = ServiceCascadeSpec {
            formula: "postgresql@17",
            service: "postgresql@17",
            install_args: &[],
            direct_launcher: None,
        };
        assert!(with.direct_launcher.is_some());
        assert!(without.direct_launcher.is_none());
    }
}
