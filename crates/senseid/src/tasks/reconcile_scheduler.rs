//! Reconcile scheduler — the fs-watcher safety net (reliability).
//!
//! A long-lived tokio task (mirroring [`crate::tasks::log_pruner`] /
//! [`crate::tasks::analyzer_scheduler`]) that periodically re-scans every watch
//! root so the index CONVERGES even when the fs-watcher misses events (daemon
//! restarts + stale FSEvents streams leave gaps — file changes since the last
//! version-rescan were never picked up). It re-enqueues one `ScanRoot` per watch
//! root, reusing the whole self-healing scan pipeline:
//!
//! - `scan_root`'s reconcile re-absorbs a `standalone` root mis-scoped inside a
//!   git repo (Bug 3) and prunes/relabels stale project roots;
//! - each `ProcessGitFolder` prunes orphan nodes for files that vanished (Bug 2,
//!   via `scan::prune_vanished`) and re-indexes changed files.
//!
//! This is the SAFETY NET, not a replacement for the fs-watcher. Because the
//! change-detection is now a two-tier gate (a stat-only mtime pass that never
//! reads/hashes an unchanged file — see
//! [`crate::tasks::handlers::scan_logic::plan_reindex`]), a no-op reconcile is
//! near-free, so it can run *frequently* rather than hourly. It is:
//! - **Boot + frequent** — the first tick fires immediately (a boot reconcile
//!   that ALWAYS runs, so a restart can never leave drift), then on
//!   `reconcile.interval_secs` (default 300s).
//! - **Idempotent / non-fatal** — a re-scan of an unchanged tree is a cheap
//!   stat-only no-op; every DB/enqueue failure is logged and swallowed.
//! - **Overlap-guarded** — a tick is skipped when a `ScanRoot` is already in
//!   flight, so reconciles never stack.
//! - **Watermarked** — the last run is persisted to `sensei.config`
//!   (`reconcile.last_run`). The watermark NO LONGER gates the cheap pass (the
//!   boot pass always runs); it is kept for observability and to throttle any
//!   future genuinely-expensive maintenance tier.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// Reconcile every 5 minutes by default. The mtime-gated re-scan makes a no-op
/// pass stat-only, so a frequent cadence closes fs-watcher gaps quickly without
/// meaningful cost. Configurable via `reconcile.interval_secs`.
const DEFAULT_INTERVAL_SECS: u64 = 300;
/// `sensei.config` key holding the last reconcile-scan run (epoch millis).
const LAST_RUN_KEY: &str = "reconcile.last_run";

/// Resolve the tick interval (seconds) from config, falling back to the default
/// for missing / unparseable / zero values.
fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// True when a reconcile is "due": never run, or the interval has elapsed since
/// the last run. Pure (clock injected) so it's testable. The cheap pass no
/// longer depends on this (the boot pass always runs for drift-safety); it is
/// retained for observability — to log when a boot reconcile runs despite a
/// recent watermark — and to gate any future expensive maintenance tier.
fn due_for_reconcile(now_ms: i64, last_run_ms: Option<i64>, interval_secs: u64) -> bool {
    match last_run_ms {
        None => true,
        Some(prev) => now_ms - prev >= interval_secs as i64 * 1000,
    }
}

/// Enqueue one `ScanRoot` per registered watch root — the same task the
/// `scan_folder` API and the version-rescan use. Returns how many were enqueued.
/// Log-and-skip on a config-read failure (never fatal).
async fn enqueue_reconcile_scans(queue: &TaskQueue, pg: &PgStore) -> u32 {
    let roots = match pg.list_watch_roots().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = %e, "reconcile_scheduler: list_watch_roots failed; no roots reconciled");
            return 0;
        }
    };
    let mut enqueued = 0u32;
    for r in &roots {
        match r["path"].as_str() {
            Some(path) if !path.is_empty() => {
                queue.enqueue(Task::new(TaskKind::ScanRoot, "", path)).await;
                enqueued += 1;
            }
            _ => {}
        }
    }
    enqueued
}

/// Spawn the reconcile scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

/// One reconcile tick. Overlap-guarded: skips (returns 0) when a `ScanRoot` is
/// already in flight so reconciles never stack. Otherwise enqueues the cheap
/// mtime-gated re-scan for every watch root and records the run watermark.
/// The watermark is persisted for observability but NEVER gates the cheap pass,
/// so a boot tick always runs and a restart can't leave drift. Non-fatal — a
/// watermark write failure is logged, not propagated. Returns the count
/// enqueued.
async fn reconcile_tick(queue: &TaskQueue, pg: &PgStore, now_ms: i64) -> u32 {
    // Overlap guard: a huge tree may still be draining from the prior tick, or a
    // user-initiated scan may be running. The mtime-gated re-scan is idempotent
    // and cheap, so a skipped tick simply catches up on the next one.
    if queue.has_pending_kind(TaskKind::ScanRoot).await {
        tracing::debug!("reconcile_scheduler: a scan is already in flight — skipping this tick");
        return 0;
    }

    let enqueued = enqueue_reconcile_scans(queue, pg).await;
    if enqueued > 0 {
        tracing::info!(roots = enqueued, "reconcile_scheduler: cheap re-scan enqueued (watcher safety net)");
    }
    // Non-fatal: log on failure; the in-memory cadence keeps running.
    if let Err(e) = pg.set_config(LAST_RUN_KEY, &now_ms.to_string()).await {
        tracing::warn!(error = %e, "reconcile_scheduler: persisting last_run watermark failed");
    }
    enqueued
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = parse_interval(pg.get_config("reconcile.interval_secs").await.ok().flatten());
    tracing::info!(interval_secs = secs, "reconcile_scheduler: started (boot + frequent watcher safety net)");
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    let mut first = true;
    loop {
        ticker.tick().await; // first tick fires immediately → boot reconcile
        let now_ms = Utc::now().timestamp_millis();

        // The boot reconcile ALWAYS runs (drift-safety): a restart must never
        // leave the index stale. The old rapid-restart storm-guard is gone
        // because the cheap mtime pass is near-free. We still read the watermark
        // to log when the boot pass runs shortly after a prior run.
        if first {
            first = false;
            let last = pg.get_config(LAST_RUN_KEY).await.ok().flatten()
                .and_then(|v| v.trim().parse::<i64>().ok());
            if !due_for_reconcile(now_ms, last, secs) {
                tracing::debug!("reconcile_scheduler: boot reconcile running despite a recent watermark (drift-safety over storm-guard)");
            }
        }

        reconcile_tick(&queue, &pg, now_ms).await;
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

    /// The reconcile must be FREQUENT now (the mtime gate makes a no-op cheap),
    /// not the old hourly (3600s) cadence. 300s is 12x more frequent.
    #[test]
    fn default_interval_is_frequent_not_hourly() {
        assert_eq!(DEFAULT_INTERVAL_SECS, 300);
    }

    #[test]
    fn due_for_reconcile_never_run_or_interval_elapsed() {
        let hour = 3600u64;
        assert!(due_for_reconcile(1_000_000, None, hour), "never run → due");
        // exactly one interval later → due
        assert!(due_for_reconcile(hour as i64 * 1000, Some(0), hour));
        // half an interval later → not due (guards rapid restart)
        assert!(!due_for_reconcile(hour as i64 * 500, Some(0), hour));
    }

    #[tokio::test]
    async fn enqueue_reconcile_scans_enqueues_one_scanroot_per_root() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::with_max_repos(64);

        let tmp = tempfile::tempdir().unwrap();
        let root_path = tmp.path().to_string_lossy().to_string();
        let rid = pg
            .add_watch_root(&root_path, "reconcile_sched_root", &serde_json::json!([]))
            .await
            .unwrap();

        let enqueued = enqueue_reconcile_scans(&queue, &pg).await;
        assert!(enqueued >= 1, "at least our root should enqueue a ScanRoot");

        // Our root must be among the enqueued ScanRoot tasks.
        let mut saw_ours = false;
        for _ in 0..enqueued {
            let t = queue.next_task().await;
            assert_eq!(t.kind, TaskKind::ScanRoot);
            if t.path == root_path {
                saw_ours = true;
            }
            queue.complete(t.id).await;
        }
        assert!(saw_ours, "the reconcile scan must target our watch root");

        pg.remove_watch_root(&rid).await.unwrap();
    }

    /// Drift-safety: the boot reconcile must enqueue even when the watermark
    /// says a run happened *this instant* — the old storm-guard would have
    /// skipped it, which is exactly what left the index stale after a restart.
    #[tokio::test]
    async fn boot_reconcile_enqueues_despite_recent_watermark() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::with_max_repos(64);

        let tmp = tempfile::tempdir().unwrap();
        let root_path = tmp.path().to_string_lossy().to_string();
        let rid = pg
            .add_watch_root(&root_path, "reconcile_boot_root", &serde_json::json!([]))
            .await
            .unwrap();

        // Watermark = now → "not due" under the old storm-guard.
        let now = Utc::now().timestamp_millis();
        pg.set_config(LAST_RUN_KEY, &now.to_string()).await.unwrap();
        assert!(
            !due_for_reconcile(now, Some(now), DEFAULT_INTERVAL_SECS),
            "guard sanity: a run this instant is not 'due'"
        );

        // The cheap pass runs regardless of the watermark.
        let enqueued = reconcile_tick(&queue, &pg, now).await;
        assert!(enqueued >= 1, "boot reconcile must enqueue despite a recent watermark");

        for _ in 0..enqueued {
            let t = queue.next_task().await;
            queue.complete(t.id).await;
        }
        pg.remove_watch_root(&rid).await.unwrap();
    }

    /// Overlap guard: a tick must NOT enqueue a second reconcile batch while a
    /// `ScanRoot` is already in flight.
    #[tokio::test]
    async fn reconcile_tick_skips_when_scan_in_flight() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::with_max_repos(64);

        // A ScanRoot already pending → overlap guard trips.
        queue.enqueue(Task::new(TaskKind::ScanRoot, "", "/some/root")).await;

        let now = Utc::now().timestamp_millis();
        let enqueued = reconcile_tick(&queue, &pg, now).await;
        assert_eq!(enqueued, 0, "must not stack a reconcile while a ScanRoot is in flight");

        // Cleanup the in-flight task we injected.
        let t = queue.next_task().await;
        queue.complete(t.id).await;
    }
}
