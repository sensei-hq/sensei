//! DB-touching tests for `sensei.resolution_quality` (#235).
//!
//! The instrument exists so a change to the indexer is ATTRIBUTABLE: a
//! corpus-wide resolution percentage mixes "the target was never indexed" with
//! "it was indexed and the resolver missed", and those need different fixes.
//! Two properties make it work, and both are easy to lose in a rewrite.
use super::*;

/// Seed a folder in its own workspace, with a `files` row ready for nodes.
async fn fixture(pg: &PgStore, tag: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
    let root = format!("/_test/resolution/{tag}");
    let root_id = pg.add_watch_root(&root, "res-wt", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "res", &root).await.unwrap();
    let workspace_id =
        pg.create_workspace(&format!("_test:resolution:{tag}"), None, None).await.unwrap();
    crate::tasks::test_support::place_folder_in_workspace(
        pg,
        &folder_id,
        &workspace_id,
        &format!("res-{tag}"),
    )
    .await
    .unwrap();
    pg.upsert_file_row(
        &crate::db::pg_store::FileFingerprint {
            folder_id: &folder_id,
            file_path: "a.rs",
            mtime: 1,
            content_hash: "seed",
        },
        None,
    )
    .await
    .unwrap();
    pg.upsert_file_row(
        &crate::db::pg_store::FileFingerprint {
            folder_id: &folder_id,
            file_path: "b.ts",
            mtime: 1,
            content_hash: "seed",
        },
        None,
    )
    .await
    .unwrap();
    (workspace_id, folder_id)
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
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let (workspace_id, folder_id) = fixture(&pg, &tag).await;

    let def = |p: &'static str| FqnDef {
        file_path: p,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: true,
        parent_id: None,
    };
    let a = pg
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &folder_id,
            fqn: "rust·p·m·a",
            kind: "function",
            name: "a",
            language: Some("rust"),
            def: Some(def("a.rs")),
        })
        .await
        .unwrap();
    let b = pg
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &folder_id,
            fqn: "rust·p·m·b",
            kind: "function",
            name: "b",
            language: Some("rust"),
            def: Some(def("a.rs")),
        })
        .await
        .unwrap();

    // Three edges, one per verdict state — including the one that is NEITHER.
    let placed = pg
        .insert_edge(&folder_id, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls")
        .await
        .unwrap();
    let missed = pg
        .insert_edge(
            &folder_id,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("gone_one"), file: None },
            "calls",
        )
        .await
        .unwrap();
    let silent = pg
        .insert_edge(
            &folder_id,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("gone_two"), file: None },
            "imports",
        )
        .await
        .unwrap();
    set_verdict(&pg, &placed, Some("declared_here"), None).await;
    set_verdict(&pg, &missed, None, Some("no_import_in_scope")).await;
    set_verdict(&pg, &silent, None, None).await;

    let rows: Vec<(String, String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT language, edge_kind, outcome, edges FROM sensei.resolution_quality($1)",
    )
    .bind(workspace_id)
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

    cleanup(&pg, &workspace_id, &folder_id).await;
}

/// **THE PACKAGE AXIS — the crate or the app an edge was written in.** A
/// language-wide number cannot say which crate or app holds the misses, and
/// the placement report is by language, crate, package and app. The package
/// is the source node's fqn segment 2, the decomposition `structure_graph`
/// uses, so the report and the diagrams agree by construction.
///
/// Mutation that must break this test: drop `package` from the GROUP BY (the
/// two packages merge into one row).
#[tokio::test]
async fn placement_is_decomposed_by_the_package_the_edge_was_written_in() {
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let (workspace_id, folder_id) = fixture(&pg, &tag).await;
    let def = |p: &'static str| FqnDef {
        file_path: p,
        signature: None,
        line_start: Some(1),
        line_end: Some(2),
        is_exported: true,
        parent_id: None,
    };
    let mut nodes = Vec::new();
    for fqn in ["rust·alpha·m·a", "rust·alpha·m·b", "rust·beta·m·c"] {
        let name = fqn.rsplit('·').next().unwrap();
        nodes.push(
            pg.upsert_node_by_fqn(crate::db::pg_store::FqnNode {
                folder_id: &folder_id,
                fqn,
                kind: "function",
                name,
                language: Some("rust"),
                def: Some(def("a.rs")),
            })
            .await
            .unwrap(),
        );
    }
    let (a, b, c) = (nodes[0], nodes[1], nodes[2]);
    let placed_a = pg
        .insert_edge(&folder_id, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls")
        .await
        .unwrap();
    let missed_a = pg
        .insert_edge(
            &folder_id,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("x"), file: None },
            "calls",
        )
        .await
        .unwrap();
    let placed_c = pg
        .insert_edge(&folder_id, &c, crate::db::pg_store::EdgeTarget::Node(&a), "calls")
        .await
        .unwrap();
    set_verdict(&pg, &placed_a, Some("resolved_by_compiler"), None).await;
    set_verdict(&pg, &missed_a, None, Some("compiler_no_symbol")).await;
    set_verdict(&pg, &placed_c, Some("resolved_by_compiler"), None).await;

    let rows: Vec<(String, String, i64, f64)> = sqlx_core::query_as::query_as(
        "SELECT package, outcome, edges, pct_of_package_kind::float8 FROM sensei.resolution_quality($1)
          WHERE edge_kind = 'calls' ORDER BY package, outcome",
    )
    .bind(workspace_id)
    .fetch_all(pg.pool())
    .await
    .unwrap();
    let got: Vec<(String, String, i64, String)> =
        rows.into_iter().map(|(p, o, n, pct)| (p, o, n, format!("{pct:.0}"))).collect();
    assert_eq!(
        got,
        vec![
            ("alpha".to_string(), "missed".to_string(), 1, "50".to_string()),
            ("alpha".to_string(), "placed".to_string(), 1, "50".to_string()),
            ("beta".to_string(), "placed".to_string(), 1, "100".to_string()),
        ]
    );
    cleanup(&pg, &workspace_id, &folder_id).await;
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
    let pg = PgStore::test_store().await;
    let tag = uuid::Uuid::new_v4();
    let (workspace_id, folder_id) = fixture(&pg, &tag).await;
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
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &folder_id,
            fqn: "rust·p·m·caller",
            kind: "function",
            name: "caller",
            language: Some("rust"),
            def: Some(def("a.rs")),
        })
        .await
        .unwrap();
    let ts = pg
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &folder_id,
            fqn: "typescript·p·m·callee",
            kind: "function",
            name: "callee",
            language: Some("typescript"),
            def: Some(def("b.ts")),
        })
        .await
        .unwrap();
    // And a source with NO language at all.
    let anon = pg
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &folder_id,
            fqn: "·p·m·anon",
            kind: "function",
            name: "anon",
            language: None,
            def: Some(def("a.rs")),
        })
        .await
        .unwrap();

    let cross = pg
        .insert_edge(&folder_id, &rs, crate::db::pg_store::EdgeTarget::Node(&ts), "calls")
        .await
        .unwrap();
    let from_anon = pg
        .insert_edge(&folder_id, &anon, crate::db::pg_store::EdgeTarget::Node(&ts), "calls")
        .await
        .unwrap();
    set_verdict(&pg, &cross, Some("declared_here"), None).await;
    set_verdict(&pg, &from_anon, Some("declared_here"), None).await;

    let rows: Vec<(String, String, i64)> = sqlx_core::query_as::query_as(
        "SELECT language, outcome, edges FROM sensei.resolution_quality($1) WHERE edge_kind = 'calls'",
    )
    .bind(workspace_id)
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

    cleanup(&pg, &workspace_id, &folder_id).await;
}

async fn cleanup(pg: &PgStore, workspace_id: &uuid::Uuid, folder_id: &uuid::Uuid) {
    crate::tasks::test_support::cleanup_metrics_fixture(pg, workspace_id, Some(folder_id), &[])
        .await;
}
