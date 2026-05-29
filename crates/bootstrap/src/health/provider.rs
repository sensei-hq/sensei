//! PlatformProvider — Template Method trait.
//!
//! Required methods (per-platform): platform, package_manager_id,
//! package_manager_checker, checker_for, resolvers, default_remedy.
//!
//! Default methods (Template Method, shared orchestration):
//!   * check  — runs every checker once, returns validated HealthPayload.
//!   * resolve — walks resolvers, finds covered failures, runs each
//!     resolver once with all its targets, re-checks, emits
//!     HealthEvents, returns terminal HealthPayload.
//!
//! IMPORTANT: every impl of `checker_for` MUST use exhaustive `match id {...}`
//! with no `_ =>` catchall, so adding a new ComponentId is a compile error
//! across every platform until each handles it.

use super::types::*;
use super::checker::Checker;
use super::resolver::{Resolver, ResolveOutcome};
use super::graph::{dependency_specs, spec_for};
use super::ids::{component_id_str, package_manager_id_str, parse_component_id};

pub trait PlatformProvider: Send + Sync {
    // ── Required (per-platform) ──────────────────────────────────────────
    fn platform(&self) -> Platform;
    fn package_manager_id(&self) -> PackageManagerId;
    fn package_manager_checker(&self) -> Box<dyn Checker>;
    /// Probe for `id`. `retry=false` is the initial-check path: fast probes,
    /// closed ports report failed immediately. `retry=true` is the
    /// post-resolver recheck path: patient probes that loop until the
    /// service finishes binding (brew services start, senseid start).
    fn checker_for(&self, id: ComponentId, retry: bool) -> Box<dyn Checker>;
    fn resolvers(&self) -> Vec<Box<dyn Resolver>>;
    fn default_remedy(&self) -> Remedy;

    // ── Default Template Methods ─────────────────────────────────────────

    /// Run every checker once. Returns a validated HealthPayload.
    /// No events are emitted — callers that want per-probe streaming
    /// should use `check_streaming` instead.
    fn check(&self, app_version: &str) -> HealthPayload {
        self.check_streaming(app_version, &|_| {})
    }

    /// Same as `check` but emits a `Component` event after each probe
    /// returns. The UI consumes these to update component rows
    /// incrementally rather than waiting for the full check phase to
    /// complete (5-13 s when remote brew shell-outs are slow).
    ///
    /// Probes still run sequentially — parallelising them is a separate
    /// concern (each probe is `&dyn Checker`, not Send-bound). The win
    /// here is purely UI-visible: each result lands the moment its
    /// probe finishes, rather than all-at-once at the end.
    ///
    /// Override only if your platform has a fundamentally different
    /// probe shape (e.g. a single batched daemon call).
    fn check_streaming(
        &self,
        app_version: &str,
        emit: &dyn Fn(HealthEvent),
    ) -> HealthPayload {
        let pm = {
            let pm_id = self.package_manager_id();
            let (label, note) = match pm_id {
                PackageManagerId::Homebrew => ("Homebrew", "which brew"),
                PackageManagerId::Winget   => ("winget",   "winget --version"),
            };
            let outcome = self.package_manager_checker().check();
            let comp = Component {
                id:      package_manager_id_str(pm_id).to_string(),
                label:   label.to_string(),
                note:    Some(note.to_string()),
                status:  outcome.status,
                version: outcome.version,
                detail:  outcome.detail,
                // Package managers are plain installs (brew, winget) — no
                // start/setup distinction needed.
                installing_verb: "installing".to_string(),
            };
            emit(HealthEvent::Component {
                id: comp.id.clone(),
                patch: ComponentPatch {
                    status: Some(comp.status),
                    version: Some(comp.version.clone()),
                    detail: Some(comp.detail.clone()),
                    ..Default::default()
                },
            });
            comp
        };

        let mut components: Vec<Component> = Vec::with_capacity(5);
        for spec in dependency_specs().iter() {
            let outcome = self.checker_for(spec.id, false).check();
            let comp = Component {
                id:      component_id_str(spec.id).to_string(),
                label:   spec.label.to_string(),
                note:    spec.note.map(str::to_string),
                status:  outcome.status,
                version: outcome.version,
                detail:  outcome.detail,
                installing_verb: spec.installing_verb.to_string(),
            };
            emit(HealthEvent::Component {
                id: comp.id.clone(),
                patch: ComponentPatch {
                    status: Some(comp.status),
                    version: Some(comp.version.clone()),
                    detail: Some(comp.detail.clone()),
                    ..Default::default()
                },
            });
            components.push(comp);
        }

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
    ///
    /// Single-pass design: the walk records per-component "walk remedies"
    /// (the most context-rich fix the resolver could surface live). After
    /// the final check, a single post-pass derives the terminal consolidated
    /// remedy from `failed_in_terminal` plus the walk-remedy map, the
    /// dependency graph, and each resolver's `fallback_remedy()`. No
    /// stale-drop or retroactive step is needed: components that recovered
    /// aren't in `failed_in_terminal`, and components without a walk-remedy
    /// fall through to `fallback_remedy()` in the same loop.
    ///
    /// Note: uses `&dyn Fn` (not generic F) so the trait remains dyn-compatible.
    fn resolve(&self, current: &HealthPayload, app_version: &str, emit: &dyn Fn(HealthEvent)) -> HealthPayload {
        emit(HealthEvent::Phase { phase: HealthStatus::Resolving });

        // Package-manager short-circuit. The dep graph's `depends_on` only
        // talks about ComponentId, which doesn't include the PackageManager
        // (homebrew/winget). If brew is missing, every component-level
        // resolver would attempt `brew install …`, fail with BrewNotFound,
        // and surface a postgres-flavoured "brew install postgresql@17"
        // remedy attributed to the postgres component — exactly the
        // postgres-before-homebrew confusion we hit on the first prod
        // install. Skip the resolver walk entirely and surface a single,
        // unambiguous brew-install remedy. On the next /resolve (after the
        // user installs brew) the check phase reports PM ready and we fall
        // back into the normal path.
        if current.package_manager.status == ComponentStatus::Failed {
            let remedy = crate::health::resolvers::brew_helpers::homebrew_install_remedy();
            tracing::warn!(
                "package manager (brew) missing — short-circuiting resolver walk and surfacing homebrew-install remedy"
            );
            emit(HealthEvent::Remedy { remedy: remedy.clone() });
            let mut terminal = self.check(app_version);
            terminal.remedy = Some(remedy);
            terminal.validate().expect("PlatformProvider::resolve produced an invalid terminal payload");
            emit(HealthEvent::Report { payload: terminal.clone() });
            return terminal;
        }

        // `now_failing` is the live set of components whose state we have not
        // yet seen recover during this pass. We start it from the initial
        // check's failed list, then remove components as resolvers verify
        // they're back. Downstream resolvers (Database, Daemon) use it as a
        // dependency gate: a resolver whose target's `depends_on` includes
        // anything in `now_failing` is skipped — its run would just produce
        // a noise remedy since the real fix is the upstream that's still
        // broken. Brew is the bottleneck either way (no parallel installs),
        // so this just trims the wasted attempts.
        let mut now_failing: Vec<ComponentId> = current.components.iter()
            .filter(|c| c.status == ComponentStatus::Failed)
            .filter_map(|c| parse_component_id(&c.id))
            .collect();
        tracing::info!(failed = ?now_failing, "resolve phase: walking resolvers");

        let resolvers = self.resolvers();
        // Per-component remedy surfaced during the walk. `NeedsHumanAction`
        // returns a context-specific remedy; `Resolved`-but-recheck-failed
        // returns the resolver's `fallback_remedy()`. The post-pass below
        // consumes this map. We use a Vec (rather than HashMap) to preserve
        // insertion order so the consolidated script reads top-to-bottom
        // in the same order as the resolver walk.
        let mut walk_remedies: Vec<(ComponentId, Remedy)> = Vec::new();

        for resolver in &resolvers {
            let targets: Vec<ComponentId> = resolver.resolves().iter().copied()
                .filter(|id| now_failing.contains(id))
                .collect();
            if targets.is_empty() {
                tracing::debug!(resolver = resolver.id(), "skipping (no failed targets)");
                continue;
            }

            // Dependency gate: if any target depends on a component that's
            // still failing in this pass, defer. The next /resolve
            // invocation (after the user runs the upstream remedy) will
            // pick it up.
            let unmet_deps: Vec<ComponentId> = targets.iter()
                .flat_map(|tid| spec_for(*tid).depends_on.iter().copied())
                .filter(|dep| now_failing.contains(dep))
                .collect();
            if !unmet_deps.is_empty() {
                tracing::info!(
                    resolver = resolver.id(),
                    targets = ?targets,
                    unmet_deps = ?unmet_deps,
                    "skipping: dependencies still failing — retry after upstream is resolved"
                );
                continue;
            }

            tracing::info!(resolver = resolver.id(), targets = ?targets, "running resolver");

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
                    tracing::info!(resolver = resolver.id(), "resolver returned Resolved — re-checking targets");
                    // Re-check each target so we don't trust a "Resolved"
                    // verdict that brew/etc handed back without confirming the
                    // component is actually up. If any target is still
                    // Failed, record the resolver's `fallback_remedy()` as
                    // its walk remedy — the post-pass will surface it (or
                    // drop it, if the component recovers by the final check).
                    let mut first_still_failed: Option<ComponentId> = None;
                    for tid in &targets {
                        let outcome = self.checker_for(*tid, true).check();
                        if outcome.status == ComponentStatus::Failed {
                            if first_still_failed.is_none() { first_still_failed = Some(*tid); }
                        } else {
                            // Target recovered — drop it from now_failing
                            // so dependents can clear their dependency gate
                            // later in this same pass.
                            now_failing.retain(|id| id != tid);
                        }
                        tracing::info!(
                            resolver = resolver.id(),
                            component = component_id_str(*tid),
                            recheck_status = ?outcome.status,
                            detail = ?outcome.detail,
                            "post-resolve re-check verdict"
                        );
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
                    if let Some(attributed) = first_still_failed {
                        let remedy = resolver.fallback_remedy();
                        tracing::warn!(resolver = resolver.id(), remedy_script = %remedy.script, "post-resolve re-check still failed — surfacing fallback_remedy");
                        walk_remedies.push((attributed, remedy.clone()));
                        emit(HealthEvent::Remedy { remedy });
                    } else {
                        tracing::info!(resolver = resolver.id(), "post-resolve re-check ok");
                    }
                }
                ResolveOutcome::NeedsHumanAction(remedy) => {
                    tracing::warn!(resolver = resolver.id(), remedy_script = %remedy.script, "resolver returned NeedsHumanAction");
                    let attributed = targets[0];
                    walk_remedies.push((attributed, remedy.clone()));
                    emit(HealthEvent::Remedy { remedy });
                }
            }
        }

        // Final re-check builds the terminal payload.
        let mut terminal = self.check(app_version);
        if terminal.status == HealthStatus::NeedsAction {
            terminal.remedy = derive_terminal_remedy(&terminal, &walk_remedies, &resolvers);
        } else {
            terminal.remedy = None;
        }
        terminal.validate().expect("PlatformProvider::resolve produced an invalid terminal payload");
        emit(HealthEvent::Report { payload: terminal.clone() });
        terminal
    }
}

/// Post-pass: derive the consolidated terminal remedy from the final
/// payload, the walk-recorded remedies, and each resolver's fallback.
///
/// For every component STILL failing in the terminal:
///   * If a dependency of it is ALSO failing, skip — upstream's remedy is
///     the real fix, surfacing a downstream one would be the noise the
///     walk-time dependency gate already suppresses.
///   * Else use the walk_remedy for that component (most context-rich), or
///     fall back to the resolver's `fallback_remedy()`.
///
/// Returns `None` when no per-component remedy can be attached (caller
/// will let the generic `default_remedy()` from `check()` stand).
fn derive_terminal_remedy(
    terminal: &HealthPayload,
    walk_remedies: &[(ComponentId, Remedy)],
    resolvers: &[Box<dyn Resolver>],
) -> Option<Remedy> {
    let failed_in_terminal: Vec<ComponentId> = terminal.components.iter()
        .filter(|c| c.status == ComponentStatus::Failed)
        .filter_map(|c| parse_component_id(&c.id))
        .collect();

    let mut out: Vec<(ComponentId, Remedy)> = Vec::new();
    for tid in &failed_in_terminal {
        if spec_for(*tid).depends_on.iter().any(|dep| failed_in_terminal.contains(dep)) {
            tracing::debug!(component = component_id_str(*tid),
                "skipping remedy: upstream dep also failing");
            continue;
        }
        let remedy = walk_remedies.iter()
            .find(|(id, _)| id == tid)
            .map(|(_, r)| r.clone())
            .or_else(|| {
                resolvers.iter()
                    .find(|r| r.resolves().contains(tid))
                    .map(|r| {
                        let fb = r.fallback_remedy();
                        tracing::warn!(
                            component = component_id_str(*tid),
                            resolver = r.id(),
                            remedy_script = %fb.script,
                            "no walk remedy captured — attaching fallback_remedy",
                        );
                        fb
                    })
            });
        if let Some(r) = remedy {
            out.push((*tid, r));
        }
    }
    if out.is_empty() { None } else { Some(consolidate_remedies(&out)) }
}

/// Merge per-component remedies into a single Remedy. For a single entry,
/// the input remedy is returned verbatim (no consolidation cruft). For
/// multiple entries, the script joins each entry's script with newlines —
/// so each line runs independently in a shell — and the message lists
/// each component's reason as a bullet.
///
/// Newline-joined (not `&&`-chained) on purpose: even if one component's
/// fix fails, the others still attempt to run. This matches the user's
/// expectation that a multi-component remedy is a "best-effort recovery
/// batch" rather than an all-or-nothing transaction.
pub(crate) fn consolidate_remedies(remedies: &[(ComponentId, Remedy)]) -> Remedy {
    if remedies.len() == 1 {
        return remedies[0].1.clone();
    }
    let bullets: Vec<String> = remedies.iter()
        .map(|(id, r)| format!("• {}: {}", component_id_str(*id), r.message))
        .collect();
    let scripts: Vec<&str> = remedies.iter()
        .map(|(_, r)| r.script.as_str())
        .collect();
    Remedy {
        message: format!(
            "{} components need attention:\n\n{}",
            remedies.len(),
            bullets.join("\n"),
        ),
        script: scripts.join("\n"),
        url: None,
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
    #[cfg(target_os = "windows")]
    { Box::new(super::platforms::windows::WindowsProvider) }
    #[cfg(not(target_os = "windows"))]
    { Box::new(super::platforms::macos::MacOSProvider) }
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

    /// Checker that returns a different `CheckOutcome` on each call by
    /// indexing into `seq`. Once the sequence is exhausted, the last value
    /// repeats forever. Use for tests that simulate state change between
    /// the initial check, the post-resolve re-check, and the final check.
    struct SequenceChecker {
        seq: Vec<CheckOutcome>,
        n:   Arc<Mutex<usize>>,
    }
    impl Checker for SequenceChecker {
        fn check(&self) -> CheckOutcome {
            let mut idx = self.n.lock().unwrap();
            let i = *idx;
            *idx += 1;
            self.seq.get(i).cloned()
                .unwrap_or_else(|| self.seq.last().unwrap().clone())
        }
    }

    struct StubResolver {
        id:               &'static str,
        targets:          &'static [ComponentId],
        outcome:          ResolveOutcome,
        fallback:         Remedy,
        calls:            Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl Resolver for StubResolver {
        fn id(&self) -> &'static str { self.id }
        fn resolves(&self) -> &'static [ComponentId] { self.targets }
        fn resolve(&self, t: &[ComponentId]) -> ResolveOutcome {
            self.calls.lock().unwrap().push(t.to_vec());
            self.outcome.clone()
        }
        fn fallback_remedy(&self) -> Remedy { self.fallback.clone() }
    }

    fn test_fallback() -> Remedy {
        Remedy { message: "stub fallback".into(), script: "stub script".into(), url: None }
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
        fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
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
        resolver_fallback: Remedy,
        calls:             Arc<Mutex<Vec<Vec<ComponentId>>>>,
    }
    impl PlatformProvider for ResolveMock {
        fn platform(&self) -> Platform { Platform::Macos }
        fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
        fn package_manager_checker(&self) -> Box<dyn Checker> {
            Box::new(StubChecker(self.pm_outcome.clone()))
        }
        fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
            let m = self.outcomes.lock().unwrap();
            Box::new(StubChecker(m.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("none"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            vec![Box::new(StubResolver {
                id: self.resolver_id,
                targets: self.resolver_targets,
                outcome: self.resolver_outcome.clone(),
                fallback: self.resolver_fallback.clone(),
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
            resolver_fallback: test_fallback(),
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
            resolver_fallback: test_fallback(),
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
            resolver_fallback: test_fallback(),
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
            resolver_fallback: test_fallback(),
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

    /// Regression: when a resolver returns `Resolved` but the post-resolve
    /// re-check still reports the target as `Failed` (e.g. brew install
    /// succeeded but `postgresql@17` is keg-only, or `brew services start
    /// ollama` raced the port-check), the orchestrator must surface the
    /// resolver's `fallback_remedy()` in the terminal payload instead of
    /// silently re-using `default_remedy()` from the final `check()`.
    #[test]
    fn resolve_resolved_but_recheck_failed_uses_resolver_fallback_remedy() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Ollama, CheckOutcome::failed("ollama not on PATH"));
        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "ollama_install",
            resolver_targets: &[ComponentId::Ollama],
            resolver_outcome: ResolveOutcome::Resolved,  // resolver claims success…
            resolver_fallback: Remedy {
                message: "Ollama didn't come up after install.".into(),
                script: "brew services restart ollama".into(),
                url: None,
            },
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        // …but the world still says ollama is failed on the re-check
        // (we deliberately do NOT swap the outcomes map to Ready here).
        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", &|e| ec.lock().unwrap().push(e));

        assert_eq!(terminal.status, HealthStatus::NeedsAction);
        let r = terminal.remedy.as_ref().expect("terminal must carry a remedy");
        assert_eq!(r.script, "brew services restart ollama",
            "terminal remedy must come from resolver.fallback_remedy(), not default_remedy()");
        let evs = events.lock().unwrap();
        assert!(evs.iter().any(|e| matches!(e, HealthEvent::Remedy { .. })),
            "a Remedy event should be emitted when post-check still failed");
    }

    // ── consolidate_remedies tests ──────────────────────────────────────

    fn r(msg: &str, script: &str) -> Remedy {
        Remedy { message: msg.into(), script: script.into(), url: None }
    }

    #[test]
    fn consolidate_single_remedy_passes_through_verbatim() {
        let r1 = r("just postgres", "brew install postgresql@17");
        let merged = consolidate_remedies(&[(ComponentId::Postgres, r1.clone())]);
        assert_eq!(merged.message, r1.message,
            "single-remedy consolidation must not wrap with N-components verbiage");
        assert_eq!(merged.script, r1.script);
    }

    #[test]
    fn consolidate_multiple_remedies_joins_scripts_with_newlines() {
        let pg = r(
            "PostgreSQL needs relinking.",
            "brew link --force --overwrite postgresql@17 && brew services restart postgresql@17",
        );
        let ol = r("Ollama needs restart.", "brew services restart ollama");
        let db = r("Database doesn't exist.", "createdb sensei_dev");
        let merged = consolidate_remedies(&[
            (ComponentId::Postgres, pg),
            (ComponentId::Ollama, ol),
            (ComponentId::Database, db),
        ]);
        // Script: newline-joined so each line runs independently.
        let expected_script = "\
brew link --force --overwrite postgresql@17 && brew services restart postgresql@17
brew services restart ollama
createdb sensei_dev";
        assert_eq!(merged.script, expected_script);
        // Message: each component appears as a bullet with its reason.
        assert!(merged.message.starts_with("3 components need attention:"),
            "message must lead with count, got: {}", merged.message);
        assert!(merged.message.contains("• postgres: PostgreSQL needs relinking."));
        assert!(merged.message.contains("• ollama: Ollama needs restart."));
        assert!(merged.message.contains("• database: Database doesn't exist."));
    }

    /// When brew is missing, resolve() must short-circuit the resolver
    /// walk and surface a single, unambiguous homebrew-install remedy.
    /// Regression test for the first-prod-install bug: every component-
    /// level resolver would otherwise produce a "brew install postgresql@17"
    /// remedy attributed to postgres, leaving the user staring at a
    /// postgres-flavored fix when the actual upstream is brew itself.
    #[test]
    fn resolve_short_circuits_when_package_manager_failed() {
        struct PmFailedMock;
        impl PlatformProvider for PmFailedMock {
            fn platform(&self) -> Platform { Platform::Macos }
            fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
            fn package_manager_checker(&self) -> Box<dyn Checker> {
                Box::new(StubChecker(CheckOutcome::failed("brew not found on PATH")))
            }
            fn checker_for(&self, _id: ComponentId, _retry: bool) -> Box<dyn Checker> {
                // Every downstream check fails too (brew is the upstream).
                Box::new(StubChecker(CheckOutcome::failed("upstream brew missing")))
            }
            fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
                // If the short-circuit didn't fire, this resolver would
                // run, return NeedsHumanAction, and pollute the terminal
                // remedy. The assertion below catches that regression.
                vec![Box::new(StubResolver {
                    id: "postgres_install",
                    targets: &[ComponentId::Postgres],
                    outcome: ResolveOutcome::NeedsHumanAction(r(
                        "PostgreSQL install failed.",
                        "brew install postgresql@17",
                    )),
                    fallback: test_fallback(),
                    calls: Arc::new(Mutex::new(Vec::new())),
                })]
            }
            fn default_remedy(&self) -> Remedy { r("default", "noop") }
        }

        let p = PmFailedMock;
        let current = p.check("0.0.0-test");
        assert_eq!(current.package_manager.status, ComponentStatus::Failed,
            "precondition: this test exists to cover the brew-missing branch");

        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});
        assert_eq!(terminal.status, HealthStatus::NeedsAction);
        let remedy = terminal.remedy.as_ref()
            .expect("short-circuit must attach the homebrew install remedy");

        // The remedy is the standalone homebrew installer — not a postgres-
        // attributed bullet from the resolver walk.
        assert!(remedy.script.contains("brew.sh") || remedy.script.contains("Homebrew/install"),
            "expected the canonical homebrew install URL, got: {}", remedy.script);
        assert!(!remedy.script.contains("postgresql@17"),
            "must NOT include the postgres-specific remedy that the regressed path would surface");
        assert!(!remedy.message.starts_with("2 components") && !remedy.message.starts_with("5 components"),
            "must NOT be a multi-component bullet list (single root cause: brew)");
    }

    /// End-to-end via resolve(): two failing components, both with
    /// per-component remedies, must surface a consolidated terminal remedy.
    #[test]
    fn resolve_two_failed_components_yields_consolidated_remedy() {
        // Use two ResolveMock providers? The mock only takes one resolver.
        // Easier: a small custom provider here with two stub resolvers.
        struct TwoResolverMock {
            outcomes: Mutex<HashMap<ComponentId, CheckOutcome>>,
            calls:    Arc<Mutex<Vec<&'static str>>>,
        }
        impl PlatformProvider for TwoResolverMock {
            fn platform(&self) -> Platform { Platform::Macos }
            fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
            fn package_manager_checker(&self) -> Box<dyn Checker> {
                Box::new(StubChecker(CheckOutcome::ready("4.0")))
            }
            fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
                let m = self.outcomes.lock().unwrap();
                Box::new(StubChecker(m.get(&id).cloned()
                    .unwrap_or_else(|| CheckOutcome::failed("none"))))
            }
            fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
                let calls1 = self.calls.clone();
                let calls2 = self.calls.clone();
                vec![
                    Box::new(StubResolver {
                        id: "postgres_install",
                        targets: &[ComponentId::Postgres],
                        outcome: ResolveOutcome::NeedsHumanAction(r(
                            "PostgreSQL needs relinking.",
                            "brew link --force --overwrite postgresql@17",
                        )),
                        fallback: test_fallback(),
                        calls: Arc::new(Mutex::new(Vec::new())),
                    }) as Box<dyn Resolver>,
                    {
                        let _ = (calls1, calls2);
                        Box::new(StubResolver {
                            id: "ollama_install",
                            targets: &[ComponentId::Ollama],
                            outcome: ResolveOutcome::NeedsHumanAction(r(
                                "Ollama needs restart.",
                                "brew services restart ollama",
                            )),
                            fallback: test_fallback(),
                            calls: Arc::new(Mutex::new(Vec::new())),
                        }) as Box<dyn Resolver>
                    },
                ]
            }
            fn default_remedy(&self) -> Remedy { r("default", "noop") }
        }

        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("pg down"));
        outcomes.insert(ComponentId::Ollama,   CheckOutcome::failed("ollama down"));
        let p = TwoResolverMock {
            outcomes: Mutex::new(outcomes),
            calls:    Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});
        assert_eq!(terminal.status, HealthStatus::NeedsAction);
        let merged = terminal.remedy.as_ref().expect("consolidated remedy");
        assert!(merged.script.contains("brew link --force --overwrite postgresql@17"));
        assert!(merged.script.contains("brew services restart ollama"));
        assert!(merged.script.contains('\n'), "script must be newline-joined");
        assert!(merged.message.starts_with("2 components need attention:"));
    }

    // ── dependency-aware skip tests ─────────────────────────────────────

    /// Reusable mock for tests that need multiple resolvers with
    /// individually-configured outcomes. Each resolver gets its own
    /// outcome and fallback; the checker outcomes are shared and mutable
    /// so a "Resolved" resolver can flip its target to Ready for the
    /// post-resolve re-check.
    struct MultiResolverMock {
        outcomes: Mutex<HashMap<ComponentId, CheckOutcome>>,
        specs:    Vec<(&'static str, &'static [ComponentId], ResolveOutcome, Remedy)>,
    }
    impl PlatformProvider for MultiResolverMock {
        fn platform(&self) -> Platform { Platform::Macos }
        fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
        fn package_manager_checker(&self) -> Box<dyn Checker> {
            Box::new(StubChecker(CheckOutcome::ready("4.0")))
        }
        fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
            let m = self.outcomes.lock().unwrap();
            Box::new(StubChecker(m.get(&id).cloned()
                .unwrap_or_else(|| CheckOutcome::failed("none"))))
        }
        fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
            self.specs.iter().map(|(id, targets, outcome, fallback)| {
                Box::new(StubResolver {
                    id,
                    targets,
                    outcome: outcome.clone(),
                    fallback: fallback.clone(),
                    calls: Arc::new(Mutex::new(Vec::new())),
                }) as Box<dyn Resolver>
            }).collect()
        }
        fn default_remedy(&self) -> Remedy { r("default", "noop") }
    }

    /// When postgres is failing and database depends on postgres, the
    /// db_setup resolver MUST be skipped — running it would just produce a
    /// remedy whose root cause is upstream. Only the postgres remedy
    /// surfaces.
    #[test]
    fn dependent_resolver_skipped_when_dependency_still_failing() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("pg down"));
        outcomes.insert(ComponentId::Database, CheckOutcome::failed("db can't connect"));
        let p = MultiResolverMock {
            outcomes: Mutex::new(outcomes),
            specs: vec![
                ("postgres_install",
                 &[ComponentId::Postgres],
                 ResolveOutcome::NeedsHumanAction(r("Fix postgres", "brew link postgresql@17")),
                 test_fallback()),
                ("db_setup",
                 &[ComponentId::Database],
                 // Set up a SUCCESS outcome here — if it ever ran, the test
                 // would pass with the wrong remedy. Failing means the skip
                 // gate actually fired.
                 ResolveOutcome::Resolved,
                 r("would-be db remedy", "createdb sensei_dev")),
            ],
        };
        let current = p.check("0.0.0-test");
        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});
        let remedy = terminal.remedy.as_ref().expect("must have a remedy");
        // Single remedy, single component — pass-through (not consolidated).
        assert_eq!(remedy.script, "brew link postgresql@17",
            "only postgres remedy must surface; database resolver should have been skipped");
        assert!(!remedy.script.contains("createdb"),
            "db_setup must NOT have contributed to the consolidated script");
    }

    /// Regression: a resolver may report `NeedsHumanAction` mid-walk
    /// (e.g. dbd partial deploy fails) and push a remedy. But by the time
    /// the final check runs, the component may have independently
    /// recovered (e.g. the partial schema is enough for the checker to
    /// say Ready). That stale remedy must NOT appear in the consolidated
    /// terminal remedy — otherwise users see a "fix database" message
    /// for a database that's already green.
    #[test]
    fn stale_remedy_for_recovered_component_is_dropped_before_consolidation() {
        // Uses the hoisted `SequenceChecker` — Database: Failed → Ready
        // (recovers between resolve walk and final check). Daemon stays
        // Failed throughout.
        struct Mock {
            db_calls:     Arc<Mutex<usize>>,
            daemon_calls: Arc<Mutex<usize>>,
        }
        impl PlatformProvider for Mock {
            fn platform(&self) -> Platform { Platform::Macos }
            fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
            fn package_manager_checker(&self) -> Box<dyn Checker> {
                Box::new(StubChecker(CheckOutcome::ready("4.0")))
            }
            fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
                match id {
                    // Database: Failed initially, Ready at the final check
                    // (partial dbd deploy was "enough" for the schema probe).
                    ComponentId::Database => Box::new(SequenceChecker {
                        seq: vec![
                            CheckOutcome::failed("schema incomplete"),
                            CheckOutcome::ready_no_version(),
                        ],
                        n: self.db_calls.clone(),
                    }),
                    // Daemon stays failed throughout.
                    ComponentId::Daemon => Box::new(SequenceChecker {
                        seq: vec![CheckOutcome::failed("port closed")],
                        n: self.daemon_calls.clone(),
                    }),
                    _ => Box::new(StubChecker(CheckOutcome::ready("1.0"))),
                }
            }
            fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
                vec![
                    Box::new(StubResolver {
                        id: "db_setup",
                        targets: &[ComponentId::Database],
                        outcome: ResolveOutcome::NeedsHumanAction(r(
                            "dbd partial deploy",
                            "createdb sensei_dev",
                        )),
                        fallback: r("db fallback", "createdb sensei_dev"),
                        calls: Arc::new(Mutex::new(Vec::new())),
                    }),
                    Box::new(StubResolver {
                        id: "daemon_start",
                        targets: &[ComponentId::Daemon],
                        outcome: ResolveOutcome::Resolved,
                        fallback: r("daemon fallback", "senseid-dev start"),
                        calls: Arc::new(Mutex::new(Vec::new())),
                    }),
                ]
            }
            fn default_remedy(&self) -> Remedy { r("default", "install") }
        }

        let p = Mock {
            db_calls:     Arc::new(Mutex::new(0)),
            daemon_calls: Arc::new(Mutex::new(0)),
        };
        let current = p.check("0.0.0-test");
        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});

        // Final check: database is Ready (recovered), daemon is Failed.
        // Database row should be green, daemon row should be red.
        let db = terminal.components.iter().find(|c| c.id == "database").unwrap();
        assert_eq!(db.status, ComponentStatus::Ready);
        let dm = terminal.components.iter().find(|c| c.id == "daemon").unwrap();
        assert_eq!(dm.status, ComponentStatus::Failed);

        let remedy = terminal.remedy.as_ref().expect("must have a remedy");
        assert!(!remedy.script.contains("createdb"),
            "database remedy must be dropped — database recovered. Got: {}", remedy.script);
        assert!(remedy.script.contains("senseid-dev start"),
            "daemon remedy must be present. Got: {}", remedy.script);
        assert!(!remedy.message.contains("database:"),
            "consolidated message must NOT mention database. Got: {}", remedy.message);
    }

    /// Regression: a "transient success" where the post-resolve re-check
    /// briefly saw the target as Ready (so applied_remedies stays empty
    /// for that target during the walk) but the FINAL check at the end
    /// of resolve() sees it as Failed again. Without the retroactive
    /// fallback, the orchestrator would default_remedy() the terminal
    /// — which is what was happening in the Tauri UI while the CLI
    /// (running against steady-state services) saw consistent results.
    #[test]
    fn transient_success_falls_back_to_resolver_fallback_remedy_retroactively() {
        // Uses the hoisted `SequenceChecker` — simulates ollama port
        // flicker: initial=Failed, post-resolve recheck=Ready, final
        // check=Failed.
        struct TransientMock {
            ollama_calls: Arc<Mutex<usize>>,
        }
        impl PlatformProvider for TransientMock {
            fn platform(&self) -> Platform { Platform::Macos }
            fn package_manager_id(&self) -> PackageManagerId { PackageManagerId::Homebrew }
            fn package_manager_checker(&self) -> Box<dyn Checker> {
                Box::new(StubChecker(CheckOutcome::ready("4.0")))
            }
            fn checker_for(&self, id: ComponentId, _retry: bool) -> Box<dyn Checker> {
                match id {
                    ComponentId::Ollama => Box::new(SequenceChecker {
                        seq: vec![
                            CheckOutcome::failed("port closed"),           // initial check
                            CheckOutcome::ready_no_version(),              // post-resolve recheck
                            CheckOutcome::failed("port flickered closed"), // final check
                        ],
                        n: self.ollama_calls.clone(),
                    }),
                    _ => Box::new(StubChecker(CheckOutcome::ready("1.0"))),
                }
            }
            fn resolvers(&self) -> Vec<Box<dyn Resolver>> {
                vec![Box::new(StubResolver {
                    id: "ollama_install",
                    targets: &[ComponentId::Ollama],
                    outcome: ResolveOutcome::Resolved,
                    fallback: r("ollama fallback", "brew services restart ollama"),
                    calls: Arc::new(Mutex::new(Vec::new())),
                })]
            }
            fn default_remedy(&self) -> Remedy {
                r("install sensei (default)", "brew install --HEAD sensei-hq/tap/sensei-dev")
            }
        }

        let p = TransientMock { ollama_calls: Arc::new(Mutex::new(0)) };
        let current = p.check("0.0.0-test");
        let terminal = p.resolve(&current, "0.0.0-test", &|_| {});

        assert_eq!(terminal.status, HealthStatus::NeedsAction,
            "ollama failed in final check, so terminal must be NeedsAction");
        let remedy = terminal.remedy.as_ref()
            .expect("retroactive fallback must attach a remedy");
        assert_eq!(remedy.script, "brew services restart ollama",
            "remedy must be the resolver's fallback_remedy, NOT default_remedy");
        assert!(!remedy.script.contains("sensei-hq/tap"),
            "must not fall through to default_remedy");
    }

    /// When the upstream resolver succeeds in the same pass, the
    /// downstream resolver's dependency gate clears and it gets a shot.
    /// Postgres recovers → Database resolver runs → both green.
    #[test]
    fn dependent_resolver_runs_when_upstream_recovers_in_same_pass() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Postgres, CheckOutcome::failed("pg down"));
        outcomes.insert(ComponentId::Database, CheckOutcome::failed("can't connect"));
        let p = MultiResolverMock {
            outcomes: Mutex::new(outcomes),
            specs: vec![
                ("postgres_install",
                 &[ComponentId::Postgres],
                 ResolveOutcome::Resolved,  // ← claims success
                 test_fallback()),
                ("db_setup",
                 &[ComponentId::Database],
                 ResolveOutcome::Resolved,  // ← also claims success
                 test_fallback()),
            ],
        };
        let current = p.check("0.0.0-test");
        // After postgres resolver runs, the world says postgres is Ready.
        // After db_setup runs, the world says database is Ready too.
        // We can't sequence those mutations precisely without per-resolver
        // hooks, so we flip both to Ready up-front — the test still proves
        // the gate works because db_setup is skipped via Postgres being in
        // now_failing at the start of its iteration.
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Postgres, CheckOutcome::ready("17"));
            m.insert(ComponentId::Database, CheckOutcome::ready_no_version());
        }
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let cc = calls.clone();
        let terminal = p.resolve(&current, "0.0.0-test", &|ev| {
            if let HealthEvent::Component { id, patch } = &ev {
                if matches!(patch.status, Some(ComponentStatus::Installing)) {
                    cc.lock().unwrap().push(id.clone());
                }
            }
        });
        assert_eq!(terminal.status, HealthStatus::Ok,
            "both resolvers should have run and recovered");
        let installing = calls.lock().unwrap();
        assert!(installing.contains(&"postgres".to_string()),
            "postgres resolver must have entered Installing state");
        assert!(installing.contains(&"database".to_string()),
            "database resolver must have entered Installing state — its dependency gate cleared after postgres recovered");
    }

    /// Counterpart: when `Resolved` is followed by a clean re-check, no
    /// remedy event is emitted and the terminal payload is `Ok` with no
    /// remedy.
    #[test]
    fn resolve_resolved_with_clean_recheck_emits_no_remedy() {
        let mut outcomes = ready_outcomes();
        outcomes.insert(ComponentId::Ollama, CheckOutcome::failed("not yet"));
        let p = ResolveMock {
            pm_outcome: CheckOutcome::ready("4.0"),
            outcomes: Mutex::new(outcomes),
            resolver_id: "ollama_install",
            resolver_targets: &[ComponentId::Ollama],
            resolver_outcome: ResolveOutcome::Resolved,
            resolver_fallback: test_fallback(),
            calls: Arc::new(Mutex::new(Vec::new())),
        };
        let current = p.check("0.0.0-test");
        // Resolver "fix" took effect: subsequent checker reads return ready.
        {
            let mut m = p.outcomes.lock().unwrap();
            m.insert(ComponentId::Ollama, CheckOutcome::ready("0.4"));
        }
        let events = Arc::new(Mutex::new(Vec::<HealthEvent>::new()));
        let ec = events.clone();
        let terminal = p.resolve(&current, "0.0.0-test", &|e| ec.lock().unwrap().push(e));
        assert_eq!(terminal.status, HealthStatus::Ok);
        assert!(terminal.remedy.is_none(), "no remedy on a fully resolved terminal");
        assert!(
            !events.lock().unwrap().iter().any(|e| matches!(e, HealthEvent::Remedy { .. })),
            "no Remedy event should be emitted when re-check is clean",
        );
    }
}
