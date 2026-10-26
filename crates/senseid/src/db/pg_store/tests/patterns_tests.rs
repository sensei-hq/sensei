use super::*;

// ── Detected Patterns tests ────────────────────────────────────────

#[tokio::test]
async fn pattern_upsert_and_list() {
    let s = pg_store().await;
    let suffix = format!("pat_upsert_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let instances =
        serde_json::json!([{"file":"src/lib.rs","line":10},{"file":"src/main.rs","line":20}]);
    let pat_id = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:Adapter",
            is_anti: false,
            confidence: Some(0.85),
            instances: &instances,
        })
        .await
        .unwrap();
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    assert!(patterns.iter().any(|p| p["name"] == "_test:Adapter" && p["instance_count"] == 2));
    // cleanup
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_promote() {
    let s = pg_store().await;
    let suffix = format!("pat_promote_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let pat_id = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:Factory",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([]),
        })
        .await
        .unwrap();
    s.promote_pattern(&pat_id, "rule").await.unwrap();
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["id"] == pat_id.to_string()).unwrap();
    assert_eq!(p["lifecycle"], "rule");
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(pat_id)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_upsert_updates_existing() {
    let s = pg_store().await;
    let suffix = format!("pat_dup_{}", uuid::Uuid::new_v4());
    let (proj_id, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    let id1 = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:Singleton",
            is_anti: false,
            confidence: Some(0.5),
            instances: &serde_json::json!([{"file":"a.rs"}]),
        })
        .await
        .unwrap();
    let id2 = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid),
            name: "_test:Singleton",
            is_anti: false,
            confidence: Some(0.9),
            instances: &serde_json::json!([{"file":"a.rs"},{"file":"b.rs"}]),
        })
        .await
        .unwrap();
    assert_eq!(id1, id2); // same row updated
    let patterns = s.list_patterns_by_folder(&fid).await.unwrap();
    let p = patterns.iter().find(|p| p["name"] == "_test:Singleton").unwrap();
    assert_eq!(p["instance_count"], 2);
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(id1)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

#[tokio::test]
async fn pattern_upsert_merges_across_folders_in_same_workspace() {
    // #82: patterns are workspace-scoped. Two folders in the same workspace
    // upserting the same pattern name collapse into a single row — the
    // second upsert updates the first row's instances/folder_id locus.
    let s = pg_store().await;
    let suffix = format!("pat_workspace_scope_{}", uuid::Uuid::new_v4());
    let (proj_id, fid_a) = create_test_workspace_and_folder(&s, &suffix).await;
    let fid_b = create_test_folder(&s, &format!("{}_b", suffix)).await;
    s.set_folder_workspace(&fid_b, &proj_id, "root", None).await.unwrap();

    let id_a = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid_a),
            name: "_test:Shared",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([{"file":"a.rs"}]),
        })
        .await
        .unwrap();
    let id_b = s
        .upsert_pattern(&crate::db::pg_store::PatternRow {
            workspace_id: &proj_id,
            folder_id: Some(&fid_b),
            name: "_test:Shared",
            is_anti: false,
            confidence: None,
            instances: &serde_json::json!([{"file":"b.rs"},{"file":"b2.rs"}]),
        })
        .await
        .unwrap();
    assert_eq!(id_a, id_b, "same (workspace_id, name) must merge into one row");

    // The row's instances reflect the latest upsert; folder_id follows too.
    let (count, locus): (i32, uuid::Uuid) = sqlx_core::query_as::query_as(
        "SELECT instance_count, folder_id FROM inference.detected_patterns WHERE id = $1",
    )
    .bind(id_a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count, 2);
    assert_eq!(locus, fid_b, "folder_id is the latest upsert's locus");

    // cleanup
    sqlx_core::query::query("DELETE FROM inference.detected_patterns WHERE id = $1")
        .bind(id_a)
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(proj_id)
        .execute(s.pool())
        .await
        .ok();
}

// ── Corrections aggregation tests ──────────────────────────────────

#[tokio::test]
async fn correction_upsert_is_idempotent_by_signature() {
    // Prunes by keep-set, which deletes every OTHER test's corrections.
    let s = pg_store().await;
    let p = uuid::Uuid::new_v4();
    let sig = format!("corr-test-{}", uuid::Uuid::new_v4());
    let row = crate::corrections::CorrectionRow {
        signature: sig.clone(),
        text: "Use $state for reactive locals".into(),
        suggestion: Some("Reinforce the svelte5 memory".into()),
        count: 3,
        workspace_ids: vec![p],
        last_seen: chrono::Utc::now(),
        memory_id: None,
        instances: serde_json::json!([{"session_id": "s1", "ts": 1, "prompt": "use $state"}]),
    };
    let id1 = s.upsert_correction(&row).await.unwrap();
    let mut row2 = row.clone();
    row2.count = 4;
    let id2 = s.upsert_correction(&row2).await.unwrap();
    assert_eq!(id1, id2, "same signature updates the same row");

    let global = s.list_corrections().await.unwrap();
    let found = global["corrections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id1.to_string())
        .unwrap()
        .clone();
    assert_eq!(found["count"], 4);
    assert_eq!(found["text"], "Use $state for reactive locals");

    // the workspace-scoped read exercises the `$1 = ANY(workspace_ids)` filter.
    let scoped = s.list_corrections_for_workspace(&p).await.unwrap();
    assert!(
        scoped["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "per-workspace read returns a correction tagged with that workspace"
    );

    // prune keeping our signature → the row survives.
    s.delete_corrections_not_in(std::slice::from_ref(&sig)).await.unwrap();
    let kept = s.list_corrections().await.unwrap();
    assert!(
        kept["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "a kept signature survives the prune"
    );

    // prune excluding our signature → the row is deleted (also leaves the
    // test DB clean — this clears the derived corrections table).
    s.delete_corrections_not_in(&["corr-nope".to_string()]).await.unwrap();
    let after = s.list_corrections().await.unwrap();
    assert!(
        !after["corrections"].as_array().unwrap().iter().any(|c| c["id"] == id1.to_string()),
        "a signature not in the keep set is pruned"
    );
}
