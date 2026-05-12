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

                let result = execute_task(&ctx, &task).await;

                match result {
                    Ok(_items) => {
                        ctx.queue.complete(task.id).await;
                        tracing::debug!("[task-worker-{}] completed {} #{}", worker_id, task.kind, task.id);
                    }
                    Err(e) => {
                        ctx.queue.fail(task.id, e.clone()).await;
                        tracing::warn!("[task-worker-{}] failed {} #{}: {}", worker_id, task.kind, task.id, e);
                    }
                }
            }
        });
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
    let exec_id = ctx.pg().start_task_execution(
        task.id as i64,
        task.parent_task_id.map(|id| id as i64),
        &task.kind.to_string(),
        &task.folder_path,
        &task.path,
    ).await.ok();

    task_logger.info("task_started", None).await;

    let result = match task.kind {
        TaskKind::ScanRoot => handlers::scan_root(ctx, task).await,
        TaskKind::ProcessGitFolder => handlers::process_git_folder(ctx, task).await,
        TaskKind::ProcessFolder => handlers::process_folder(ctx, task).await,
        TaskKind::ProcessFile => handlers::process_file(ctx, task).await,
        TaskKind::DeleteFile => handlers::delete_file(ctx, task).await,
        TaskKind::DeleteFolder => handlers::delete_folder(ctx, task).await,
        TaskKind::ResolveEdges => handlers::resolve_edges(ctx, task).await,
        TaskKind::ResolveLibs => handlers::resolve_libs(ctx, task).await,
        TaskKind::ImportLib => handlers::import_lib(ctx, task).await,
        TaskKind::BranchSwitch => handlers::branch_switch(ctx, task).await,
        TaskKind::BuildConnections => handlers::build_connections(ctx, task).await,
        TaskKind::ReconcileConnections => handlers::reconcile_connections(ctx, task).await,
        TaskKind::IndexLibrary => handlers::index_library(ctx, task).await,
        TaskKind::IndexLibraryPage => handlers::index_library_page(ctx, task).await,
        TaskKind::DetectCommunities => handlers::detect_communities(ctx, task).await,
        TaskKind::ExtractDeps => handlers::extract_deps(ctx, task).await,
        TaskKind::MeasureVerdicts => handlers::measure_verdicts(ctx, task).await,
    };

    let duration_ms = start.elapsed().as_millis() as i32;

    // Update task_executions + log to public.logs
    match &result {
        Ok(items) => {
            if let Some(eid) = &exec_id {
                let _ = ctx.pg().complete_task_execution(eid, *items as i32, duration_ms).await;
            }
            task_logger.info("task_completed", Some(serde_json::json!({
                "duration_ms": duration_ms,
                "items_processed": items,
            }))).await;
        }
        Err(e) => {
            if let Some(eid) = &exec_id {
                let _ = ctx.pg().fail_task_execution(eid, duration_ms, e).await;
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
    use crate::api::state::SharedState;

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

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
    async fn execute_task_dispatches_resolve_edges() {
        let ctx = make_ctx().await;
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", "/tmp/repo").await.unwrap();
        }
        let task = Task::new(TaskKind::ResolveEdges, "repo", "");
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
    async fn execute_task_dispatches_reconcile_connections() {
        let ctx = make_ctx().await;
        {
            let root_id = ctx.pg().add_watch_root("/tmp/repo", "test", &serde_json::json!([])).await.unwrap();
            ctx.pg().upsert_repo(&root_id, "repo", "/tmp/repo").await.unwrap();
        }
        // ReconcileConnections with no solutions should succeed (no-op)
        let task = Task::new(TaskKind::ReconcileConnections, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
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
