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
//! This is the SAFETY NET, not a replacement for the fs-watcher. It is:
//! - **Boot + hourly** — the first tick fires immediately (a boot reconcile),
//!   then on `reconcile.interval_secs` (default 3600s).
//! - **Idempotent / non-fatal** — a re-scan of an unchanged tree is a cheap
//!   no-op; every DB/enqueue failure is logged and swallowed.
//! - **Watermarked** — the last run is persisted to `sensei.config`
//!   (`reconcile.last_run`) so a rapid restart shortly after a run doesn't
//!   re-storm on boot; steady ticks run on cadence regardless.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// Reconcile hourly by default — conservative; the fs-watcher handles the
/// steady state, this only closes the gaps it misses.
const DEFAULT_INTERVAL_SECS: u64 = 3600;
/// `sensei.config` key holding the last reconcile-scan run (epoch millis).
const LAST_RUN_KEY: &str = "reconcile.last_run";

/// Resolve the tick interval (seconds) from config, falling back to the default
/// for missing / unparseable / zero values.
fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

/// True when a reconcile is due: never run, or the interval has elapsed since
/// the last run. Pure (clock injected) so it's testable. Used only to gate the
/// immediate boot tick against a rapid-restart storm — later ticks run on the
/// fixed cadence.
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

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = parse_interval(pg.get_config("reconcile.interval_secs").await.ok().flatten());
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    let mut first = true;
    loop {
        ticker.tick().await; // first tick fires immediately → boot reconcile
        let now_ms = Utc::now().timestamp_millis();

        // Guard the immediate boot tick against a rapid-restart storm: skip if we
        // reconciled within the last interval already. Steady ticks (interval
        // elapsed) always run.
        if first {
            first = false;
            let last = pg.get_config(LAST_RUN_KEY).await.ok().flatten()
                .and_then(|v| v.trim().parse::<i64>().ok());
            if !due_for_reconcile(now_ms, last, secs) {
                tracing::debug!("reconcile_scheduler: recent run — skipping boot reconcile");
                continue;
            }
        }

        let enqueued = enqueue_reconcile_scans(&queue, &pg).await;
        if enqueued > 0 {
            tracing::info!(roots = enqueued, "reconcile_scheduler: re-scan enqueued (watcher safety net)");
        }
        // Persist the watermark so a rapid restart doesn't re-storm on boot.
        // Non-fatal: log on failure; the in-memory cadence keeps running.
        if let Err(e) = pg.set_config(LAST_RUN_KEY, &now_ms.to_string()).await {
            tracing::warn!(error = %e, "reconcile_scheduler: persisting last_run watermark failed");
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
}
