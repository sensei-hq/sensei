//! Resolve phase: build doc↔code connections, reconcile cross-repo links.
//!
//! Phase 7.1 retired the `resolve_edges` bare-name pass — FQN call/import edges
//! now resolve to their target node AT EMIT (`source_id → target_id` in
//! `process_file`), so there is no separate resolution barrier. Node degree is
//! recomputed at the `DetectCommunities` terminal barrier (its sole consumer).

use super::super::executor::TaskContext;
use super::super::Task;

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc↔code traceability edges and mark as indexed.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder_path = &task.folder_path;
    // abs_path lookup avoids name collisions across roots.
    let folder = match ctx.pg().get_repo_by_path(folder_path).await {
        Ok(f) => f,
        Err(e) => { tracing::warn!(error = %e, path = %folder_path, "build_connections: get_repo_by_path failed"); None }
    };
    let folder_name = folder.as_ref()
        .and_then(|f| f["name"].as_str())
        .unwrap_or_else(|| task.folder_name());
    let folder_id = match folder.as_ref()
        .and_then(|f| crate::api::util::json_uuid(&f["id"])) {
        Some(id) => id,
        None => { tracing::info!("build_connections: {} — folder not found", folder_path); return Ok(0); }
    };

    let nodes = ctx.pg().get_nodes_by_folder(&folder_id).await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "build_connections: get_nodes_by_folder failed"); Vec::new() });

    // Separate docs and code nodes
    let docs: Vec<&serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("doc"))
        .collect();
    let _functions: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter(|n| matches!(n["kind"].as_str(), Some("function" | "method")))
        .filter_map(|n| n["name"].as_str().map(|name| (name, n)))
        .collect();
    let files: std::collections::HashMap<&str, &serde_json::Value> = nodes.iter()
        .filter(|n| n["kind"].as_str() == Some("file"))
        .filter_map(|n| n["file_path"].as_str().map(|fp| (fp, n)))
        .collect();

    // Covers = doc-stem × file-stem proximity, a folder-DERIVED set. D2: build
    // the current set and REPLACE it in one transaction, so a doc that no longer
    // matches a file (renamed/deleted/moved) drops its stale covers instead of
    // them accumulating. `covers` becomes a pure function of the current
    // (docs, files) — idempotent, no duplication (which D1 also prevents).
    let mut covers: Vec<crate::db::pg_store::EdgeSpec> = Vec::new();
    for doc in &docs {
        let doc_id = match crate::api::util::json_uuid(&doc["id"]) { Some(id) => id, None => continue };
        let doc_path = doc["file_path"].as_str().unwrap_or("");
        // e.g. docs/api/auth.md → src/api/auth.ts
        let doc_stem = std::path::Path::new(doc_path)
            .file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if doc_stem.is_empty() { continue; }

        for (file_path, file_node) in &files {
            let file_stem = std::path::Path::new(file_path)
                .file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if file_stem == doc_stem && file_path != &doc_path
                && let Some(file_id) = crate::api::util::json_uuid(&file_node["id"]) {
                    covers.push(crate::db::pg_store::EdgeSpec {
                        source_id: doc_id, target_id: Some(file_id), target_name: None, target_file: None,
                    });
                }
        }
    }
    let edges_created = covers.len() as u32;
    // Atomic replace (rolls back to the old set on failure — never zero covers).
    if let Err(e) = ctx.pg().replace_edges_of_kind(&folder_id, "covers", &covers).await {
        tracing::warn!(error = %e, folder = %folder_name, "build_connections: replace covers failed");
    }

    // Collect libs from detected import targets
    let edges = ctx.pg().get_edges_by_kind(&folder_id, "imports").await.unwrap_or_else(|e| { tracing::warn!(error = %e, folder = %folder_name, "build_connections: get_edges_by_kind failed"); Vec::new() });
    let mut lib_set = std::collections::HashSet::new();
    for edge in &edges {
        if let Some(target_name) = edge["target_name"].as_str() {
            // External imports (not resolved to local files) are likely library imports
            if edge["target_id"].is_null() {
                lib_set.insert(target_name.to_string());
            }
        }
    }
    let libs: Vec<String> = lib_set.into_iter().collect();

    // D4.1: build_connections is NO LONGER the terminal barrier — it stamps the
    // detected libs (folder metadata read by the Observatory/query views) but
    // does NOT advance `folder_status`. DetectCommunities, chained after this,
    // is the sole writer of `indexed` (so `indexed` implies communities exist).
    // Stamping libs on a `failed` folder is harmless — it's metadata, not status.
    if let Err(e) = ctx.pg().set_folder_props(&folder_id, &serde_json::json!({"libs": libs})).await {
        tracing::warn!(error = %e, folder = %folder_name, "build_connections: set libs props failed");
    }

    // D4.5: recompute node degree here (in+out edge count, incl. the covers edges
    // just built) so it is fresh before the DetectCommunities terminal barrier
    // ranks god nodes. This is a folder-wide barrier AFTER all file/edge work, with
    // its OWN watchdog budget — it was briefly folded into detect_communities (7.1)
    // but that pushed edge-heavy giants (e.g. 287k-edge folders) past detect's 600s
    // watchdog, so degree-recompute moved back to its own barrier. Fail-open: a
    // degree miss must not strand the folder.
    if let Err(e) = ctx.pg().recompute_degrees_for_folder(&folder_id).await {
        tracing::warn!(error = %e, folder = %folder_name, "build_connections: recompute_degrees failed");
    }

    tracing::info!("build_connections: {} — {} traceability edges, {} libs detected", folder_name, edges_created, libs.len());
    Ok(edges_created)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use crate::tasks::{Task, TaskKind};
    
    

    /// Build a TaskContext backed by PgStore and a fresh TaskQueue.
    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn build_connections_recomputes_node_degree() {
        // D4.5 (relocated here from detect_communities in the 7.3 timeout fix):
        // degree is recomputed at the build_connections barrier — its OWN watchdog
        // budget — so it is fresh before DetectCommunities ranks god nodes, without
        // eating detect's 600s budget on edge-heavy giants.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/bc_degree_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "bcd", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "bcd-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();
        let a = ctx.pg().upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2)).await.unwrap();
        let b = ctx.pg().upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
        ctx.pg().insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        let (da,): (Option<i32>,) = sqlx_core::query_as::query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
            .bind(a).fetch_one(ctx.pg().pool()).await.unwrap();
        let (db,): (Option<i32>,) = sqlx_core::query_as::query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
            .bind(b).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(da, Some(1), "build_connections recomputed degree — a is the source of one call");
        assert_eq!(db, Some(1), "build_connections recomputed degree — b is the target of one call");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn build_connections_is_fail_closed_on_a_failed_folder() {
        // D4.1/D6d: build_connections stamps libs but no longer advances the
        // folder status, so a folder a ProcessFile marked `failed` (D6c-trigger)
        // stays `failed` — only DetectCommunities (fail-closed) can flip it, and
        // only from `indexing`.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/failclosed_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "the barrier must not mark a failed folder indexed");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn build_connections_does_not_flip_status_indexed() {
        // D4.1: build_connections is NO LONGER the terminal barrier. It stamps
        // libs (folder metadata) but leaves the folder `indexing`; DetectCommunities
        // — the new terminal barrier — flips it to `indexed` after communities are
        // computed, so `indexed` implies communities exist.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/clean_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "cl", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cl-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("indexing"),
            "build_connections leaves the folder indexing (D4.1 moved the terminal barrier to DetectCommunities)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn resolve_libs_is_fail_closed_on_a_failed_folder() {
        // D4.1/D6d: resolve_libs stamps the walked libs but no longer advances
        // the folder status, so a `failed` folder stays `failed` here — the
        // terminal barrier (DetectCommunities) is the only writer of `indexed`.
        let ctx = make_ctx().await;
        let tmp = tempfile::tempdir().unwrap(); // empty dir → no libs to walk
        let folder_path = tmp.path().to_string_lossy().to_string();
        let root_id = ctx.pg().add_watch_root(&folder_path, "rl_fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "rl-fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::ResolveLibs, &folder_path, &folder_path);
        super::super::resolve_libs(&ctx, &task).await.unwrap();

        assert_eq!(ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(), Some("failed"),
            "resolve_libs must not mark a failed folder indexed (fail-closed)");
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn build_connections_replaces_stale_covers() {
        // D2: covers is REPLACED, not appended. A covers edge whose covered file
        // no longer matches (renamed/removed) is GONE after build_connections —
        // the shrink case nothing exercised before. Re-running is idempotent.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/coversreplace_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "cr", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cr-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        // Doc "auth.md" + a matching file "auth.rs" (stem "auth"); plus "other.rs".
        let doc = ctx.pg().upsert_node(&fid, "doc", "auth", "docs/auth.md", None, None, None, None).await.unwrap();
        let auth = ctx.pg().upsert_node(&fid, "file", "auth", "src/auth.rs", None, None, None, None).await.unwrap();
        let other = ctx.pg().upsert_node(&fid, "file", "other", "src/other.rs", None, None, None, None).await.unwrap();

        // A STALE covers edge doc→other (as if a prior scan matched it).
        ctx.pg().insert_edge(&fid, &doc, Some(&other), None, None, "covers").await.unwrap();

        let covers_count = "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind";

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        let (n,): (i64,) = sqlx_core::query_as::query_as(covers_count).bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(n, 1, "exactly the current covers match — stale one removed");
        let (tgt,): (Option<uuid::Uuid>,) = sqlx_core::query_as::query_as(
            "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(tgt, Some(auth), "the surviving covers edge points at the matching file");

        // Idempotent: a second run yields the same single edge.
        build_connections(&ctx, &task).await.unwrap();
        let (n2,): (i64,) = sqlx_core::query_as::query_as(covers_count).bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(n2, 1, "re-running build_connections is idempotent");

        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    #[tokio::test]
    async fn build_connections_does_not_clobber_doc_references() {
        // Regression (D2): the covers replace is folder-wide BY KIND, so it must
        // touch only `covers` — never `references`. process_file emits a doc's
        // explicit file/symbol refs as `references`; before the file-refs→
        // references fix those were `covers` and build_connections' wholesale
        // replace destroyed them.
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/coversref_{}", uuid::Uuid::new_v4());
        let root_id = ctx.pg().add_watch_root(&folder_path, "cx", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cx-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let doc = ctx.pg().upsert_node(&fid, "doc", "guide", "docs/guide.md", None, None, None, None).await.unwrap();
        ctx.pg().upsert_node(&fid, "file", "engine", "src/engine.rs", None, None, None, None).await.unwrap();
        // An explicit doc→file reference, as process_file now emits it: `references`.
        ctx.pg().insert_edge(&fid, &doc, None, Some("src/engine.rs"), None, "references").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        let (refs,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='references'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(refs, 1, "build_connections must not wipe doc→file `references` edges");

        ctx.pg().remove_watch_root(&root_id).await.ok();
    }
}
