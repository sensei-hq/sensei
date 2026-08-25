//! Daemon-side orchestration: merges the DB-backed capture-freshness check into
//! each adapter's pure config_health, runs the hourly watchdog with a circuit
//! breaker, and fires notifications. The pure parts (config keys, tick policy)
//! are unit-tested; the loop is thin glue.

use crate::assistants::health::{AdapterCheck, AdapterHealth, CheckStatus, capture_freshness};
use crate::db::pg_store::PgStore;
use crate::notifications::{Notifier, NotifyLevel};
use std::collections::HashMap;
use std::sync::Mutex;

pub const DEFAULT_WINDOW_HOURS: f64 = 24.0;
pub const DEFAULT_EXCLUDE_WEEKENDS: bool = true;

/// Assistant family that captures hook events into the DB (and thus gets the
/// freshness check). Matches `ClaudeCodeAssistant::family()` and the
/// `assistant_family` enum value written by `ingest_hook_event`.
const CLAUDE_FAMILY: &str = "claude";

#[derive(Debug, Clone, Copy)]
pub struct CaptureWindow {
    pub hours: f64,
    pub exclude_weekends: bool,
}

impl Default for CaptureWindow {
    fn default() -> Self {
        Self { hours: DEFAULT_WINDOW_HOURS, exclude_weekends: DEFAULT_EXCLUDE_WEEKENDS }
    }
}

/// Parse the two config strings into a CaptureWindow, falling back to defaults
/// on missing/garbage values. Pure — unit-tested without a DB.
pub fn parse_window(hours: Option<&str>, exclude_weekends: Option<&str>) -> CaptureWindow {
    CaptureWindow {
        // Reject NaN AND infinities — `"inf"` parses to f64::INFINITY and would
        // silently make the staleness check a permanent no-op (everything is
        // `<= INFINITY`). Only a finite, positive window is meaningful.
        hours: hours
            .and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|h| h.is_finite() && *h > 0.0)
            .unwrap_or(DEFAULT_WINDOW_HOURS),
        exclude_weekends: exclude_weekends
            .and_then(|s| s.trim().parse::<bool>().ok())
            .unwrap_or(DEFAULT_EXCLUDE_WEEKENDS),
    }
}

/// Load the capture window from sensei.config (keys
/// `capture.max_inactivity_hours`, `capture.exclude_weekends`). A DB read error
/// is logged (not swallowed silently — a missing key and an unreachable DB must
/// be distinguishable in the log) and falls back to defaults.
pub async fn load_window(pg: &PgStore) -> CaptureWindow {
    let hours = pg.get_config("capture.max_inactivity_hours").await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "capture watchdog: read capture.max_inactivity_hours failed");
        None
    });
    let weekends = pg.get_config("capture.exclude_weekends").await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "capture watchdog: read capture.exclude_weekends failed");
        None
    });
    parse_window(hours.as_deref(), weekends.as_deref())
}

/// Whether the watchdog should monitor an adapter: it's in the user's opt-in
/// list (`SenseiLocalConfig.configured_assistants`, persisted by `configure()`).
/// Pure so it's unit-testable. NOTE: we deliberately scope on the durable opt-in
/// list, NOT live `is_configured()` — otherwise a WIPED plugin reads
/// `configured=false` and gets skipped, so the watchdog could never detect or
/// repair the exact breakage it exists to catch.
fn is_opted_in(adapter_id: &str, opted_in: &[String]) -> bool {
    opted_in.iter().any(|id| id == adapter_id)
}

/// Compute health for every adapter the user opted into, appending the DB-backed
/// `events` freshness check to the `claude` family.
pub async fn health_report(pg: &PgStore, now_ms: i64) -> Vec<AdapterHealth> {
    let window = load_window(pg).await;
    let opted_in = sensei_bootstrap::SenseiLocalConfig::load(&crate::paths::sensei_dir())
        .configured_assistants;
    let mut out = Vec::new();
    for status in crate::assistants::detect() {
        if !is_opted_in(&status.id, &opted_in) {
            continue;
        }
        let mut checks = config_health_for(&status.id);
        if status.family == CLAUDE_FAMILY {
            let last = pg.latest_hook_event_ts(CLAUDE_FAMILY).await.unwrap_or_else(|e| {
                tracing::warn!(error = %e, "capture watchdog: latest_hook_event_ts failed");
                None
            });
            checks.push(capture_freshness(last, now_ms, window.hours, window.exclude_weekends));
        }
        out.push(AdapterHealth::new(&status.id, &status.family, checks, true));
    }
    out
}

/// config_health for an adapter id, via the registry. Returns a single Unknown
/// check if the id is not in the registry (defensive).
fn config_health_for(adapter_id: &str) -> Vec<AdapterCheck> {
    crate::assistants::config_health_for_id(adapter_id).unwrap_or_else(|| {
        vec![AdapterCheck::new(
            "configured",
            "configured",
            CheckStatus::Unknown,
            Some(format!("unknown adapter {adapter_id}")),
        )]
    })
}

/// Per-adapter watchdog state. `suspended` short-circuits future ticks;
/// `stale_notified` dedups the events-stale warning so it fires once per
/// stale episode, not every hour.
#[derive(Default)]
pub struct AdapterWatch {
    pub suspended: Option<String>,
    pub stale_notified: bool,
}

pub type BreakerMap = Mutex<HashMap<String, AdapterWatch>>;

/// The config-side checks whose failure justifies an auto-reinstall.
fn config_side_failing(h: &AdapterHealth) -> bool {
    h.checks.iter().any(|c| {
        c.status == CheckStatus::Fail
            && matches!(c.id.as_str(), "marketplace" | "plugin" | "enabled" | "hooks")
    })
}

fn events_failing(h: &AdapterHealth) -> bool {
    h.checks.iter().any(|c| c.id == "events" && c.status == CheckStatus::Fail)
}

/// What a tick did — returned so the async caller (`run_sweep`) can write the
/// DB audit trail without `tick_adapter` itself needing to be async.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    Skipped,
    Healthy,
    StaleWarned,
    StaleAlreadyNotified,
    Resolved,
    Suspended,
}

/// Decide + act for one adapter. `resolve_fn` runs the reinstall for its side
/// effect; whether repair worked is judged by `recheck_fn` (the oracle), not by
/// the resolver's own report. Sync + pure of IO except via the injected
/// closures + notifier, so it's fully unit-testable; all `.await` logging
/// happens in the caller off the returned `TickOutcome`.
pub fn tick_adapter(
    health: &AdapterHealth,
    watch: &mut AdapterWatch,
    notifier: &dyn Notifier,
    resolve_fn: &dyn Fn(),
    recheck_fn: &dyn Fn() -> AdapterHealth,
) -> TickOutcome {
    if watch.suspended.is_some() {
        return TickOutcome::Skipped;
    }

    if config_side_failing(health) {
        resolve_fn();
        let after = recheck_fn();
        if config_side_failing(&after) {
            let reason = "auto-repair failed; manual action needed".to_string();
            notifier.notify(NotifyLevel::Critical,
                &format!("{} capture broken", health.family),
                &format!("Config checks still failing after reinstall. Run `sensei doctor --fix`. ({reason})"));
            watch.suspended = Some(reason);
            return TickOutcome::Suspended;
        } else {
            notifier.notify(
                NotifyLevel::Info,
                &format!("{} capture auto-resolved", health.family),
                "A config check failed and was repaired by reinstalling the plugin.",
            );
            watch.stale_notified = false;
            return TickOutcome::Resolved;
        }
    }

    if events_failing(health) {
        if !watch.stale_notified {
            notifier.notify(NotifyLevel::Warn,
                &format!("{} capture stale", health.family),
                "No hook events within the inactivity window. Config looks fine — a session restart may be needed.");
            watch.stale_notified = true;
            return TickOutcome::StaleWarned;
        }
        return TickOutcome::StaleAlreadyNotified;
    }
    watch.stale_notified = false;
    TickOutcome::Healthy
}

use std::sync::Arc;

/// Resolve a single adapter by id (re-run configure) and return the report.
/// Clears any breaker suspension for that adapter (explicit manual retry).
pub fn resolve_adapter(
    adapter_id: &str,
    breaker: &BreakerMap,
) -> crate::assistants::AdapterResolveReport {
    if let Ok(mut map) = breaker.lock() {
        let e = map.entry(adapter_id.to_string()).or_default();
        e.suspended = None;
        e.stale_notified = false;
    }
    crate::assistants::resolve_by_id(adapter_id).unwrap_or_else(|| {
        crate::assistants::AdapterResolveReport {
            adapter_id: adapter_id.to_string(),
            ok: false,
            actions: vec![],
            errors: vec![format!("unknown adapter {adapter_id}")],
        }
    })
}

/// One full sweep over configured adapters: compute health, log each verdict
/// to the DB (via `logger` → public.logs), run the tick policy (auto-resolve
/// config-fails, notify, trip breaker). Used by the hourly loop. `breaker`
/// persists circuit-breaker state across ticks.
pub async fn run_sweep(
    pg: &PgStore,
    notifier: &Arc<dyn Notifier>,
    breaker: &Arc<BreakerMap>,
    logger: &sensei_logger::Logger,
    now_ms: i64,
) {
    let report = health_report(pg, now_ms).await;
    for h in report {
        // DB audit trail (the user-requested "log in db" → public.logs via the
        // daemon logger). Warn on any non-Ok verdict so it's queryable; info
        // otherwise. Logger methods are async, so all logging stays here in the
        // async body — `tick_adapter` is sync and returns what it did.
        let summary = format!(
            "adapter {} status={:?} checks=[{}]",
            h.adapter_id,
            h.status,
            h.checks
                .iter()
                .map(|c| format!("{}:{:?}", c.id, c.status))
                .collect::<Vec<_>>()
                .join(",")
        );
        if h.status == CheckStatus::Ok {
            logger.info(&summary, None).await;
        } else {
            logger.warn(&summary, None).await;
        }

        // Pull state out under the lock, run the (sync) policy, write back.
        let mut watch = {
            let mut map = breaker.lock().unwrap();
            std::mem::take(map.entry(h.adapter_id.clone()).or_default())
        };
        let id = h.adapter_id.clone();
        let fam = h.family.clone();
        let outcome = tick_adapter(
            &h,
            &mut watch,
            notifier.as_ref(),
            &|| {
                crate::assistants::resolve_by_id(&id);
            },
            // Re-read config health LAZILY inside the closure so the recheck
            // reflects state AFTER the reinstall. A snapshot computed before
            // tick_adapter would always equal the pre-resolve (failing) state
            // and force a spurious Suspend, making the Resolved path unreachable.
            &|| AdapterHealth::new(&id, &fam, config_health_for(&id), true),
        );
        breaker.lock().unwrap().insert(h.adapter_id.clone(), watch);

        // Refresh the keep-alive marker so Claude Code's plugin in-use sweep
        // doesn't prune the daemon-installed plugin (anthropics/claude-code#69626).
        // Runs after the tick so a just-resolved (reinstalled) plugin is marked,
        // and is refreshed hourly while healthy. No-op for non-Claude adapters.
        crate::assistants::keep_alive_by_id(&h.adapter_id);

        // Log what the policy decided (resolution / suspension is the important
        // audit signal — it means the daemon mutated the user's config or gave up).
        match outcome {
            TickOutcome::Resolved => {
                logger.warn(&format!("watchdog auto-resolved {}", h.adapter_id), None).await
            }
            TickOutcome::Suspended => {
                logger
                    .error(
                        &format!(
                            "watchdog SUSPENDED {} — auto-repair failed, manual action needed",
                            h.adapter_id
                        ),
                        None,
                        None,
                    )
                    .await
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_window_defaults_on_garbage() {
        let w = parse_window(None, None);
        assert_eq!(w.hours, 24.0);
        assert!(w.exclude_weekends);
        let w2 = parse_window(Some("abc"), Some("nope"));
        assert_eq!(w2.hours, 24.0);
        assert!(w2.exclude_weekends);
    }
    #[test]
    fn is_opted_in_matches_only_listed_ids() {
        let opted = vec!["claude-code".to_string(), "cursor".to_string()];
        assert!(is_opted_in("claude-code", &opted));
        assert!(is_opted_in("cursor", &opted));
        assert!(!is_opted_in("zed", &opted));
        assert!(!is_opted_in("claude-code", &[]));
    }

    #[test]
    fn parse_window_reads_values() {
        let w = parse_window(Some("6"), Some("false"));
        assert_eq!(w.hours, 6.0);
        assert!(!w.exclude_weekends);
    }
    #[test]
    fn parse_window_rejects_invalid_hours() {
        assert_eq!(parse_window(Some("0"), None).hours, 24.0);
        assert_eq!(parse_window(Some("-3"), None).hours, 24.0);
        // Infinities parse as f64 but would disable staleness detection — reject.
        assert_eq!(parse_window(Some("inf"), None).hours, 24.0);
        assert_eq!(parse_window(Some("infinity"), None).hours, 24.0);
        assert_eq!(parse_window(Some("NaN"), None).hours, 24.0);
    }

    struct Rec(Mutex<Vec<(NotifyLevel, String)>>);
    impl Notifier for Rec {
        fn notify(&self, l: NotifyLevel, t: &str, _b: &str) {
            self.0.lock().unwrap().push((l, t.into()));
        }
    }
    fn health(checks: Vec<(&str, CheckStatus)>) -> AdapterHealth {
        let cs = checks.into_iter().map(|(id, s)| AdapterCheck::new(id, id, s, None)).collect();
        AdapterHealth::new("claude-code", "claude", cs, true)
    }

    #[test]
    fn config_fail_then_resolve_ok_notifies_info_and_stays_active() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let bad = health(vec![("plugin", CheckStatus::Fail), ("events", CheckStatus::Ok)]);
        let good = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Ok)]);
        tick_adapter(&bad, &mut w, &rec, &|| {}, &|| good.clone());
        assert!(w.suspended.is_none());
        assert_eq!(rec.0.lock().unwrap()[0].0, NotifyLevel::Info);
    }

    #[test]
    fn config_fail_then_still_fail_notifies_critical_and_suspends() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let bad = health(vec![("plugin", CheckStatus::Fail)]);
        tick_adapter(&bad, &mut w, &rec, &|| {}, &|| bad.clone());
        assert!(w.suspended.is_some());
        assert_eq!(rec.0.lock().unwrap()[0].0, NotifyLevel::Critical);
    }

    #[test]
    fn suspended_adapter_is_skipped() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch { suspended: Some("x".into()), stale_notified: false };
        let bad = health(vec![("plugin", CheckStatus::Fail)]);
        tick_adapter(&bad, &mut w, &rec, &|| panic!("must not resolve"), &|| bad.clone());
        assert!(rec.0.lock().unwrap().is_empty());
    }

    #[test]
    fn pure_events_stale_warns_once_never_resolves() {
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let stale = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Fail)]);
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        let calls = rec.0.lock().unwrap();
        assert_eq!(calls.len(), 1, "stale should warn once, not every tick");
        assert_eq!(calls[0].0, NotifyLevel::Warn);
    }

    #[test]
    fn stale_then_healthy_then_stale_rewarns() {
        // Guards the stale_notified reset: a recovered-then-stale-again adapter
        // must produce a fresh Warn, not stay silent.
        let rec = Rec(Mutex::new(vec![]));
        let mut w = AdapterWatch::default();
        let stale = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Fail)]);
        let healthy = health(vec![("plugin", CheckStatus::Ok), ("events", CheckStatus::Ok)]);
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        assert!(w.stale_notified, "set after first warn");
        tick_adapter(&healthy, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        assert!(!w.stale_notified, "reset on healthy tick");
        tick_adapter(&stale, &mut w, &rec, &|| panic!("must not resolve"), &|| stale.clone());
        let calls = rec.0.lock().unwrap();
        assert_eq!(calls.len(), 2, "second stale episode must re-warn");
        assert_eq!(calls[1].0, NotifyLevel::Warn);
    }
}
