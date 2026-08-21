//! Re-scan/re-analyze on a daemon binary-version change (D2).
//!
//! A freshly-installed daemon binary must not keep serving an index + derived
//! data that an OLDER binary built — a new binary parses code and derives
//! signals differently, so a stale graph silently misrepresents the codebase.
//! On boot we compare the running binary's version against the last version
//! that scanned this DB (persisted in `sensei.config`). When it changed we
//! re-scan every indexed root and force a full re-analysis, then record the
//! new version so the rebuild fires exactly ONCE per version change.
//!
//! Reuse (no hand-rolled scanning):
//! - **Re-scan** — one `ScanRoot` per watch root, the same task the
//!   `scan_folder` API enqueues; `ScanRoot` fans out `ProcessGitFolder` so the
//!   code graph rebuilds under the new binary.
//! - **Re-analyze** — clear the analyzer scheduler's full-refresh watermark
//!   (`analyzer.last_full_refresh`) so its next tick re-analyzes every active
//!   project via the existing daily-refresh path, rather than duplicating the
//!   per-project `AnalyzeProject` enqueue loop.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// `sensei.config` key holding the last daemon binary version that scanned +
/// analyzed this DB. Absent or differing ⇒ the index/derived data are stale.
pub const LAST_VERSION_KEY: &str = "daemon.last_version";

/// Analyzer scheduler's full-refresh watermark. Clearing it makes the next
/// scheduler tick treat every active project as due for a full re-analysis
/// (its documented daily-refresh path). Must match the key the scheduler owns.
const ANALYZER_LAST_REFRESH_KEY: &str = "analyzer.last_full_refresh";

/// Pure gate: the index is stale when we have never recorded a version
/// (`None`) or the stored version differs from the running binary's.
pub fn version_changed(stored: Option<&str>, current: &str) -> bool {
    match stored {
        None => true,
        Some(s) => s != current,
    }
}

/// Boot hook: if the binary version changed since this DB was last scanned,
/// re-scan all indexed roots and force a full re-analysis. Returns `true` when
/// it triggered a rebuild — the caller then arms [`spawn_version_commit_watcher`]
/// to record the new version once the rescan actually completes.
///
/// - **Non-fatal**: every DB/enqueue failure is logged and swallowed — a
///   version-rescan hiccup must never block daemon boot.
/// - **Crash-safe**: it deliberately does NOT write `daemon.last_version` here.
///   The version is committed only after the enqueued rescan has drained (see
///   [`spawn_version_commit_watcher`]). If the daemon aborts mid-rescan (e.g.
///   an embed GGML abort), the version stays unwritten and the next boot
///   re-triggers the rebuild instead of silently skipping it.
/// - **Idempotent** for the completed case: once the watcher records the
///   version, a restart on the same version is a no-op.
///
/// Structured as a free async fn (not inlined into boot) so it is DB-testable
/// without standing up the whole daemon.
pub async fn maybe_rescan_on_version_change(
    pg: &PgStore,
    queue: &TaskQueue,
    current_version: &str,
) -> bool {
    let stored = match pg.get_config(LAST_VERSION_KEY).await {
        Ok(v) => v,
        Err(e) => {
            // Can't read the gate ⇒ don't guess. Skip (and DON'T write the
            // version) so a genuine change still triggers on a later boot.
            tracing::warn!(error = %e, "version rescan: reading {LAST_VERSION_KEY} failed; skipping rebuild this boot");
            return false;
        }
    };

    if !version_changed(stored.as_deref(), current_version) {
        return false;
    }

    let old = stored.as_deref().unwrap_or("(none)");
    tracing::info!("sensei upgraded {old}→{current_version}: re-scanning + re-analyzing");

    // Re-scan: one ScanRoot per watch root (the trigger `scan_folder` uses).
    // Bounded — one task per root, not per file.
    match pg.list_watch_roots().await {
        Ok(roots) => {
            let mut enqueued = 0u32;
            for r in &roots {
                let Some(path) = r["path"].as_str().filter(|p| !p.is_empty()) else { continue };
                // Single-writer (D6e/W5): a version-bump rescan must not race the
                // reconcile scheduler's tick — skip a root that already has a
                // ScanRoot pending/running.
                if queue.enqueue_unique(Task::new(TaskKind::ScanRoot, "", path)).await.is_some() {
                    enqueued += 1;
                }
            }
            tracing::info!("version rescan: enqueued {enqueued} ScanRoot task(s)");
        }
        Err(e) => {
            tracing::warn!(error = %e, "version rescan: list_watch_roots failed; no roots re-scanned");
        }
    }

    // Re-analyze: reset the scheduler's full-refresh watermark so its next tick
    // re-analyzes every active project. delete is idempotent (absent ⇒ no-op).
    // Safe to clear eagerly — re-clearing on a re-triggered boot is a no-op.
    if let Err(e) = pg.delete_config(ANALYZER_LAST_REFRESH_KEY).await {
        tracing::warn!(error = %e, "version rescan: clearing analyzer full-refresh watermark failed; re-analysis waits for the daily cadence");
    }

    // NOTE: `daemon.last_version` is intentionally NOT written here. It is
    // committed by `spawn_version_commit_watcher` only after the rescan drains,
    // so an aborted rescan re-triggers next boot instead of being skipped.
    true
}

/// Arm the crash-safe version commit. After a rescan is triggered, this waits
/// (off the boot path) for the task queue to fully drain — the observable
/// signal that the enqueued rescan fan-out has completed — and only then writes
/// `daemon.last_version`.
///
/// Because the queue is in-memory, a daemon abort before the drain simply
/// leaves the version unwritten, so the next boot re-runs the rebuild. On a
/// clean completion the version is recorded and future boots on the same
/// version are no-ops. Returns the spawned handle so tests can await it.
pub fn spawn_version_commit_watcher(
    pg: Arc<PgStore>,
    queue: Arc<TaskQueue>,
    version: String,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        wait_for_scan_drain(&queue).await;
        match pg.set_config(LAST_VERSION_KEY, &version).await {
            Ok(_) => tracing::info!("version rescan: scan drained — recorded {LAST_VERSION_KEY}={version}"),
            Err(e) => tracing::warn!(error = %e, "version rescan: recording {LAST_VERSION_KEY} after drain failed; rebuild may re-trigger next boot"),
        }
    })
}

/// True when the queue holds no pending, blocked, or running tasks.
async fn queue_idle(queue: &TaskQueue) -> bool {
    let s = queue.status().await;
    s.pending == 0 && s.blocked == 0 && s.running == 0
}

/// Poll until the queue has stayed idle across a short debounce window. The
/// debounce guards against a momentary gap between fan-out levels being
/// mistaken for completion (a barrier sits in `blocked` while its file tasks
/// run, so a live scan is never fully idle — but the debounce is cheap
/// insurance). Committing after the whole queue drains is safe: the rescan has
/// certainly finished by then, and committing late only risks a cheap redundant
/// ScanRoot on an intervening boot, never data loss.
async fn wait_for_scan_drain(queue: &TaskQueue) {
    const POLL: Duration = Duration::from_secs(2);
    const DEBOUNCE: Duration = Duration::from_millis(750);
    loop {
        if queue_idle(queue).await {
            tokio::time::sleep(DEBOUNCE).await;
            if queue_idle(queue).await {
                return;
            }
        }
        tokio::time::sleep(POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn version_changed_truth_table() {
        // Never recorded → stale → rebuild.
        assert!(version_changed(None, "0.2.40"));
        // Same version → not stale → no rebuild.
        assert!(!version_changed(Some("0.2.40"), "0.2.40"));
        // Different version → stale → rebuild.
        assert!(version_changed(Some("0.2.39"), "0.2.40"));
    }

    /// Drain immediately-available tasks AND complete each so the per-folder
    /// concurrency cap never accumulates (all ScanRoots share folder_path="",
    /// so drain-without-complete would stall at the cap). Mirrors a boot whose
    /// scan runs to completion; leaves the queue idle.
    async fn drain_all(queue: &TaskQueue) -> Vec<Task> {
        let mut out = Vec::new();
        while let Ok(task) =
            tokio::time::timeout(Duration::from_millis(50), queue.next_task()).await
        {
            queue.complete(task.id).await;
            out.push(task);
        }
        out
    }

    /// Count ScanRoot tasks in a drained set targeting a specific root path.
    fn scanroots_for<'a>(tasks: &'a [Task], root_path: &str) -> Vec<&'a Task> {
        tasks
            .iter()
            .filter(|t| t.kind == TaskKind::ScanRoot && t.path == root_path)
            .collect()
    }

    /// Serialises the tests below.
    ///
    /// `LAST_VERSION_KEY` is a single production config row, not a per-test
    /// fixture, and these tests must set, read and delete it. Run concurrently
    /// against the shared test DB they raced: one asserted the key still held
    /// `0.0.0-old` while the other had already deleted it, so the read came back
    /// `None`. There is no way to scope a fixed global key per test, so the
    /// contention is serialised instead of pretended away.
    static VERSION_KEY_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

    /// End-to-end D2 contract: a version change re-scans WITHOUT committing the
    /// version; an aborted (never-committed) rescan re-triggers next boot; the
    /// commit watcher records the version only once the queue drains; and once
    /// committed the same version is a no-op. Each "boot" uses a fresh queue,
    /// mirroring the in-memory queue being recreated on every daemon start.
    #[tokio::test]
    async fn rescan_is_crash_safe_then_commits_and_is_idempotent() {
        let _serialised = VERSION_KEY_LOCK.lock().await;
        let pg = PgStore::connect_test().await.unwrap();

        // A watch root we can assert a ScanRoot targets. The shared test DB may
        // hold other roots, so we always filter observations to this path.
        let tmp = tempfile::tempdir().unwrap();
        let root_path = tmp.path().to_string_lossy().to_string();
        let rid = pg
            .add_watch_root(&root_path, "version_rescan_root", &serde_json::json!([]))
            .await
            .unwrap();

        // Simulate an old binary having scanned this DB, and a stale analyzer
        // full-refresh watermark that a rebuild must clear.
        pg.set_config(LAST_VERSION_KEY, "0.0.0-old").await.unwrap();
        pg.set_config(ANALYZER_LAST_REFRESH_KEY, "1234567890").await.unwrap();

        let current = "9.9.9-new";

        // ── Boot 1: version change triggers a rescan but does NOT record it ──
        let q1 = Arc::new(TaskQueue::with_max_repos(4096));
        assert!(
            maybe_rescan_on_version_change(&pg, &q1, current).await,
            "version change must trigger a rebuild",
        );
        assert_eq!(scanroots_for(&drain_all(&q1).await, &root_path).len(), 1, "one ScanRoot for our root");
        assert_eq!(
            pg.get_config(LAST_VERSION_KEY).await.unwrap().as_deref(),
            Some("0.0.0-old"),
            "version must NOT be recorded before the rescan completes (crash-safety)",
        );
        assert_eq!(
            pg.get_config(ANALYZER_LAST_REFRESH_KEY).await.unwrap(),
            None,
            "analyzer full-refresh watermark cleared to force re-analysis",
        );

        // ── Boot 2: the daemon 'aborted' before commit (fresh queue) → version
        //    still old → the rescan MUST re-trigger, not be silently skipped. ──
        let q2 = Arc::new(TaskQueue::with_max_repos(4096));
        assert!(
            maybe_rescan_on_version_change(&pg, &q2, current).await,
            "an uncommitted (aborted) rescan must re-trigger next boot",
        );
        assert_eq!(scanroots_for(&drain_all(&q2).await, &root_path).len(), 1, "ScanRoot re-enqueued after abort");

        // The commit watcher must hold off while work is in flight, then record
        // the version once the queue drains. Drive it with a controlled task so
        // the transition idle→busy→idle is deterministic.
        let busy = q2.enqueue(Task::new(TaskKind::ScanRoot, "", "/probe")).await;
        let _running = q2.next_task().await; // → `running`, queue not idle
        let handle =
            spawn_version_commit_watcher(Arc::new(pg.clone()), q2.clone(), current.to_string());
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(!handle.is_finished(), "watcher must block while the rescan is in-flight");
        assert_eq!(
            pg.get_config(LAST_VERSION_KEY).await.unwrap().as_deref(),
            Some("0.0.0-old"),
            "version must stay uncommitted while work is in-flight",
        );
        q2.complete(busy).await; // queue idle → watcher wakes, debounces, commits
        handle.await.unwrap();
        assert_eq!(
            pg.get_config(LAST_VERSION_KEY).await.unwrap().as_deref(),
            Some(current),
            "version recorded once the rescan drained",
        );

        // ── Boot 3: same version, now committed → no-op (idempotent). ──
        let q3 = Arc::new(TaskQueue::with_max_repos(4096));
        assert!(
            !maybe_rescan_on_version_change(&pg, &q3, current).await,
            "same version must not re-trigger once committed",
        );
        assert!(scanroots_for(&drain_all(&q3).await, &root_path).is_empty(), "no rescan on a committed version");

        // Cleanup shared-DB state.
        pg.remove_watch_root(&rid).await.unwrap();
        pg.delete_config(LAST_VERSION_KEY).await.unwrap();
    }

    #[tokio::test]
    async fn version_rescan_skips_a_root_already_scanning() {
        // Single-writer (D6e/W5): a version-bump rescan must not stack a second
        // ScanRoot for a root the reconcile tick is already scanning — the race
        // the review flagged. Without the guard the root would show 2 ScanRoots.
        let _serialised = VERSION_KEY_LOCK.lock().await;
        let pg = PgStore::connect_test().await.unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let root_path = tmp.path().to_string_lossy().to_string();
        let rid = pg
            .add_watch_root(&root_path, "version_rescan_dedup_root", &serde_json::json!([]))
            .await
            .unwrap();
        pg.set_config(LAST_VERSION_KEY, "0.0.0-old").await.unwrap();

        let q = Arc::new(TaskQueue::with_max_repos(4096));
        // Pre-seed: this root already has a ScanRoot in flight.
        q.enqueue(Task::new(TaskKind::ScanRoot, "", &root_path)).await;

        assert!(
            maybe_rescan_on_version_change(&pg, &q, "9.9.9-new").await,
            "version change still triggers a rebuild",
        );
        // Only the pre-seeded ScanRoot exists for our root — the rescan deduped.
        assert_eq!(
            scanroots_for(&drain_all(&q).await, &root_path).len(),
            1,
            "an in-flight ScanRoot is not stacked by the version rescan",
        );

        pg.remove_watch_root(&rid).await.unwrap();
        pg.delete_config(LAST_VERSION_KEY).await.unwrap();
    }
}
