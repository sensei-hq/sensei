//! Metrics scheduler (Phase 4).
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler`] /
//! [`crate::tasks::reconcile_scheduler`]) that wakes on an interval and, at most
//! once per `metrics.run_secs` (default daily), enqueues the metrics-compute
//! graph for every project:
//!
//! - one [`TaskKind::ComputeMetrics`] per (project, active base `task_name`) — the
//!   group rides in `task.path`, the project id in `task.folder_path`; then
//! - one [`TaskKind::ComputeHealth`] barrier per project, `blocked_by` that
//!   project's `ComputeMetrics` ids, so health is derived only after the base
//!   metrics land (Phase 6).
//!
//! The base `task_name`s come from the ACTIVE registry
//! ([`PgStore::active_task_names`]) with the health barrier's own name
//! ([`HEALTH_TASK_NAME`]) filtered out (`health` is the `ComputeHealth` kind, not
//! a base group). A genuinely-unknown group is still enqueued so the handler logs
//! it (see `handlers::metrics::compute`).
//!
//! Cadence + honesty:
//! - **Watermarked** — the last run is persisted to `sensei.config`
//!   (`metrics.last_run`), so a restart resumes the daily cadence instead of
//!   recomputing on every boot. The first tick fires immediately, so a freshly
//!   started daemon backfills if a day has elapsed.
//! - **Fail-closed** — a DB read failure (`active_task_names` / project list)
//!   propagates out of the tick; `run` logs it and retries next tick WITHOUT
//!   advancing the watermark. A failure is never turned into an empty "success"
//!   that would look like "no metrics to compute".
//! - **Honest-empty** — no active base metrics, or no projects, enqueues nothing.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use crate::db::pg_store::PgStore;
use crate::tasks::handlers::metrics::HEALTH_TASK_NAME;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// Wake hourly by default; the daily-run watermark (`metrics.run_secs`) is what
/// actually gates the compute, so a frequent tick just tightens how soon after a
/// day boundary the pass fires. Configurable via `metrics.interval_secs`.
const DEFAULT_INTERVAL_SECS: u64 = 3600;
/// Compute at most once per day by default. Configurable via `metrics.run_secs`.
const DEFAULT_RUN_SECS: i64 = 86_400;
/// The rolling window (days) the per-group computers measure over. Read here for
/// observability (and reused by the Phase 5 computers via the same key + parser).
/// Configurable via `metrics.window_days`.
const DEFAULT_WINDOW_DAYS: u32 = 14;

/// `sensei.config` keys for the persisted scheduler state / knobs.
const INTERVAL_KEY: &str = "metrics.interval_secs";
const RUN_SECS_KEY: &str = "metrics.run_secs";
const LAST_RUN_KEY: &str = "metrics.last_run";
const WINDOW_DAYS_KEY: &str = "metrics.window_days";

/// Resolve the tick interval (seconds) from config, falling back to the default
/// for missing / unparseable / zero values.
fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// Resolve the run cadence (seconds) from config, falling back to the default for
/// missing / unparseable / non-positive values.
fn parse_run_secs(cfg: Option<String>) -> i64 {
    cfg.and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_RUN_SECS)
}

/// Resolve the compute window (days) from config, falling back to the default for
/// missing / unparseable / zero values.
fn parse_window_days(cfg: Option<String>) -> u32 {
    cfg.and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_WINDOW_DAYS)
}

/// True when a metrics run is "due": never run, or the cadence has elapsed since
/// the last run. Pure (clock injected) so it's testable.
fn due_for_run(now_ms: i64, last_run_ms: Option<i64>, run_secs: i64) -> bool {
    match last_run_ms {
        None => true,
        Some(prev) => now_ms - prev >= run_secs * 1000,
    }
}

/// The watermark to carry forward after a tick — the fail-closed core of the
/// never-mask-a-failure rule, extracted pure so it is directly testable (rather
/// than true only by inspection of `run`). A successful tick (INCLUDING an
/// honest-empty `Ok(0)`) advances to `now_ms`; a failed tick HOLDS the prior
/// watermark so the next tick retries instead of skipping a day of metrics.
fn next_watermark(
    now_ms: i64,
    last_run_ms: Option<i64>,
    tick_result: &Result<u32, String>,
) -> Option<i64> {
    match tick_result {
        Ok(_) => Some(now_ms),
        Err(_) => last_run_ms,
    }
}

/// The base metric groups to enqueue as `ComputeMetrics`: the ACTIVE registry's
/// `task_name`s minus the health barrier's own name (`health` is computed by the
/// separate `ComputeHealth` kind). Kept pure so the filter is unit-testable.
fn base_task_names(active: Vec<String>) -> Vec<String> {
    active.into_iter().filter(|t| t != HEALTH_TASK_NAME).collect()
}

/// Enqueue the metrics-compute graph for one pass. For each project: one
/// `ComputeMetrics` per base group (`task_name` in `path`, project id in
/// `folder_path`), then one `ComputeHealth` barrier (project id in `folder_path`)
/// `blocked_by` that project's `ComputeMetrics` ids so health is computed only
/// after the base metrics land. Returns the number of tasks enqueued.
///
/// Extracted (like [`crate::tasks::analyzer_scheduler`]'s `enqueue_due_project`)
/// so the enqueue contract is unit-testable against a real queue without driving
/// `run`. Honest-empty: an empty `task_names` (no active base metrics) or empty
/// `project_ids` enqueues nothing.
async fn enqueue_metrics_pass(
    queue: &TaskQueue,
    project_ids: &[uuid::Uuid],
    task_names: &[String],
) -> u32 {
    if task_names.is_empty() {
        return 0; // no active base metrics → nothing to compute for anyone
    }
    let mut enqueued = 0u32;
    for pid in project_ids {
        let owner = pid.to_string();
        let mut compute_ids = Vec::with_capacity(task_names.len());
        for task_name in task_names {
            let id = queue
                .enqueue(Task::new(TaskKind::ComputeMetrics, &owner, task_name))
                .await;
            compute_ids.push(id);
            enqueued += 1;
        }
        // The per-project health barrier: blocked on THIS project's base compute
        // ids, so it runs only after they complete (Phase 6 derives the score).
        queue
            .enqueue(Task::new(TaskKind::ComputeHealth, &owner, "").blocked_by(compute_ids))
            .await;
        enqueued += 1;
    }
    enqueued
}

/// One metrics run: read the active base `task_name`s + the project set, then
/// enqueue the compute graph. DB reads propagate `Err` (never masked into a fake
/// empty success). Returns the number of tasks enqueued (0 = honest-empty).
async fn metrics_tick(queue: &TaskQueue, pg: &PgStore) -> Result<u32, String> {
    let task_names = base_task_names(pg.active_task_names().await?);
    if task_names.is_empty() {
        tracing::debug!("metrics_scheduler: no active base metrics — nothing to compute");
        return Ok(0);
    }

    let project_ids: Vec<uuid::Uuid> = pg
        .list_projects()
        .await?
        .iter()
        .filter_map(|p| crate::api::util::json_uuid(&p["id"]))
        .collect();

    // Read for observability (and to establish the config key + default). The
    // Phase 5 computers read the same key via [`parse_window_days`].
    let window = window_days(pg).await;

    let enqueued = enqueue_metrics_pass(queue, &project_ids, &task_names).await;
    tracing::info!(
        projects = project_ids.len(),
        groups = task_names.len(),
        window_days = window,
        enqueued,
        "metrics_scheduler: daily compute enqueued",
    );
    Ok(enqueued)
}

/// Resolve the compute window (days) from config — the shared read the scheduler
/// AND the Phase 5 per-group computers use (e.g.
/// [`crate::tasks::handlers::metrics::session_outcomes`]), so the config key +
/// parser + default live in exactly one place. Config-read failure isn't fatal:
/// fall back to the default and log, like the analyzer scheduler's `interval_secs`.
pub(crate) async fn window_days(pg: &PgStore) -> u32 {
    let raw = match pg.get_config(WINDOW_DAYS_KEY).await {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "metrics_scheduler: window_days config read failed — using default");
            None
        }
    };
    parse_window_days(raw)
}

/// Spawn the metrics scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = parse_interval(pg.get_config(INTERVAL_KEY).await.ok().flatten());
    let run_secs = parse_run_secs(pg.get_config(RUN_SECS_KEY).await.ok().flatten());
    tracing::info!(interval_secs = secs, run_secs, "metrics_scheduler: started");
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    // Restore the watermark so a restart resumes the daily cadence instead of
    // recomputing on every boot. A bad/absent value → None → the first tick runs
    // (a fresh daemon backfills), which is the safe default.
    let mut last_run_ms: Option<i64> = match pg.get_config(LAST_RUN_KEY).await {
        Ok(raw) => raw.and_then(|v| v.trim().parse::<i64>().ok()),
        Err(e) => {
            tracing::warn!(error = %e, "metrics_scheduler: last_run watermark read failed — treating as never-run");
            None
        }
    };
    loop {
        ticker.tick().await; // first tick fires immediately → boot backfill if due
        let now_ms = Utc::now().timestamp_millis();
        if !due_for_run(now_ms, last_run_ms, run_secs) {
            continue;
        }
        let result = metrics_tick(&queue, &pg).await;
        // Fail-closed watermark (governance-critical): advance on success —
        // including honest-empty `Ok(0)` — and HOLD on failure so the next tick
        // retries rather than skipping a day. The decision is [`next_watermark`]
        // (pure + tested); this arm only owns persistence + logging.
        let next = next_watermark(now_ms, last_run_ms, &result);
        match &result {
            Ok(_) => {
                // Persist the advanced watermark. Same restart-risk category as the
                // analyzer's watermark — log on failure so an operator can see we
                // may recompute after a reboot; the in-memory value still advances.
                if let Err(e) = pg.set_config(LAST_RUN_KEY, &now_ms.to_string()).await {
                    tracing::warn!(error = %e, "metrics_scheduler: persisting last_run watermark failed — restart may recompute");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "metrics_scheduler: tick failed — will retry next tick");
            }
        }
        last_run_ms = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_falls_back_on_missing_invalid_or_zero() {
        assert_eq!(parse_interval(None), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("nope".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("0".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("900".into())), 900);
        assert_eq!(parse_interval(Some("  1800 ".into())), 1800);
    }

    #[test]
    fn parse_run_secs_falls_back_on_missing_invalid_or_nonpositive() {
        assert_eq!(parse_run_secs(None), DEFAULT_RUN_SECS);
        assert_eq!(parse_run_secs(Some("nope".into())), DEFAULT_RUN_SECS);
        assert_eq!(parse_run_secs(Some("0".into())), DEFAULT_RUN_SECS);
        assert_eq!(parse_run_secs(Some("-5".into())), DEFAULT_RUN_SECS);
        assert_eq!(parse_run_secs(Some("43200".into())), 43_200);
    }

    #[test]
    fn parse_window_days_falls_back_on_missing_invalid_or_zero() {
        // Pin the DOCUMENTED default to the literal 14 (not just the constant) so
        // changing DEFAULT_WINDOW_DAYS to another value fails this test instead of
        // silently violating the "default 14" contract.
        assert_eq!(DEFAULT_WINDOW_DAYS, 14, "the documented metrics window default is 14 days");
        assert_eq!(parse_window_days(None), 14);
        assert_eq!(parse_window_days(Some("nope".into())), 14);
        assert_eq!(parse_window_days(Some("0".into())), 14);
        assert_eq!(parse_window_days(Some("30".into())), 30);
        assert_eq!(parse_window_days(Some("  7 ".into())), 7);
    }

    #[test]
    fn watermark_holds_on_tick_failure() {
        // Fail-closed: a failed tick keeps the prior watermark so the next tick
        // retries rather than skipping a day of metrics.
        let err: Result<u32, String> = Err("db down".into());
        assert_eq!(next_watermark(5000, Some(1000), &err), Some(1000), "Err holds the prior watermark");
        // Never-run stays never-run on failure (so the next tick is still due).
        assert_eq!(next_watermark(5000, None, &err), None, "Err never fabricates a watermark");
    }

    #[test]
    fn watermark_advances_on_success_including_honest_empty() {
        // Ok advances to now — including Ok(0): honest-empty IS success, not a
        // failure to retry.
        assert_eq!(next_watermark(5000, Some(1000), &Ok(3)), Some(5000));
        assert_eq!(next_watermark(5000, Some(1000), &Ok(0)), Some(5000), "honest-empty still advances");
        assert_eq!(next_watermark(5000, None, &Ok(0)), Some(5000), "first successful run sets the watermark");
    }

    #[test]
    fn due_for_run_never_run_or_cadence_elapsed() {
        let day = 86_400i64;
        assert!(due_for_run(1_000_000, None, day), "never run → due");
        // exactly one cadence later → due
        assert!(due_for_run(day * 1000, Some(0), day));
        // half a cadence later → not due (guards recompute-on-every-tick)
        assert!(!due_for_run(day * 500, Some(0), day));
    }

    #[test]
    fn base_task_names_drops_the_health_barrier_name() {
        // `health` is the ComputeHealth kind's name, not a base ComputeMetrics
        // group — the scheduler must not enqueue a ComputeMetrics for it.
        let active = vec![
            "session_outcomes".to_string(),
            "churn".to_string(),
            HEALTH_TASK_NAME.to_string(),
        ];
        let base = base_task_names(active);
        assert_eq!(base, vec!["session_outcomes".to_string(), "churn".to_string()]);
        assert!(!base.iter().any(|t| t == HEALTH_TASK_NAME));
    }

    #[tokio::test]
    async fn metrics_scheduler_enqueues_active_task_names_per_project() {
        // Given 2 projects and an active base registry spanning {session_outcomes,
        // churn}, one pass enqueues exactly one ComputeMetrics per (project,
        // task_name) — the task_name in `path`, the project id in `folder_path` —
        // plus one ComputeHealth per project that is blocked_by that project's
        // ComputeMetrics ids.
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let task_names = vec!["session_outcomes".to_string(), "churn".to_string()];

        let enqueued = enqueue_metrics_pass(&queue, &[p1, p2], &task_names).await;
        // 2 projects × (2 ComputeMetrics + 1 ComputeHealth) = 6.
        assert_eq!(enqueued, 6, "one ComputeMetrics per (project, task_name) + one ComputeHealth per project");

        // The health barriers are BLOCKED (each on its project's 2 compute ids),
        // the compute tasks are pending — proving blocked_by is wired.
        let status = queue.status().await;
        assert_eq!(status.pending, 4, "4 ComputeMetrics pending (2 projects × 2 groups)");
        assert_eq!(status.blocked, 2, "2 ComputeHealth barriers blocked on their project's ComputeMetrics");

        // Per project: exactly one ComputeMetrics per task_name (task_name in
        // path, project id in folder_path) and exactly one ComputeHealth.
        let snap = queue.snapshot().await;
        for pid in [p1, p2] {
            let owner = pid.to_string();
            let mut compute_paths: Vec<String> = snap
                .iter()
                .filter(|(k, f, _)| *k == TaskKind::ComputeMetrics && *f == owner)
                .map(|(_, _, p)| p.clone())
                .collect();
            compute_paths.sort();
            assert_eq!(
                compute_paths,
                vec!["churn".to_string(), "session_outcomes".to_string()],
                "one ComputeMetrics per (project, task_name), task_name carried in path",
            );
            let health = snap
                .iter()
                .filter(|(k, f, p)| *k == TaskKind::ComputeHealth && *f == owner && p.is_empty())
                .count();
            assert_eq!(health, 1, "exactly one ComputeHealth barrier per project");
        }
    }

    #[tokio::test]
    async fn empty_active_registry_enqueues_nothing() {
        // Honest-empty: no active base metrics → no tasks for any project.
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        let enqueued = enqueue_metrics_pass(&queue, &[p1], &[]).await;
        assert_eq!(enqueued, 0);
        let status = queue.status().await;
        assert_eq!(status.pending, 0);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn health_barrier_unblocks_only_after_its_project_metrics_complete() {
        // The barrier is per-project: completing p1's ComputeMetrics releases p1's
        // ComputeHealth while p2's health stays blocked on its own (still-pending)
        // metrics — proving blocked_by targets THAT project's ids.
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let task_names = vec!["session_outcomes".to_string(), "churn".to_string()];
        enqueue_metrics_pass(&queue, &[p1, p2], &task_names).await;

        // Drain all four pending ComputeMetrics, completing ONLY p1's (the rest
        // are left running so p2's deps never complete). Bounded by the four
        // compute tasks, so next_task never blocks waiting for more.
        let owner1 = p1.to_string();
        for _ in 0..4 {
            let t = queue.next_task().await;
            if t.kind == TaskKind::ComputeMetrics && t.folder_path == owner1 {
                queue.complete(t.id).await;
            }
        }

        // p1's health barrier is now released (both its metrics completed); p2's
        // is still blocked on its two outstanding (running, not completed) metrics.
        let status = queue.status().await;
        assert_eq!(status.blocked, 1, "p2's ComputeHealth stays blocked until its own metrics complete");

        // Barrier IDENTITY, not just count: the ONE unblocked task must be P1's
        // health barrier. A cross-wire (p1's health blocked on p2's ids) would
        // leave blocked==1 but release p2's health here — this catches that.
        let released = queue.next_task().await;
        assert_eq!(released.kind, TaskKind::ComputeHealth, "the released task is a health barrier");
        assert_eq!(
            released.folder_path, owner1,
            "the released barrier belongs to p1 (whose metrics completed), not p2",
        );
    }
}
