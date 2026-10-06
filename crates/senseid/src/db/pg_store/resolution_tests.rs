//! DB-touching tests for `sensei.resolution_quality` (#235).
//!
//! The instrument exists so a change to the indexer is ATTRIBUTABLE: a
//! corpus-wide resolution percentage mixes "the target was never indexed" with
//! "it was indexed and the resolver missed", and those need different fixes.
//! Two properties make it work, and both are easy to lose in a rewrite.
use super::*;

/// Seed a folder in its own project, with a `files` row ready for nodes.
async fn fixture(pg: &PgStore, tag: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
    let root = format!("/_test/resolution/{tag}");
    let root_id = pg.add_watch_root(&root, "res-wt", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "res", &root).await.unwrap();
    let project_id =
        pg.create_project(&format!("_test:resolution:{tag}"), None, None).await.unwrap();
    crate::tasks::test_support::place_folder_in_project(
        pg,
        &folder_id,
        &project_id,
        &format!("res-{tag}"),
    )
    .await
    .unwrap();
    pg.upsert_file_row(&folder_id, "a.rs", 1, "seed", None).await.unwrap();
    pg.upsert_file_row(&folder_id, "b.ts", 1, "seed", None).await.unwrap();
    (project_id, folder_id)
}

/// Stamp an edge's verdict the way the resolver would.
async fn set_verdict(pg: &PgStore, edge: &uuid::Uuid, via: Option<&str>, reason: Option<&str>) {
    sqlx_core::query::query(
        "UPDATE sensei.edges SET resolved_via = $2, unresolved_reason = $3 WHERE id = $1",
    )
    .bind(edge)
    .bind(via)
    .bind(reason)
    .execute(pg.pool())
    .await
    .unwrap();
}

/// AN EDGE NOBODY ASKED ABOUT IS ITS OWN OUTCOME, not a miss.
///
/// This is the property the whole instrument turns on. An edge carrying neither
/// `resolved_via` nor `unresolved_reason` was never attempted; folding it into
/// "missed" reports a resolver that tried and failed when nothing tried.
/// Measured on the live corpus, 251,270 of 258,623 import edges are in exactly
/// that state (#242) — so a two-way split would have described 97% of imports as
/// a resolver failure.
///
/// Mutation that must break this test: collapse the `else 'no verdict'` arm into
/// `'missed'`.
#[tokio::test]
async fn an_edge_nobody_asked_about_is_not_reported_as_a_miss() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let tag = uuid::Uuid::new_v4();
    let (project_id, folder_id) = fixture(&pg, &tag).await;

    let def = |p: &'static str| FqnDef {
        file_path: p,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: true,
        parent_id: None,
    };
    let a = pg
        .upsert_node_by_fqn(
            &folder_id,
            "rust·p·m·a",
            "function",
            "a",
            Some("rust"),
            Some(def("a.rs")),
        )
        .await
        .unwrap();
    let b = pg
        .upsert_node_by_fqn(
            &folder_id,
            "rust·p·m·b",
            "function",
            "b",
            Some("rust"),
            Some(def("a.rs")),
        )
        .await
        .unwrap();

    // Three edges, one per verdict state — including the one that is NEITHER.
    let placed = pg.insert_edge(&folder_id, &a, Some(&b), None, None, "calls").await.unwrap();
    let missed =
        pg.insert_edge(&folder_id, &a, None, Some("gone_one"), None, "calls").await.unwrap();
    let silent =
        pg.insert_edge(&folder_id, &a, None, Some("gone_two"), None, "imports").await.unwrap();
    set_verdict(&pg, &placed, Some("declared_here"), None).await;
    set_verdict(&pg, &missed, None, Some("no_import_in_scope")).await;
    set_verdict(&pg, &silent, None, None).await;

    let rows: Vec<(String, String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT language, edge_kind, outcome, edges FROM sensei.resolution_quality($1)",
    )
    .bind(project_id)
    .fetch_all(pg.pool())
    .await
    .unwrap();

    let find = |kind: &str, outcome: &str| {
        rows.iter().find(|(_, k, o, _)| k == kind && o == outcome).map(|(_, _, _, n)| *n)
    };
    assert_eq!(find("calls", "placed"), Some(1), "rows: {rows:?}");
    assert_eq!(find("calls", "missed"), Some(1), "rows: {rows:?}");
    assert_eq!(
        find("imports", "no verdict"),
        Some(1),
        "the unattempted edge is its OWN outcome, not a miss: {rows:?}"
    );
    assert_eq!(find("imports", "missed"), None, "and it is NOT counted as one");

    cleanup(&pg, &project_id, &folder_id).await;
}

/// Language is the SOURCE node's, and an edge with no language is reported
/// rather than dropped.
///
/// The question the instrument answers is "how well does the adapter that wrote
/// this edge resolve", so attribution has to follow the file the edge was
/// written in — not the target, which may be library surface with no language at
/// all. A population that cannot be attributed is a finding, so it surfaces as
/// `(unknown)` instead of vanishing.
///
/// Mutation that must break this test: join the language from the TARGET node,
/// or drop rows whose source language is NULL.
#[tokio::test]
async fn language_follows_the_source_and_an_unattributable_edge_still_appears() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let tag = uuid::Uuid::new_v4();
    let (project_id, folder_id) = fixture(&pg, &tag).await;
    let def = |p: &'static str| FqnDef {
        file_path: p,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: true,
        parent_id: None,
    };

    // A rust source calling a TYPESCRIPT target. Attribution must say `rust`.
    let rs = pg
        .upsert_node_by_fqn(
            &folder_id,
            "rust·p·m·caller",
            "function",
            "caller",
            Some("rust"),
            Some(def("a.rs")),
        )
        .await
        .unwrap();
    let ts = pg
        .upsert_node_by_fqn(
            &folder_id,
            "typescript·p·m·callee",
            "function",
            "callee",
            Some("typescript"),
            Some(def("b.ts")),
        )
        .await
        .unwrap();
    // And a source with NO language at all.
    let anon = pg
        .upsert_node_by_fqn(&folder_id, "·p·m·anon", "function", "anon", None, Some(def("a.rs")))
        .await
        .unwrap();

    let cross = pg.insert_edge(&folder_id, &rs, Some(&ts), None, None, "calls").await.unwrap();
    let from_anon =
        pg.insert_edge(&folder_id, &anon, Some(&ts), None, None, "calls").await.unwrap();
    set_verdict(&pg, &cross, Some("declared_here"), None).await;
    set_verdict(&pg, &from_anon, Some("declared_here"), None).await;

    let rows: Vec<(String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT language, outcome, edges FROM sensei.resolution_quality($1) WHERE edge_kind = 'calls'",
    )
    .bind(project_id)
    .fetch_all(pg.pool())
    .await
    .unwrap();

    let by_lang = |l: &str| rows.iter().find(|(lang, _, _)| lang == l).map(|(_, _, n)| *n);
    assert_eq!(by_lang("rust"), Some(1), "attributed to the SOURCE, not the target: {rows:?}");
    assert_eq!(by_lang("typescript"), None, "the target's language is not the attribution");
    assert_eq!(
        by_lang("(unknown)"),
        Some(1),
        "an edge that cannot be attributed is reported, not dropped: {rows:?}"
    );

    cleanup(&pg, &project_id, &folder_id).await;
}

async fn cleanup(pg: &PgStore, project_id: &uuid::Uuid, folder_id: &uuid::Uuid) {
    crate::tasks::test_support::cleanup_metrics_fixture(pg, project_id, Some(folder_id), &[]).await;
}
