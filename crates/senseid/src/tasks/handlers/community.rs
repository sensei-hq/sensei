//! Community detection handler — runs label propagation on a folder's call graph.

use super::super::executor::TaskContext;
use super::super::Task;

/// Test-only fault seam: lets a test force the DetectCommunities terminal barrier
/// to hit its fatal path (folder → `failed`, `Err`) without a live DB fault, so
/// the fail-closed behaviour is exercised deterministically.
#[cfg(test)]
pub(super) mod fault {
    use std::collections::HashSet;
    use std::sync::Mutex;

    static FAIL_PATHS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

    pub fn fail_for(folder_path: &str) {
        FAIL_PATHS.lock().unwrap().get_or_insert_with(HashSet::new).insert(folder_path.to_string());
    }
    pub fn clear(folder_path: &str) {
        if let Some(set) = FAIL_PATHS.lock().unwrap().as_mut() {
            set.remove(folder_path);
        }
    }
    pub fn should_fail(folder_path: &str) -> bool {
        FAIL_PATHS.lock().unwrap().as_ref().is_some_and(|s| s.contains(folder_path))
    }
}

pub async fn detect_communities(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder = ctx.pg().get_repo_by_path(&task.folder_path).await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Folder '{}' not found", task.folder_path))?;

    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or("Invalid folder id")?;
    let folder_name = folder["name"].as_str().unwrap_or_else(|| task.folder_name());

    // Authoritative detection — no model calls; this is what the terminal barrier
    // gates on. A failure here is FATAL for the folder: record `failed` (D6d
    // fail-closed) so it is not stranded at `indexing` (boot-reconcile / bounded
    // retry re-drives it), then surface the error. Without this, a detection
    // error would `?`-propagate before the mark below and leave the folder at
    // `indexing` forever.
    let detect = crate::indexer::community::detect_communities_for_folder(ctx.pg(), &folder_id).await;
    #[cfg(test)]
    let detect = if fault::should_fail(&task.folder_path) {
        Err("injected: detect failure (test fault seam)".to_string())
    } else {
        detect
    };
    let count = match detect {
        Ok(c) => c,
        Err(e) => {
            if let Err(se) = ctx.pg().update_folder_status(&folder_id, "failed").await {
                tracing::warn!(error = %se, folder = %folder_name,
                    "detect_communities: marking folder failed also failed");
            }
            return Err(e);
        }
    };

    // D4.1 terminal barrier: communities computed → flip the folder to `indexed`
    // (fail-closed, promotes only from `indexing`). Sole writer of `indexed`, so
    // `indexed` implies communities exist.
    super::helpers::mark_folder_indexed_fail_closed(ctx, &folder_id, folder_name).await;

    // D4.5 description enrichment — NON-authoritative, fail-open (spec W3): runs
    // AFTER `indexed`, synchronously but best-effort and bounded, so a slow or
    // failing model can never strand the folder. Never a template.
    crate::indexer::community::enrich_community_descriptions(
        ctx.pg(), ctx.app_state.gateway.as_ref(), &folder_id,
    ).await;

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use crate::tasks::{Task, TaskKind};
    

    use crate::tasks::test_support::make_ctx;

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

    #[tokio::test]
    async fn detect_communities_marks_folder_failed_on_detect_error() {
        // Review finding #1: a DetectCommunities failure is FATAL for the folder —
        // it must be recorded `failed` (fail-closed), never left stranded at
        // `indexing`. The terminal barrier moved onto detection (which can Err),
        // so the handler must translate that Err into a `failed` status the same
        // way ProcessFile's fatal path does.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/detect_err_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "der", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "der-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        fault::fail_for(&folder_path);
        let task = Task::new(TaskKind::DetectCommunities, &folder_path, "");
        let r = detect_communities(&ctx, &task).await;
        fault::clear(&folder_path);

        assert!(r.is_err(), "a detection failure propagates as Err");
        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the folder is recorded failed, not stranded at indexing");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }
}
