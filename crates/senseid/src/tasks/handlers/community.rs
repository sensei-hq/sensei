//! Community detection handler — runs label propagation on a folder's call graph.

use super::super::executor::TaskContext;
use super::super::Task;

pub async fn detect_communities(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Folder '{}' not found", task.folder_path))?;

    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or("Invalid folder id")?;
    let folder_name = folder["name"].as_str().unwrap_or_else(|| task.folder_name());

    let count = crate::indexer::community::detect_communities_for_folder(
        ctx.pg(), &folder_id, Some(ctx.app_state.gateway.as_ref()),
    ).await?;

    // D4.1: DetectCommunities is the TERMINAL scan barrier. Only on a successful
    // detection do we flip the folder to `indexed` — so `indexed` implies
    // communities are computed (a mid-detect failure returns `Err` above and the
    // folder stays `indexing` for recovery). Fail-closed (D6d): the helper
    // promotes only from `indexing`, so a folder a ProcessFile marked `failed`
    // stays `failed`, and the daily analyzer re-detect of an already-`indexed`
    // folder is left untouched (no spurious `indexed_at` bump).
    super::helpers::mark_folder_indexed_fail_closed(ctx, &folder_id, folder_name).await;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::tasks::queue::TaskQueue;
    use crate::tasks::{Task, TaskKind};
    use crate::api::state::SharedState;

    async fn make_ctx() -> Arc<TaskContext> {
        let queue = Arc::new(TaskQueue::new());
        let gateway = crate::api::gateway_init::init_gateway_test().await;
        let app_state = Arc::new(SharedState {
            task_queue: queue.clone(),
            pg: crate::db::pg_store::PgStore::connect_test().await.unwrap(),
            gateway,
            event_tx: { let (tx, _) = tokio::sync::broadcast::channel(16); tx },
            breaker: std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new())),
            provisioning: None,
        });
        Arc::new(TaskContext {
            queue,
            app_state,
            _graph_path: None,
            logger: sensei_logger::Logger::noop(),
        })
    }

    #[tokio::test]
    async fn detect_communities_marks_indexing_folder_indexed() {
        // D4.1: DetectCommunities is the terminal scan barrier — on success it
        // flips an in-flight (`indexing`) folder to `indexed`, so `indexed`
        // implies communities are computed.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/detect_terminal_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "dt", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "dt-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let task = Task::new(TaskKind::DetectCommunities, &folder_path, "");
        detect_communities(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexed"),
            "DetectCommunities flips an indexing folder to indexed (terminal barrier)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn detect_communities_is_fail_closed_on_failed_folder() {
        // D6d: a folder a ProcessFile marked `failed` mid-scan must NOT be
        // advanced to `indexed` by the terminal barrier — leave it `failed` so
        // boot-reconcile / bounded-retry re-drives it.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/detect_failclosed_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "df", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "df-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::DetectCommunities, &folder_path, "");
        detect_communities(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the terminal barrier must not mark a failed folder indexed (fail-closed)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn detect_communities_does_not_restamp_an_indexed_folder() {
        // Daily analyzer re-detect: DetectCommunities runs on an already-`indexed`
        // folder. It must leave it `indexed` and NOT bump `indexed_at` (the folder
        // was not re-indexed, only re-detected) — the helper promotes only from
        // `indexing`.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/detect_norestamp_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "dn", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "dn-repo", &folder_path).await.unwrap();
        ctx.pg().set_folder_props(&fid, &serde_json::json!({"indexed_at": "2020-01-01T00:00:00+00:00"})).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexed").await.unwrap();

        let task = Task::new(TaskKind::DetectCommunities, &folder_path, "");
        detect_communities(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexed"),
            "an already-indexed folder stays indexed");
        let row: (serde_json::Value,) = sqlx_core::query_as::query_as(
            "SELECT props FROM sensei.folders WHERE id = $1"
        ).bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(row.0.get("indexed_at").and_then(|v| v.as_str()), Some("2020-01-01T00:00:00+00:00"),
            "indexed_at is not bumped on a re-detect (promote only from indexing)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }
}
