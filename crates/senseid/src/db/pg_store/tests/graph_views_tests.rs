use super::*;

// ── sensei.graph_nodes — locality + hierarchy ────────────────────────────────
//
// The view a dependency count, a stub count, and the patterns-vs-graph split all
// read from. It exists because the ONLY previous way to ask "is this external?"
// was to ask "did the edge fail to resolve?" — a proxy that was wrong in both
// directions (`resolve.rs:102` reported 791 of sensei's 1,040 "dependencies" as
// this repo's own code, and would have lost every genuine one the moment
// resolution started working, since target_id and target_name are mutually
// exclusive).
//
// Locality is a property of the NODE, so the view reads what the WRITER already
// decided (`kind`, `file_path`) rather than re-deriving it. That is the line
// between this and `classify_import`: that function parses a specifier string and
// exercises judgment, so it stays in Rust with one owner; this workspaces columns
// the writer already set.

/// Three-valued on purpose. A boolean would bin the 85,530 nodes that have
/// neither a file nor an external fqn as external and reproduce exactly
/// the false positives this view exists to kill. `unknown` makes them countable,
/// which is what turns "stub count → 0" into a query.
#[tokio::test]
async fn graph_nodes_locality_is_three_valued_not_boolean() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("loc_{}", uuid::Uuid::new_v4())).await;

    // A definition in a local file.
    let internal = s
        .seed_node(&fid, "function", "compute", "src/lib.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    // A dependency's symbol: the writer records this with the `lib·` prefix.
    let external = s
        .seed_node_by_fqn(
            &fid,
            "lib·axum·axum::response·Json",
            "unknown",
            "Json",
            Some("rust"),
            None,
        )
        .await
        .unwrap();
    // A reference the parser could not resolve: no file, not a lib_symbol.
    let stub = s
        .seed_node_by_fqn(
            &fid,
            "rust·senseid·codebase·HashMap·get",
            "function",
            "get",
            Some("rust"),
            None,
        )
        .await
        .unwrap();

    let rows: Vec<(uuid::Uuid, String)> = sqlx_core::query_as::query_as(
        "SELECT id, locality FROM sensei.graph_nodes WHERE folder_id = $1 ORDER BY locality",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    let by_id: std::collections::HashMap<uuid::Uuid, String> = rows.into_iter().collect();

    assert_eq!(by_id.get(&internal).map(String::as_str), Some("internal"), "has a local file");
    assert_eq!(by_id.get(&external).map(String::as_str), Some("external"), "lib_symbol kind");
    assert_eq!(
        by_id.get(&stub).map(String::as_str),
        Some("unknown"),
        "no file and not a lib_symbol — neither internal nor external, and saying \
         'external' here is the bug this view replaces"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Locality must NOT read `nodes.resolved`. Measured live: 140,051 `section`
/// rows are `resolved=false` yet sit in real files, so `resolved` answers "did
/// FQN enrichment run", not "where does this live". Both fixtures below are
/// `resolved=false` and must still classify differently.
#[tokio::test]
async fn graph_nodes_locality_does_not_depend_on_resolved() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("locres_{}", uuid::Uuid::new_v4())).await;
    let unresolved_but_local = s
        .seed_node(&fid, "section", "Overview", "docs/design.md", None, None, Some(1), Some(3))
        .await
        .unwrap();
    let unresolved_lib = s
        .seed_node_by_fqn(
            &fid,
            "lib·serde·serde·Serialize",
            "unknown",
            "Serialize",
            Some("rust"),
            None,
        )
        .await
        .unwrap();

    let rows: Vec<(uuid::Uuid, String, bool)> = sqlx_core::query_as::query_as(
        "SELECT id, locality, resolved FROM sensei.graph_nodes WHERE folder_id = $1",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    let map: std::collections::HashMap<uuid::Uuid, (String, bool)> =
        rows.into_iter().map(|(i, l, r)| (i, (l, r))).collect();

    let (loc_local, res_local) = map.get(&unresolved_but_local).cloned().unwrap();
    let (loc_lib, res_lib) = map.get(&unresolved_lib).cloned().unwrap();
    assert!(!res_local && !res_lib, "both fixtures are resolved=false — that is the point");
    assert_eq!(loc_local, "internal", "a doc section in a real file is internal");
    assert_eq!(loc_lib, "external", "a lib_symbol is external regardless of enrichment");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The hierarchy half: `parent_id` already carries containment (296,744 rows
/// populated), so grouping does not need a `contains` edge kind. The view
/// surfaces the parent's name and kind so a caller can group without a
/// self-join.
#[tokio::test]
async fn graph_nodes_exposes_the_parent_for_grouping() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("locpar_{}", uuid::Uuid::new_v4())).await;
    let class = s
        .seed_node(&fid, "class", "Widget", "src/widget.rs", None, None, Some(1), Some(40))
        .await
        .unwrap();
    let method = s
        .seed_node(&fid, "method", "render", "src/widget.rs", Some(&class), None, Some(5), Some(9))
        .await
        .unwrap();

    let row: (Option<String>, Option<String>, Option<uuid::Uuid>) = sqlx_core::query_as::query_as(
        "SELECT parent_name, parent_kind, parent_id FROM sensei.graph_nodes WHERE id = $1",
    )
    .bind(method)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0.as_deref(), Some("Widget"));
    assert_eq!(row.1.as_deref(), Some("class"));
    assert_eq!(row.2, Some(class));

    let top: (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT parent_name, parent_kind FROM sensei.graph_nodes WHERE id = $1",
    )
    .bind(class)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(top, (None, None), "a top-level node has no parent — null, not a placeholder");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The dependency count the `libs` computation got wrong. Counting distinct
/// external targets of `calls` edges is now a one-line query that cannot mistake
/// an unresolved internal reference for a dependency.
#[tokio::test]
async fn graph_nodes_makes_dependency_counting_a_query() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("locdep_{}", uuid::Uuid::new_v4())).await;
    let caller = s
        .seed_node(&fid, "function", "handler", "src/api.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let lib_a = s
        .seed_node_by_fqn(
            &fid,
            "lib·axum·axum::response·Json",
            "unknown",
            "Json",
            Some("rust"),
            None,
        )
        .await
        .unwrap();
    let lib_b = s
        .seed_node_by_fqn(
            &fid,
            "lib·serde·serde·to_string",
            "unknown",
            "to_string",
            Some("rust"),
            None,
        )
        .await
        .unwrap();
    // A stub — an internal reference the parser failed to resolve. The OLD rule
    // counted this as a dependency; the new one must not.
    let stub = s
        .seed_node_by_fqn(
            &fid,
            "rust·senseid·api·HashMap·get",
            "function",
            "get",
            Some("rust"),
            None,
        )
        .await
        .unwrap();
    for t in [lib_a, lib_b, stub] {
        s.insert_edge(&fid, &caller, crate::db::pg_store::EdgeTarget::Node(&t), "calls")
            .await
            .unwrap();
    }

    let (deps, stubs): (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT count(*) FILTER (WHERE n.locality = 'external')
              , count(*) FILTER (WHERE n.locality = 'unknown')
           FROM sensei.edges e
           JOIN sensei.graph_nodes n ON n.id = e.target_id
          WHERE e.folder_id = $1 AND e.kind = 'calls'",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(deps, 2, "two real dependencies");
    assert_eq!(stubs, 1, "the stub is counted as a stub, NOT as a dependency");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

// ── sensei.doc_coverage — the pairing is computed, not stored ─────────────────

/// `covers` edges are gone: the pairing is a pure function of the current
/// (docs, files), so `build_connections` no longer writes 601 rows into
/// `sensei.edges` for a view to read back off the very nodes they came from.
///
/// This test writes NO edge at all and still expects the pair.
#[tokio::test]
async fn doc_coverage_pairs_without_any_stored_edge() {
    let s = pg_store().await;
    let suffix = format!("doccov_{}", uuid::Uuid::new_v4().simple());
    let fid = create_test_folder(&s, &suffix).await;
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "doc",
        "design",
        "docs/design.md",
        None,
        None,
        Some(1),
        Some(1),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "design.rs",
        "src/design.rs",
        None,
        None,
        Some(1),
        Some(1),
    )
    .await
    .unwrap();
    // A file whose stem does NOT match must not pair.
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "file",
        "other.rs",
        "src/other.rs",
        None,
        None,
        Some(1),
        Some(1),
    )
    .await
    .unwrap();

    let drift = s.get_doc_drift(&suffix).await.unwrap();
    assert_eq!(drift.len(), 1, "exactly the stem-matched pair, got {drift:?}");
    assert_eq!(drift[0]["docFile"], "docs/design.md");
    assert_eq!(drift[0]["codeFile"], "src/design.rs");

    let covers = s.get_edges_by_kind(&fid, "covers").await.unwrap();
    assert!(covers.is_empty(), "no covers edge was written, and none is needed");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The stem expression in the view reproduces Rust `Path::file_stem()`. Verified
/// against all 45,186 distinct `file_path` values in the live DB when the view
/// was written; these are the cases that expression could plausibly get wrong.
#[tokio::test]
async fn doc_coverage_stem_matches_rust_file_stem() {
    let s = pg_store().await;
    let cases = [
        "docs/api/auth.md",
        "src/appstate.svelte.ts",
        ".gitignore",
        "Makefile",
        "a/b/.env.local",
        "x.tar.gz",
        "no_ext",
        "dir.with.dots/file.rs",
    ];
    for path in cases {
        let want = std::path::Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let got: (String,) = sqlx_core::query_as::query_as(
            "select case
                      when position('.' in substring(regexp_replace($1, '^.*/', '') from 2)) = 0
                      then regexp_replace($1, '^.*/', '')
                      else substring(regexp_replace($1, '^.*/', '') from 1
                             for length(regexp_replace($1, '^.*/', ''))
                               - position('.' in reverse(regexp_replace($1, '^.*/', ''))))
                    end",
        )
        .bind(path)
        .fetch_one(s.pool())
        .await
        .unwrap();
        assert_eq!(got.0, want, "stem of {path:?}");
    }
}

/// The D2 concern the old `covers` replace guarded — "a covers edge whose covered
/// file no longer matches is stale and must be removed" — is now structurally
/// impossible rather than maintained. Nothing is stored, so nothing can go stale:
/// rename the file and the pairing simply is not there on the next read. There is
/// no replace pass and therefore no window in which a stale row is visible.
#[tokio::test]
async fn doc_coverage_cannot_hold_a_stale_pairing() {
    let s = pg_store().await;
    let suffix = format!("staleprs_{}", uuid::Uuid::new_v4().simple());
    let fid = create_test_folder(&s, &suffix).await;
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "doc",
        "auth",
        "docs/auth.md",
        None,
        None,
        Some(1),
        Some(1),
    )
    .await
    .unwrap();
    let code = s
        .seed_node(&fid, "file", "auth", "src/auth.rs", None, None, Some(1), Some(1))
        .await
        .unwrap();

    let drift = s.get_doc_drift(&suffix).await.unwrap();
    assert_eq!(drift.len(), 1, "the stem match pairs, got {drift:?}");
    assert_eq!(drift[0]["codeFile"], "src/auth.rs");

    // Rename the file out from under the doc. No task runs; no edge is rewritten.
    // Renaming is a `files` edit now, not a node edit (R13): the node keys on
    // the file's id, so the path it reports changes when the FILE's does.
    sqlx_core::query::query(
        "UPDATE sensei.files SET file_path = 'src/renamed.rs'
          WHERE id = (SELECT file_id FROM sensei.nodes WHERE id = $1)",
    )
    .bind(code)
    .execute(s.pool())
    .await
    .unwrap();

    let drift = s.get_doc_drift(&suffix).await.unwrap();
    assert!(drift.is_empty(), "the pairing is gone the instant the stem stops matching: {drift:?}");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A re-detect must not destroy the model-authored `description`.
///
/// `replace_communities_for_folder` DELETEs the folder's rows and re-INSERTs with
/// `description = NULL`, so every DetectCommunities pass discards prose that cost
/// a model call to produce — and `enrich_community_descriptions` is capped at 25
/// communities per folder, so what it wipes it cannot fully replace.
#[tokio::test]
async fn re_detect_preserves_the_model_authored_description() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("desckeep_{}", uuid::Uuid::new_v4())).await;
    let n1 = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let n2 = s
        .seed_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    s.insert_edge(&fid, &n1, crate::db::pg_store::EdgeTarget::Node(&n2), "calls").await.unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    // What enrich_community_descriptions writes, off-barrier, after a model call.
    sqlx_core::query::query(
        "UPDATE inference.communities
            SET description = 'the auth module', props = '{\"source\":\"narration-cache\"}'::jsonb
          WHERE folder_id = $1",
    )
    .bind(fid)
    .execute(s.pool())
    .await
    .unwrap();

    // An unchanged graph re-detected — the common case on any re-scan.
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let rows: Vec<(Option<String>, serde_json::Value)> = sqlx_core::query_as::query_as(
        "SELECT description, props FROM inference.communities WHERE folder_id = $1",
    )
    .bind(fid)
    .fetch_all(s.pool())
    .await
    .unwrap();
    assert!(!rows.is_empty(), "the folder still has communities");
    assert!(
        rows.iter().any(|(d, _)| d.as_deref() == Some("the auth module")),
        "the model-authored description survives a re-detect of an unchanged graph; got {rows:?}"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// The other half of preserving a description: it must be DISCARDED when the
/// community is no longer the same cluster.
///
/// `community_id` is positional (rank+1), so on a changed graph id 3 can be an
/// entirely different set of symbols. Keeping the old prose would caption the
/// wrong thing — a fabricated-looking summary nobody could distinguish from a
/// real one. The hub set is the evidence; when it differs the text goes.
#[tokio::test]
async fn re_detect_discards_a_description_whose_cluster_changed() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("descdrop_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .seed_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap();
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    sqlx_core::query::query(
        "UPDATE inference.communities
            SET description = 'describes the OLD cluster',
                props = '{\"source\":\"narration-cache\"}'::jsonb
          WHERE folder_id = $1",
    )
    .bind(fid)
    .execute(s.pool())
    .await
    .unwrap();

    // Grow the graph so the hubs change: c becomes the hub of that community.
    let c = s
        .seed_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    let d = s
        .seed_node(&fid, "function", "d", "a.rs", None, Some("()"), Some(7), Some(8))
        .await
        .unwrap();
    for src in [a, b, d] {
        s.insert_edge(&fid, &src, crate::db::pg_store::EdgeTarget::Node(&c), "calls")
            .await
            .unwrap();
    }
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let stale: (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM inference.communities
          WHERE folder_id = $1 AND description = 'describes the OLD cluster'",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        stale.0, 0,
        "a description must not survive onto a community whose hubs changed — \
         captioning the wrong cluster is worse than having no caption"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

// ── Structure diagram (#205) ────────────────────────────────────────────────

/// THE LEVEL ROLLUP IS A DERIVATION, and `module` means the module's TOP
/// segment — not the whole module path.
///
/// Measured on sensei's own corpus before this was written: the fqn's module
/// segment is per-FILE for most of the tree, so grouping on it whole collapses
/// 1,751 files into 1,446 groups — 1.21x, with 1,390 of those groups holding
/// exactly one file. A "module" level that returns a node per file is not a
/// level at all, and #205's done gate asks for an order of magnitude.
///
/// Grouping on the module's FIRST segment (`tasks` of `tasks::metrics`,
/// `routes` of `routes/(observatory)/insights`) gives 150 groups over the same
/// 1,751 files — 11.7x — and the groups are the ones a reader would name:
/// `senseid/tasks`, `senseid/api`, `senseid/indexer`.
///
/// This test pins the property on a fixture rather than on the corpus: two
/// files in different sub-modules of ONE top module must collapse to ONE node,
/// while a file in another top module stays separate.
///
/// Mutation that must break it: group by `module` instead of its first segment.
#[tokio::test]
async fn structure_level_module_groups_by_the_modules_top_segment() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, fid) = create_test_workspace_and_folder(&s, &format!("struct_{uniq}")).await;

    // Three files: two under `tasks`, one under `api`. Same package.
    let specs = [
        ("rust·pkg·tasks::alpha·run·item", "src/tasks/alpha.rs"),
        ("rust·pkg·tasks::beta·run·item", "src/tasks/beta.rs"),
        ("rust·pkg·api::handler·run·item", "src/api/handler.rs"),
    ];
    for (fqn, path) in specs {
        s.seed_node_by_fqn(
            &fid,
            fqn,
            "function",
            "run",
            Some("rust"),
            Some(crate::db::pg_store::FqnDef {
                file_path: path,
                signature: None,
                line_start: Some(1),
                line_end: Some(2),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();
    }

    let files = s.structure_nodes(&pid, "file").await.unwrap();
    assert_eq!(files.len(), 3, "file level is one node per file");

    let modules = s.structure_nodes(&pid, "module").await.unwrap();
    let mut labels: Vec<String> = modules.iter().map(|m| m.id.clone()).collect();
    labels.sort();
    assert_eq!(
        labels,
        vec!["pkg/api".to_string(), "pkg/tasks".to_string()],
        "two sub-modules of one top module collapse to ONE node; grouping on the \
         whole module path would leave three"
    );

    let packages = s.structure_nodes(&pid, "package").await.unwrap();
    assert_eq!(packages.len(), 1, "one package");
    assert_eq!(packages[0].files, 3, "and it carries every file beneath it");

    // THE CONTAINMENT PATH IS THE FQN'S, NOT THE FILESYSTEM'S. The diagram draws
    // its rim from `path`, and deriving that from directories disagrees with the
    // call graph for every workspace member — a file's directory says nothing
    // about the package it declares. Derived here so three consumers cannot each
    // re-derive it differently.
    let api = files.iter().find(|n| n.id.ends_with("handler.rs")).expect("the api file");
    assert_eq!(
        api.path,
        vec!["pkg".to_string(), "api".to_string(), "src/api/handler.rs".to_string()],
        "a file's path is package -> top module -> itself"
    );
    let tasks = modules.iter().find(|m| m.id == "pkg/tasks").expect("the tasks module");
    assert_eq!(
        tasks.path,
        vec!["pkg".to_string(), "tasks".to_string()],
        "a module node stops at the module — it IS the leaf at this level"
    );
    assert_eq!(packages[0].path, vec!["pkg".to_string()], "and a package node at the package");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Every file of the workspace with the module the map files it under — what L0
/// counts by role (M17). Grouped by the same top segment as `structure_nodes`.
#[tokio::test]
async fn codemap_module_paths_names_each_files_module() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, fid) = create_test_workspace_and_folder(&s, &format!("paths_{uniq}")).await;
    for (fqn, path) in [
        ("rust·pkg·tasks::alpha·run·item", "src/tasks/alpha.rs"),
        ("rust·pkg·tasks::alpha_test·run·item", "src/tasks/tests/alpha_test.rs"),
        ("rust·pkg·api::handler·run·item", "src/api/handler.rs"),
    ] {
        s.seed_node_by_fqn(
            &fid,
            fqn,
            "function",
            "run",
            Some("rust"),
            Some(crate::db::pg_store::FqnDef {
                file_path: path,
                signature: None,
                line_start: Some(1),
                line_end: Some(2),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();
    }
    let mut paths = s.codemap_module_paths(&pid).await.unwrap();
    paths.sort();
    s.delete_nodes_by_folder(&fid).await.unwrap();
    assert_eq!(
        paths,
        [
            ("pkg/api", "src/api/handler.rs"),
            ("pkg/tasks", "src/tasks/alpha.rs"),
            ("pkg/tasks", "src/tasks/tests/alpha_test.rs"),
        ]
        .map(|(m, p)| (m.to_string(), p.to_string()))
    );
}

/// COVERAGE COUNTS WHAT COULD NOT BE PLACED, not every edge there is.
///
/// The first version of this asked `graph_placement` for
/// `outcome <> 'resolved'`. That view's outcomes are `placed` / `missed` /
/// `no verdict` — there is NO `resolved` — so the predicate matched every row
/// and the screen reported the workspace's TOTAL edge count as its unplaced one.
/// Measured on sensei before the fix: 69,747 "unplaced" against 69,747 total,
/// where the truth is 38,578. A coverage line exists to say what the diagram is
/// hiding; one that reports the whole population says nothing and looks precise
/// doing it.
///
/// It also cost 107 of the endpoint's 122 seconds, because filtering
/// `graph_placement` by workspace cannot push past its window functions.
///
/// Mutation that must break this test: drop the `target_id IS NULL` filter.
#[tokio::test]
async fn structure_coverage_counts_only_the_edges_that_could_not_be_placed() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, fid) = create_test_workspace_and_folder(&s, &format!("cov_{uniq}")).await;

    let a = s
        .seed_node_by_fqn(
            &fid,
            "rust·pkg·tasks::a·run·item",
            "function",
            "run",
            Some("rust"),
            Some(crate::db::pg_store::FqnDef {
                file_path: "src/tasks/a.rs",
                signature: None,
                line_start: Some(1),
                line_end: Some(2),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();
    let b = s
        .seed_node_by_fqn(
            &fid,
            "rust·pkg·api::b·run·item",
            "function",
            "run",
            Some("rust"),
            Some(crate::db::pg_store::FqnDef {
                file_path: "src/api/b.rs",
                signature: None,
                line_start: Some(1),
                line_end: Some(2),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();

    // One PLACED edge (a real target) and two that resolved to nothing.
    s.replace_edges_of_kind(
        &fid,
        "calls",
        &[
            crate::db::pg_store::EdgeSpec {
                source_id: a,
                target_id: Some(b),
                target_name: None,
                target_file: None,
            },
            crate::db::pg_store::EdgeSpec {
                source_id: a,
                target_id: None,
                target_name: Some("nowhere".into()),
                target_file: Some("src/gone.rs".into()),
            },
            crate::db::pg_store::EdgeSpec {
                source_id: b,
                target_id: None,
                target_name: Some("also_nowhere".into()),
                target_file: Some("src/gone.rs".into()),
            },
        ],
    )
    .await
    .unwrap();

    let unplaced = s.structure_unplaced(&pid, &["calls".to_string()]).await.unwrap();
    assert_eq!(unplaced, 2, "only the two edges with no target are unplaced — not all three");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A local Dōjō membership for the persona-binding tests. The credential_ref is
/// a placeholder name only — nothing here touches the Keychain.
async fn persona_test_membership(s: &PgStore) -> uuid::Uuid {
    let mid = uuid::Uuid::new_v4();
    s.create_dojo_membership(&NewDojoMembership {
        id: mid,
        registry_url: "http://localhost:7755".into(),
        tenant_key: format!("organization/persona-bind-{mid}"),
        dojo_url: format!("http://localhost:7755/organization/persona-bind-{mid}"),
        kind: "employer".into(),
        org_slugs: vec![],
        role: "contributor".into(),
        authenticated_via: "github_oauth".into(),
        attribution_default: "named".into(),
        sync_status: "healthy".into(),
    })
    .await
    .unwrap();
    mid
}

// #236 step 4: each local membership records the persona whose session its
// tenant calls present — unknown until matched, one persona per membership,
// gone with either side.
#[tokio::test]
async fn a_membership_records_its_persona() {
    let s = pg_store().await;
    let mid = persona_test_membership(&s).await;
    let label = format!("persona-bind-{}", uuid::Uuid::new_v4());
    let pid = s.upsert_persona(&label, true).await.unwrap();

    // Unmatched: no persona, and listed for the backfill.
    assert_eq!(s.membership_persona(&mid).await.unwrap(), None);
    assert!(s.unbound_dojo_memberships().await.unwrap().contains(&mid));

    // Matched, persona signed out: bound, but no session to present.
    s.set_membership_persona(&mid, &pid).await.unwrap();
    assert_eq!(
        s.membership_persona(&mid).await.unwrap(),
        Some(MembershipPersona { persona_id: pid, session_slot: None })
    );
    assert!(!s.unbound_dojo_memberships().await.unwrap().contains(&mid));

    // Signed in: the slot live_access_token mints from.
    let slot = format!("slot-{}", uuid::Uuid::new_v4());
    sqlx_core::query::query("UPDATE sensei.personas SET session_slot = $2 WHERE id = $1")
        .bind(pid)
        .bind(&slot)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(
        s.membership_persona(&mid).await.unwrap(),
        Some(MembershipPersona { persona_id: pid, session_slot: Some(slot) })
    );

    // Re-matched to another persona: one persona per membership, the latest.
    let other = s.upsert_persona(&format!("{label}-other"), true).await.unwrap();
    s.set_membership_persona(&mid, &other).await.unwrap();
    assert_eq!(s.membership_persona(&mid).await.unwrap().map(|p| p.persona_id), Some(other));

    // Gone with the persona…
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(other)
        .execute(s.pool())
        .await
        .unwrap();
    assert_eq!(s.membership_persona(&mid).await.unwrap(), None);

    // …and with the membership.
    s.set_membership_persona(&mid, &pid).await.unwrap();
    s.delete_dojo_membership(&mid).await.unwrap();
    let left: (i64,) =
        query_as("SELECT count(*) FROM sensei.dojo_membership_personas WHERE membership_id = $1")
            .bind(mid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(left.0, 0);
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

// #236 step 4d: the membership row carries its persona, so every DojoClient
// built from it presents the right credential without a second lookup.
#[tokio::test]
async fn a_loaded_membership_carries_its_persona() {
    let s = pg_store().await;
    let mid = persona_test_membership(&s).await;
    assert_eq!(s.get_dojo_membership(&mid).await.unwrap().unwrap().persona, None);

    let pid =
        s.upsert_persona(&format!("persona-row-{}", uuid::Uuid::new_v4()), true).await.unwrap();
    let slot = format!("slot-{}", uuid::Uuid::new_v4());
    sqlx_core::query::query("UPDATE sensei.personas SET session_slot = $2 WHERE id = $1")
        .bind(pid)
        .bind(&slot)
        .execute(s.pool())
        .await
        .unwrap();
    s.set_membership_persona(&mid, &pid).await.unwrap();

    let want = Some(MembershipPersona { persona_id: pid, session_slot: Some(slot) });
    assert_eq!(s.get_dojo_membership(&mid).await.unwrap().unwrap().persona, want);
    let listed = s.list_dojo_memberships().await.unwrap();
    assert_eq!(listed.iter().find(|m| m.id == mid).map(|m| m.persona.clone()), Some(want));

    s.delete_dojo_membership(&mid).await.unwrap();
    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .unwrap();
}

/// #240: a user-scoped row leaves the machine only when it is provably the
/// signed-in person's — a row with no identity (this machine's own session
/// metrics) when the persona is the only one that elected the repository, or a
/// row whose author email is one of the persona's CLAIMED (provider-verified)
/// addresses. An unverified `git` address, and a colleague's, never travel.
#[tokio::test]
async fn only_rows_provably_the_persons_are_pushable_user_metrics() {
    let s = pg_store().await;
    let uniq = uuid::Uuid::new_v4();
    let (pid, rid) = seed_sync_fixture_at(&s, &uniq, "private").await; // one no-identity row
    let key = format!("test/bare-{uniq}");
    let persona = s.upsert_persona(&format!("_test:provable:{uniq}"), true).await.unwrap();
    let (claimed, unverified, colleague) = (
        format!("me-{uniq}@x.test"),
        format!("me-git-{uniq}@x.test"),
        format!("them-{uniq}@x.test"),
    );
    s.link_persona_email(&persona, &claimed, "claimed").await.unwrap();
    s.link_persona_email(&persona, &unverified, "git").await.unwrap();
    let churn =
        seed_metric(&s, &format!("_test:provable:{uniq}:churn"), "ComputeChurn", 0, None).await;
    for who in [&claimed, &unverified, &colleague] {
        s.upsert_workspace_metric_repo(&MetricRow {
            metric_id: &churn,
            repository_id: &rid,
            scope: "user",
            identity: Some(who.as_str()),
            commit_sha: None,
            computed_on: chrono::NaiveDate::from_ymd_opt(2020, 5, 1).unwrap(),
            grain: "daily",
            value: 0.3,
            props: &serde_json::json!({}),
            source: "measured",
        })
        .await
        .unwrap();
    }
    let identities = |rows: Vec<crate::db::pg_store::sync::PushableMetric>| {
        let s = &s;
        async move {
            let mut out = vec![];
            for r in rows {
                let (who,): (Option<String>,) = sqlx_core::query_as::query_as(
                    "SELECT identity FROM sensei.repository_metrics WHERE id = $1",
                )
                .bind(r.id)
                .fetch_one(s.pool())
                .await
                .unwrap();
                out.push(who);
            }
            out.sort();
            out
        }
    };

    let exclusive = s.unpushed_user_metric_rows(&persona, &[&key], &[&key], 500).await.unwrap();
    let shared = s.unpushed_user_metric_rows(&persona, &[&key], &[], 500).await.unwrap();
    let held = s.unprovable_user_metric_count(&persona, &[&key], &[&key]).await.unwrap();
    let got_exclusive = identities(exclusive).await;
    let got_shared = identities(shared).await;

    sqlx_core::query::query("DELETE FROM sensei.personas WHERE id = $1")
        .bind(persona)
        .execute(s.pool())
        .await
        .unwrap();
    crate::tasks::test_support::cleanup_metrics_fixture(&s, &pid, None, &[]).await;

    assert_eq!(
        got_exclusive,
        vec![None, Some(claimed.clone())],
        "own session row + verified address"
    );
    assert_eq!(
        got_shared,
        vec![Some(claimed)],
        "another persona elected it too: only the verified address"
    );
    assert_eq!(held, 2, "the unverified address and the colleague are held back, and counted");
    assert_eq!(
        got_exclusive.len() as i64 + held,
        4,
        "pushed + held = every user row in play: the window and the count see one population"
    );
}

/// #290: a workspace's file count is the distinct files its nodes sit in. It
/// counted `kind = 'file'` nodes, which the indexer no longer writes — 0 files,
/// so `rework_density` had no denominator and stopped being written.
#[tokio::test]
async fn workspace_file_count_is_the_distinct_files_its_declarations_sit_in() {
    let s = pg_store().await;
    let suffix = format!("filecount_{}", uuid::Uuid::new_v4());
    let (pid, fid) = create_test_workspace_and_folder(&s, &suffix).await;
    s.seed_node(&fid, "function", "a", "src/lib.rs", None, None, Some(1), Some(3)).await.unwrap();
    s.seed_node(&fid, "struct", "B", "src/lib.rs", None, None, Some(5), Some(9)).await.unwrap();
    s.seed_node(&fid, "function", "c", "src/other.rs", None, None, Some(1), Some(2)).await.unwrap();

    let count = s.churn_workspace_file_count(&pid).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    assert_eq!(count, 2, "two files, whatever their nodes' kinds");
}

/// #290: the scoped node read carries `is_exported` — Atlas's export count read
/// a field the query never selected, so it was 0 for every repository.
#[tokio::test]
async fn scoped_nodes_carry_is_exported() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("exported_{}", uuid::Uuid::new_v4())).await;
    let id = s
        .seed_node(&fid, "function", "run", "src/a.rs", None, None, Some(1), Some(3))
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.nodes SET is_exported = true WHERE id = $1")
        .bind(id)
        .execute(s.pool())
        .await
        .unwrap();

    let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();

    sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
        .bind(fid)
        .execute(s.pool())
        .await
        .ok();
    assert_eq!(nodes.iter().find(|n| n["name"] == "run").unwrap()["is_exported"], true);
}

/// #290: the drift read says how many documents the workspace has indexed, so a
/// screen can tell "no drift" from "no documents to check" — markdown has no
/// indexer adapter, so every live workspace has 0 and its drift is never scanned.
#[tokio::test]
async fn workspace_drift_reports_the_indexed_documents() {
    let s = pg_store().await;
    let (pid, fid) =
        create_test_workspace_and_folder(&s, &format!("docs_{}", uuid::Uuid::new_v4())).await;
    let before = s.get_workspace_drift(&pid).await.unwrap()["indexedDocs"].clone();
    s.seed_node(&fid, "doc", "Design", "docs/design.md", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let after = s.get_workspace_drift(&pid).await.unwrap()["indexedDocs"].clone();

    sqlx_core::query::query("DELETE FROM sensei.workspaces WHERE id = $1")
        .bind(pid)
        .execute(s.pool())
        .await
        .ok();
    assert_eq!((before, after), (serde_json::json!(0), serde_json::json!(1)));
}
