//! `DeriveOrmSchema` (#251 step 7c): the database an ORM maps, derived from
//! what the walks wrote, into the schema store. EF Core first (spec 20 §7c).
//!
//! Enqueued by `process_repo_files` for a repository with a `.csproj`, BEHIND
//! the file tasks — the derivation reads their classes, properties and call
//! sites — and ahead of the `DetectCommunities` barrier.

use std::path::Path;

use super::super::Task;
use super::super::executor::TaskContext;

/// Whether a repository holds a .NET project, found by the same search every
/// other walk uses.
pub fn has_dotnet_project(repo_root: &Path) -> Result<bool, String> {
    Ok(!crate::indexer::search::search(&crate::indexer::search::SearchSpec {
        root: repo_root,
        patterns: &["*.csproj".to_string()],
        kinds: crate::indexer::search::Kinds::Files,
        exclusions: &[],
        prunes: &crate::indexer::repo::default_prunes(),
        gitignore: false,
        hidden: false,
    })?
    .found
    .is_empty())
}

pub async fn derive_orm_schema(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(repo) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        tracing::warn!(repo = %task.folder_path, "derive_orm_schema: repo folder row missing");
        return Ok(0);
    };
    let Some(folder_id) = crate::api::util::json_uuid(&repo["id"]) else {
        return Err(format!("derive_orm_schema: repo row for {} has no id", task.folder_path));
    };

    let input = crate::indexer::ef_core::load(ctx.pg(), &folder_id, &task.folder_path).await?;
    let derived = crate::indexer::ef_core::derive(&input)?;
    let mut objects = 0usize;
    for (scope, facts) in &derived.batches {
        let written =
            crate::indexer::persist::write_schema(ctx.pg(), &folder_id, scope, facts).await?;
        objects += written.objects;
        if !written.unplaced_refs.is_empty() {
            tracing::warn!(repo = %task.folder_path, package = %scope.package,
                unplaced = written.unplaced_refs.len(), "derive_orm_schema: keys to tables not described");
        }
    }
    if !derived.skipped.is_empty() {
        tracing::warn!(repo = %task.folder_path, skipped = derived.skipped.len(),
            first = ?derived.skipped.first(), "derive_orm_schema: entities with no place in the store");
    }
    tracing::info!(repo = %task.folder_path, objects, "derive_orm_schema");
    Ok(u32::try_from(objects).unwrap_or(u32::MAX))
}
