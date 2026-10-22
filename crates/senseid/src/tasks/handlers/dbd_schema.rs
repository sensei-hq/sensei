//! `ImportDbdSchema` (#251 step 4): one dbd design's structure into the schema
//! store.
//!
//! Enqueued by `process_repo_files` per `design.yaml`, BEHIND the repo's file
//! tasks: the SQL walk writes the table/view/routine nodes with their spans,
//! and this fills in the detail on those nodes. Run before them, every object
//! would land on a stub. `DetectCommunities` waits on it, so a folder is not
//! `indexed` until its schema is.

use std::path::{Path, PathBuf};

use super::super::Task;
use super::super::executor::TaskContext;

/// The `design.yaml` files under a repository, repo-relative — found by the
/// same search every other walk uses, under the filename the dbd manifest
/// adapter claims.
pub fn designs_under(repo_root: &Path) -> Result<Vec<String>, String> {
    let Some(adapter) =
        crate::adapters::manifest::registered_adapters().iter().find(|a| a.ecosystem() == "dbd")
    else {
        return Err("no dbd manifest adapter is registered".to_string());
    };
    let names: Vec<String> =
        adapter.manifest_filenames().iter().map(|s| (*s).to_string()).collect();
    Ok(crate::indexer::search::search(&crate::indexer::search::SearchSpec {
        root: repo_root,
        patterns: &names,
        kinds: crate::indexer::search::Kinds::Files,
        exclusions: &[],
        prunes: &crate::indexer::repo::default_prunes(),
        gitignore: false,
        hidden: false,
    })?
    .found
    .iter()
    .map(|f| f.rel_path.to_string_lossy().to_string())
    .collect())
}

/// Import the design at `task.path` (repo-relative) into the repo folder's
/// schema store. Returns the number of objects written.
pub async fn import_dbd_schema(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let Some(repo) = ctx.pg().get_repo_by_path(&task.folder_path).await? else {
        // The same answer the sibling handlers give: the structure write has
        // not run, so there is no folder to hang the detail off.
        tracing::warn!(repo = %task.folder_path, "import_dbd_schema: repo folder row missing");
        return Ok(0);
    };
    let Some(folder_id) = crate::api::util::json_uuid(&repo["id"]) else {
        return Err(format!("import_dbd_schema: repo row for {} has no id", task.folder_path));
    };

    let design: PathBuf = Path::new(&task.folder_path).join(&task.path);
    let text = std::fs::read_to_string(&design)
        .map_err(|e| format!("import_dbd_schema: {}: {e}", design.display()))?;
    // The package the SQL walk placed this design's DDL under. A manifest that
    // names none placed nothing, so there are no nodes to describe.
    let Some(package) = crate::indexer::placement::package_named_by(&design, &text) else {
        tracing::info!(design = %design.display(), "import_dbd_schema: the design names no workspace — its DDL is not placed either");
        return Ok(0);
    };

    let scope = crate::indexer::persist::BatchScope {
        source: crate::indexer::facts::SchemaSource::Dbd,
        package: package.clone(),
    };
    // Parsing a whole design is CPU-bound; off the runtime, as `process_file`
    // does for one file.
    let imported = {
        let design = design.clone();
        tokio::task::spawn_blocking(move || {
            crate::indexer::dbd_import::import_design(&design, &package)
        })
        .await
        .map_err(|e| format!("import_dbd_schema: the import task failed: {e}"))??
    };

    let written =
        crate::indexer::persist::write_schema(ctx.pg(), &folder_id, &scope, &imported.facts)
            .await?;
    if !imported.skipped.is_empty() || !written.unplaced_refs.is_empty() {
        tracing::warn!(
            design = %design.display(),
            skipped = imported.skipped.len(),
            unplaced_refs = written.unplaced_refs.len(),
            "import_dbd_schema: some of the design has no place in the store"
        );
    }
    tracing::info!(design = %design.display(), objects = written.objects, "import_dbd_schema");
    Ok(u32::try_from(written.objects).unwrap_or(u32::MAX))
}
