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
//!   the `reconcile` row in `sensei.schedules` (seeded 300s).
//! - **Idempotent / non-fatal** — a re-scan of an unchanged tree is a cheap
//!   stat-only no-op; every DB/enqueue failure is logged and swallowed.
//! - **Overlap-guarded** — a tick is skipped when a `ScanRoot` is already in
//!   flight, so reconciles never stack.
//! - **Watermarked** — the last run is persisted to `sensei.config`
//!   (`reconcile.last_run`). The watermark NO LONGER gates the cheap pass (the
//!   boot pass always runs); it is kept for observability and to throttle any
//!   future genuinely-expensive maintenance tier.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::ticker;
use crate::tasks::{Task, TaskKind};
use chrono::Utc;

/// `sensei.config` key holding the last reconcile-scan run (epoch millis).
const LAST_RUN_KEY: &str = "reconcile.last_run";

/// `sensei.config` key for the watcher-stall threshold (seconds). A live watch
/// thread that has delivered NO fs event for longer than this is *suspected*
/// stalled — the watchdog then forces a reconcile + re-establishes the stream.
/// Configurable so a large idle tree can widen it if the periodic restart is
/// unwanted.
const WATCHER_STALL_KEY: &str = "watcher.stall_secs";
/// 30 minutes. The 2026-07-13 incident froze silently for ~5h; catching within
/// one stall window + one reconcile tick (~35 min worst case) is the point.
/// Floored at 60s so a pathological config can't force a restart storm.
const DEFAULT_WATCHER_STALL_SECS: i64 = 1800;

/// Resolve the watcher-stall threshold (seconds) from config, flooring at 60s
/// and falling back to the default for missing / unparseable / too-small values.
fn parse_stall_secs(cfg: Option<String>) -> i64 {
    cfg.and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n >= 60)
        .unwrap_or(DEFAULT_WATCHER_STALL_SECS)
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
        tracing::info!(
            roots = enqueued,
            "reconcile_scheduler: cheap re-scan enqueued (watcher safety net)"
        );
    }
    // Non-fatal: log on failure; the in-memory cadence keeps running.
    if let Err(e) = pg.set_config(LAST_RUN_KEY, &now_ms.to_string()).await {
        tracing::warn!(error = %e, "reconcile_scheduler: persisting last_run watermark failed");
    }
    enqueued
}

/// Watcher liveness watchdog (P1): make a stalled/dead/degraded fs-watcher LOUD
/// and self-correcting instead of silently frozen (the 2026-07-13 incident).
///
/// Reads the lock-free [`WatcherHealth`] snapshot off the `RootWatcher` singleton
/// (no await under the mutex) and decides via the pure
/// [`crate::watcher::root_watcher::watcher_is_stalled`]. On trouble it:
/// 1. WARNs (definitive dead/errored stream) or INFOs (quiet/idle stream) —
///    once per episode, gated by the `healthy` flag so it never spams;
/// 2. marks the watcher unhealthy so `/api/watcher/status` shows it;
/// 3. forces a reconcile (reusing [`enqueue_reconcile_scans`], overlap-guarded —
///    a no-op if this tick's [`reconcile_tick`] already enqueued one);
/// 4. re-establishes the FSEvents stream via `RootWatcher::start()`, which resets
///    the stall clock so an idle repo restarts at most once per stall window.
///
/// Non-fatal: every failure path is logged, never propagated.
async fn watcher_watchdog(queue: &Arc<TaskQueue>, pg: &PgStore, now_ms: i64, stall_ms: i64) {
    use crate::watcher::root_watcher::{self, RootWatcher};
    let w_mutex = RootWatcher::instance(queue.clone());

    // Snapshot health + roots WITHOUT holding the std mutex across an await.
    let (health, roots_registered) = match w_mutex.lock() {
        Ok(w) => (w.health(), w.roots().len()),
        Err(_) => {
            tracing::warn!("watcher_watchdog: RootWatcher mutex poisoned; skipping health check");
            return;
        }
    };

    // No roots registered ⇒ nothing to watch, nothing to police.
    if roots_registered == 0 {
        return;
    }

    let alive = health.thread_alive();
    let stream_ok = health.stream_healthy();
    let last = health.last_event_at_ms();
    let definitive = !alive || !stream_ok;
    let stalled = root_watcher::watcher_is_stalled(last, now_ms, stall_ms, alive);

    if !definitive && !stalled {
        // Healthy: clear any prior degraded verdict, once.
        if !health.healthy() {
            health.set_healthy(true);
            tracing::info!("watcher_watchdog: watcher recovered — events flowing again");
        }
        return;
    }

    // Log once per episode (the healthy→degraded transition). A definitive
    // failure (dead thread / errored callback) is a WARN; a merely-quiet stream
    // (idle and silently-dead are indistinguishable) is a softer INFO because the
    // corrective restart is harmless when the tree is simply idle.
    if health.healthy() {
        if definitive {
            tracing::warn!(
                thread_alive = alive,
                stream_healthy = stream_ok,
                last_event_at_ms = last,
                stall_ms,
                "watcher_watchdog: watch thread dead/degraded — forcing reconcile + restart",
            );
        } else {
            tracing::info!(
                last_event_at_ms = last,
                stall_ms,
                "watcher_watchdog: no fs events past the stall window — forcing reconcile + re-establishing stream",
            );
        }
    }
    health.set_healthy(false);

    // Force a reconcile so drift converges now (overlap-guarded, cheap no-op).
    // Usually a no-op because this tick's reconcile_tick already enqueued one.
    if !queue.has_pending_kind(TaskKind::ScanRoot).await {
        let n = enqueue_reconcile_scans(queue, pg).await;
        if n > 0 {
            tracing::info!(roots = n, "watcher_watchdog: forced reconcile enqueued");
        }
    }

    // Re-establish the stream. start() tears down the old thread and spawns a
    // fresh one; on_thread_start resets the stall clock + healthy flag.
    match w_mutex.lock() {
        Ok(mut w) => {
            // Ensure the store survives a restart even if boot wiring was skipped.
            w.set_store(pg.clone());
            match w.start() {
                Ok(()) => tracing::info!("watcher_watchdog: watcher restarted"),
                Err(e) => tracing::warn!(error = %e, "watcher_watchdog: watcher restart failed"),
            }
        }
        Err(_) => tracing::warn!("watcher_watchdog: RootWatcher mutex poisoned; cannot restart"),
    }
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let stall_ms = parse_stall_secs(pg.get_config(WATCHER_STALL_KEY).await.ok().flatten()) * 1000;

    // THE BOOT RECONCILE ALWAYS RUNS — drift-safety over storm-guard: a restart
    // must never leave the index stale. That is a guarantee, not a cadence, so it
    // happens HERE rather than inside the schedule, which would skip it as
    // not-due after a quick restart. The cheap mtime-gated pass is near-free, so
    // running it unconditionally costs nothing.
    let boot_ms = Utc::now().timestamp_millis();
    reconcile_tick(&queue, &pg, boot_ms).await;
    watcher_watchdog(&queue, &pg, boot_ms, stall_ms).await;

    // From here the schedule owns the cadence (name `reconcile`).
    let store = pg.clone();
    ticker::run_scheduled(pg, "reconcile", move || {
        let (queue, pg) = (queue.clone(), store.clone());
        async move {
            let now_ms = Utc::now().timestamp_millis();
            reconcile_tick(&queue, &pg, now_ms).await;
            // The watchdog rides the same cadence, so a watcher stall is caught
            // within ~one pass of the stall window.
            watcher_watchdog(&queue, &pg, now_ms, stall_ms).await;
            Ok(())
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_stall_secs_falls_back_and_floors() {
        assert_eq!(parse_stall_secs(None), DEFAULT_WATCHER_STALL_SECS);
        assert_eq!(parse_stall_secs(Some("nope".into())), DEFAULT_WATCHER_STALL_SECS);
        // Below the 60s floor → default (guards a restart storm).
        assert_eq!(parse_stall_secs(Some("5".into())), DEFAULT_WATCHER_STALL_SECS);
        assert_eq!(parse_stall_secs(Some("60".into())), 60);
        assert_eq!(parse_stall_secs(Some("  900 ".into())), 900);
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

        // A run THIS INSTANT is not "due" by the schedule predicate that now
        // owns that question — so if the boot pass went through the schedule it
        // would be skipped, which is exactly what left the index stale.
        let now = Utc::now().timestamp_millis();
        pg.set_config(LAST_RUN_KEY, &now.to_string()).await.unwrap();
        assert!(
            !crate::tasks::schedule::is_due(Utc::now(), Some(Utc::now()), 300),
            "guard sanity: a run this instant is not 'due'"
        );

        // The boot pass runs regardless — it is called before run_scheduled.
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
