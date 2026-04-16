//! Task executor — worker loop that pulls tasks from the queue and dispatches to handlers.

use std::sync::Arc;
use super::queue::TaskQueue;
use super::{TaskKind, Task};
use super::handlers;

/// Shared state passed to task handlers.
pub struct TaskContext {
    pub queue: Arc<TaskQueue>,
    pub store: Arc<tokio::sync::Mutex<crate::db::Store>>,
    pub graph: Arc<tokio::sync::Mutex<crate::indexer::graph::GraphDb>>,
    pub graph_path: Option<std::path::PathBuf>,
}

/// Spawn N worker threads that process tasks from the queue.
pub fn spawn_workers(ctx: Arc<TaskContext>, n: usize) {
    for worker_id in 0..n {
        let ctx = ctx.clone();
        tokio::spawn(async move {
            tracing::info!("[worker-{}] started", worker_id);
            loop {
                let task = ctx.queue.next_task().await;
                tracing::debug!("[worker-{}] running {} for {} ({})", worker_id, task.kind, task.repo_id, task.path);

                let result = execute_task(&ctx, &task).await;

                match result {
                    Ok(()) => {
                        ctx.queue.complete(task.id).await;
                        tracing::debug!("[worker-{}] completed {} #{}", worker_id, task.kind, task.id);
                    }
                    Err(e) => {
                        ctx.queue.fail(task.id, e.clone()).await;
                        tracing::warn!("[worker-{}] failed {} #{}: {}", worker_id, task.kind, task.id, e);
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
        TaskKind::BuildConnections => handlers::build_connections(ctx, task).await,
    }
}
