//! Community detection handler — runs label propagation on a folder's call graph.

use super::super::executor::TaskContext;
use super::super::Task;

pub async fn detect_communities(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder_name = task.folder_name();
    let folder = ctx.pg().get_repo_by_name(folder_name).await
        .map_err(|e| format!("DB error: {}", e))?
        .ok_or_else(|| format!("Folder '{}' not found", folder_name))?;

    let folder_id = crate::api::util::json_uuid(&folder["id"])
        .ok_or("Invalid folder id")?;

    crate::indexer::community::detect_communities_for_folder(ctx.pg(), &folder_id).await
}
