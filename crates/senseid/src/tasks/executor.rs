//! Task executor — worker loop that pulls tasks from the queue and dispatches to handlers.

use std::sync::Arc;
use super::queue::TaskQueue;
use super::{TaskKind, Task};
use super::handlers;

/// Shared state passed to task handlers.
/// Wraps the same store/graph as the API routes via AppState.
pub struct TaskContext {
    pub queue: Arc<TaskQueue>,
    pub app_state: crate::api::routes::AppState,
    pub _graph_path: Option<std::path::PathBuf>,
}

impl TaskContext {
    pub async fn store(&self) -> tokio::sync::MutexGuard<'_, crate::db::Store> {
        self.app_state.store.lock().await
    }

    pub async fn graph(&self) -> tokio::sync::MutexGuard<'_, crate::indexer::graph::GraphDb> {
        self.app_state.graph.lock().await
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
                tracing::debug!("[task-worker-{}] running {} for {} ({})", worker_id, task.kind, task.repo_id, task.path);

                let result = execute_task(&ctx, &task).await;

                match result {
                    Ok(()) => {
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

async fn execute_task(ctx: &TaskContext, task: &Task) -> Result<(), String> {
    match task.kind {
        TaskKind::ScanRoot => handlers::scan_root(ctx, task).await,
        TaskKind::ProcessRepo => handlers::process_repo(ctx, task).await,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::Mutex;
    use crate::db::Store;
    use crate::indexer::graph::GraphDb;
    use crate::api::routes::SharedState;

    /// Build a TaskContext backed by in-memory Store + GraphDb.
    fn make_ctx() -> Arc<TaskContext> {
        let store = Store::open_memory().unwrap();
        let graph = GraphDb::open_memory().unwrap();
        let queue = Arc::new(TaskQueue::new());
        let app_state = Arc::new(SharedState {
            store: Mutex::new(store),
            graph: Mutex::new(graph),
            task_queue: queue.clone(),
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
        })
    }

    #[tokio::test]
    async fn execute_task_dispatches_scan_root() {
        let ctx = make_ctx();
        let task = Task::new(TaskKind::ScanRoot, "", "/nonexistent/path");
        let result = execute_task(&ctx, &task).await;
        // ScanRoot with a nonexistent path should fail with a clear error
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_process_repo() {
        let ctx = make_ctx();
        let task = Task::new(TaskKind::ProcessRepo, "repo", "/nonexistent/repo");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_delete_file() {
        let ctx = make_ctx();
        // DeleteFile on an empty graph should succeed (no-op)
        let task = Task::new(TaskKind::DeleteFile, "repo", "/some/file.rs");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_delete_folder() {
        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", "/tmp/repo").unwrap();
        }
        let task = Task::new(TaskKind::DeleteFolder, "repo", "/tmp/repo/src");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_resolve_edges() {
        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", "/tmp/repo").unwrap();
        }
        let task = Task::new(TaskKind::ResolveEdges, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_resolve_libs() {
        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", "/tmp/repo").unwrap();
        }
        let task = Task::new(TaskKind::ResolveLibs, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_build_connections() {
        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", "/tmp/repo").unwrap();
        }
        let task = Task::new(TaskKind::BuildConnections, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_task_dispatches_import_lib_without_url() {
        let ctx = make_ctx();
        // ImportLib without a URL should fail
        let task = Task::new(TaskKind::ImportLib, "repo", "react");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("requires a URL"));
    }

    #[tokio::test]
    async fn execute_task_dispatches_branch_switch_without_branch() {
        let ctx = make_ctx();
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

        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", &tmp.path().to_string_lossy()).unwrap();
        }

        let mut task = Task::new(TaskKind::ProcessFolder, "repo", &src_dir.to_string_lossy());
        task.module_id = Some("pkg:repo:(root)".into());
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());

        // Verify module node was created by the dispatched handler
        let graph = ctx.graph().await;
        let nodes = graph.get_nodes("repo").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].kind, "module");
    }

    #[tokio::test]
    async fn execute_task_dispatches_reconcile_connections() {
        let ctx = make_ctx();
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("repo", "repo", "/tmp/repo").unwrap();
        }
        // ReconcileConnections with no solutions should succeed (no-op)
        let task = Task::new(TaskKind::ReconcileConnections, "repo", "");
        let result = execute_task(&ctx, &task).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn task_context_provides_store_and_graph() {
        let ctx = make_ctx();
        // Verify we can access both store and graph
        {
            let store = ctx.store().await;
            store.upsert_repo_basic("test", "test", "/tmp/test").unwrap();
            let p = store.get_repo("test").unwrap();
            assert!(p.is_some());
        }
        {
            let graph = ctx.graph().await;
            let counts = graph.count_by_kind("empty").unwrap();
            assert!(counts.is_empty());
        }
    }
}
