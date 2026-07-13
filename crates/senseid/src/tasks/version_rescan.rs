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
/// re-scan all indexed roots, force a full re-analysis, and persist the new
/// version. Returns `true` when it triggered a rebuild.
///
/// - **Non-fatal**: every DB/enqueue failure is logged and swallowed — a
///   version-rescan hiccup must never block daemon boot.
/// - **Idempotent**: gated by the persisted `daemon.last_version`, so a
///   restart on the same version is a no-op; it fires once per version change.
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
                match r["path"].as_str() {
                    Some(path) if !path.is_empty() => {
                        queue.enqueue(Task::new(TaskKind::ScanRoot, "", path)).await;
                        enqueued += 1;
                    }
                    _ => {}
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
    if let Err(e) = pg.delete_config(ANALYZER_LAST_REFRESH_KEY).await {
        tracing::warn!(error = %e, "version rescan: clearing analyzer full-refresh watermark failed; re-analysis waits for the daily cadence");
    }

    // Persist the new version LAST: if a crash lands between the enqueue and
    // here, the (idempotent) rescan simply re-triggers next boot rather than
    // being silently skipped.
    if let Err(e) = pg.set_config(LAST_VERSION_KEY, current_version).await {
        tracing::warn!(error = %e, "version rescan: persisting {LAST_VERSION_KEY} failed; rebuild may re-trigger next boot");
    }

    true
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

    /// Drain immediately-available tasks (short per-call timeout so a test can
    /// never hang), mirroring the resume-module helper.
    async fn drain(queue: &TaskQueue) -> Vec<Task> {
        let mut out = Vec::new();
        while let Ok(task) =
            tokio::time::timeout(Duration::from_millis(50), queue.next_task()).await
        {
            out.push(task);
        }
        out
    }

    #[tokio::test]
    async fn rescans_once_per_version_change_and_is_idempotent() {
        let pg = PgStore::connect_test().await.unwrap();
        // Generous per-folder budget: all ScanRoot tasks share folder_path=""
        // (matching the scan_folder API), so drain-without-complete would
        // otherwise stall at the default concurrency cap before reaching ours.
        let queue = TaskQueue::with_max_repos(4096);

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

        // First boot on the new version → triggers.
        let triggered = maybe_rescan_on_version_change(&pg, &queue, current).await;
        assert!(triggered, "version change must trigger a rebuild");

        let tasks = drain(&queue).await;
        let mine: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.kind == TaskKind::ScanRoot && t.path == root_path)
            .collect();
        assert_eq!(mine.len(), 1, "exactly one ScanRoot for our root, got {mine:?}");

        // Version persisted to current.
        assert_eq!(
            pg.get_config(LAST_VERSION_KEY).await.unwrap().as_deref(),
            Some(current),
            "daemon.last_version advanced to the running version",
        );
        // Analyzer full-refresh watermark cleared → scheduler re-analyzes all.
        assert_eq!(
            pg.get_config(ANALYZER_LAST_REFRESH_KEY).await.unwrap(),
            None,
            "analyzer full-refresh watermark cleared to force re-analysis",
        );

        // Second boot on the SAME version → no-op (idempotent).
        let triggered_again = maybe_rescan_on_version_change(&pg, &queue, current).await;
        assert!(!triggered_again, "same version must not re-trigger");
        let after = drain(&queue).await;
        let mine_after: Vec<&Task> = after
            .iter()
            .filter(|t| t.kind == TaskKind::ScanRoot && t.path == root_path)
            .collect();
        assert!(mine_after.is_empty(), "no rescan on an unchanged version, got {mine_after:?}");

        // Cleanup shared-DB state.
        pg.remove_watch_root(&rid).await.unwrap();
        pg.delete_config(LAST_VERSION_KEY).await.unwrap();
    }
}
