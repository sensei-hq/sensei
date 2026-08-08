//! Doc-drift scan — populate `inference.drift_items` from the analyzer.
//!
//! Reuses [`PgStore::scan_project_doc_drift`](crate::db::pg_store::PgStore::scan_project_doc_drift)
//! — the same writer the manual `POST /api/projects/{id}/drift/scan` endpoint
//! calls — so drift populates on its own as projects become due, instead of
//! only when a user clicks scan. `task.path` carries the project id (UUID
//! string), exactly like `AnalyzeProject`.

use super::super::executor::TaskContext;
use super::super::Task;

/// Handler for `TaskKind::ScanDocDrift`: scan the project's doc nodes for
/// backtick identifier mentions that no longer resolve to a live code node,
/// writing `broken` rows and resolving any that the code caught up to. The
/// scan itself (dedup + resolve-in-place) lives in `scan_project_doc_drift`;
/// this handler is the thin task-side wrapper. Returns the count of
/// newly-broken references.
pub async fn scan_doc_drift(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(&task.path)
        .map_err(|_| format!("ScanDocDrift: invalid project id '{}'", task.path))?;
    let summary = ctx
        .pg()
        .scan_project_doc_drift(&project_id)
        .await
        .map_err(|e| format!("scan_project_doc_drift failed: {e}"))?;
    let field = |k: &str| summary.get(k).and_then(serde_json::Value::as_i64).unwrap_or(0);
    let (scanned, new_broken, resolved) = (field("scannedDocs"), field("newBroken"), field("resolved"));
    tracing::info!(
        project = %project_id, scanned_docs = scanned, new_broken, resolved,
        "scan_doc_drift: {project_id} — scanned {scanned} docs, {new_broken} new broken, {resolved} resolved"
    );
    Ok(new_broken as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    
    use crate::tasks::TaskKind;
    

    use crate::tasks::test_support::make_ctx;

    #[tokio::test]
    async fn rejects_bad_project_id_payload() {
        let ctx = make_ctx().await;
        // A non-UUID payload is a bug, not empty work — fail loudly (parse guard).
        let bad = Task::new(TaskKind::ScanDocDrift, "", "not-a-uuid");
        let err = scan_doc_drift(&ctx, &bad).await.unwrap_err();
        assert!(err.contains("invalid project id"), "got: {err}");
        // An empty payload is likewise rejected rather than scanning nothing.
        let empty = Task::new(TaskKind::ScanDocDrift, "", "");
        assert!(scan_doc_drift(&ctx, &empty).await.is_err());
    }

    #[tokio::test]
    async fn flags_broken_only_after_a_real_symbol_is_removed_then_resolves() {
        // Symbol-history semantics: a doc mention is drift ONLY when it names
        // something that WAS a real symbol and is now gone — not merely "absent
        // from the graph" (that over-fired on enum variants / camelCase fields /
        // tool strings). So: resolvable → not drift; removed → drift; re-added →
        // resolved. The scan reads the doc off disk via the folder abs_path.
        let ctx = make_ctx().await;
        let pg = ctx.pg();

        let sym = format!("drift_probe_{}", uuid::Uuid::new_v4().simple());
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), format!("The `{sym}` helper is the entry point.")).unwrap();
        let abs = dir.path().to_string_lossy().to_string();

        let pid = pg.create_project(&format!("_test:drift-{}", uuid::Uuid::new_v4()), None, None).await.unwrap();
        let root = pg.add_watch_root(&format!("/_test/drift-root-{}", uuid::Uuid::new_v4()), "t", &serde_json::json!([])).await.unwrap();
        let fid = pg.upsert_repo(&root, "drift-repo", &abs).await.unwrap();
        pg.set_folder_project(&fid, &pid, "primary", None).await.unwrap();
        pg.merge_doc(&fid, "README", "README.md").await.unwrap();

        let task = Task::new(TaskKind::ScanDocDrift, "", &pid.to_string());
        let broken = |pid| async move {
            let n: (i64,) = sqlx_core::query_as::query_as(
                "SELECT count(*) FROM inference.drift_items di
                   JOIN sensei.folders f ON f.id = di.folder_id
                  WHERE f.project_id = $1 AND di.status = 'broken' AND di.resolved_at IS NULL"
            ).bind(pid).fetch_one(pg.pool()).await.unwrap();
            n.0
        };

        // 1. The symbol EXISTS as code ⇒ the mention resolves ⇒ not drift. This
        //    scan also records `sym` into the symbol-name history.
        pg.merge_function(&fid, &sym, "src/events.rs", Some("fn process_event()"), Some(1), Some(9), None).await.unwrap();
        assert_eq!(scan_doc_drift(&ctx, &task).await.unwrap(), 0, "a resolvable mention is not drift");
        assert_eq!(broken(pid).await, 0);

        // 2. Remove the symbol ⇒ the doc now references something that WAS real
        //    and is gone ⇒ one broken row.
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1 AND kind = 'function' AND name = $2")
            .bind(fid).bind(&sym).execute(pg.pool()).await.unwrap();
        assert_eq!(scan_doc_drift(&ctx, &task).await.unwrap(), 1, "removed-symbol mention ⇒ one broken row");
        assert_eq!(broken(pid).await, 1);

        // 3. Idempotent: re-scan adds no duplicate.
        assert_eq!(scan_doc_drift(&ctx, &task).await.unwrap(), 0, "no new broken on re-scan");
        let total: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM inference.drift_items di
               JOIN sensei.folders f ON f.id = di.folder_id WHERE f.project_id = $1"
        ).bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total.0, 1, "dedup — still a single row");

        // 4. Add the code back ⇒ the mention resolves in place.
        pg.merge_function(&fid, &sym, "src/events.rs", Some("fn process_event()"), Some(1), Some(9), None).await.unwrap();
        assert_eq!(scan_doc_drift(&ctx, &task).await.unwrap(), 0, "nothing newly broken");
        assert_eq!(broken(pid).await, 0, "the previously-broken row is no longer open");
        let resolved: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM inference.drift_items di
               JOIN sensei.folders f ON f.id = di.folder_id
              WHERE f.project_id = $1 AND di.status = 'current' AND di.resolved_at IS NOT NULL"
        ).bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(resolved.0, 1, "the drift row resolved once the code node exists again");

        // cleanup — drift_items FK doc/code nodes, so delete them before nodes.
        let pool = pg.pool();
        sqlx_core::query::query("DELETE FROM inference.drift_items WHERE folder_id = $1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.nodes WHERE folder_id = $1").bind(fid).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.symbol_names WHERE name = $1").bind(&sym).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE root_id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.folders_to_watch WHERE id = $1").bind(root).execute(pool).await.ok();
        sqlx_core::query::query("DELETE FROM sensei.projects WHERE id = $1").bind(pid).execute(pool).await.ok();
    }
}
