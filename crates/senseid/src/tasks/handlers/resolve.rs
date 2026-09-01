//! Resolve phase: build doc↔code connections, reconcile cross-repo links.
//!
//! Phase 7.1 retired the `resolve_edges` bare-name pass — FQN call/import edges
//! now resolve to their target node AT EMIT (`source_id → target_id` in
//! `process_file`), so there is no separate resolution barrier. Node degree is
//! recomputed at the `DetectCommunities` terminal barrier (its sole consumer).

use super::super::Task;
use super::super::executor::TaskContext;

// ── Build Connections ─────────────────────────────────────────────────────

/// Build doc↔code traceability edges and mark as indexed.
pub async fn build_connections(ctx: &TaskContext, task: &Task) -> Result<u32, String> {
    let folder_path = &task.folder_path;
    // abs_path lookup avoids name collisions across roots.
    let folder = match ctx.pg().get_repo_by_path(folder_path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(error = %e, path = %folder_path, "build_connections: get_repo_by_path failed");
            None
        }
    };
    let folder_name =
        folder.as_ref().and_then(|f| f["name"].as_str()).unwrap_or_else(|| task.folder_name());
    let folder_id = match folder.as_ref().and_then(|f| crate::api::util::json_uuid(&f["id"])) {
        Some(id) => id,
        None => {
            tracing::info!("build_connections: {} — folder not found", folder_path);
            return Ok(0);
        }
    };

    // Doc↔code traceability is NOT built here any more.
    //
    // It was 601 `covers` rows in `sensei.edges`, rebuilt wholesale on every index
    // via `replace_edges_of_kind`, and then read back by the `sensei.doc_coverage`
    // view — which joined those rows to the very nodes they had been derived from.
    // The comment that used to sit here conceded what that made it: "covers becomes
    // a pure function of the current (docs, files) — idempotent."
    //
    // A pure function of current stored state is a view, not a row somebody writes.
    // `doc_coverage` now computes the stem match itself and reproduces all 601 pairs
    // exactly (verified live, 0 divergent). The `Path::file_stem()` call that lived
    // here is gone, so the stem rule has exactly one implementation — a move, not a
    // SQL copy of a Rust rule.

    // Dependency detection deliberately does NOT happen here.
    //
    // This used to derive a lib list from the import edges by treating "the edge
    // did not resolve" as "the target is external", then overwrite the list
    // `resolve_libs` had already walked out of the manifest ("last write wins, as
    // before"). The proxy was wrong in both directions. Measured 2026-09-01 on the
    // live DB: 791 of the `sensei` folder's 1,040 entries were this repo's own
    // code (`crate::log_collector::LogCollector`, `./WizardRail.svelte`,
    // `$lib/nav`), rokkit 807 of 890, OmniRoute 4,085 of 5,468 — and it would have
    // lost every genuine dependency the moment resolution started working, because
    // `target_id` and `target_name` are mutually exclusive: resolving an edge
    // erases the name the count was reading.
    //
    // The two questions now have one owner each. What a folder DECLARES it depends
    // on comes from the manifest walk in `resolve_libs`, which this no longer
    // clobbers. What its code actually CALLS INTO is a query over
    // `sensei.graph_nodes`, which reads locality off the node the writer created
    // rather than guessing from an edge:
    //
    //   SELECT count(DISTINCT n.name) FROM sensei.edges e
    //     JOIN sensei.graph_nodes n ON n.id = e.target_id
    //    WHERE e.folder_id = $1 AND n.locality = 'external'
    //
    // D4.1 still holds: this is NOT the terminal barrier and does not advance
    // `folder_status`. DetectCommunities, chained after it, is the sole writer of
    // `indexed` (so `indexed` implies communities exist).

    // D4.5: recompute node degree here (in+out edge count, incl. the covers edges
    // just built) so it is fresh before the DetectCommunities terminal barrier
    // ranks god nodes. This is a folder-wide barrier AFTER all file/edge work, with
    // its OWN watchdog budget — it was briefly folded into detect_communities (7.1)
    // but that pushed edge-heavy giants (e.g. 287k-edge folders) past detect's 600s
    // watchdog, so degree-recompute moved back to its own barrier. Fail-open: a
    // degree miss must not strand the folder.
    let degrees_changed = match ctx.pg().recompute_degrees_for_folder(&folder_id).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, folder = %folder_name, "build_connections: recompute_degrees failed");
            0
        }
    };

    tracing::info!(
        "build_connections: {} — {} node degrees refreshed",
        folder_name,
        degrees_changed
    );
    Ok(degrees_changed as u32)
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
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "bcd", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "bcd-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();
        let a = ctx
            .pg()
            .upsert_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
            .await
            .unwrap();
        let b = ctx
            .pg()
            .upsert_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
            .await
            .unwrap();
        ctx.pg().insert_edge(&fid, &a, Some(&b), None, None, "calls").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        let (da,): (Option<i32>,) =
            sqlx_core::query_as::query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
                .bind(a)
                .fetch_one(ctx.pg().pool())
                .await
                .unwrap();
        let (db,): (Option<i32>,) =
            sqlx_core::query_as::query_as("SELECT degree FROM sensei.nodes WHERE id=$1")
                .bind(b)
                .fetch_one(ctx.pg().pool())
                .await
                .unwrap();
        assert_eq!(
            da,
            Some(1),
            "build_connections recomputed degree — a is the source of one call"
        );
        assert_eq!(
            db,
            Some(1),
            "build_connections recomputed degree — b is the target of one call"
        );
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
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("failed"),
            "the barrier must not mark a failed folder indexed"
        );
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
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "cl", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cl-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("indexing"),
            "build_connections leaves the folder indexing (D4.1 moved the terminal barrier to DetectCommunities)"
        );
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    /// `build_connections` no longer produces `covers` edges — `sensei.doc_coverage`
    /// computes the doc↔code pairing from (docs, files) directly, so storing 601
    /// rows for a view to read back off the same nodes was duplication.
    #[tokio::test]
    async fn build_connections_writes_no_covers_edges() {
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/nocovers_{}", uuid::Uuid::new_v4());
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "nc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "nc-repo", &folder_path).await.unwrap();
        // A doc and a stem-matching file: the input the old builder paired.
        ctx.pg()
            .upsert_node(&fid, "doc", "design", "docs/design.md", None, None, Some(1), Some(1))
            .await
            .unwrap();
        ctx.pg()
            .upsert_node(&fid, "file", "design.rs", "src/design.rs", None, None, Some(1), Some(1))
            .await
            .unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        assert!(
            ctx.pg().get_edges_by_kind(&fid, "covers").await.unwrap().is_empty(),
            "the pairing lives in sensei.doc_coverage, not in sensei.edges"
        );
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    /// `build_connections` must NOT overwrite the dependency list that
    /// `resolve_libs` derived from the manifest.
    ///
    /// It used to, by documented design ("re-stamps libs from its import-derived
    /// set afterwards (last write wins, as before)"), and the set it stamped came
    /// from asking "did this edge fail to resolve?" as a proxy for "is this
    /// external?" — measured live 2026-09-01, that reported 791 of the `sensei`
    /// folder's 1,040 entries as this repo's own code, and rokkit 807 of 890.
    /// The locality question now belongs to `sensei.graph_nodes`, which reads what
    /// the writer recorded on the node instead of guessing from an edge.
    #[tokio::test]
    async fn build_connections_does_not_clobber_the_manifest_derived_libs() {
        let ctx = make_ctx().await;
        let folder_path = format!("/tmp/libskeep_{}", uuid::Uuid::new_v4());
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "lk", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "lk-repo", &folder_path).await.unwrap();

        // What resolve_libs (the manifest walk) would have left behind.
        ctx.pg()
            .set_folder_props(&fid, &serde_json::json!({"libs": ["axum", "serde"]}))
            .await
            .unwrap();

        // An unresolved import edge whose target is plainly local — exactly the row
        // the old proxy would have promoted to a "dependency".
        let f = ctx
            .pg()
            .upsert_node(&fid, "file", "api.rs", "src/api.rs", None, None, Some(1), Some(9))
            .await
            .unwrap();
        ctx.pg()
            .insert_edge(&fid, &f, None, Some("crate::helpers::compute"), None, "imports")
            .await
            .unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        // Read by folder id, not by name: the repo name is fixed and `sensei_test`
        // is shared, so a name lookup can return a folder left by an earlier run.
        let props: (serde_json::Value,) =
            sqlx_core::query_as::query_as("SELECT props FROM sensei.folders WHERE id = $1")
                .bind(fid)
                .fetch_one(ctx.pg().pool())
                .await
                .unwrap();
        let libs: Vec<String> = props.0["libs"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        assert_eq!(
            libs,
            vec!["axum".to_string(), "serde".to_string()],
            "the manifest-derived list survives; and `crate::helpers::compute` — a \
             local module — is never promoted to a dependency"
        );
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
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "rl_fc", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "rl-fc-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "failed").await.unwrap();

        let task = Task::new(TaskKind::ResolveLibs, &folder_path, &folder_path);
        super::super::resolve_libs(&ctx, &task).await.unwrap();

        assert_eq!(
            ctx.pg().get_folder_status(&fid).await.unwrap().as_deref(),
            Some("failed"),
            "resolve_libs must not mark a failed folder indexed (fail-closed)"
        );
        ctx.pg().remove_watch_root(&root_id).await.ok();
    }

    /// The D2 concern this replaces — "a covers edge whose covered file no longer
    /// matches is stale and must be removed" — is now structurally impossible
    /// rather than maintained by a wholesale replace. Nothing is stored, so
    /// nothing can go stale: rename the file and the pairing simply is not there
    /// on the next read.
    #[tokio::test]
    async fn doc_coverage_cannot_hold_a_stale_pairing() {
        let ctx = make_ctx().await;
        let suffix = format!("cr_{}", uuid::Uuid::new_v4().simple());
        let folder_path = format!("/tmp/{suffix}");
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "cr", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, &suffix, &folder_path).await.unwrap();

        ctx.pg()
            .upsert_node(&fid, "doc", "auth", "docs/auth.md", None, None, None, None)
            .await
            .unwrap();
        let code = ctx
            .pg()
            .upsert_node(&fid, "file", "auth", "src/auth.rs", None, None, None, None)
            .await
            .unwrap();

        let drift = ctx.pg().get_doc_drift(&suffix).await.unwrap();
        assert_eq!(drift.len(), 1, "the stem match pairs, got {drift:?}");
        assert_eq!(drift[0]["codeFile"], "src/auth.rs");

        // Rename the file out from under the doc. No task runs; no edge is rewritten.
        sqlx_core::query::query(
            "UPDATE sensei.nodes SET file_path = 'src/renamed.rs' WHERE id = $1",
        )
        .bind(code)
        .execute(ctx.pg().pool())
        .await
        .unwrap();

        let drift = ctx.pg().get_doc_drift(&suffix).await.unwrap();
        assert!(
            drift.is_empty(),
            "the pairing is gone the instant the stem stops matching — no replace \
             pass, no window in which a stale row is visible; got {drift:?}"
        );
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
        let root_id =
            ctx.pg().add_watch_root(&folder_path, "cx", &serde_json::json!([])).await.unwrap();
        let fid = ctx.pg().upsert_repo(&root_id, "cx-repo", &folder_path).await.unwrap();
        ctx.pg().update_folder_status(&fid, "indexing").await.unwrap();

        let doc = ctx
            .pg()
            .upsert_node(&fid, "doc", "guide", "docs/guide.md", None, None, None, None)
            .await
            .unwrap();
        ctx.pg()
            .upsert_node(&fid, "file", "engine", "src/engine.rs", None, None, None, None)
            .await
            .unwrap();
        // An explicit doc→file reference, as process_file now emits it: `references`.
        ctx.pg()
            .insert_edge(&fid, &doc, None, Some("src/engine.rs"), None, "references")
            .await
            .unwrap();

        let task = Task::new(TaskKind::BuildConnections, &folder_path, &folder_path);
        build_connections(&ctx, &task).await.unwrap();

        let (refs,): (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM sensei.edges WHERE folder_id=$1 AND kind='references'::sensei.edge_kind")
            .bind(fid).fetch_one(ctx.pg().pool()).await.unwrap();
        assert_eq!(refs, 1, "build_connections must not wipe doc→file `references` edges");

        ctx.pg().remove_watch_root(&root_id).await.ok();
    }
}
