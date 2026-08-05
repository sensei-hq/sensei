//! Startup recovery for the in-memory task queue.
//!
//! `TaskQueue` is in-memory only — every queued task is lost on daemon
//! restart. Folders in a non-terminal index state — `discovered` (the default
//! after `ScanRoot` upserts a row), `queued`, `indexing` (a scan was in-flight
//! when the daemon stopped, D6a), or `failed` — never finish indexing.
//!
//! This module reads those rows back from PostgreSQL on startup and
//! re-enqueues a `ProcessGitFolder` task per row, mirroring what
//! `scan_root` does on the initial scan. The single-writer guard
//! (`enqueue_unique`) makes a re-enqueue of an already-running folder a no-op.

use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

/// Re-enqueue `ProcessGitFolder` for every folder row in a non-terminal
/// index state. Returns the number of tasks enqueued. Safe to call when
/// nothing is pending — returns 0.
pub async fn resume_pending_scans(queue: &TaskQueue, pg: &PgStore) -> u32 {
    let rows = match pg.list_pending_folders().await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("resume_pending_scans: list_pending_folders failed: {}", e);
            return 0;
        }
    };

    let mut enqueued = 0u32;
    for row in &rows {
        let kind = row["kind"].as_str().unwrap_or("");
        // Only git/subtree folders are persisted in `sensei.folders` today;
        // both resume via ProcessGitFolder (the same task scan_root enqueues).
        if !matches!(kind, "git" | "subtree") {
            continue;
        }
        let abs_path = match row["abs_path"].as_str() {
            Some(p) if !p.is_empty() => p,
            _ => continue,
        };
        // Skip folders that have vanished, and `git` rows that no longer have a
        // `.git` marker at this level — this guards the subfolder-as-project
        // inflation (#3), where a subfolder was mis-recorded as a repo. Subtrees
        // are deliberately-registered subdirs and carry no `.git` of their own.
        let path = std::path::Path::new(abs_path);
        if !path.exists() {
            continue;
        }
        if kind == "git" && !path.join(".git").exists() {
            continue;
        }
        let task = Task::new(TaskKind::ProcessGitFolder, abs_path, abs_path);
        // Single-writer (D6e/W5): only count a folder as re-enqueued if it wasn't
        // already in flight — resume must not stack a duplicate scan.
        if queue.enqueue_unique(task).await.is_some() {
            enqueued += 1;
        }
    }

    if enqueued > 0 {
        tracing::info!("resume_pending_scans: re-enqueued {} folder(s)", enqueued);
    }
    enqueued
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::TaskKind;
    use std::time::Duration;

    /// Seed a real folder dir (optionally with a `.git` marker) plus a DB row
    /// at the given status. Returns the abs_path.
    async fn seed(
        pg: &PgStore,
        root_id: &uuid::Uuid,
        base: &std::path::Path,
        name: &str,
        status: &str,
        with_git: bool,
    ) -> String {
        let dir = base.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        if with_git {
            std::fs::create_dir_all(dir.join(".git")).unwrap();
        }
        let abs_path = dir.to_string_lossy().to_string();
        let fid = pg.upsert_folder(root_id, "git", name, name, &abs_path, None, None).await.unwrap();
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = $2::sensei.folder_status WHERE id = $1"
        ).bind(fid).bind(status).execute(pg.pool()).await.unwrap();
        abs_path
    }

    /// Drain every immediately-available task from the queue, with a short
    /// timeout per call so the test can't hang. Returns the drained tasks.
    async fn drain(queue: &TaskQueue) -> Vec<Task> {
        let mut out = Vec::new();
        while let Ok(task) = tokio::time::timeout(
            Duration::from_millis(50), queue.next_task(),
        ).await {
            out.push(task);
        }
        out
    }

    #[tokio::test]
    async fn resume_enqueues_only_real_git_roots() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid = pg.add_watch_root(&root_path, "resume_root", &serde_json::json!([])).await.unwrap();

        let pending_a = seed(&pg, &rid, base, "a", "discovered", true).await;
        let pending_b = seed(&pg, &rid, base, "b", "queued", true).await;
        // A subfolder mis-recorded as a git folder (no .git marker) — must be
        // skipped so it isn't promoted to a project (#3 inflation fix).
        let nogit = seed(&pg, &rid, base, "src", "discovered", false).await;
        // Terminal rows are filtered out by list_pending_folders, never resumed.
        seed(&pg, &rid, base, "c", "indexed", true).await;

        resume_pending_scans(&queue, &pg).await;

        let tasks = drain(&queue).await;
        let paths: std::collections::BTreeSet<&str> = tasks.iter()
            .filter(|t| t.path.starts_with(&root_path))
            .map(|t| t.path.as_str())
            .collect();

        assert!(paths.contains(pending_a.as_str()), "missing discovered git root: {pending_a}");
        assert!(paths.contains(pending_b.as_str()), "missing queued git root: {pending_b}");
        assert!(!paths.contains(nogit.as_str()), "subfolder without .git should not resume: {nogit}");
        assert_eq!(paths.len(), 2, "only real git roots should resume, got {paths:?}");

        for t in tasks.iter().filter(|t| t.path.starts_with(&root_path)) {
            assert_eq!(t.kind, TaskKind::ProcessGitFolder, "wrong task kind: {:?}", t.kind);
            // scan_root sets folder_path = path for git roots; resume mirrors that.
            assert_eq!(t.folder_path, t.path, "folder_path should equal path for resumed git roots");
        }

        pg.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn resume_skips_a_folder_already_scanning() {
        // Single-writer (D6e/W5): a folder already being scanned must not be
        // re-enqueued by resume (which would give it two writers). Without the
        // guard, folder "a" would appear twice — the pre-seed + resume's enqueue.
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid = pg.add_watch_root(&root_path, "resume_dedup_root", &serde_json::json!([])).await.unwrap();

        let a = seed(&pg, &rid, base, "a", "discovered", true).await;
        let b = seed(&pg, &rid, base, "b", "discovered", true).await;

        // Pre-seed: folder "a" is already being scanned.
        queue.enqueue(Task::new(TaskKind::ProcessGitFolder, &a, &a)).await;

        resume_pending_scans(&queue, &pg).await;

        let tasks = drain(&queue).await;
        let a_count = tasks.iter().filter(|t| t.path == a).count();
        let b_count = tasks.iter().filter(|t| t.path == b).count();
        assert_eq!(a_count, 1, "an already-scanning folder is not double-enqueued");
        assert_eq!(b_count, 1, "a not-yet-scanning folder is still resumed");

        pg.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn resume_recovers_interrupted_indexing_and_failed_folders() {
        // D6a/D6b: a scan interrupted mid-flight (`indexing`) or errored
        // (`failed`) must be re-enqueued on the next boot — not stranded. A
        // terminal `indexed` folder must NOT resume.
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid = pg.add_watch_root(&root_path, "resume_recover_root", &serde_json::json!([])).await.unwrap();

        let indexing = seed(&pg, &rid, base, "i", "indexing", true).await;
        let failed = seed(&pg, &rid, base, "f", "failed", true).await;
        seed(&pg, &rid, base, "done", "indexed", true).await; // terminal — must not resume

        resume_pending_scans(&queue, &pg).await;

        let paths: std::collections::BTreeSet<String> = drain(&queue).await.into_iter()
            .map(|t| t.path)
            .filter(|p| p.starts_with(&root_path))
            .collect();
        assert!(paths.contains(&indexing), "an interrupted `indexing` folder is recovered");
        assert!(paths.contains(&failed), "a `failed` folder is retried");
        assert_eq!(paths.len(), 2, "only non-terminal folders resume (indexed excluded), got {paths:?}");

        pg.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn resume_is_noop_when_nothing_pending() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid = pg.add_watch_root(&root_path, "resume_empty_root", &serde_json::json!([])).await.unwrap();
        seed(&pg, &rid, base, "only", "indexed", true).await;

        resume_pending_scans(&queue, &pg).await;
        let drained = drain(&queue).await;
        let ours: Vec<&Task> = drained.iter()
            .filter(|t| t.path.starts_with(&root_path))
            .collect();
        assert!(ours.is_empty(), "expected no tasks under {}, got {:?}",
            root_path, ours.iter().map(|t| &t.path).collect::<Vec<_>>());

        pg.remove_watch_root(&rid).await.unwrap();
    }
}
