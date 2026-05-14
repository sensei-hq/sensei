//! Bootstrap engine — dependency-aware check-and-fix orchestrator.
//!
//! `BootstrapEngine::check_and_fix` runs in three phases:
//!   A. Parallel check — all components checked concurrently
//!   B. Fix plan — topo-sort pending, mark dep-blocked, group bundle batch
//!   C. Sequential fix — execute plan, handle human actions, post_fix_triggers
//!   D. Return BootstrapReport

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use crate::platform::PlatformProvider;
use crate::config::SenseiConfig;
use super::checker::Checker;
use super::fixer::Fixer;
use super::{BootstrapReport, CheckResult, FixResult, GateKind, GateReport, GateStatus,
            HumanAction, ProgressEvent};
use super::registry::{BuildContext, COMPONENTS};

// ── BootstrapContext ──────────────────────────────────────────────────────────

/// Full bootstrap context — owns platform, config, app_version, and test-injection maps.
pub struct BootstrapContext {
    pub platform:    Arc<dyn PlatformProvider>,
    pub config:      SenseiConfig,
    pub app_version: String,
    /// Test-only checker overrides indexed by component id.
    pub(crate) checker_overrides: HashMap<&'static str, Arc<dyn Checker>>,
    /// Test-only fixer overrides indexed by component id.
    pub(crate) fixer_overrides:   HashMap<&'static str, Arc<dyn Fixer>>,
}

impl BootstrapContext {
    pub fn new(platform: Arc<dyn PlatformProvider>, config: SenseiConfig, app_version: String) -> Self {
        Self {
            platform,
            config,
            app_version,
            checker_overrides: HashMap::new(),
            fixer_overrides:   HashMap::new(),
        }
    }

    /// Inject a mock checker for a component id (used in tests).
    pub fn with_checker(mut self, id: &'static str, checker: Arc<dyn Checker>) -> Self {
        self.checker_overrides.insert(id, checker);
        self
    }

    /// Inject a mock fixer for a component id (used in tests).
    pub fn with_fixer(mut self, id: &'static str, fixer: Arc<dyn Fixer>) -> Self {
        self.fixer_overrides.insert(id, fixer);
        self
    }

    /// Build a BuildContext borrowing from this context.
    pub(crate) fn build_context(&self) -> BuildContext<'_> {
        BuildContext {
            platform:    &self.platform,
            config:      &self.config,
            app_version: &self.app_version,
        }
    }
}

// ── Plan step ─────────────────────────────────────────────────────────────────

/// One step in the sequential fix plan.
enum PlanStep {
    /// A single component fix (not batched).
    Individual(&'static str),
    /// A batch of bundle components fixed together via one BrewBundleFixer call.
    Batch(Vec<&'static str>),
}

// ── BootstrapEngine ───────────────────────────────────────────────────────────

/// The bootstrap engine — runs check_and_fix against the component registry.
pub struct BootstrapEngine {
    ctx: Arc<BootstrapContext>,
}

impl BootstrapEngine {
    pub fn new(ctx: Arc<BootstrapContext>) -> Self {
        Self { ctx }
    }

    /// Run checker for a component id (uses override or spec fn).
    fn run_check(&self, id: &'static str) -> CheckResult {
        if let Some(c) = self.ctx.checker_overrides.get(id) {
            c.check()
        } else {
            let bctx = self.ctx.build_context();
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            (spec.checker_fn)(&bctx).check()
        }
    }

    /// Get fixer for a component id (uses override or spec fn).
    fn get_fixer(&self, id: &'static str) -> Arc<dyn Fixer> {
        if let Some(f) = self.ctx.fixer_overrides.get(id) {
            Arc::clone(f)
        } else {
            let bctx = self.ctx.build_context();
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            Arc::from((spec.fixer_fn)(&bctx))
        }
    }

    /// Topo-sort pending components, mark dep-blocked, group bundle components into batches.
    /// Returns (plan steps, dep-blocked set).
    fn build_fix_plan(
        pending: &[&'static str],
        _ready:  &HashSet<&'static str>,
    ) -> (Vec<PlanStep>, HashSet<&'static str>) {
        let pending_set: HashSet<&'static str> = pending.iter().copied().collect();

        // A component is dep-blocked if any of its depends_on is pending (i.e. not ready).
        let mut dep_blocked: HashSet<&'static str> = HashSet::new();
        for spec in COMPONENTS {
            if !pending_set.contains(spec.id) { continue; }
            if spec.depends_on.iter().any(|dep| pending_set.contains(dep)) {
                dep_blocked.insert(spec.id);
            }
        }

        // Topo-sort: COMPONENTS definition order is a valid topological order
        // (dependencies always appear before their dependents in the static array).
        let ordered: Vec<&'static str> = COMPONENTS.iter()
            .map(|s| s.id)
            .filter(|id| pending_set.contains(id))
            .collect();

        // Group consecutive fix_group="bundle" non-dep-blocked components into Batch steps.
        let mut plan: Vec<PlanStep> = Vec::new();
        let mut bundle_batch: Vec<&'static str> = Vec::new();

        let flush_bundle = |batch: &mut Vec<&'static str>, plan: &mut Vec<PlanStep>| {
            match batch.len() {
                0 => {}
                1 => { plan.push(PlanStep::Individual(batch[0])); }
                _ => { plan.push(PlanStep::Batch(batch.clone())); }
            }
            batch.clear();
        };

        for &id in &ordered {
            let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
            if dep_blocked.contains(id) {
                flush_bundle(&mut bundle_batch, &mut plan);
                // dep-blocked items appear as Individual; Phase C will emit Failed and skip fixing.
                plan.push(PlanStep::Individual(id));
                continue;
            }
            if spec.fix_group == Some("bundle") {
                bundle_batch.push(id);
            } else {
                flush_bundle(&mut bundle_batch, &mut plan);
                plan.push(PlanStep::Individual(id));
            }
        }
        flush_bundle(&mut bundle_batch, &mut plan);

        (plan, dep_blocked)
    }

    /// Check all components, fix what's broken, return a full report.
    /// Emits ProgressEvents via callback throughout.
    pub fn check_and_fix<F>(&self, callback: F) -> BootstrapReport
    where
        F: Fn(ProgressEvent) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);

        // ── Phase A: Parallel check ───────────────────────────────────────────
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel::<(&'static str, CheckResult)>();

        for spec in COMPONENTS {
            let tx      = tx.clone();
            let cb      = Arc::clone(&callback);
            let checker: Arc<dyn Checker> = self.ctx.checker_overrides
                .get(spec.id)
                .map(Arc::clone)
                .unwrap_or_else(|| {
                    let bctx = self.ctx.build_context();
                    Arc::from((spec.checker_fn)(&bctx))
                });
            let id = spec.id;
            std::thread::spawn(move || {
                cb(ProgressEvent::Gate { id: id.to_string(), status: GateStatus::Checking });
                let result = checker.check();
                let _ = tx.send((id, result));
            });
        }
        drop(tx); // closing sender so rx terminates

        // Collect results; emit Ready events immediately for passing checks
        let mut check_results: HashMap<&'static str, CheckResult> = HashMap::new();
        for (id, result) in rx {
            if result.ok {
                callback(ProgressEvent::Gate {
                    id: id.to_string(),
                    status: GateStatus::Ready {
                        version: result.version.clone(),
                        detail:  result.detail.clone(),
                    },
                });
            }
            check_results.insert(id, result);
        }

        let ready: HashSet<&'static str> = check_results.iter()
            .filter(|(_, r)| r.ok)
            .map(|(id, _)| *id)
            .collect();
        let pending: Vec<&'static str> = COMPONENTS.iter()
            .map(|s| s.id)
            .filter(|id| !ready.contains(id))
            .collect();

        // Early return: everything already healthy
        if pending.is_empty() {
            let gates = COMPONENTS.iter().map(|spec| GateReport {
                id:            spec.id.to_string(),
                status:        GateStatus::Ready {
                    version: check_results[spec.id].version.clone(),
                    detail:  check_results[spec.id].detail.clone(),
                },
                fix_attempted: false,
                fix_detail:    None,
                check_error:   None,
            }).collect();
            return BootstrapReport { gates, all_ok: true, blocked_on: None };
        }

        // ── Phase B: Build fix plan ───────────────────────────────────────────
        let (plan, dep_blocked) = Self::build_fix_plan(&pending, &ready);

        // ── Phase C: Sequential fix execution ────────────────────────────────
        // Seed gate_reports with already-ready components.
        let mut gate_reports: HashMap<&'static str, GateReport> = HashMap::new();
        for spec in COMPONENTS {
            if ready.contains(spec.id) {
                gate_reports.insert(spec.id, GateReport {
                    id:            spec.id.to_string(),
                    status:        GateStatus::Ready {
                        version: check_results[spec.id].version.clone(),
                        detail:  check_results[spec.id].detail.clone(),
                    },
                    fix_attempted: false,
                    fix_detail:    None,
                    check_error:   None,
                });
            }
        }

        let mut blocked_on: Option<HumanAction>      = None;
        let mut post_trigger_ids: Vec<&'static str>   = Vec::new();

        'plan: for step in &plan {
            if blocked_on.is_some() { break; }

            match step {
                PlanStep::Individual(id) => {
                    let id   = *id;
                    let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();

                    if dep_blocked.contains(id) {
                        let check_error = check_results.get(id).and_then(|r| r.error.clone());
                        let err = "dep-blocked: dependency not ready".to_string();
                        callback(ProgressEvent::Gate {
                            id:     id.to_string(),
                            status: GateStatus::Failed { error: err.clone() },
                        });
                        gate_reports.insert(id, GateReport {
                            id:            id.to_string(),
                            status:        GateStatus::Failed { error: err },
                            fix_attempted: false,
                            fix_detail:    None,
                            check_error,
                        });
                        continue;
                    }

                    let fixer = self.get_fixer(id);

                    // Human action check — stops execution and surfaces the original check error.
                    if let Some(action) = fixer.human_action() {
                        let check_error = check_results.get(id).and_then(|r| r.error.clone());
                        let err = check_error.clone()
                            .unwrap_or_else(|| "not ready".to_string());
                        callback(ProgressEvent::Gate {
                            id:     id.to_string(),
                            status: GateStatus::Failed { error: err.clone() },
                        });
                        gate_reports.insert(id, GateReport {
                            id:            id.to_string(),
                            status:        GateStatus::Failed { error: err },
                            fix_attempted: false,
                            fix_detail:    None,
                            check_error,
                        });
                        blocked_on = Some(action);
                        break 'plan;
                    }

                    // Emit in-progress status
                    callback(ProgressEvent::Gate {
                        id:     id.to_string(),
                        status: match spec.gate_kind {
                            GateKind::Install => GateStatus::Installing,
                            GateKind::Service => GateStatus::Starting,
                        },
                    });

                    // Run fix then recheck
                    let check_error = check_results.get(id).and_then(|r| r.error.clone());
                    let fix_result = fixer.fix();
                    let recheck    = self.run_check(id);
                    Self::record_fix_result(
                        id, fix_result, recheck, check_error, &callback, &mut gate_reports,
                    );

                    // post_fix_trigger
                    if matches!(gate_reports[id].status, GateStatus::Ready { .. }) {
                        for &tid in spec.post_fix_trigger {
                            if !post_trigger_ids.contains(&tid) {
                                post_trigger_ids.push(tid);
                            }
                        }
                    }
                }

                PlanStep::Batch(ids) => {
                    // One BrewBundleFixer call covers all bundle components.
                    let representative_id = ids[0];
                    let fixer = self.get_fixer(representative_id);

                    // Human action check — if the batch fixer requires human action,
                    // surface the original check error for each component and stop.
                    if let Some(action) = fixer.human_action() {
                        for &id in ids {
                            let check_error = check_results.get(id).and_then(|r| r.error.clone());
                            let err = check_error.clone()
                                .unwrap_or_else(|| "not ready".to_string());
                            let status = GateStatus::Failed { error: err };
                            callback(ProgressEvent::Gate { id: id.to_string(), status: status.clone() });
                            gate_reports.insert(id, GateReport {
                                id: id.to_string(), status,
                                fix_attempted: false,
                                fix_detail:    None,
                                check_error,
                            });
                        }
                        blocked_on = Some(action);
                        break 'plan;
                    }

                    // Emit Installing for all ids in the batch
                    for &id in ids {
                        callback(ProgressEvent::Gate {
                            id:     id.to_string(),
                            status: GateStatus::Installing,
                        });
                    }

                    let fix_result = fixer.fix();

                    match fix_result {
                        Err(e) => {
                            for &id in ids {
                                let check_error = check_results.get(id).and_then(|r| r.error.clone());
                                let status = GateStatus::Failed { error: e.clone() };
                                callback(ProgressEvent::Gate { id: id.to_string(), status: status.clone() });
                                gate_reports.insert(id, GateReport {
                                    id: id.to_string(), status,
                                    fix_attempted: true,
                                    fix_detail:    None,
                                    check_error,
                                });
                            }
                        }
                        Ok(fix_res) => {
                            for &id in ids {
                                let check_error = check_results.get(id).and_then(|r| r.error.clone());
                                let recheck = self.run_check(id);
                                let final_status = if recheck.ok {
                                    GateStatus::Ready { version: recheck.version, detail: recheck.detail }
                                } else {
                                    GateStatus::Failed { error: "recheck failed after brew bundle".to_string() }
                                };
                                callback(ProgressEvent::Gate {
                                    id:     id.to_string(),
                                    status: final_status.clone(),
                                });
                                gate_reports.insert(id, GateReport {
                                    id:            id.to_string(),
                                    status:        final_status.clone(),
                                    fix_attempted: true,
                                    fix_detail:    Some(fix_res.approach.clone()),
                                    check_error,
                                });

                                // post_fix_trigger after bundle
                                if matches!(final_status, GateStatus::Ready { .. }) {
                                    let spec = COMPONENTS.iter().find(|s| s.id == id).unwrap();
                                    for &tid in spec.post_fix_trigger {
                                        if !post_trigger_ids.contains(&tid) {
                                            post_trigger_ids.push(tid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Execute post_fix_trigger steps (e.g. database after senseid upgrade)
        for &tid in &post_trigger_ids {
            if blocked_on.is_some() { break; }
            let spec   = COMPONENTS.iter().find(|s| s.id == tid).unwrap();
            let fixer  = self.get_fixer(tid);

            callback(ProgressEvent::Gate {
                id:     tid.to_string(),
                status: match spec.gate_kind {
                    GateKind::Install => GateStatus::Installing,
                    GateKind::Service => GateStatus::Starting,
                },
            });

            let check_error = check_results.get(tid).and_then(|r| r.error.clone());
            let fix_result = fixer.fix();
            let recheck    = self.run_check(tid);
            Self::record_fix_result(
                tid, fix_result, recheck, check_error, &callback, &mut gate_reports,
            );
        }

        // ── Phase D: Return ───────────────────────────────────────────────────
        // Collect gates in COMPONENTS order; fill in any remaining pending that were skipped.
        let gates: Vec<GateReport> = COMPONENTS.iter().map(|spec| {
            gate_reports.remove(spec.id).unwrap_or_else(|| GateReport {
                id:            spec.id.to_string(),
                status:        GateStatus::Failed { error: "not reached".to_string() },
                fix_attempted: false,
                fix_detail:    None,
                check_error:   check_results.get(spec.id).and_then(|r| r.error.clone()),
            })
        }).collect();

        let all_ok = blocked_on.is_none()
            && gates.iter().all(|g| matches!(g.status, GateStatus::Ready { .. }));

        BootstrapReport { gates, all_ok, blocked_on }
    }

    /// Record the result of a fix attempt into gate_reports and emit the final gate event.
    fn record_fix_result(
        id:            &'static str,
        fix_result:    Result<FixResult, String>,
        recheck:       CheckResult,
        check_error:   Option<String>,
        callback:      &Arc<impl Fn(ProgressEvent) + Send + Sync + 'static>,
        gate_reports:  &mut HashMap<&'static str, GateReport>,
    ) {
        let (final_status, fix_detail) = if recheck.ok {
            (
                GateStatus::Ready { version: recheck.version, detail: recheck.detail },
                fix_result.ok().map(|r| r.approach),
            )
        } else {
            (
                GateStatus::Failed {
                    error: fix_result.err().unwrap_or_else(|| "recheck failed".to_string()),
                },
                None,
            )
        };
        callback(ProgressEvent::Gate { id: id.to_string(), status: final_status.clone() });
        gate_reports.insert(id, GateReport {
            id:            id.to_string(),
            status:        final_status,
            fix_attempted: true,
            fix_detail,
            check_error,
        });
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prereq::registry::COMPONENTS;

    // ── Shared mock helpers ───────────────────────────────────────────────────

    fn make_ctx() -> BootstrapContext {
        use crate::{platform, config::SenseiConfig};
        BootstrapContext::new(
            Arc::from(platform::detect()),
            SenseiConfig::from_env(),
            "0.1.0".to_string(),
        )
    }

    fn make_mock_checker(ok: bool) -> Arc<dyn Checker> {
        struct M(bool);
        impl Checker for M {
            fn check(&self) -> CheckResult {
                if self.0 { CheckResult::ok("mock") } else { CheckResult::fail("mock fail") }
            }
        }
        Arc::new(M(ok))
    }

    fn make_mock_fixer(succeed: bool) -> Arc<dyn Fixer> {
        struct M(bool);
        impl Fixer for M {
            fn fix(&self) -> Result<FixResult, String> {
                if self.0 { Ok(FixResult::new("mock fixed")) } else { Err("mock fix failed".into()) }
            }
        }
        Arc::new(M(succeed))
    }

    /// Sequenced mock checker: returns values from a VecDeque (first=initial, second=after fix)
    struct SeqChecker(std::sync::Mutex<std::collections::VecDeque<bool>>);
    impl SeqChecker {
        fn new(seq: &[bool]) -> Arc<dyn Checker> {
            Arc::new(Self(std::sync::Mutex::new(std::collections::VecDeque::from(seq.to_vec()))))
        }
    }
    impl Checker for SeqChecker {
        fn check(&self) -> CheckResult {
            let val = self.0.lock().unwrap().pop_front().unwrap_or(false);
            if val { CheckResult::ok("mock") } else { CheckResult::fail("mock fail") }
        }
    }

    /// Fixer that requires human action
    struct HumanFixer { id: &'static str }
    impl Fixer for HumanFixer {
        fn fix(&self) -> Result<FixResult, String> { Err("human action".into()) }
        fn human_action(&self) -> Option<HumanAction> {
            Some(HumanAction {
                component_id: self.id.to_string(),
                title:        format!("Fix {}", self.id),
                command:      "manual".to_string(),
                url:          None,
            })
        }
    }

    fn all_ok_ctx() -> BootstrapContext {
        let mut ctx = make_ctx();
        for spec in COMPONENTS { ctx = ctx.with_checker(spec.id, make_mock_checker(true)); }
        ctx
    }

    // ── Context tests ─────────────────────────────────────────────────────────

    #[test]
    fn bootstrap_context_constructs() {
        let ctx = make_ctx();
        assert_eq!(ctx.app_version, "0.1.0");
    }

    #[test]
    fn with_checker_stores_override() {
        let ctx = make_ctx().with_checker("postgresql", make_mock_checker(true));
        assert!(ctx.checker_overrides.contains_key("postgresql"));
    }

    #[test]
    fn with_fixer_stores_override() {
        let ctx = make_ctx().with_fixer("postgresql", make_mock_fixer(true));
        assert!(ctx.fixer_overrides.contains_key("postgresql"));
    }

    // ── Phase B: fix plan tests ───────────────────────────────────────────────

    #[test]
    fn build_fix_plan_dep_blocked_when_homebrew_fails() {
        let pending = vec!["homebrew", "postgresql", "ollama"];
        let ready: HashSet<&'static str> = HashSet::new();
        let (plan, dep_blocked) = BootstrapEngine::build_fix_plan(&pending, &ready);
        assert!(dep_blocked.contains("postgresql"), "postgresql should be dep-blocked by homebrew");
        assert!(dep_blocked.contains("ollama"), "ollama should be dep-blocked by homebrew");
        assert!(!dep_blocked.contains("homebrew"), "homebrew itself is not dep-blocked");
        assert!(plan.iter().any(|s| matches!(s, PlanStep::Individual("homebrew"))));
    }

    #[test]
    fn build_fix_plan_bundles_postgresql_and_ollama() {
        let pending = vec!["postgresql", "ollama"];
        let mut ready: HashSet<&'static str> = HashSet::new();
        ready.insert("homebrew");
        let (plan, dep_blocked) = BootstrapEngine::build_fix_plan(&pending, &ready);
        assert!(dep_blocked.is_empty(), "no dep-blocked when homebrew is ready");
        assert!(
            plan.iter().any(|s| matches!(s, PlanStep::Batch(ids)
                if ids.contains(&"postgresql") && ids.contains(&"ollama"))),
            "postgresql and ollama should be batched together"
        );
    }

    // ── Integration test 1: All ready ─────────────────────────────────────────

    #[test]
    fn all_ready_returns_all_ok_true() {
        let engine = BootstrapEngine::new(Arc::new(all_ok_ctx()));
        let report = engine.check_and_fix(|_| {});
        assert!(report.all_ok, "all ready mocks should produce all_ok=true");
        assert!(report.blocked_on.is_none());
        assert_eq!(report.gates.len(), 10);
    }

    #[test]
    fn all_ready_emits_checking_then_ready_events() {
        let events: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(vec![]));
        let events_cb = Arc::clone(&events);
        let engine = BootstrapEngine::new(Arc::new(all_ok_ctx()));
        engine.check_and_fix(move |e| {
            if let ProgressEvent::Gate { id, status } = &e {
                let tag = match status {
                    GateStatus::Checking    => format!("{id}:checking"),
                    GateStatus::Ready { .. }=> format!("{id}:ready"),
                    _                       => format!("{id}:other"),
                };
                events_cb.lock().unwrap().push(tag);
            }
        });
        let ev = events.lock().unwrap();
        for spec in COMPONENTS {
            assert!(ev.contains(&format!("{}:checking", spec.id)), "missing checking for {}", spec.id);
            assert!(ev.contains(&format!("{}:ready", spec.id)),    "missing ready for {}",    spec.id);
        }
    }

    // ── Integration test 2: Homebrew missing → blocked_on ────────────────────

    #[test]
    fn homebrew_missing_returns_blocked_on() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew", make_mock_checker(false));
        ctx = ctx.with_fixer("homebrew", Arc::new(HumanFixer { id: "homebrew" }));
        for spec in COMPONENTS {
            if spec.id != "homebrew" { ctx = ctx.with_checker(spec.id, make_mock_checker(true)); }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        assert!(!report.all_ok);
        assert!(report.blocked_on.is_some());
        assert_eq!(report.blocked_on.unwrap().component_id, "homebrew");
    }

    // ── Integration test 3: Homebrew blocks postgresql fix ────────────────────

    #[test]
    fn homebrew_missing_blocks_postgresql_fix() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(false));
        ctx = ctx.with_fixer  ("homebrew",   Arc::new(HumanFixer { id: "homebrew" }));
        ctx = ctx.with_checker("postgresql", make_mock_checker(false));
        ctx = ctx.with_fixer  ("postgresql", make_mock_fixer(true)); // should never be called
        for spec in COMPONENTS {
            if !["homebrew", "postgresql"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        assert!(report.blocked_on.is_some(), "should be blocked on homebrew");
        assert_eq!(report.blocked_on.unwrap().component_id, "homebrew");
        if let Some(pg) = report.gates.iter().find(|g| g.id == "postgresql") {
            assert!(!pg.fix_attempted, "postgresql fix should not have been attempted");
        }
    }

    // ── Integration test 4: postgresql + ollama → BrewBundleFixer called once ─

    #[test]
    fn postgresql_and_ollama_fixed_via_bundle() {
        let fix_call_count = Arc::new(std::sync::Mutex::new(0u32));
        let fcc = Arc::clone(&fix_call_count);
        struct CountingFixer(Arc<std::sync::Mutex<u32>>);
        impl Fixer for CountingFixer {
            fn fix(&self) -> Result<FixResult, String> {
                *self.0.lock().unwrap() += 1;
                Ok(FixResult::new("bundle ran"))
            }
        }
        let bundle: Arc<dyn Fixer> = Arc::new(CountingFixer(fcc));

        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(true));
        ctx = ctx.with_checker("postgresql", SeqChecker::new(&[false, true]));
        ctx = ctx.with_checker("ollama",     SeqChecker::new(&[false, true]));
        ctx = ctx.with_fixer  ("postgresql", Arc::clone(&bundle));
        ctx = ctx.with_fixer  ("ollama",     Arc::clone(&bundle));
        for spec in COMPONENTS {
            if !["homebrew","postgresql","ollama"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        // Bundle should run only once regardless of how many components are batched
        assert_eq!(*fix_call_count.lock().unwrap(), 1, "BrewBundleFixer should be called exactly once");
        let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
        let ol = report.gates.iter().find(|g| g.id == "ollama").unwrap();
        assert!(matches!(pg.status, GateStatus::Ready { .. }));
        assert!(matches!(ol.status, GateStatus::Ready { .. }));
    }

    // ── Integration test 5: senseid outdated → bundle ──────────────────────────

    #[test]
    fn senseid_outdated_fixed_via_bundle() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew", make_mock_checker(true));
        ctx = ctx.with_checker("senseid",  SeqChecker::new(&[false, true]));
        ctx = ctx.with_fixer  ("senseid",  make_mock_fixer(true));
        // database always passes so post_fix_trigger doesn't run a failing db fixer
        for spec in COMPONENTS {
            if !["homebrew","senseid"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let sd = report.gates.iter().find(|g| g.id == "senseid").unwrap();
        assert!(matches!(sd.status, GateStatus::Ready { .. }), "senseid should be Ready after fix");
        assert!(sd.fix_attempted);
    }

    // ── Integration test 6: DB schema not deployed ────────────────────────────

    #[test]
    fn db_schema_not_deployed_gets_fixed() {
        let mut ctx = make_ctx();
        for spec in COMPONENTS {
            if spec.id == "database" {
                ctx = ctx.with_checker("database", SeqChecker::new(&[false, true]));
                ctx = ctx.with_fixer  ("database", make_mock_fixer(true));
            } else {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let db = report.gates.iter().find(|g| g.id == "database").unwrap();
        assert!(matches!(db.status, GateStatus::Ready { .. }), "database should be Ready after deploy");
        assert!(db.fix_attempted);
        assert!(db.fix_detail.is_some(), "fix_detail should be populated");
    }

    // ── Integration test 7: Fix succeeds but recheck fails → gate=Failed ─────

    #[test]
    fn fix_succeeds_but_recheck_fails_gate_is_failed() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(true));
        ctx = ctx.with_checker("postgresql", make_mock_checker(false)); // always fails
        ctx = ctx.with_fixer  ("postgresql", make_mock_fixer(true));    // fixer says ok but recheck still fails
        for spec in COMPONENTS {
            if !["homebrew","postgresql"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
        assert!(matches!(pg.status, GateStatus::Failed { .. }), "gate should be Failed when recheck fails");
        assert!(pg.fix_attempted);
    }

    // ── Integration test 8: Resume after human action ─────────────────────────

    #[test]
    fn resume_after_human_action() {
        // First call: homebrew fails with HumanActionFixer
        let mut ctx1 = make_ctx();
        ctx1 = ctx1.with_checker("homebrew", make_mock_checker(false));
        ctx1 = ctx1.with_fixer  ("homebrew", Arc::new(HumanFixer { id: "homebrew" }));
        for spec in COMPONENTS {
            if spec.id != "homebrew" { ctx1 = ctx1.with_checker(spec.id, make_mock_checker(true)); }
        }
        let report1 = BootstrapEngine::new(Arc::new(ctx1)).check_and_fix(|_| {});
        assert!(report1.blocked_on.is_some());

        // Second call: all pass (human action completed)
        let report2 = BootstrapEngine::new(Arc::new(all_ok_ctx())).check_and_fix(|_| {});
        assert!(report2.all_ok, "second call should be all_ok after homebrew fixed");
    }

    // ── Integration test 9: senseid upgrade triggers db deploy ────────────────

    #[test]
    fn senseid_upgrade_triggers_database_deploy() {
        let db_called = Arc::new(std::sync::Mutex::new(false));
        let db_called_clone = Arc::clone(&db_called);
        struct DbFixer(Arc<std::sync::Mutex<bool>>);
        impl Fixer for DbFixer {
            fn fix(&self) -> Result<FixResult, String> {
                *self.0.lock().unwrap() = true;
                Ok(FixResult::new("schema deployed"))
            }
        }
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew", make_mock_checker(true));
        ctx = ctx.with_checker("senseid",  SeqChecker::new(&[false, true]));
        ctx = ctx.with_fixer  ("senseid",  make_mock_fixer(true));
        // database initially passes — post_fix_trigger forces deploy anyway
        ctx = ctx.with_checker("database", SeqChecker::new(&[true, true]));
        ctx = ctx.with_fixer  ("database", Arc::new(DbFixer(db_called_clone)));
        for spec in COMPONENTS {
            if !["homebrew","senseid","database"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        assert!(*db_called.lock().unwrap(), "database fixer should be called via post_fix_trigger");
        let db = report.gates.iter().find(|g| g.id == "database").unwrap();
        assert!(matches!(db.status, GateStatus::Ready { .. }));
        assert_eq!(db.fix_detail.as_deref(), Some("schema deployed"));
    }

    // ── Integration test 10: senseid upgrade, db deploy fails ─────────────────

    #[test]
    fn senseid_upgrade_db_deploy_fails() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew", make_mock_checker(true));
        ctx = ctx.with_checker("senseid",  SeqChecker::new(&[false, true]));
        ctx = ctx.with_fixer  ("senseid",  make_mock_fixer(true));
        ctx = ctx.with_checker("database", make_mock_checker(false)); // db fails even after deploy
        ctx = ctx.with_fixer  ("database", make_mock_fixer(false));
        for spec in COMPONENTS {
            if !["homebrew","senseid","database"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let db = report.gates.iter().find(|g| g.id == "database").unwrap();
        assert!(matches!(db.status, GateStatus::Failed { .. }), "db gate should be Failed");
        assert!(db.fix_detail.is_none(), "no fix_detail when fixer fails");
    }

    // ── Integration test 11: senseid upgrade, db already current ──────────────

    #[test]
    fn senseid_upgrade_db_already_current() {
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew", make_mock_checker(true));
        ctx = ctx.with_checker("senseid",  SeqChecker::new(&[false, true]));
        ctx = ctx.with_fixer  ("senseid",  make_mock_fixer(true));
        ctx = ctx.with_checker("database", SeqChecker::new(&[true, true]));
        ctx = ctx.with_fixer  ("database", make_mock_fixer(true)); // no-op deploy succeeds
        for spec in COMPONENTS {
            if !["homebrew","senseid","database"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let db = report.gates.iter().find(|g| g.id == "database").unwrap();
        assert!(matches!(db.status, GateStatus::Ready { .. }));
        assert!(db.fix_detail.is_some(), "fix_detail populated even for no-op deploy");
    }

    // ── Integration test 12: db deploy success populates fix_detail ───────────

    #[test]
    fn db_deploy_success_populates_fix_detail() {
        struct DetailFixer;
        impl Fixer for DetailFixer {
            fn fix(&self) -> Result<FixResult, String> {
                Ok(FixResult::new("3 migrations applied"))
            }
        }
        let mut ctx = make_ctx();
        for spec in COMPONENTS {
            if spec.id == "database" {
                ctx = ctx.with_checker("database", SeqChecker::new(&[false, true]));
                ctx = ctx.with_fixer  ("database", Arc::new(DetailFixer));
            } else {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let db = report.gates.iter().find(|g| g.id == "database").unwrap();
        assert_eq!(db.fix_detail.as_deref(), Some("3 migrations applied"));
    }

    // ── Integration test 13: Daemon dep-blocked by database failure ───────────
    // postgresql_service fails (and can't be fixed) → database is dep-blocked (also fails) →
    // daemon is dep-blocked by database failing.

    #[test]
    fn daemon_blocked_when_database_fails() {
        let mut ctx = make_ctx();
        for spec in COMPONENTS {
            // postgresql_service fails and its fixer fails too.
            // database and daemon must also fail their initial checks so they enter the pending set —
            // with mocked checkers they are independent of postgresql_service's actual state.
            if ["postgresql_service", "database", "daemon"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(false));
                ctx = ctx.with_fixer  (spec.id, make_mock_fixer(false));
            } else {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let daemon = report.gates.iter().find(|g| g.id == "daemon").unwrap();
        assert!(matches!(daemon.status, GateStatus::Failed { .. }), "daemon should be dep-blocked/Failed");
        assert!(!daemon.fix_attempted, "daemon fix should not be attempted when dep-blocked");
    }

    // ── Integration test 14: Batch fixer fails → all batch members Failed ─────

    #[test]
    fn batch_fixer_fails_marks_all_batch_components_failed() {
        // postgresql and ollama share fix_group="bundle"; both fail initial check.
        // The batch fixer returns Err → both components must be Failed, fix_attempted=true.
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(true));
        ctx = ctx.with_checker("postgresql", make_mock_checker(false));
        ctx = ctx.with_checker("ollama",     make_mock_checker(false));
        ctx = ctx.with_fixer  ("postgresql", make_mock_fixer(false)); // batch fixer fails
        ctx = ctx.with_fixer  ("ollama",     make_mock_fixer(false));
        for spec in COMPONENTS {
            if !["homebrew","postgresql","ollama"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
        let ol = report.gates.iter().find(|g| g.id == "ollama").unwrap();
        assert!(matches!(pg.status, GateStatus::Failed { .. }), "postgresql should be Failed when batch fixer fails");
        assert!(matches!(ol.status, GateStatus::Failed { .. }), "ollama should be Failed when batch fixer fails");
        assert!(pg.fix_attempted, "fix_attempted should be true (batch fixer ran)");
        assert!(ol.fix_attempted, "fix_attempted should be true (batch fixer ran)");
    }

    // ── Integration test 15: all_ok false when component unfixable ────────────

    #[test]
    fn all_ok_false_when_component_fix_fails() {
        // postgresql fails and its fixer fails too → all_ok must be false,
        // blocked_on must be None (no human action, just a failed fix).
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(true));
        ctx = ctx.with_checker("postgresql", make_mock_checker(false));
        ctx = ctx.with_fixer  ("postgresql", make_mock_fixer(false));
        for spec in COMPONENTS {
            if !["homebrew","postgresql"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        assert!(!report.all_ok, "all_ok must be false when any component cannot be fixed");
        assert!(report.blocked_on.is_none(), "blocked_on should be None (no human action)");
    }

    // ── Integration test 16: Gate event count ────────────────────────────────
    // The engine emits exactly 2 Gate events per component in the all-ok path:
    // one Checking and one Ready (no fixing events).

    #[test]
    fn all_ok_emits_exactly_two_gate_events_per_component() {
        let gate_events: Arc<std::sync::Mutex<Vec<String>>> =
            Arc::new(std::sync::Mutex::new(vec![]));
        let cb = Arc::clone(&gate_events);
        let engine = BootstrapEngine::new(Arc::new(all_ok_ctx()));
        engine.check_and_fix(move |e| {
            if let ProgressEvent::Gate { id, .. } = &e {
                cb.lock().unwrap().push(id.clone());
            }
        });
        let ev = gate_events.lock().unwrap();
        // Each of the 10 components should emit exactly 2 Gate events
        for spec in COMPONENTS {
            let count = ev.iter().filter(|id| id.as_str() == spec.id).count();
            assert_eq!(count, 2, "component '{}' should emit exactly 2 Gate events (Checking + Ready)", spec.id);
        }
    }

    // ── Integration test 17: check_error preserved on gate report ─────────────

    #[test]
    fn check_error_preserved_on_gate_report() {
        // After a failed check, the check_error field on GateReport should be populated
        // even if the fix is attempted.
        let mut ctx = make_ctx();
        ctx = ctx.with_checker("homebrew",   make_mock_checker(true));
        ctx = ctx.with_checker("postgresql", make_mock_checker(false)); // always fails
        ctx = ctx.with_fixer  ("postgresql", make_mock_fixer(false));
        for spec in COMPONENTS {
            if !["homebrew","postgresql"].contains(&spec.id) {
                ctx = ctx.with_checker(spec.id, make_mock_checker(true));
            }
        }
        let report = BootstrapEngine::new(Arc::new(ctx)).check_and_fix(|_| {});
        let pg = report.gates.iter().find(|g| g.id == "postgresql").unwrap();
        assert!(
            pg.check_error.is_some(),
            "check_error should be preserved for diagnosis: {:?}", pg
        );
    }
}
