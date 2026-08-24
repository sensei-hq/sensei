//! Task executor — worker loop that pulls tasks from the queue and dispatches to handlers.

use std::sync::Arc;
use super::queue::TaskQueue;
use super::{TaskKind, Task};
use super::handlers;

/// Shared state passed to task handlers.
/// Wraps the same store/graph as the API routes via AppState.
pub struct TaskContext {
    pub queue: Arc<TaskQueue>,
    pub app_state: crate::api::state::AppState,
    pub _graph_path: Option<std::path::PathBuf>,
    pub logger: sensei_logger::Logger,
}

impl TaskContext {
    /// Access the PostgreSQL store (no mutex — PgPool is thread-safe).
    pub fn pg(&self) -> &crate::db::pg_store::PgStore {
        &self.app_state.pg
    }
}

/// Spawn N worker threads that process tasks from the queue.
pub fn spawn_workers(ctx: Arc<TaskContext>, n: usize) {
    for worker_id in 0..n {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            tracing::info!("[task-worker-{}] started", worker_id);
            loop {
                let task = ctx.queue.next_task().await;
                tracing::debug!("[task-worker-{}] running {} for {} ({})", worker_id, task.kind, task.folder_path, task.path);
                // The retry handle is detached: the bounded retry (if any) runs
                // on its own timer; the worker moves straight to the next task.
                let _ = run_task(&ctx, worker_id, task).await;
            }
        });
    }
}

/// Handle one dequeued task: dispatch it, record completion/failure, and on
/// failure schedule a bounded retry (D6c). Returns the spawned retry handle
/// when one was scheduled — `None` on success or a terminal failure. The worker
/// loop ignores the handle (the retry runs detached); tests inspect it to prove
/// the failure→retry wiring without waiting out the real backoff.
async fn run_task(
    ctx: &Arc<TaskContext>,
    worker_id: usize,
    task: Task,
) -> Option<tokio::task::JoinHandle<Option<u64>>> {
    match execute_task(ctx, &task).await {
        Ok(_items) => {
            ctx.queue.complete(task.id).await;
            tracing::debug!("[task-worker-{}] completed {} #{}", worker_id, task.kind, task.id);
            None
        }
        Err(e) => {
            ctx.queue.fail(task.id, e.clone()).await;
            tracing::warn!("[task-worker-{}] failed {} #{}: {}", worker_id, task.kind, task.id, e);
            // D6c: schedule a bounded, backed-off retry for a failed
            // graph-pipeline task. A no-op for non-retryable kinds and once
            // MAX_RETRIES is reached (terminal failure). The retry runs detached;
            // enqueue_unique keeps it single-writer safe.
            super::retry::schedule_if_retryable(ctx.queue.clone(), &task)
        }
    }
}

async fn execute_task(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let start = std::time::Instant::now();
    let task_logger = ctx.logger.with_method(&task.kind.to_string())
        .with_context(serde_json::json!({
            "task_id": task.id,
            "folder": &task.folder_path,
            "path": &task.path,
        }));

    // Record execution start in task_executions (fire-and-forget on failure)
    let exec_id = match ctx.pg().start_task_execution(
        task.id as i64,
        task.parent_task_id.map(|id| id as i64),
        &task.kind.to_string(),
        &task.folder_path,
        &task.path,
        task.retry_number as i32,
    ).await {
        Ok(id) => Some(id),
        Err(e) => {
            tracing::warn!(error = %e, task_id = task.id, kind = %task.kind, "failed to record task_execution start");
            None
        }
    };

    task_logger.info("task_started", None).await;

    // Dispatch to the handler under a watchdog: a wedged handler (e.g. a stalled
    // network/DB call that never returns) would otherwise hold its worker
    // forever, keep its task in `running`, and leave the folder's resolve
    // barrier blocked — freezing the whole pool. The timeout cancels the handler
    // future and the task is failed (which unblocks dependents and frees the
    // worker). Abandoned work is retried/backfilled. Note: this preempts async
    // waits (the realistic hangs); a purely CPU-bound sync loop would not yield.
    let dispatch = async {
        match task.kind {
            TaskKind::ScanRoot => handlers::scan_root(ctx, task).await,
            TaskKind::ProcessGitFolder => handlers::process_git_folder(ctx, task).await,
            TaskKind::ProcessFolder => handlers::process_folder(ctx, task).await,
            TaskKind::ProcessFile => handlers::process_file(ctx, task).await,
            TaskKind::DeleteFile => handlers::delete_file(ctx, task).await,
            TaskKind::DeleteFolder => handlers::delete_folder(ctx, task).await,
            TaskKind::ResolveLibs => handlers::resolve_libs(ctx, task).await,
            TaskKind::ImportLib => handlers::import_lib(ctx, task).await,
            TaskKind::BranchSwitch => handlers::branch_switch(ctx, task).await,
            TaskKind::BuildConnections => handlers::build_connections(ctx, task).await,
            TaskKind::EmbedNodes => handlers::embed_nodes(ctx, task).await,
            TaskKind::IndexLibrary => handlers::index_library(ctx, task).await,
            TaskKind::IndexLibraryPage => handlers::index_library_page(ctx, task).await,
            TaskKind::DetectCommunities => handlers::detect_communities(ctx, task).await,
            TaskKind::ExtractDeps => handlers::extract_deps(ctx, task).await,
            TaskKind::MeasureVerdicts => handlers::measure_verdicts(ctx, task).await,
            TaskKind::ReconcileRepoMetadata => handlers::reconcile_repo_metadata(ctx, task).await,
            TaskKind::AnalyzeProject => handlers::analyze_project(ctx, task).await,
            TaskKind::AnalyzeSessionProcess => handlers::analyze_session_process(ctx, task).await,
            TaskKind::ScanDocDrift => handlers::scan_doc_drift(ctx, task).await,
            TaskKind::AggregateCorrections => handlers::aggregate_corrections(ctx, task).await,
            TaskKind::AggregateToolInsights => handlers::aggregate_tool_insights(ctx, task).await,
            TaskKind::ClassifyPendingVerdicts => handlers::classify_pending_verdicts(ctx, task).await,
            TaskKind::ConsolidateGovernance => handlers::consolidate_governance(ctx, task).await,
            TaskKind::WarmInsightCopy => handlers::warm_insight_copy(ctx, task).await,
            TaskKind::LearnPlaybooks => handlers::learn_playbooks(ctx, task).await,
            TaskKind::PublishRelaySegments => handlers::publish_relay_segments(ctx, task).await,
            TaskKind::AdvanceRun => handlers::advance_run(ctx, task).await,
            TaskKind::PublishRun => handlers::publish_run(ctx, task).await,
            TaskKind::ComputeProjectMetrics => handlers::metrics::compute_project(ctx, task).await,
            TaskKind::ComputeGroupMetrics => handlers::metrics::compute_group(ctx, task).await,
            TaskKind::ComputeHealth => handlers::metrics::compute_health(ctx, task).await,
            TaskKind::IngestCaptures => crate::transcript::run_backfill(ctx, task).await,
            TaskKind::IngestCapture => crate::transcript::run_ingest_capture(ctx, task).await,
            TaskKind::BackfillCoverage => handlers::metrics::coverage::run_backfill(ctx, task).await,
        }
    };
    let cap = task.kind.watchdog_timeout();
    let result = match tokio::time::timeout(cap, dispatch).await {
        Ok(r) => r,
        Err(_) => Err(format!(
            "watchdog: {} task #{} exceeded {}s and was abandoned (worker freed; retry/backfill)",
            task.kind, task.id, cap.as_secs()
        )),
    };

    let duration_ms = start.elapsed().as_millis() as i32;

    // Update task_executions + log to public.logs
    match &result {
        Ok(items) => {
            if let Some(eid) = &exec_id
                && let Err(e) = ctx.pg().complete_task_execution(eid, *items as i32, duration_ms).await {
                    tracing::warn!(error = %e, task_id = task.id, kind = %task.kind, "failed to record task_execution completion");
                }
            task_logger.info("task_completed", Some(serde_json::json!({
                "duration_ms": duration_ms,
                "items_processed": items,
            }))).await;
        }
        Err(e) => {
            if let Some(eid) = &exec_id
                && let Err(db_err) = ctx.pg().fail_task_execution(eid, duration_ms, e).await {
                    tracing::warn!(error = %db_err, task_id = task.id, kind = %task.kind, "failed to record task_execution failure");
                }
            task_logger.error("task_failed", Some(serde_json::json!({
                "duration_ms": duration_ms,
            })), Some(serde_json::json!({"message": e}))).await;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn execute_task_dispatches_scan_root() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path");
        let result = execute_task(&ctx, &task).await;
        // ScanRoot with a nonexistent path should fail with a clear error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_process_git_folder() {
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ProcessGitFolder, "repo", "/nonexistent/repo");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_delete_file() {
        let ctx = make_ctx().await;
        // DeleteFile on an empty graph should succeed (no-op)
        let task = Task::new(TaskKind::DeleteFile, "repo", "/some/file.rs");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_delete_folder() {
        let ctx = make_ctx().await;
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", "/tmp/repo").await.unwrap();
        }
        let task = Task::new(TaskKind::DeleteFolder, "repo", "/tmp/repo/src");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_resolve_libs() {
        let ctx = make_ctx().await;
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", "/tmp/repo").await.unwrap();
        }
        let task = Task::new(TaskKind::ResolveLibs, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_build_connections() {
        let ctx = make_ctx().await;
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", "/tmp/repo").await.unwrap();
        }
        let task = Task::new(TaskKind::BuildConnections, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_import_lib_without_url() {
        let ctx = make_ctx().await;
        // ImportLib without a URL should fail
        let task = Task::new(TaskKind::ImportLib, "repo", "react");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a URL"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_branch_switch_without_branch() {
        let ctx = make_ctx().await;
        // BranchSwitch without branch field should fail
        let task = Task::new(TaskKind::BranchSwitch, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("branch"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_scan_doc_drift() {
        let ctx = make_ctx().await;
        // A bad project-id payload reaches the doc-drift handler's parse guard —
        // proving the executor routes ScanDocDrift to scan_doc_drift.
        let task = Task::new(TaskKind::ScanDocDrift, "", "not-a-uuid");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expected a project id"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_classify_pending_verdicts() {
        let ctx = make_ctx().await;
        // Seed a classifiable, unclassified in-window session, then dispatch the
        // global task. If the executor routes to classify_pending_verdicts, the
        // handler classifies the session and it drops out of the pending set.
        let sid = format!("_test-exec-classify-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().timestamp_millis();
        ctx.pg().insert_hook_event(
            &sid, "claude", "PostToolUse", Some("Read"), None, now, Some(true),
            &serde_json::json!({
                "tool_input": {"file_path": "crates/senseid/src/db/pg_store.rs"},
                "tool_response": "see crates/senseid/src/db/pg_store.rs:3421",
            }),
        ).await.unwrap();
        ctx.pg().insert_hook_event(
            &sid, "claude", "PreToolUse", Some("Edit"), None, now + 1, None,
            &serde_json::json!({"tool_input": {"file_path": "crates/senseid/src/db/pg_store.rs"}}),
        ).await.unwrap();

        let window = crate::tasks::handlers::tool_insights::HEALTH_VERDICT_WINDOW_DAYS;
        assert!(ctx.pg().unclassified_verdict_sessions(window).await.unwrap().contains(&sid));

        let task = Task::new(TaskKind::ClassifyPendingVerdicts, "", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok(), "classify pass should succeed: {result:?}");
        assert!(
            !ctx.pg().unclassified_verdict_sessions(window).await.unwrap().contains(&sid),
            "dispatched handler classified the fixture session",
        );
    }

    #[tokio::test]
    async fn execute_task_dispatches_metrics_graph() {
        // The executor must route the metrics graph by IDENTITY:
        //   ComputeProjectMetrics → handlers::metrics::compute_project (the parent),
        //   ComputeGroupMetrics   → handlers::metrics::compute_group   (the child),
        //   ComputeHealth         → handlers::metrics::compute_health  (the barrier).
        // The handlers can honestly return Ok(0) on an empty fixture, so `.is_ok()`
        // alone can't tell the arms apart (swapping them would stay green). Assert
        // by IDENTITY via the test-only routing probe so a crossed match arm fails
        // here, not silently downstream.
        use crate::tasks::handlers::metrics::probe;
        let ctx = make_ctx().await;
        let pid = uuid::Uuid::new_v4().to_string();

        // ComputeGroupMetrics (known group) → compute_group (the per-group child).
        probe::reset();
        let group = Task::new(TaskKind::ComputeGroupMetrics, &pid, "session_outcomes");
        assert!(execute_task(&ctx, &group).await.is_ok(),
            "ComputeGroupMetrics routes to the metrics group-compute handler");
        assert_eq!(probe::take(), Some("compute_group"),
            "ComputeGroupMetrics must dispatch to compute_group, not compute_health");

        // ComputeGroupMetrics (unknown group) → still compute_group (routed),
        // logged no-op — the dispatch never panics on an unrecognised task_name.
        probe::reset();
        let unknown = Task::new(TaskKind::ComputeGroupMetrics, &pid, "no_such_group");
        assert!(execute_task(&ctx, &unknown).await.is_ok(),
            "an unknown metrics task_name is a logged no-op, not a queue error");
        assert_eq!(probe::take(), Some("compute_group"),
            "an unknown group still routes through compute_group");

        // ComputeHealth → compute_health (the barrier).
        probe::reset();
        let health = Task::new(TaskKind::ComputeHealth, &pid, "");
        assert!(execute_task(&ctx, &health).await.is_ok(),
            "ComputeHealth routes to the metrics health handler");
        assert_eq!(probe::take(), Some("compute_health"),
            "ComputeHealth must dispatch to compute_health, not compute_group");

        // ComputeProjectMetrics → compute_project (the per-project parent), NOT a
        // child group-compute or the health barrier.
        probe::reset();
        let project = Task::new(TaskKind::ComputeProjectMetrics, &pid, "");
        assert!(execute_task(&ctx, &project).await.is_ok(),
            "ComputeProjectMetrics routes to the metrics project-parent handler");
        assert_eq!(probe::take(), Some("compute_project"),
            "ComputeProjectMetrics must dispatch to compute_project");
    }

    #[tokio::test]
    async fn execute_task_dispatches_process_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();

        let ctx = make_ctx().await;
        {
            let repo_path = tmp.path().to_string_lossy().to_string();
            let root_id = ctx.pg().add_watch_root(&repo_path, "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", &repo_path).await.unwrap();
        }

        let mut task = Task::new(TaskKind::ProcessFolder, "repo", &src_dir.to_string_lossy());
        task.module_id = Some("pkg:repo:(root)".into());
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());

        // TODO: verify module node once module writes are implemented
    }

    #[tokio::test]
    async fn run_task_schedules_bounded_retry_on_failed_retryable_kind() {
        // D6c wiring: a failed graph-pipeline task is re-driven. ProcessGitFolder
        // on a nonexistent path fails (proven by execute_task_dispatches_*), and
        // ProcessGitFolder is retryable → run_task returns a scheduled retry.
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ProcessGitFolder, "/nonexistent/repo", "/nonexistent/repo");
        let handle = run_task(&ctx, 0, task).await
            .expect("a failed retryable task schedules a bounded retry");
        handle.abort(); // don't wait out the real backoff
    }

    #[tokio::test]
    async fn run_task_does_not_retry_a_non_retryable_failure() {
        // ScanRoot on a nonexistent path fails, but ScanRoot is NOT a retryable
        // kind (a deleted root is permanent) → terminal, no retry scheduled.
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path");
        assert!(run_task(&ctx, 0, task).await.is_none(),
            "a non-retryable kind's failure is terminal");
    }

    #[tokio::test]
    async fn run_task_schedules_no_retry_on_success() {
        // A succeeding task (DeleteFile on an empty graph is a no-op) never
        // schedules a retry.
        let ctx = make_ctx().await;
        let task = Task::new(TaskKind::DeleteFile, "repo", "/some/file.rs");
        assert!(run_task(&ctx, 0, task).await.is_none(),
            "a successful task schedules no retry");
    }

    #[tokio::test]
    async fn run_task_stops_retrying_once_exhausted() {
        // A retryable kind that has already hit MAX_RETRIES is terminal — the
        // bounded cap holds at the worker boundary, not just in the policy.
        let ctx = make_ctx().await;
        let mut task = Task::new(TaskKind::ProcessGitFolder, "/nonexistent/repo", "/nonexistent/repo");
        task.retry_number = super::super::retry::MAX_RETRIES;
        assert!(run_task(&ctx, 0, task).await.is_none(),
            "an exhausted retryable task is terminal");
    }

    #[tokio::test]
    async fn task_context_provides_pg() {
        let ctx = make_ctx().await;
        let unique = format!("test-ctx-{}", std::process::id());
        let path = format!("/tmp/{}", unique);
        {
            let root_id = ctx.pg().add_watch_root(&path, &unique, &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, &unique, &path).await.unwrap();
            let p = ctx.pg().get_repo_by_name(&unique).await.unwrap();
            assert!(p.is_some());
        }
    }
}
