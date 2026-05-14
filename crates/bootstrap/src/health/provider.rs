//! PlatformProvider — Template Method trait.
//!
//! Required methods (per-platform): platform, package_manager_id,
//! package_manager_checker, checker_for, resolvers, default_remedy.
//!
//! Default methods (Template Method, shared orchestration):
//!   * check  — runs every checker once, returns validated HealthPayload.
//!   * resolve — walks resolvers, finds covered failures, runs each
//!               resolver once with all its targets, re-checks, emits
//!               HealthEvents, returns terminal HealthPayload.
//!
//! IMPORTANT: every impl of `checker_for` MUST use exhaustive `match id {...}`
//! with no `_ =>` catchall, so adding a new ComponentId is a compile error
//! across every platform until each handles it.

use super::types::*;
use super::checker::Checker;
use super::resolver::{Resolver, ResolveOutcome};
use super::graph::dependency_specs;
use super::ids::{component_id_str, package_manager_id_str, parse_component_id};

pub trait PlatformProvider: Send + Sync {
    // ── Required (per-platform) ──────────────────────────────────────────
    fn platform(&self) -> Platform;
    fn package_manager_id(&self) -> PackageManagerId;
    fn package_manager_checker(&self) -> Box<dyn Checker>;
    fn checker_for(&self, id: ComponentId) -> Box<dyn Checker>;
    fn resolvers(&self) -> Vec<Box<dyn Resolver>>;
    fn default_remedy(&self) -> Remedy;

    // ── Default Template Methods ─────────────────────────────────────────

    /// Run every checker once. Returns a validated HealthPayload.
    /// Override only if your platform has a fundamentally different probe shape.
    fn check(&self, app_version: &str) -> HealthPayload {
        let pm = {
            let pm_id = self.package_manager_id();
            let (label, note) = match pm_id {
                PackageManagerId::Homebrew => ("Homebrew", "which brew"),
                PackageManagerId::Winget   => ("winget",   "winget --version"),
            };
            let outcome = self.package_manager_checker().check();
            Component {
                id:      package_manager_id_str(pm_id).to_string(),
                label:   label.to_string(),
                note:    Some(note.to_string()),
                status:  outcome.status,
                version: outcome.version,
                detail:  outcome.detail,
            }
        };
        let components: Vec<Component> = dependency_specs().iter().map(|spec| {
            let outcome = self.checker_for(spec.id).check();
            Component {
                id:      component_id_str(spec.id).to_string(),
                label:   spec.label.to_string(),
                note:    spec.note.map(str::to_string),
                status:  outcome.status,
                version: outcome.version,
                detail:  outcome.detail,
            }
        }).collect();

        let status = overall_status(&pm, &components);
        let remedy = if matches!(status, HealthStatus::NeedsAction) {
            Some(self.default_remedy())
        } else {
            None
        };

        let payload = HealthPayload {
            version: app_version.to_string(),
            uptime_seconds: 0,
            platform: self.platform(),
            package_manager: pm,
            components,
            status,
            remedy,
        };
        payload.validate().expect("PlatformProvider::check produced an invalid payload");
        payload
    }

    /// Walk the resolvers, find covered failed deps, run each once, re-check.
    /// Emits HealthEvent values; returns terminal payload.
    /// Note: uses `&dyn Fn` (not generic F) so the trait remains dyn-compatible.
    fn resolve(&self, current: &HealthPayload, app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
        emit(HealthEvent::Phase { phase: HealthStatus::Resolving });

        let failed: Vec<ComponentId> = current.components.iter()
            .filter(|c| c.status == ComponentStatus::Failed)
            .filter_map(|c| parse_component_id(&c.id))
            .collect();

        let resolvers = self.resolvers();
        let mut applied_remedy: Option<Remedy> = None;

        for resolver in &resolvers {
            let targets: Vec<ComponentId> = resolver.resolves().iter().copied()
                .filter(|id| failed.contains(id))
                .collect();
            if targets.is_empty() { continue; }

            // Mark each target as Installing so the UI ledger animates.
            for tid in &targets {
                emit(HealthEvent::Component {
                    id: component_id_str(*tid).to_string(),
                    patch: ComponentPatch {
                        status: Some(ComponentStatus::Installing),
                        ..Default::default()
                    },
                });
            }

            match resolver.resolve(&targets) {
                ResolveOutcome::Resolved => {
                    for tid in &targets {
                        let outcome = self.checker_for(*tid).check();
                        emit(HealthEvent::Component {
                            id: component_id_str(*tid).to_string(),
                            patch: ComponentPatch {
                                status:  Some(outcome.status),
                                version: Some(outcome.version),
                                detail:  Some(outcome.detail),
                                ..Default::default()
                            },
                        });
                    }
                }
                ResolveOutcome::NeedsHumanAction(remedy) => {
                    applied_remedy.get_or_insert(remedy.clone());
                    emit(HealthEvent::Remedy { remedy });
                }
            }
        }

        // Final re-check builds the terminal payload.
        let mut terminal = self.check(app_version);
        if terminal.status == HealthStatus::NeedsAction {
            // Prefer the resolver-supplied remedy over the generic default.
            if let Some(r) = applied_remedy {
                terminal.remedy = Some(r);
            }
        } else {
            terminal.remedy = None;
        }
        terminal.validate().expect("PlatformProvider::resolve produced an invalid terminal payload");
        emit(HealthEvent::Report { payload: terminal.clone() });
        terminal
    }
}

fn overall_status(pm: &Component, components: &[Component]) -> HealthStatus {
    let all_ready = pm.status == ComponentStatus::Ready
        && components.iter().all(|c| c.status == ComponentStatus::Ready);
    if all_ready { return HealthStatus::Ok; }
    let any_failed = pm.status == ComponentStatus::Failed
        || components.iter().any(|c| c.status == ComponentStatus::Failed);
    if any_failed { return HealthStatus::NeedsAction; }
    HealthStatus::Checking
}

/// Detect the current platform and return its provider.
/// IMPORTANT: C4 (macOS) / C5 (Windows) replace this body. For C1, it panics
/// so callers don't accidentally use it before the platform impls are in.
pub fn detect_provider() -> Box<dyn PlatformProvider> {
    unimplemented!("detect_provider() is wired in Tasks C4/C5. Tests use MockProvider directly.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::checker::{Checker, CheckOutcome};
    use super::super::resolver::{Resolver, ResolveOutcome};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // ── Mock building blocks ─────────────────────────────────────────────

    struct StubChecker(CheckOutcome);
    impl Checker for StubChecker {
        fn check(&self) -> CheckOutcome { self.0.clone() }
    }

    struct StubResolver {
        id:      &'static str,
        targets: &'static [ComponentId],
        outcome: ResolveOutcome,
        calls:   Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl Resolver for StubResolver {
        fn id(&self) -> &'static str { self.id }
        fn resolves(&self) -> &'static [ComponentId] { self.targets }
        fn resolve(&self, t: &[ComponentId]) -> ResolveOutcome {
            self.calls.lock().unwrap().push(t.to_vec());
            self.outcome.clone()
        }
    }

    /// Simple provider whose checkers return preconfigured outcomes.
    /// Used by `check` tests — does NOT need to return inspectable resolvers,
    /// so the `resolvers()` here returns an empty Vec.
    struct CheckOnlyMock {
        pm_id:      PackageManagerId,
        outcomes:   HashMap<ComponentId, CheckOutcome>,
        pm_outcome: CheckOutcome,
    }
    impl PlatformProvider for CheckOnlyMock {
        fn platform(&self) -> Platform {
            if self.pm_id == PackageManagerId::Winget { Platform::Windows } else { Platform::Macos }
        }
        fn package_manager_id(&self) -> PackageManagerId { self.pm_id }
        fn package_manager_checker(&self) -> Box<dyn Checker> {
            Box::new(StubChecker(self.pm_outcome.clone()))
        }
        fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
            Box::new(StubChecker(self.outcomes.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("no stub for id"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> { vec![] }
        fn default_remedy(&self) -> Remedy {
            Remedy { message: "m".into(), script: "s".into(), url: None }
        }
    }

    /// Provider where `resolvers()` returns a single StubResolver. The
    /// `outcomes` HashMap is wrapped in a Mutex so tests can mutate it
    /// between the initial check() and the resolve() to simulate a real
    /// fix taking effect.
    struct ResolveMock {
        pm_outcome:        CheckOutcome,
        outcomes:          Mutex<HashMap<ComponentId, CheckOutcome>>,
        resolver_id:       &'static str,
        resolver_targets:  &'static [ComponentId],
        resolver_outcome:  ResolveOutcome,
        calls:             Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl PlatformProvider for ResolveMock {
        fn platform(&self) -> Platform { Platform::Macos }
        fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
        fn package_manager_checker(&self) -> Box<dyn Checker> {
            Box::new(StubChecker(self.pm_outcome.clone()))
        }
        fn checker_for(&self, id: ComponentId) -> Box<dyn Checker> {
            let m = self.outcomes.lock().unwrap();
            Box::new(StubChecker(m.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("none"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            vec![Box::new(StubResolver {
                id: self.resolver_id,
                targets: self.resolver_targets,
                outcome: self.resolver_outcome.clone(),
                calls: self.calls.clone(),
            })]
        }
        fn default_remedy(&self) -> Remedy {
            Remedy { message: "m".into(), script: "s".into(), url: None }
        }
    }

    fn ready_outcomes() -> HashMap<ComponentId, CheckOutcome> {
        let mut m = HashMap::new();
        for id in [ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei,
                   ComponentId::Database, ComponentId::Daemon] {
            m.insert(id, CheckOutcome::ready("1.0"));
        }
        m
    }

    fn one_failed(failed: ComponentId, detail: &str) -> HashMap<ComponentId, CheckOutcome> {
        let mut m = ready_outcomes();
        m.insert(failed, CheckOutcome::failed(detail));
        m
    }

    // ── check() tests (5) ────────────────────────────────────────────────

    #[test]
    fn check_all_ready_yields_ok() {
        let p = CheckOnlyMock {
            pm_id: PackageManagerId::Homebrew,
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(),
        };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::Ok);
        assert!(payload.remedy.is_none());
        assert_eq!(payload.components.len(), 5);
        for c in &payload.components {
            assert_eq!(c.status, ComponentStatus::Ready);
        }
    }

    #[test]
    fn check_with_one_failed_yields_needs_action_with_default_remedy() {
        let p = CheckOnlyMock {
            pm_id: PackageManagerId::Homebrew,
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: one_failed(ComponentId::Postgres, "pg_isready failed"),
        };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::NeedsAction);
        assert!(payload.remedy.is_some());
        let pg = payload.components.iter().find(|c| c.id == "postgres").unwrap();
        assert_eq!(pg.status, ComponentStatus::Failed);
        assert_eq!(pg.detail.as_deref(), Some("pg_isready failed"));
    }

    #[test]
    fn check_pm_failed_with_components_ready_yields_needs_action() {
        let p = CheckOnlyMock {
            pm_id: PackageManagerId::Homebrew,
            pm_outcome: CheckOutcome::failed("brew missing"),
            outcomes: ready_outcomes(),
        };
        let payload = p.check("0.0.0-test");
        assert_eq!(payload.status, HealthStatus::NeedsAction);
        assert_eq!(payload.package_manager.status, ComponentStatus::Failed);
    }

    #[test]
    fn check_payload_validates() {
        let p = CheckOnlyMock {
            pm_id: PackageManagerId::Homebrew,
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(),
        };
        let payload = p.check("0.0.0-test");
        payload.validate().expect("must validate");
    }

    #[test]
    fn check_components_in_canonical_order() {
        let p = CheckOnlyMock {
            pm_id: PackageManagerId::Homebrew,
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: ready_outcomes(),
        };
        let payload = p.check("0.0.0-test");
        let ids: Vec<&str> = payload.components.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["postgres","ollama","sensei","database","daemon"]);
    }

    // ── resolve() tests (4) ──────────────────────────────────────────────

    #[test]
    fn resolve_runs_covering_resolver_once_with_all_targets() {
        const TARGETS: &[ComponentId] = &[ComponentId::Postgres, ComponentId::Ollama, ComponentId::Sensei];
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("x"));
        outcomes.insert(ComponentId::Ollama,   CheckOutcome::failed("x"));
        outcomes.insert(ComponentId::Sensei,   CheckOutcome::failed("x"));
        let calls = Arc::new(Mutex::new(Vec::new()));

        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "brew_bundle",
            resolver_targets: TARGETS,
            resolver_outcome: ResolveOutcome::Resolved,
            calls: calls.clone(),
        };
        let current = p.check("0.0.0-test");
        assert_eq!(current.status, HealthStatus::NeedsAction);

        // Simulate the resolver succeeding: subsequent checker calls see Ready.
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Postgres, CheckOutcome::ready("16.3"));
            m.insert(ComponentId::Ollama,   CheckOutcome::ready("0.4"));
            m.insert(ComponentId::Sensei,   CheckOutcome::ready_no_version());
        }

        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", &|e| ec.lock().unwrap().push(e));

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1, "resolver invoked once");
        assert_eq!(recorded[0].len(), 3, "with all three targets in one call");
        assert_eq!(terminal.status, HealthStatus::Ok);

        let evs = events.lock().unwrap();
        assert!(matches!(evs.first(), Some(HealthEvent::Phase { phase: HealthStatus::Resolving })));
        assert!(matches!(evs.last(),  Some(HealthEvent::Report { .. })));
        let installing_count = evs.iter().filter(|e| matches!(
            e, HealthEvent::Component { patch: ComponentPatch { status: Some(ComponentStatus::Installing), .. }, .. }
        )).count();
        assert_eq!(installing_count, 3, "installing emitted per-target");
    }

    #[test]
    fn resolve_skips_resolvers_with_no_targets() {
        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(ready_outcomes()),
            resolver_id: "brew_bundle",
            resolver_targets: &[ComponentId::Postgres],
            resolver_outcome: ResolveOutcome::Resolved,
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        let p_calls = p.calls.clone();
        let _ = p.resolve(&current, "0.0.0-test", &|_| {});
        assert!(p_calls.lock().unwrap().is_empty(), "resolver not called when no targets");
    }

    #[test]
    fn resolve_needs_human_action_sets_remedy() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("x"));
        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "brew_bundle",
            resolver_targets: &[ComponentId::Postgres],
            resolver_outcome: ResolveOutcome::NeedsHumanAction(Remedy {
                message: "run this".into(),
                script: "brew install postgres".into(),
                url: None,
            }),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", &|e| ec.lock().unwrap().push(e));
        assert_eq!(terminal.status, HealthStatus::NeedsAction);
        assert_eq!(terminal.remedy.as_ref().unwrap().script, "brew install postgres");
        assert!(events.lock().unwrap().iter().any(|e| matches!(e, HealthEvent::Remedy { .. })));
    }

    #[test]
    fn resolve_terminal_payload_validates() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Daemon, CheckOutcome::failed("not running"));
        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "daemon_start",
            resolver_targets: &[ComponentId::Daemon],
            resolver_outcome: ResolveOutcome::Resolved,
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Daemon, CheckOutcome::ready_no_version());
        }
        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});
        terminal.validate().expect("terminal must validate");
        assert_eq!(terminal.status, HealthStatus::Ok);
    }
}
