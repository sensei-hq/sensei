//! Metrics scheduler (Phase 4, repo-grain watermark engine).
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler`] /
//! [`crate::tasks::reconcile_scheduler`]) that wakes on an interval and enqueues
//! ONE [`TaskKind::ComputeProjectMetrics`] per project (project id in
//! `task.folder_path`).
//!
//! The per-project parent ([`crate::tasks::handlers::metrics::compute_project`])
//! then owns the whole compute graph — it freezes a single `as_of`, enqueues one
//! [`TaskKind::ComputeGroupMetrics`] child per active base group, and enqueues the
//! [`TaskKind::ComputeHealth`] barrier `blocked_by` those children. Each group
//! child schedules + seals its own days against the per-(repo × group)
//! `sensei.metric_watermarks` cursor (day-keyed groups backfill unsealed days +
//! reopen the trailing window; snapshot groups compute today-only). So a run is
//! "enqueue the project wave → each group computes today + any gap → seal".
//!
//! The active base `task_name`s from the registry ([`PgStore::active_task_names`])
//! — with the health barrier's own name ([`HEALTH_TASK_NAME`]) filtered out — gate
//! the pass: an empty base set means there is nothing to plan (honest-empty). This
//! filter lives in [`base_task_names`] so the scheduler doesn't enqueue a wave when
//! the only "active" metric is the health barrier itself; the parent re-derives the
//! same base set to decide its children.
//!
//! Cadence + honesty:
//! - **No global clock** — coverage is now the per-(repo × group)
//!   `sensei.metric_watermarks` cursor (spec §5), NOT a daemon-wide `last_run`
//!   timestamp. The tick simply enqueues the `ComputeProjectMetrics` wave every
//!   interval (overlap-guarded so it never stacks); because the watermark seals
//!   settled days, a re-tick recomputes only today + any gap. There is nothing to
//!   persist across a restart: a rebooted daemon re-derives its work from the
//!   watermark rows, so it never recomputes the whole history and never skips a
//!   day either.
//! - **Overlap-guarded** — a tick enqueues nothing while a `ComputeProjectMetrics`
//!   wave (a prior tick or the boot/on-demand backfill) is still in flight
//!   (`has_pending_kind`), so waves never stack.
//! - **Fail-closed** — a DB read failure (`active_task_names` / project list)
//!   propagates out of the tick; `run` logs it and retries next tick. A failure is
//!   never turned into an empty "success" that would look like "no metrics to
//!   compute". Per-day/per-group compute failures are held by the group child's own
//!   watermark (it does not advance on error), never here.
//! - **Honest-empty** — no active base metrics, or no projects, enqueues nothing.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::tasks::handlers::metrics::HEALTH_TASK_NAME;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// Wake hourly by default. There is no per-run watermark any more — the tick just
/// enqueues the `ComputeProjectMetrics` wave (overlap-guarded), and each group
/// child's `metric_watermarks` cursor decides which days to (re)compute. A more
/// frequent tick just tightens how soon after a day boundary today is picked up.
/// Configurable via `metrics.interval_secs`.
const DEFAULT_INTERVAL_SECS: u64 = 3600;
/// The rolling window (days) the per-group computers measure over, and the
/// trailing window the day-keyed groups reopen for late data. Read here for
/// observability (and reused by the per-group computers / planner via the same key
/// + parser). Configurable via `metrics.window_days`.
const DEFAULT_WINDOW_DAYS: u32 = 14;

/// `sensei.config` keys for the scheduler knobs.
const INTERVAL_KEY: &str = "metrics.interval_secs";
const WINDOW_DAYS_KEY: &str = "metrics.window_days";

/// Resolve the tick interval (seconds) from config, falling back to the default
/// for missing / unparseable / zero values.
fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// Resolve the compute window (days) from config, falling back to the default for
/// missing / unparseable / zero values.
fn parse_window_days(cfg: Option<String>) -> u32 {
    cfg.and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_WINDOW_DAYS)
}

/// The base metric groups whose presence gates a pass: the ACTIVE registry's
/// `task_name`s minus the health barrier's own name (`health` is computed by the
/// separate `ComputeHealth` kind, enqueued by the per-project parent — never as a
/// base group). Kept pure so the filter is unit-testable.
fn base_task_names(active: Vec<String>) -> Vec<String> {
    active.into_iter().filter(|t| t != HEALTH_TASK_NAME).collect()
}

/// Enqueue the metrics pass: exactly ONE [`TaskKind::ComputeProjectMetrics`] per
/// project (project id in `folder_path`, empty `path`). The per-project parent
/// ([`crate::tasks::handlers::metrics::compute_project`]) then owns the whole graph
/// — it freezes one `as_of`, fans out one `ComputeGroupMetrics` child per active
/// base group, and enqueues the `ComputeHealth` barrier. Returns the number of
/// tasks enqueued.
///
/// Extracted (like [`crate::tasks::analyzer_scheduler`]'s `enqueue_due_project`) so
/// the enqueue contract is unit-testable against a real queue without driving
/// `run`. Honest-empty: an empty `task_names` (no active base metrics) or empty
/// `project_ids` enqueues nothing — there is nothing to plan.
///
/// Overlap-guarded via [`TaskQueue::has_pending_kind`] (like [`enqueue_backfill_all`]):
/// if a `ComputeProjectMetrics` wave is already in flight — e.g. a
/// first-install/boot backfill — the tick enqueues nothing rather than stacking a
/// second per-project wave on top of it.
async fn enqueue_metrics_pass(
    queue: &TaskQueue,
    project_ids: &[uuid::Uuid],
    task_names: &[String],
) -> u32 {
    if task_names.is_empty() {
        return 0; // no active base metrics → nothing to plan for anyone
    }
    if queue.has_pending_kind(TaskKind::ComputeProjectMetrics).await {
        // A wave (boot backfill or a prior tick) is still pending/blocked/running
        // — don't stack a second. The in-flight wave already covers every project.
        return 0;
    }
    enqueue_project_wave(queue, project_ids).await
}

/// Enqueue one [`TaskKind::ComputeProjectMetrics`] per project (project id in
/// `folder_path`, empty `path`). The shared enqueue core of the interval pass and
/// the backfill wave (so the "one project-parent per project" shape lives in ONE
/// place). Returns the number enqueued.
async fn enqueue_project_wave(queue: &TaskQueue, project_ids: &[uuid::Uuid]) -> u32 {
    let mut enqueued = 0u32;
    for pid in project_ids {
        queue
            .enqueue(Task::new(TaskKind::ComputeProjectMetrics, &pid.to_string(), ""))
            .await;
        enqueued += 1;
    }
    enqueued
}

/// Enqueue a one-time full wave — one [`TaskKind::ComputeProjectMetrics`] per
/// project — for first-install / boot and the manual `POST /api/metrics/backfill`
/// re-plan. Overlap-guarded via [`TaskQueue::has_pending_kind`] so it never stacks a
/// second wave while one is still in flight. Returns the number enqueued (`0` when a
/// wave is already pending, or there are no projects). Reads the project set via
/// [`PgStore::list_projects`] and propagates a read failure as `Err` — never a masked
/// empty success that would read as "no projects to backfill".
///
/// A backfill is just a normal wave: the per-(repo × group) watermarks decide what
/// actually recomputes. An unset watermark fills the full history (min_date fill); a
/// sealed one recomputes only today + the trailing window. A "reset the watermark"
/// re-plan (out-of-window backdated data) is a separate concern handled by the
/// group children / the manual re-plan path, not by this scheduler.
pub async fn enqueue_backfill_all(queue: &TaskQueue, pg: &PgStore) -> Result<u32, String> {
    if queue.has_pending_kind(TaskKind::ComputeProjectMetrics).await {
        // A wave is already pending/blocked/running — don't stack a second.
        return Ok(0);
    }
    let project_ids: Vec<uuid::Uuid> = pg
        .list_projects()
        .await?
        .iter()
        .filter_map(|p| crate::api::util::json_uuid(&p["id"]))
        .collect();
    let enqueued = enqueue_project_wave(queue, &project_ids).await;
    tracing::info!(
        projects = project_ids.len(),
        enqueued,
        "metrics backfill: ComputeProjectMetrics enqueued for all projects",
    );
    Ok(enqueued)
}

/// One metrics tick: read the active base `task_name`s + the project set, then
/// enqueue the project wave. DB reads propagate `Err` (never masked into a fake
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
    // per-group computers read the same key via [`parse_window_days`].
    let window = window_days(pg).await;

    let enqueued = enqueue_metrics_pass(queue, &project_ids, &task_names).await;
    tracing::info!(
        projects = project_ids.len(),
        groups = task_names.len(),
        window_days = window,
        enqueued,
        "metrics_scheduler: compute wave enqueued",
    );
    Ok(enqueued)
}

/// Resolve the compute window (days) from config — the shared read the scheduler
/// AND the per-group computers use (e.g.
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
    tracing::info!(interval_secs = secs, "metrics_scheduler: started");
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    loop {
        // First tick fires immediately → a freshly booted daemon enqueues a wave;
        // the per-(repo × group) watermarks then decide what actually recomputes
        // (full history on an unset cursor, today + gap on a sealed one). Every tick
        // is overlap-guarded, so a still-in-flight wave never stacks a second.
        ticker.tick().await;
        // Fail-closed: a DB read failure propagates as Err — log it and retry on the
        // next tick. It is NEVER turned into an empty "success" that would read as
        // "no metrics to compute". There is no watermark to advance/hold here; the
        // per-group children own their own fail-closed watermark.
        if let Err(e) = metrics_tick(&queue, &pg).await {
            tracing::warn!(error = %e, "metrics_scheduler: tick failed — will retry next tick");
        }
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
    fn base_task_names_drops_the_health_barrier_name() {
        // `health` is the ComputeHealth kind's name, not a base group — the
        // scheduler must not gate/enqueue a compute for it.
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
    async fn metrics_scheduler_enqueues_one_compute_project_metrics_per_project() {
        // Given 2 projects and a non-empty active base registry, one pass enqueues
        // EXACTLY one ComputeProjectMetrics per project (project id in `folder_path`,
        // empty `path`) and nothing else — the per-project parent owns the rest of
        // the graph (frozen as_of, per-group children, the health barrier).
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let task_names = vec!["session_outcomes".to_string(), "churn".to_string()];

        let enqueued = enqueue_metrics_pass(&queue, &[p1, p2], &task_names).await;
        assert_eq!(enqueued, 2, "exactly one ComputeProjectMetrics per project");

        // All ComputeProjectMetrics are immediately runnable (no cross-project blocking).
        let status = queue.status().await;
        assert_eq!(status.pending, 2, "2 ComputeProjectMetrics pending (one per project)");
        assert_eq!(status.blocked, 0, "the scheduler enqueues no barriers itself");

        // Per project: exactly one ComputeProjectMetrics, and no
        // ComputeGroupMetrics/ComputeHealth (those are the parent's job now).
        let snap = queue.snapshot().await;
        for pid in [p1, p2] {
            let owner = pid.to_string();
            let parents = snap
                .iter()
                .filter(|(k, f, p)| {
                    *k == TaskKind::ComputeProjectMetrics && *f == owner && p.is_empty()
                })
                .count();
            assert_eq!(parents, 1, "exactly one ComputeProjectMetrics per project");
        }
        assert!(
            !snap.iter().any(|(k, _, _)| {
                *k == TaskKind::ComputeGroupMetrics || *k == TaskKind::ComputeHealth
            }),
            "the scheduler enqueues ONLY ComputeProjectMetrics — group computes + the health barrier are the parent's",
        );
    }

    #[tokio::test]
    async fn metrics_pass_is_guarded_against_a_second_wave() {
        // The pass is overlap-guarded like the backfill wave — a
        // ComputeProjectMetrics already in flight means the pass enqueues nothing,
        // so a first-install/boot backfill and a scheduler tick can't double-enqueue
        // the per-project parents.
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        // A wave is already pending (e.g. the boot backfill).
        queue
            .enqueue(Task::new(TaskKind::ComputeProjectMetrics, &uuid::Uuid::new_v4().to_string(), ""))
            .await;
        let task_names = vec!["session_outcomes".to_string()];

        let enqueued = enqueue_metrics_pass(&queue, &[p1], &task_names).await;
        assert_eq!(enqueued, 0, "a pending ComputeProjectMetrics guards the pass against a second wave");
        assert_eq!(queue.status().await.pending, 1, "still just the one pre-existing wave — none stacked");
    }

    #[tokio::test]
    async fn empty_active_registry_enqueues_nothing() {
        // Honest-empty: no active base metrics → nothing to plan for any project.
        let queue = TaskQueue::with_max_repos(64);
        let p1 = uuid::Uuid::new_v4();
        let enqueued = enqueue_metrics_pass(&queue, &[p1], &[]).await;
        assert_eq!(enqueued, 0);
        let status = queue.status().await;
        assert_eq!(status.pending, 0);
        assert_eq!(status.blocked, 0);
    }

    #[tokio::test]
    async fn backfill_enqueues_one_parent_per_project() {
        // The boot / on-demand backfill enqueues one ComputeProjectMetrics per
        // project (empty path) and nothing else — the same per-project shape as the
        // interval pass.
        let ctx = crate::tasks::test_support::make_ctx().await;
        let enqueued = enqueue_backfill_all(&ctx.queue, ctx.pg()).await.unwrap();
        let snap = ctx.queue.snapshot().await;
        let parents = snap
            .iter()
            .filter(|(k, _, _)| *k == TaskKind::ComputeProjectMetrics)
            .count() as u32;
        assert_eq!(parents, enqueued, "every enqueued task is a ComputeProjectMetrics (one per project)");
        assert!(
            snap.iter().all(|(k, _, p)| *k == TaskKind::ComputeProjectMetrics && p.is_empty()),
            "backfill enqueues ONLY ComputeProjectMetrics with an empty path",
        );
    }

    #[tokio::test]
    async fn backfill_is_guarded_against_a_second_wave() {
        // Dedupe guard: a ComputeProjectMetrics already in flight means the backfill
        // enqueues nothing (never stacks a second wave). Deterministic regardless of
        // the shared test DB's project count.
        let ctx = crate::tasks::test_support::make_ctx().await;
        ctx.queue
            .enqueue(Task::new(TaskKind::ComputeProjectMetrics, &uuid::Uuid::new_v4().to_string(), ""))
            .await;
        let enqueued = enqueue_backfill_all(&ctx.queue, ctx.pg()).await.unwrap();
        assert_eq!(enqueued, 0, "a pending ComputeProjectMetrics guards against a second backfill wave");
    }
}
