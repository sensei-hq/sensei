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
        // `.git` marker at this level — this guards the subfolder-as-workspace
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

/// Task kinds that frame waves — dispatchers open them, barriers close them.
/// Waves re-drive as a unit (fresh backfill / missed-day compute), so the
/// orphan sweep never takes these: re-queueing a lone dispatcher would fan
/// out beside the re-drive, and a lone barrier would terminal against dead
/// parents. Leaf and one-off work is the sweep's; framing is the re-drive's.
fn is_wave_framing(kind: &TaskKind) -> bool {
    use TaskKind::*;
    matches!(
        kind,
        IngestCaptures | ComputeWorkspaceMetrics | FinalizeTranscriptWave | ComputeHealth
    )
}

/// Re-queue crash-orphaned tasks as new tracked tasks. The last recovery
/// layer at boot, after wave re-drive and folder resume: whatever those
/// fan-outs did not already cover — one-offs, retries, embeds — returns
/// here, each with its retry number carried forward and lineage to the exact
/// prior attempt row. Bounds: orphan marker only (legit failures already got
/// bounded in-session retries), unparented only (wave/folder children belong
/// to their re-drive), retryable kinds under MAX (poison terminates), no
/// wave-framing kinds (waves re-drive as a unit). `enqueue_unique` dedups
/// against earlier layers, so nothing runs twice. Returns tasks admitted
/// (covers don't count). Safe to call when nothing is orphaned — returns 0.
pub async fn resume_orphaned_tasks(queue: &TaskQueue, pg: &PgStore) -> u32 {
    let rows = match pg.orphaned_task_rows(crate::tasks::retry::MAX_RETRIES as i32).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("resume_orphaned_tasks: orphaned_task_rows failed: {}", e);
            return 0;
        }
    };

    let mut admitted = 0u32;
    for row in &rows {
        let Some(kind_name) = row.get("kind").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(kind) = TaskKind::from_name(kind_name) else {
            tracing::warn!(kind = kind_name, "resume_orphaned_tasks: log names an unknown kind");
            continue;
        };
        if !kind.is_retryable() || is_wave_framing(&kind) {
            continue;
        }
        let folder = row.get("folderPath").and_then(|v| v.as_str()).unwrap_or_default();
        let path = row.get("path").and_then(|v| v.as_str()).unwrap_or_default();
        let retry_number = row.get("retryNumber").and_then(|v| v.as_u64()).unwrap_or(0) as u32 + 1;
        let mut task = Task::new(kind, folder, path);
        task.retry_number = retry_number;
        // Dead parents do not carry over: session IDs restart, so the old
        // parent names nothing. Lineage (below) is what links the retry to
        // its history, not the parent pointer.
        task.parent_task_id = None;
        task.retried_from =
            row.get("id").and_then(|v| v.as_str()).and_then(|s| uuid::Uuid::parse_str(s).ok());
        if queue.enqueue_unique(task).await.is_some() {
            admitted += 1;
        }
    }
    if admitted > 0 {
        tracing::info!("resume_orphaned_tasks: re-queued {} orphaned task(s)", admitted);
    }
    admitted
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
        let fid = pg
            .upsert_folder(&crate::db::pg_store::FolderRow {
                root_id,
                kind: "git",
                name,
                path: name,
                abs_path: &abs_path,
                parent_id: None,
                workspace_root_id: None,
            })
            .await
            .unwrap();
        sqlx_core::query::query(
            "UPDATE sensei.folders SET status = $2::sensei.folder_status WHERE id = $1",
        )
        .bind(fid)
        .bind(status)
        .execute(pg.pool())
        .await
        .unwrap();
        abs_path
    }

    /// Drain every immediately-available task from the queue, with a short
    /// timeout per call so the test can't hang. Returns the drained tasks.
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
    async fn resume_enqueues_only_real_git_roots() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid =
            pg.add_watch_root(&root_path, "resume_root", &serde_json::json!([])).await.unwrap();

        let pending_a = seed(&pg, &rid, base, "a", "discovered", true).await;
        let pending_b = seed(&pg, &rid, base, "b", "queued", true).await;
        // A subfolder mis-recorded as a git folder (no .git marker) — must be
        // skipped so it isn't promoted to a workspace (#3 inflation fix).
        let nogit = seed(&pg, &rid, base, "src", "discovered", false).await;
        // Terminal rows are filtered out by list_pending_folders, never resumed.
        seed(&pg, &rid, base, "c", "indexed", true).await;

        resume_pending_scans(&queue, &pg).await;

        let tasks = drain(&queue).await;
        let paths: std::collections::BTreeSet<&str> = tasks
            .iter()
            .filter(|t| t.path.starts_with(&root_path))
            .map(|t| t.path.as_str())
            .collect();

        assert!(paths.contains(pending_a.as_str()), "missing discovered git root: {pending_a}");
        assert!(paths.contains(pending_b.as_str()), "missing queued git root: {pending_b}");
        assert!(
            !paths.contains(nogit.as_str()),
            "subfolder without .git should not resume: {nogit}"
        );
        assert_eq!(paths.len(), 2, "only real git roots should resume, got {paths:?}");

        for t in tasks.iter().filter(|t| t.path.starts_with(&root_path)) {
            assert_eq!(t.kind, TaskKind::ProcessGitFolder, "wrong task kind: {:?}", t.kind);
            // scan_root sets folder_path = path for git roots; resume mirrors that.
            assert_eq!(
                t.folder_path, t.path,
                "folder_path should equal path for resumed git roots"
            );
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
        let rid = pg
            .add_watch_root(&root_path, "resume_dedup_root", &serde_json::json!([]))
            .await
            .unwrap();

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
        let rid = pg
            .add_watch_root(&root_path, "resume_recover_root", &serde_json::json!([]))
            .await
            .unwrap();

        let indexing = seed(&pg, &rid, base, "i", "indexing", true).await;
        let failed = seed(&pg, &rid, base, "f", "failed", true).await;
        seed(&pg, &rid, base, "done", "indexed", true).await; // terminal — must not resume

        resume_pending_scans(&queue, &pg).await;

        let paths: std::collections::BTreeSet<String> = drain(&queue)
            .await
            .into_iter()
            .map(|t| t.path)
            .filter(|p| p.starts_with(&root_path))
            .collect();
        assert!(paths.contains(&indexing), "an interrupted `indexing` folder is recovered");
        assert!(paths.contains(&failed), "a `failed` folder is retried");
        assert_eq!(
            paths.len(),
            2,
            "only non-terminal folders resume (indexed excluded), got {paths:?}"
        );

        pg.remove_watch_root(&rid).await.unwrap();
    }

    #[tokio::test]
    async fn resume_is_noop_when_nothing_pending() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();

        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let root_path = base.to_string_lossy().to_string();
        let rid = pg
            .add_watch_root(&root_path, "resume_empty_root", &serde_json::json!([]))
            .await
            .unwrap();
        seed(&pg, &rid, base, "only", "indexed", true).await;

        resume_pending_scans(&queue, &pg).await;
        let drained = drain(&queue).await;
        let ours: Vec<&Task> = drained.iter().filter(|t| t.path.starts_with(&root_path)).collect();
        assert!(
            ours.is_empty(),
            "expected no tasks under {}, got {:?}",
            root_path,
            ours.iter().map(|t| &t.path).collect::<Vec<_>>()
        );

        pg.remove_watch_root(&rid).await.unwrap();
    }

    async fn seed_orphan(
        pg: &PgStore,
        tag: &str,
        kind: &str,
        parent: Option<i64>,
        retry: i32,
        error: &str,
    ) -> uuid::Uuid {
        let id = pg
            .start_task_execution(&crate::db::pg_store::TaskExecutionStart {
                task_id: 320000 + rand_suffix(),
                parent_task_id: parent,
                task_kind: kind,
                folder_path: &format!("/_test/sweep/{tag}"),
                path: "src/x.rs",
                retry_number: retry,
                retried_from: None,
            })
            .await
            .unwrap();
        pg.fail_task_execution(&id, 5, error).await.unwrap();
        id
    }

    fn rand_suffix() -> i64 {
        (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
            % 1_000_000) as i64
    }

    #[tokio::test]
    async fn sweep_requeues_orphans_with_lineage_and_carried_retries() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();
        let tag = uuid::Uuid::new_v4().to_string();
        let prior = seed_orphan(
            &pg,
            &tag,
            "process_file",
            None,
            1,
            "orphaned: daemon restarted while task was running",
        )
        .await;

        let n = super::resume_orphaned_tasks(&queue, &pg).await;
        assert!(n >= 1, "the orphan returns");

        let tasks = drain(&queue).await;
        let ours: Vec<&Task> = tasks.iter().filter(|t| t.folder_path.contains(&tag)).collect();
        assert_eq!(ours.len(), 1, "exactly one re-queue: {ours:?}");
        assert_eq!(ours[0].kind, TaskKind::ProcessFile);
        assert_eq!(ours[0].retry_number, 2, "retry number carried forward");
        assert_eq!(ours[0].retried_from, Some(prior), "lineage names the prior row");
        assert_eq!(ours[0].parent_task_id, None, "dead parents do not carry over");
    }

    #[tokio::test]
    async fn sweep_skips_parented_non_retryable_exhausted_and_legit_failures() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();
        let tag = uuid::Uuid::new_v4().to_string();
        // Parented: wave/folder children belong to their re-drive.
        seed_orphan(
            &pg,
            &tag,
            "process_file",
            Some(99),
            0,
            "orphaned: daemon restarted while task was running",
        )
        .await;
        // Non-retryable kind.
        seed_orphan(
            &pg,
            &tag,
            "import_lib",
            None,
            0,
            "orphaned: daemon restarted while task was running",
        )
        .await;
        // Exhausted: at MAX already.
        seed_orphan(
            &pg,
            &tag,
            "process_file",
            None,
            crate::tasks::retry::MAX_RETRIES as i32,
            "orphaned: daemon restarted while task was running",
        )
        .await;
        // Legit failure: already got bounded in-session retries.
        seed_orphan(&pg, &tag, "process_file", None, 0, "parse blew up").await;
        // Wave framing: dispatchers and barriers re-drive as a unit.
        seed_orphan(
            &pg,
            &tag,
            "ingest_captures",
            None,
            0,
            "orphaned: daemon restarted while task was running",
        )
        .await;

        super::resume_orphaned_tasks(&queue, &pg).await;

        let tasks = drain(&queue).await;
        let ours: Vec<&Task> = tasks.iter().filter(|t| t.folder_path.contains(&tag)).collect();
        assert!(ours.is_empty(), "nothing swept: {ours:?}");
    }

    #[tokio::test]
    async fn sweep_dedups_against_already_queued_twins() {
        let pg = PgStore::connect_test().await.unwrap();
        let queue = TaskQueue::new();
        let tag = uuid::Uuid::new_v4().to_string();
        let fp = format!("/_test/sweep/{tag}");
        seed_orphan(
            &pg,
            &tag,
            "process_file",
            None,
            0,
            "orphaned: daemon restarted while task was running",
        )
        .await;
        // The folder re-drive got here first (boot order): same identity.
        queue.enqueue_unique(Task::new(TaskKind::ProcessFile, &fp, "src/x.rs")).await;

        let n = super::resume_orphaned_tasks(&queue, &pg).await;

        let tasks = drain(&queue).await;
        let ours: Vec<&Task> = tasks.iter().filter(|t| t.folder_path.contains(&tag)).collect();
        assert_eq!(ours.len(), 1, "one task does the work: {ours:?}");
        assert_eq!(n, 0, "the sweep counts admissions, not covers");
    }
}
