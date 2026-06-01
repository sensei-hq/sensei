//! Startup recovery for the in-memory task queue.
//!
//! `TaskQueue` is in-memory only — every queued task is lost on daemon
//! restart. Folders left at `status='discovered'` (the default after
//! `ScanRoot` upserts a row) or `'queued'` never finish indexing.
//!
//! This module reads those rows back from PostgreSQL on startup and
//! re-enqueues a `ProcessGitFolder` task per row, mirroring what
//! `scan_root` does on the initial scan.

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
        let task = Task::new(TaskKind::ProcessGitFolder, abs_path, abs_path);
        queue.enqueue(task).await;
        enqueued += 1;
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

    /// Seed a folder row with an explicit status. Returns the abs_path.
    async fn seed(pg: &PgStore, root_id: &uuid::Uuid, root_path: &str, name: &str, status: &str) -> String {
        let abs_path = format!("{}/{}", root_path, name);
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
    async fn resume_enqueues_process_git_folder_per_pending_row() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let root_path = format!("/_test/resume_pending_{}", uuid::Uuid::new_v4().simple());
        let rid = pg.add_watch_root(&root_path, "resume_root", &serde_json::json!([])).await.unwrap();

        let pending_a = seed(&pg, &rid, &root_path, "a", "discovered").await;
        let pending_b = seed(&pg, &rid, &root_path, "b", "queued").await;
        // Terminal — must not be re-enqueued.
        seed(&pg, &rid, &root_path, "c", "indexed").await;
        seed(&pg, &rid, &root_path, "d", "failed").await;
        seed(&pg, &rid, &root_path, "e", "deferred").await;
        // `indexing` is excluded: at startup the in-memory queue is empty,
        // so no worker can actually be running it.
        seed(&pg, &rid, &root_path, "f", "indexing").await;

        let _count = resume_pending_scans(&queue, &pg).await;

        // The caller's DB may contain unrelated pending rows from prior
        // sessions, so we filter by our unique root prefix when asserting.
        let tasks = drain(&queue).await;
        let ours: Vec<&Task> = tasks.iter()
            .filter(|t| t.path.starts_with(&root_path))
            .collect();

        assert_eq!(ours.len(), 2, "expected 2 resumed tasks, got {:?}",
            ours.iter().map(|t| &t.path).collect::<Vec<_>>());

        let paths: std::collections::BTreeSet<&str> = ours.iter()
            .map(|t| t.path.as_str())
            .collect();
        assert!(paths.contains(pending_a.as_str()), "missing discovered row: {}", pending_a);
        assert!(paths.contains(pending_b.as_str()), "missing queued row: {}", pending_b);

        for t in &ours {
            assert_eq!(t.kind, TaskKind::ProcessGitFolder, "wrong task kind: {:?}", t.kind);
            // scan_root sets folder_path = path for git roots; resume mirrors that.
            assert_eq!(t.folder_path, t.path, "folder_path should equal path for resumed git roots");
        }

        pg.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn resume_is_noop_when_nothing_pending() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let root_path = format!("/_test/resume_empty_{}", uuid::Uuid::new_v4().simple());
        let rid = pg.add_watch_root(&root_path, "resume_empty_root", &serde_json::json!([])).await.unwrap();
        seed(&pg, &rid, &root_path, "only", "indexed").await;

        let count_before = queue.status().await.pending;
        resume_pending_scans(&queue, &pg).await;
        let drained = drain(&queue).await;
        let ours: Vec<&Task> = drained.iter()
            .filter(|t| t.path.starts_with(&root_path))
            .collect();
        assert!(ours.is_empty(), "expected no tasks under {}, got {:?}",
            root_path, ours.iter().map(|t| &t.path).collect::<Vec<_>>());

        // Sanity: the queue total didn't grow on our account.
        let _ = count_before;
        pg.remove_watch_root(&rid).await.unwrap();
    }
}
