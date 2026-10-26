use super::*;

// ── PG Function tests ─────────────────────────────────────────────

#[tokio::test]
async fn rank_bm25_returns_results() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("bm25_{}", uuid::Uuid::new_v4())).await;
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "authenticate_user",
        "src/auth.rs",
        None,
        Some("fn authenticate_user(token: &str)"),
        Some(1),
        Some(20),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "validate_email",
        "src/validation.rs",
        None,
        Some("fn validate_email(email: &str)"),
        Some(1),
        Some(10),
    )
    .await
    .unwrap();
    let results = s.rank_bm25(&fid, "authenticate").await.unwrap();
    assert!(!results.is_empty());
    assert_eq!(results[0].0, "src/auth.rs");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn rank_bm25_empty_folder() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("bm25_empty_{}", uuid::Uuid::new_v4())).await;
    let results = s.rank_bm25(&fid, "anything").await.unwrap();
    assert!(results.is_empty());
}

// ── Nodes + Edges tests ────────────────────────────────────────────

#[tokio::test]
async fn node_upsert_and_query() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("node_{}", uuid::Uuid::new_v4())).await;
    let file_id =
        s.seed_node(&fid, "file", "main.rs", "src/main.rs", None, None, None, None).await.unwrap();
    let fn_id = s
        .seed_node(
            &fid,
            "function",
            "main",
            "src/main.rs",
            Some(&file_id),
            Some("fn main()"),
            Some(1),
            Some(10),
        )
        .await
        .unwrap();
    let nodes = s.get_nodes_by_folder(&fid).await.unwrap();
    assert_eq!(nodes.len(), 2);
    let by_file = s.get_nodes_by_file(&fid, "src/main.rs").await.unwrap();
    assert_eq!(by_file.len(), 2);
    s.delete_nodes_by_folder(&fid).await.unwrap();
    assert_eq!(s.get_nodes_by_folder(&fid).await.unwrap().len(), 0);
    let _ = (file_id, fn_id);
}

#[tokio::test]
async fn upsert_persists_doc_and_symbol_kinds() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("kinds_{}", uuid::Uuid::new_v4())).await;
    // Each of these failed the enum cast before the fix and was dropped.
    for (kind, name, path) in [
        ("doc", "README", "README.md"),
        ("struct", "Point", "src/geo.rs"),
        ("component", "Button", "src/Button.svelte"),
        ("hook", "useState", "src/Button.svelte"),
        ("extension", "review", "marketplace/commands/review.md"),
    ] {
        crate::tasks::test_support::seed_node(
            &s,
            &fid,
            kind,
            name,
            path,
            None,
            None,
            Some(1),
            Some(2),
        )
        .await
        .unwrap_or_else(|e| panic!("upsert {kind} failed: {e}"));
    }
    let kinds = s.count_nodes_by_kind(&fid).await.unwrap();
    for kind in ["doc", "struct", "component", "hook", "extension"] {
        assert_eq!(kinds.get(kind), Some(&1), "missing {kind} node");
    }
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn doc_nodes_are_embeddable() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("embed_{}", uuid::Uuid::new_v4())).await;
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "doc",
        "README",
        "README.md",
        None,
        None,
        Some(1),
        Some(2),
    )
    .await
    .unwrap();
    let pending = s.nodes_without_embeddings(&fid, 100).await.unwrap();
    assert!(
        pending.iter().any(|(_, kind, name, _, _)| kind == "doc" && name == "README"),
        "doc node not returned by nodes_without_embeddings"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn semantic_search_nodes_ranks_by_cosine() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("sem_{}", uuid::Uuid::new_v4())).await;

    // Two function nodes whose *names* share no keyword with the query.
    // 384-dim (matches the vector(384) column). `alpha` points along dim 0,
    // `beta` along dim 1 — orthogonal, so the query vector's direction alone
    // decides ranking (purely semantic, no lexical overlap).
    let dim = 384usize;
    let mut e_alpha = vec![0.0f32; dim];
    e_alpha[0] = 1.0;
    let mut e_beta = vec![0.0f32; dim];
    e_beta[1] = 1.0;

    let id_alpha =
        s.seed_node(&fid, "function", "alpha", "a.rs", None, None, Some(1), Some(9)).await.unwrap();
    let id_beta =
        s.seed_node(&fid, "function", "beta", "b.rs", None, None, Some(1), Some(9)).await.unwrap();
    s.set_node_embedding(&id_alpha, &e_alpha).await.unwrap();
    s.set_node_embedding(&id_beta, &e_beta).await.unwrap();

    // Query vector leans toward alpha's direction.
    let mut query = vec![0.0f32; dim];
    query[0] = 0.9;
    query[1] = 0.1;

    // Permissive bound (1.0 admits everything): this test is about ORDER, and
    // `beta` is deliberately orthogonal, so a production-strength cutoff would
    // correctly drop it. The bound itself is exercised by the test below.
    let hits =
        s.semantic_search_nodes(&[fid], &query, &["function", "method"], 10, 1.0).await.unwrap();

    let names: Vec<&str> = hits.iter().map(|(_, name, ..)| name.as_str()).collect();
    assert!(
        names.contains(&"alpha") && names.contains(&"beta"),
        "both nodes should surface, got {names:?}"
    );
    assert_eq!(
        names.first(),
        Some(&"alpha"),
        "alpha is the closest by cosine — must rank first, got {names:?}"
    );

    // A kind filter that matches neither node returns nothing.
    let none = s.semantic_search_nodes(&[fid], &query, &["class"], 10, 1.0).await.unwrap();
    assert!(none.is_empty(), "kind filter should exclude functions, got {none:?}");

    // Empty inputs are cheap no-ops, never a query.
    assert!(s.semantic_search_nodes(&[], &query, &["function"], 10, 1.0).await.unwrap().is_empty());
    assert!(s.semantic_search_nodes(&[fid], &[], &["function"], 10, 1.0).await.unwrap().is_empty());

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn a_lib_node_records_the_language_that_minted_it() {
    // Every one of the 21,937 lib nodes in the live graph has `language = NULL`,
    // because `upsert_lib_node_by_fqn` never set the column. That absence is not
    // cosmetic: it is what blocks the last wrong-merge guard on the receiver
    // resolver. A bare return type (`use reqwest::Client; fn f() -> Client`)
    // could be refused when a lib symbol of that name is in scope — but with no
    // language on the lib node the guard is unavoidably cross-language and would
    // refuse a Rust `Session` because a PYTHON package exports that name.
    // Measured: 23 first-party rust type names collide this way workspace-wide
    // (Action, Column, Config, Gateway, Message, Node, Plan, Request, Session,
    // Table, Transport, ...).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("liblang_{}", uuid::Uuid::new_v4())).await;

    let rust_id = s
        .upsert_lib_node_by_fqn(
            &fid,
            "lib·serde_json·serde_json·Value",
            "Value",
            "serde_json",
            Some("rust"),
        )
        .await
        .unwrap();
    let py_id = s
        .upsert_lib_node_by_fqn(
            &fid,
            "lib·pydantic·pydantic·Value",
            "Value",
            "pydantic",
            Some("python"),
        )
        .await
        .unwrap();

    macro_rules! lang {
        ($id:expr) => {{
            let r: (Option<String>,) =
                sqlx_core::query_as::query_as("SELECT language FROM sensei.nodes WHERE id = $1")
                    .bind($id)
                    .fetch_one(s.pool())
                    .await
                    .unwrap();
            r.0
        }};
    }
    assert_eq!(lang!(rust_id).as_deref(), Some("rust"));
    assert_eq!(
        lang!(py_id).as_deref(),
        Some("python"),
        "two libs exporting the SAME symbol name must be distinguishable by \
         language — that is the whole point of recording it"
    );

    // The package CONTAINER is language-scoped too. Without it, a `lib·serde_json`
    // container minted from rust and one minted from a TS file of the same name
    // would be one row with no way to tell which language's dependency it is.
    let container: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT language FROM sensei.nodes WHERE folder_id = $1 AND fqn = 'lib·serde_json'",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(container.0.as_deref(), Some("rust"), "the package container carries it too");

    s.delete_nodes_by_folder(&fid).await.ok();
}

#[tokio::test]
async fn docs_are_served_for_the_version_a_folder_pins_and_labelled_when_they_cannot_be() {
    // 02b S9 end to end through the store. A workspace on 1.2 handed 3.0's docs
    // gets a confident answer about an API it does not have; the label is what
    // makes that answer usable instead of wrong (R4).
    let s = pg_store().await;
    let rid = s
        .add_watch_root(&format!("/tmp/s9_{}", uuid::Uuid::new_v4()), "s9", &serde_json::json!([]))
        .await
        .unwrap();
    let abs = format!("/_test/s9-app-{}", uuid::Uuid::new_v4());
    let folder = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "s9-app",
            path: &abs,
            abs_path: &abs,
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();

    // Unique per run, like the watch root and the folder above. It was the one
    // FIXED identifier in the test, and `delete_library` at the bottom only
    // runs on the happy path — so a single run that died partway left
    // `library_content` rows that made every later run fail on
    // `library_content_identity_uq`, permanently, with an error naming the
    // constraint rather than the abandoned cleanup.
    let lib_name = format!("_test:s9lib:{}", uuid::Uuid::new_v4());
    let lib = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: &lib_name,
            ecosystem: "npm",
            version: Some("1.2.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    // Two held versions: the one this folder pins, and a newer one.
    for (v, page) in [("1.2.0", "list @ 1.2"), ("3.0.0", "list @ 3.0")] {
        let vid = s.ensure_library_version(&lib, Some(v)).await.unwrap();
        sqlx_core::query::query(
            "INSERT INTO sensei.library_content(library_version_id, kind, name, body, component, source_type)
             VALUES($1, 'page'::sensei.library_content_kind, 'List', $2, 'list', 'local'::sensei.library_source_type)",
        )
        .bind(vid)
        .bind(page)
        .execute(s.pool())
        .await
        .unwrap();
    }
    s.upsert_referenced_library(&folder, &lib, Some("1.2.0"), None).await.unwrap();

    // The folder pins 1.2 → it gets 1.2's docs, and NO caveat.
    let d = s.get_library_docs(&lib_name, Some("list"), Some(&abs)).await.unwrap();
    assert_eq!(d.served_version.as_deref(), Some("1.2.0"));
    assert_eq!(d.version_note, None, "an exact answer needs no caveat");
    assert_eq!(d.pages[0]["content"], "list @ 1.2");

    // Re-pin to a version nothing serves → closest, LABELLED.
    s.upsert_referenced_library(&folder, &lib, Some("2.0.0"), None).await.unwrap();
    let d2 = s.get_library_docs(&lib_name, Some("list"), Some(&abs)).await.unwrap();
    assert!(d2.version_note.is_some(), "a wrong-version answer must say so");
    assert!(
        d2.version_note.as_deref().unwrap().contains("you are on 2.0.0"),
        "the label names both versions: {:?}",
        d2.version_note
    );

    // No folder stated → latest, and no caveat INVENTED. There is no pin to miss.
    let d3 = s.get_library_docs(&lib_name, Some("list"), None).await.unwrap();
    assert_eq!(d3.version_note, None, "absent context is not a mismatch");
    assert!(d3.pinned_version.is_none());

    s.delete_library(&lib).await.unwrap();
    s.remove_watch_root(&rid).await.ok();
}

#[tokio::test]
async fn registry_urls_land_on_the_right_level_and_a_later_silence_does_not_wipe_them() {
    // 02b S8. repository/homepage are identity-level; docs_url describes where
    // a RELEASE's documentation lives. And registries disagree about what they
    // expose — npm has no documentation field — so a response that omits a URL
    // means "this one did not say", never "there is none".
    use crate::libraries::registry::RegistryUrls;
    let s = pg_store().await;
    let lib = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:urls",
            ecosystem: "cargo",
            version: Some("1.0.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();

    s.set_library_urls(
        &lib,
        &RegistryUrls {
            repository: Some("https://github.com/serde-rs/serde".into()),
            homepage: Some("https://serde.rs".into()),
            docs: Some("https://docs.rs/serde".into()),
        },
    )
    .await
    .unwrap();

    let row: (Option<String>, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT l.repository_url, l.homepage_url, v.docs_url
           FROM sensei.libraries l
           JOIN sensei.library_versions v ON v.library_id = l.id AND v.is_latest
          WHERE l.id = $1",
    )
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row.0.as_deref(), Some("https://github.com/serde-rs/serde"));
    assert_eq!(row.1.as_deref(), Some("https://serde.rs"));
    assert_eq!(row.2.as_deref(), Some("https://docs.rs/serde"), "docs live on the VERSION");

    // A second lookup from a registry that states none of them.
    s.set_library_urls(&lib, &RegistryUrls::default()).await.unwrap();
    let row2: (Option<String>, Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT l.repository_url, l.homepage_url, v.docs_url
           FROM sensei.libraries l
           JOIN sensei.library_versions v ON v.library_id = l.id AND v.is_latest
          WHERE l.id = $1",
    )
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(row2, row, "silence does not wipe what an earlier response established");

    s.delete_library(&lib).await.unwrap();
}

#[tokio::test]
async fn a_docs_read_failure_is_recorded_as_a_gap_and_never_deletes_the_pages() {
    // 02b S7b.2/S7b.3. dbd served 36 pages for two months pointing at a
    // directory that had been deleted, and nothing registered it as a gap —
    // the failure only ever reached a task log. Two properties here:
    //   1. the error is RECORDED against the version, so it is queryable;
    //   2. the pages SURVIVE it. Stale content is the last known-true content,
    //      and dropping it on a read error trades a stale answer for no answer.
    let s = pg_store().await;
    let lib = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:stale",
            ecosystem: "npm",
            version: Some("1.0.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();
    s.upsert_library_page(&LibraryPageRow {
        library_id: &lib,
        title: "Overview",
        url: None,
        local_path: Some("/gone/llms/overview.txt"),
        description: None,
        content: Some("body"),
        source_type: "local",
        component: Some("overview"),
        package_name: None,
        version: None,
    })
    .await
    .unwrap();

    const COUNT_PAGES: &str = "SELECT count(*) FROM sensei.library_content c
           JOIN sensei.library_versions v ON v.id = c.library_version_id
          WHERE v.library_id = $1 AND c.kind = 'page'::sensei.library_content_kind";
    let n: (i64,) =
        sqlx_core::query_as::query_as(COUNT_PAGES).bind(lib).fetch_one(s.pool()).await.unwrap();
    assert_eq!(n.0, 1);

    // The source vanishes.
    s.record_library_docs_error(&lib, Some("llms root not a directory: /gone/llms")).await.unwrap();

    let (err, checked): (Option<String>, Option<String>) = sqlx_core::query_as::query_as(
        "SELECT props->>'docs_error', props->>'docs_checked_at'
           FROM sensei.library_versions WHERE library_id = $1 AND is_latest",
    )
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(err.as_deref(), Some("llms root not a directory: /gone/llms"));
    assert!(checked.is_some(), "checked-and-fine differs from never-checked");
    let n: (i64,) =
        sqlx_core::query_as::query_as(COUNT_PAGES).bind(lib).fetch_one(s.pool()).await.unwrap();
    assert_eq!(n.0, 1, "the last known-true content survives the failure");

    // The docs come back. The gap clears itself — nobody has to intervene.
    s.record_library_docs_error(&lib, None).await.unwrap();
    let err2: (Option<String>,) = sqlx_core::query_as::query_as(
        "SELECT props->>'docs_error' FROM sensei.library_versions
          WHERE library_id = $1 AND is_latest",
    )
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(err2.0, None, "a recovered library stops reporting stale");

    s.delete_library(&lib).await.unwrap();
}

#[tokio::test]
async fn two_packages_of_one_library_can_each_document_the_same_component() {
    // A library publishes several packages, and docs are PACKAGE-level while
    // skills and agents are library-level. `rokkit` ships both `@rokkit/ui`
    // and `@rokkit/chart`, and each can have a `List` page. Keyed without the
    // package, the second upserts over the first and one is simply lost.
    let s = pg_store().await;
    let lib = s
        .upsert_library(&crate::db::pg_store::LibraryRow {
            name: "_test:pagekey",
            ecosystem: "npm",
            version: Some("1.0.0"),
            description: None,
            source_type: None,
            base_url: None,
        })
        .await
        .unwrap();

    let ui = s
        .upsert_library_page(&LibraryPageRow {
            library_id: &lib,
            title: "List",
            url: None,
            local_path: None,
            description: None,
            content: Some("ui list docs"),
            source_type: "local",
            component: Some("List"),
            package_name: Some("@_test/ui"),
            version: None,
        })
        .await
        .unwrap();
    let chart = s
        .upsert_library_page(&LibraryPageRow {
            library_id: &lib,
            title: "List",
            url: None,
            local_path: None,
            description: None,
            content: Some("chart list docs"),
            source_type: "local",
            component: Some("List"),
            package_name: Some("@_test/chart"),
            version: None,
        })
        .await
        .unwrap();
    assert_ne!(ui, chart, "two packages, two rows — not one evicting the other");

    // A LIBRARY-LEVEL page (no package) is a third, distinct row.
    let overview = s
        .upsert_library_page(&LibraryPageRow {
            library_id: &lib,
            title: "List",
            url: None,
            local_path: None,
            description: None,
            content: Some("overview"),
            source_type: "local",
            component: Some("List"),
            package_name: None,
            version: None,
        })
        .await
        .unwrap();
    assert_ne!(overview, ui);
    assert_ne!(overview, chart);

    // ...but NULLS NOT DISTINCT means re-ingesting it UPDATES rather than
    // inserting a duplicate. Postgres treats NULLs as distinct by default, so
    // without that clause every re-ingest would add another overview row.
    let overview_again = s
        .upsert_library_page(&LibraryPageRow {
            library_id: &lib,
            title: "List",
            url: None,
            local_path: None,
            description: None,
            content: Some("overview v2"),
            source_type: "local",
            component: Some("List"),
            package_name: None,
            version: None,
        })
        .await
        .unwrap();
    assert_eq!(overview, overview_again, "library-level pages are still constrained");

    let n: (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM sensei.library_content c
           JOIN sensei.library_versions v ON v.id = c.library_version_id
          WHERE v.library_id = $1 AND c.kind = 'page'::sensei.library_content_kind",
    )
    .bind(lib)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(n.0, 3, "two package pages plus one library-level page");

    s.delete_library(&lib).await.unwrap();
}

#[tokio::test]
async fn a_repo_relative_path_resolves_to_a_file_in_a_module_folder() {
    // 06 S6 / R13. v1 callers hold the REPO folder and a repo-relative path;
    // the file belongs to a MODULE folder under a folder-relative one. Matching
    // `(folder_id, file_path)` directly misses every file in a module — 17 of
    // this repo's 18 folders — and the miss is indistinguishable from "not
    // indexed". Anchoring both sides to the absolute path is what makes the two
    // grains meet.
    let s = pg_store().await;
    let base = format!("/tmp/fidfor_{}", uuid::Uuid::new_v4());
    let rid = s.add_watch_root(&base, "fidfor", &serde_json::json!([])).await.unwrap();

    let repo = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "repo",
            path: &base,
            abs_path: &base,
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    let mod_abs = format!("{base}/crates/senseid");
    let module = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "module",
            name: "senseid",
            path: "crates/senseid",
            abs_path: &mod_abs,
            parent_id: Some(&repo),
            workspace_root_id: None,
        })
        .await
        .unwrap();

    // The file is FOLDER-relative to the module.
    let fid = s.upsert_file_row(&module, "src/lib.rs", 1, "h", None).await.unwrap();

    // Asked for from the REPO folder, repo-relative — the v1 caller's shape.
    assert_eq!(
        s.file_id_for(&repo, "crates/senseid/src/lib.rs").await.unwrap(),
        Some(fid),
        "the repo-relative path must reach the module's file"
    );
    // Asked for from the module itself, folder-relative.
    assert_eq!(s.file_id_for(&module, "src/lib.rs").await.unwrap(), Some(fid));
    // An absolute path resolves from either anchor.
    assert_eq!(s.file_id_for(&repo, &format!("{mod_abs}/src/lib.rs")).await.unwrap(), Some(fid));

    // FAILS CLOSED. A miss is None — never a licence to create a phantom row.
    assert_eq!(s.file_id_for(&repo, "crates/senseid/src/nope.rs").await.unwrap(), None);
    let n: (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.files WHERE folder_id = $1")
            .bind(module)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(n.0, 1, "a miss created nothing");

    s.remove_watch_root(&rid).await.ok();
}

/// When two folders hold a row for the SAME file, the caller's own folder wins.
///
/// A nested duplicate checkout is a real, deliberate state — it is what
/// `contained_duplicate_folders` reports on — and both folders legitimately
/// carry a `files` row for the same file on disk. Resolving purely by absolute
/// path matches both and picks whichever the planner returns first, so a node in
/// the inner folder can end up keyed to the OUTER folder's row.
///
/// Nothing errors when that happens. The node reads back with a path assembled
/// from the wrong folder's prefix — `crates/member/crates/member/src/lib.rs`,
/// which is how this was found — and a plausible-looking wrong path is exactly
/// the wrong answer R4 ranks below no answer at all.
///
/// Asserted from BOTH anchors, because a rule that resolves an ambiguity has to
/// resolve it the same way for each side or it has only moved the ambiguity.
#[tokio::test]
async fn a_file_two_folders_both_track_resolves_to_the_callers_own_folder() {
    let s = pg_store().await;
    let base = format!("/tmp/fidtwo_{}", uuid::Uuid::new_v4());
    let rid = s.add_watch_root(&base, "fidtwo", &serde_json::json!([])).await.unwrap();

    let outer = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "outer",
            path: &base,
            abs_path: &base,
            parent_id: None,
            workspace_root_id: None,
        })
        .await
        .unwrap();
    let inner_abs = format!("{base}/vendored");
    let inner = s
        .upsert_folder(&crate::db::pg_store::FolderRow {
            root_id: &rid,
            kind: "git",
            name: "vendored",
            path: "vendored",
            abs_path: &inner_abs,
            parent_id: Some(&outer),
            workspace_root_id: None,
        })
        .await
        .unwrap();

    // One file on disk, tracked by both folders at their own grains.
    let outer_row = s.upsert_file_row(&outer, "vendored/src/lib.rs", 1, "h", None).await.unwrap();
    let inner_row = s.upsert_file_row(&inner, "src/lib.rs", 1, "h", None).await.unwrap();
    assert_ne!(outer_row, inner_row, "the fixture must really create two rows");

    assert_eq!(
        s.file_id_for(&inner, "src/lib.rs").await.unwrap(),
        Some(inner_row),
        "the inner folder's own row wins for the inner folder"
    );
    assert_eq!(
        s.file_id_for(&outer, "vendored/src/lib.rs").await.unwrap(),
        Some(outer_row),
        "the outer folder's own row wins for the outer folder"
    );

    s.remove_watch_root(&rid).await.ok();
}

#[tokio::test]
async fn folder_completeness_propagates_incompleteness_up_the_tree() {
    // The whole point of the view: a folder is complete only when everything
    // BENEATH it is too. One unfinished file deep in a subtree must keep every
    // ancestor incomplete, or a link phase fires against a partial graph.
    //
    // The recursion is over TREE STRUCTURE (parent_id), never over folder
    // status — a view that read `folders.status` to decide `folders.status`
    // would be self-referential and need a fixpoint loop to converge. This test
    // pins that one pass suffices.
    let s = pg_store().await;
    let root_path = format!("/tmp/fc_{}", uuid::Uuid::new_v4());
    let root_id = s.add_watch_root(&root_path, "fc", &serde_json::json!([])).await.unwrap();

    // root ── child ── grandchild
    let root = crate::tasks::test_support::seed_repo_folder(&s, &root_id, "fc-root", &root_path)
        .await
        .unwrap();
    let child = crate::tasks::test_support::seed_repo_folder(
        &s,
        &root_id,
        "fc-child",
        &format!("{root_path}/child"),
    )
    .await
    .unwrap();
    let grand = crate::tasks::test_support::seed_repo_folder(
        &s,
        &root_id,
        "fc-grand",
        &format!("{root_path}/child/grand"),
    )
    .await
    .unwrap();
    for (c, p) in [(child, root), (grand, child)] {
        sqlx_core::query::query("UPDATE sensei.folders SET parent_id = $2 WHERE id = $1")
            .bind(c)
            .bind(p)
            .execute(s.pool())
            .await
            .unwrap();
    }

    macro_rules! complete {
        ($id:expr) => {{
            let r: (bool,) = sqlx_core::query_as::query_as(
                "SELECT subtree_complete FROM sensei.folder_completeness WHERE id = $1",
            )
            .bind($id)
            .fetch_one(s.pool())
            .await
            .unwrap();
            r.0
        }};
    }

    // Nothing walked yet: expected IS NULL everywhere ⇒ all incomplete.
    // Never-walked must NOT read as complete — that is the fail-safe direction.
    assert!(!complete!(root), "a never-walked root is not complete");
    assert!(!complete!(grand), "a never-walked leaf is not complete");

    // Walk them all. root and child hold no files of their own; grand holds one.
    s.set_folder_expected_files(&root, 0).await.unwrap();
    s.set_folder_expected_files(&child, 0).await.unwrap();
    s.set_folder_expected_files(&grand, 1).await.unwrap();

    // grand's single file is still undecided (no `files` row at all) — which
    // is exactly the case a bare "no undecided rows" check gets wrong.
    assert!(!complete!(grand), "a folder short of its denominator is incomplete");
    assert!(!complete!(child), "incompleteness propagates to the parent");
    assert!(!complete!(root), "...and all the way to the root");

    // WALKING the file is not deciding it. The row now exists, but no parse
    // has run — the file is `discovered`, and the folder is still short of its
    // denominator. This assertion is the one that catches a `decided`
    // predicate keyed on `indexed_at`, which is NOT NULL DEFAULT now() and so
    // counts every walked file the instant its row appears.
    s.upsert_scan_state(&grand, "a.rs", 1, "h").await.unwrap();
    assert!(!complete!(grand), "a walked-but-unparsed file is discovered, not decided");

    // NOW decide it.
    assert_eq!(s.mark_file_parsed(&grand, "a.rs").await.unwrap(), 1);
    assert!(complete!(grand), "denominator met");
    assert!(complete!(child), "an empty folder whose subtree is done is done");
    assert!(complete!(root), "completeness reaches the root in ONE pass, no loop");

    // A CONTENT CHANGE un-decides it. The recorded parse described bytes that
    // no longer exist, so the file returns to `discovered` and the whole tree
    // is incomplete again — otherwise a stale parse counts as current work.
    s.upsert_scan_state(&grand, "a.rs", 2, "DIFFERENT").await.unwrap();
    assert!(!complete!(grand), "a changed file needs re-parsing");
    assert!(!complete!(root), "...and that reaches the root too");

    s.remove_watch_root(&root_id).await.ok();
}

#[tokio::test]
async fn folder_completeness_counts_a_deliberate_skip_as_decided() {
    // A binary or non-UTF-8 file can never be indexed, and it is fingerprinted
    // with a `skip_reason` precisely so it stops being re-enqueued. If the view
    // treated a skip as undecided, any folder holding one would never complete
    // and the whole subtree above it would be stuck forever.
    let s = pg_store().await;
    let root_path = format!("/tmp/fcskip_{}", uuid::Uuid::new_v4());
    let root_id = s.add_watch_root(&root_path, "fcs", &serde_json::json!([])).await.unwrap();
    let f = crate::tasks::test_support::seed_repo_folder(&s, &root_id, "fcs-f", &root_path)
        .await
        .unwrap();

    s.set_folder_expected_files(&f, 1).await.unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.files (folder_id, file_path, mtime, content_hash, skip_reason)
         VALUES ($1, 'logo.png', 1, 'h', 'binary_content')",
    )
    .bind(f)
    .execute(s.pool())
    .await
    .unwrap();

    let r: (bool,) = sqlx_core::query_as::query_as(
        "SELECT subtree_complete FROM sensei.folder_completeness WHERE id = $1",
    )
    .bind(f)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(r.0, "a skip is a verdict — the folder is decided, not stuck");

    s.remove_watch_root(&root_id).await.ok();
}

/// `decided` is a SUM of two different outcomes, and progress must be able to
/// tell them apart (08 S3).
///
/// "40 of 100 decided" reads as work done. If 35 of those 40 are files that can
/// never be indexed, the folder is not 40% understood — it is 5% understood and
/// 35% skipped, and only the breakdown says so. A single total is the shape of
/// count that has produced real defects in this design's history: correct over
/// the wrong population, with nothing to disagree with.
///
/// The two are kept as separate columns rather than one ratio, because a
/// consumer that wants the ratio can divide and a consumer that wants the counts
/// cannot un-divide.
#[tokio::test]
async fn folder_completeness_separates_a_parse_from_a_skip() {
    let s = pg_store().await;
    let root_path = format!("/tmp/fcsplit_{}", uuid::Uuid::new_v4());
    let root_id = s.add_watch_root(&root_path, "fcsplit", &serde_json::json!([])).await.unwrap();
    let f = crate::tasks::test_support::seed_repo_folder(&s, &root_id, "fcsplit-f", &root_path)
        .await
        .unwrap();

    // Three files, one of each state — so a column that reported the wrong one
    // cannot coincide with the right answer.
    s.set_folder_expected_files(&f, 3).await.unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.files (folder_id, file_path, mtime, content_hash, skip_reason, parsed_at)
         VALUES ($1, 'a.rs',    1, 'h', NULL,             now())
              , ($1, 'logo.png',1, 'h', 'binary_content', NULL)
              , ($1, 'b.rs',    1, 'h', NULL,             NULL)",
    )
    .bind(f)
    .execute(s.pool())
    .await
    .unwrap();

    let (expected, decided, parsed, skipped, complete): (i64, i64, i64, i64, bool) =
        sqlx_core::query_as::query_as(
            "SELECT expected, decided, parsed, skipped, subtree_complete
               FROM sensei.folder_completeness WHERE id = $1",
        )
        .bind(f)
        .fetch_one(s.pool())
        .await
        .unwrap();

    assert_eq!(expected, 3, "the denominator is the barrier's count");
    assert_eq!(parsed, 1, "one file actually reached a parse");
    assert_eq!(skipped, 1, "one can never be parsed and said so");
    assert_eq!(decided, 2, "a verdict is either of those");
    assert_eq!(decided, parsed + skipped, "and is exactly their sum — no third way to decide");
    assert!(!complete, "the third file has no verdict at all, so the folder is not done");

    s.remove_watch_root(&root_id).await.ok();
}

#[tokio::test]
async fn folder_expected_files_round_trips_and_is_the_completeness_denominator() {
    // The denominator for folder completeness, persisted at walk time.
    //
    // Counting only the `files` rows that EXIST cannot decide completeness: a
    // walk that dies at file 40 of 100 leaves 40 rows all marked decided and 60
    // with no row at all, so "no undecided rows" is vacuously true. The walk is
    // the only place that knows how many files there should be, so it records it.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("expfiles_{}", uuid::Uuid::new_v4())).await;

    assert_eq!(
        s.folder_expected_files(&fid).await.unwrap(),
        None,
        "a folder never walked has no denominator — that is None, NOT zero, because \
         zero would mean 'complete' for a folder nothing has looked at yet"
    );

    s.set_folder_expected_files(&fid, 42).await.unwrap();
    assert_eq!(s.folder_expected_files(&fid).await.unwrap(), Some(42));

    // A re-walk that finds fewer files must LOWER it — otherwise deleting files
    // would leave the folder permanently short of a denominator it can never meet.
    s.set_folder_expected_files(&fid, 7).await.unwrap();
    assert_eq!(s.folder_expected_files(&fid).await.unwrap(), Some(7));

    // An empty folder is complete once walked, so zero must be storable and
    // distinguishable from "never walked".
    s.set_folder_expected_files(&fid, 0).await.unwrap();
    assert_eq!(s.folder_expected_files(&fid).await.unwrap(), Some(0));

    s.delete_nodes_by_folder(&fid).await.ok();
}

#[tokio::test]
async fn set_folder_expected_files_preserves_other_props() {
    // It writes ONE key into a shared jsonb blob. Clobbering the folder's
    // identity (label/role/summary) to record a file count would be a bad trade.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("expprops_{}", uuid::Uuid::new_v4())).await;

    s.set_folder_props(&fid, &serde_json::json!({"label": "keep me", "role": "app"}))
        .await
        .unwrap();
    s.set_folder_expected_files(&fid, 9).await.unwrap();

    let row: (serde_json::Value,) =
        sqlx_core::query_as::query_as("SELECT props FROM sensei.folders WHERE id = $1")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(row.0.get("label").and_then(|v| v.as_str()), Some("keep me"));
    assert_eq!(row.0.get("role").and_then(|v| v.as_str()), Some("app"));
    assert_eq!(s.folder_expected_files(&fid).await.unwrap(), Some(9));

    s.delete_nodes_by_folder(&fid).await.ok();
}

#[tokio::test]
async fn semantic_search_nodes_drops_neighbours_beyond_the_distance_bound() {
    // An ANN query returns its k nearest neighbours HOWEVER FAR AWAY they are.
    // Unbounded, a query naming a symbol that does not exist still comes back
    // with a confident list of whatever happened to be closest — indistinguishable
    // from a real match, which is the fabricate-on-miss shape this repo forbids.
    //
    // The bound is measured, not guessed. Over all 108,420 embedded function
    // nodes in the live corpus, distance from one symbol to its genuine
    // relatives runs 0.12–0.26, while the corpus p01 is 0.4865 and the median
    // 0.8892. Related and unrelated are separated by a wide empty band, so a
    // cutoff anywhere in it — 0.45 — keeps every real hit and admits under 1%
    // of the corpus as candidates.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("semcut_{}", uuid::Uuid::new_v4())).await;

    let dim = 384usize;
    let mut e_near = vec![0.0f32; dim];
    e_near[0] = 1.0;
    // Orthogonal to the query direction → cosine distance ~1.0, far outside
    // any plausible bound. This is the "nearest neighbour that is not a match".
    let mut e_far = vec![0.0f32; dim];
    e_far[1] = 1.0;

    let id_near =
        s.seed_node(&fid, "function", "near", "n.rs", None, None, Some(1), Some(9)).await.unwrap();
    let id_far = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "far",
        "f.rs",
        None,
        None,
        Some(1),
        Some(9),
    )
    .await
    .unwrap();
    s.set_node_embedding(&id_near, &e_near).await.unwrap();
    s.set_node_embedding(&id_far, &e_far).await.unwrap();

    let mut query = vec![0.0f32; dim];
    query[0] = 1.0;

    let hits =
        s.semantic_search_nodes(&[fid], &query, &["function", "method"], 10, 0.45).await.unwrap();
    let names: Vec<&str> = hits.iter().map(|(_, name, ..)| name.as_str()).collect();
    assert_eq!(
        names,
        vec!["near"],
        "the orthogonal node is beyond the bound and must be dropped, not returned as a hit"
    );

    // The distance is REPORTED, not discarded. Ranking alone cannot tell a
    // caller whether hit #1 is an exact match or the least-bad of a bad set;
    // only the score can, so it has to survive the query.
    assert!(
        hits[0].5 < 0.01,
        "an exact-direction match reports a near-zero distance, got {}",
        hits[0].5
    );

    // A query pointing at nothing in the corpus returns NOTHING — the honest
    // empty. Before the bound this returned both nodes.
    let mut orthogonal = vec![0.0f32; dim];
    orthogonal[2] = 1.0;
    let none = s
        .semantic_search_nodes(&[fid], &orthogonal, &["function", "method"], 10, 0.45)
        .await
        .unwrap();
    assert!(none.is_empty(), "no node is within the bound — must return empty, got {none:?}");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn edge_insert_and_query() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edge_{}", uuid::Uuid::new_v4())).await;
    let fn_a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let fn_b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "b",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    s.insert_edge(&fid, &fn_a, crate::db::pg_store::EdgeTarget::Node(&fn_b), "calls")
        .await
        .unwrap();
    let callers = s.get_callers(&fn_b).await.unwrap();
    assert_eq!(callers.len(), 1);
    assert_eq!(callers[0]["caller_id"], fn_a.to_string());
    let callees = s.get_callees(&fn_a).await.unwrap();
    assert_eq!(callees.len(), 1);
    let by_kind = s.get_edges_by_kind(&fid, "calls").await.unwrap();
    assert_eq!(by_kind.len(), 1);
    s.delete_nodes_by_folder(&fid).await.unwrap(); // cascades edges
}

#[tokio::test]
async fn insert_edge_is_idempotent() {
    // D1: edges have identity — inserting the same edge twice returns the
    // SAME id and adds no second row, for both resolved and unresolved edges.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edgeidem_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "b",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();

    // Resolved edge: a repeated identical insert upserts to the same row.
    let e1 =
        s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap();
    let e2 =
        s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap();
    assert_eq!(e1, e2, "a repeated resolved edge returns the same id");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "no duplicate resolved edge"
    );

    // Unresolved edge: a repeated insert (same source, target_name, kind)
    // upserts to the same row (nulls-not-distinct target_file).
    let u1 = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("ext_fn"), file: None },
            "calls",
        )
        .await
        .unwrap();
    let u2 = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("ext_fn"), file: None },
            "calls",
        )
        .await
        .unwrap();
    assert_eq!(u1, u2, "a repeated unresolved edge returns the same id");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "one resolved (a→b) + one unresolved (a→ext_fn), no dupes"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_merges_into_existing_resolved_edge() {
    // D1: promoting an unresolved edge to a target that already has a resolved
    // edge from the same (source, kind) must MERGE (delete the loser), not
    // throw a unique violation against edges_unique_resolved.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolvemerge_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "b",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();

    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap(); // resolved a→b
    let u = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("b"), file: None },
            "calls",
        )
        .await
        .unwrap(); // unresolved a→"b"
    // The resolved and unresolved partial indexes are DISJOINT: both edges
    // coexist (no collision on insert) until resolution merges them.
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "resolved a→b and unresolved a→\"b\" coexist as two rows"
    );

    s.resolve_edge(&u, &b).await.unwrap(); // collides with a→b → merge

    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "the redundant edge is merged away, not duplicated"
    );
    let exists: (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.edges WHERE id=$1)")
        .bind(u)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(!exists.0, "the loser unresolved edge is deleted");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_communities_for_folder_kills_stale_and_orphans() {
    // D4 invariant 5: the per-folder replace DELETEs stale community rows,
    // CLEARs every node's community_id, then writes the new set — no orphaned
    // rows, no stranded community_ids. Per-folder, sum(node_count) equals the
    // count of nodes actually carrying a community_id.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .seed_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let c = s
        .seed_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();

    // Stale prior state: community 99 + nodes a & c assigned to it.
    s.upsert_community(&fid, 99, "stale", 2).await.unwrap();
    s.update_node_community(&a, 99).await.unwrap();
    s.update_node_community(&c, 99).await.unwrap();

    // Replace with a single community {1: [a, b]} — c must be orphaned out.
    s.replace_communities_for_folder(
        &fid,
        &[CommunityAssignment {
            community_id: 1,
            label: "new".into(),
            member_node_ids: vec![a, b],
            god_node_ids: vec![a],
        }],
    )
    .await
    .unwrap();

    assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "stale community 99 is gone");
    let cid = |id: uuid::Uuid| {
        let s = &s;
        async move {
            let (v,): (Option<i32>,) =
                query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id)
                    .fetch_one(s.pool())
                    .await
                    .unwrap();
            v
        }
    };
    assert_eq!(cid(a).await, Some(1), "a assigned to the new community");
    assert_eq!(cid(b).await, Some(1), "b assigned to the new community");
    assert_eq!(cid(c).await, None, "c's stale community_id is cleared (orphan removed)");

    // Per-folder integrity: claimed node_count == real nodes carrying a community_id.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (real,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, real, "claimed == real (invariant 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_communities_reruns_change_zero_rows() {
    // Same class of fix as the degree guard: community_id is deterministic for
    // an identical graph (invariant 2), so re-applying the SAME assignment must
    // rewrite 0 node rows. The old null-all-then-reset rewrote every community
    // node TWICE per DetectCommunities pass — dead-tuple churn on every re-scan.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_norerun_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .seed_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let assignment = vec![CommunityAssignment {
        community_id: 1,
        label: "one".into(),
        member_node_ids: vec![a, b],
        god_node_ids: vec![a],
    }];

    let first = s.replace_communities_for_folder(&fid, &assignment).await.unwrap();
    assert_eq!(first, 2, "first assignment sets community_id on both members");

    // Re-detect with the SAME (deterministic) assignment → 0 node rows change.
    let second = s.replace_communities_for_folder(&fid, &assignment).await.unwrap();
    assert_eq!(second, 0, "an unchanged re-detect rewrites 0 nodes (no dead tuples)");

    // A real change (b moves to its own community 2) still writes only what moved.
    let moved = s
        .replace_communities_for_folder(
            &fid,
            &[
                CommunityAssignment {
                    community_id: 1,
                    label: "one".into(),
                    member_node_ids: vec![a],
                    god_node_ids: vec![a],
                },
                CommunityAssignment {
                    community_id: 2,
                    label: "two".into(),
                    member_node_ids: vec![b],
                    god_node_ids: vec![b],
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(moved, 1, "only b changed community (a stays in 1)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn detect_communities_assigns_deterministic_ids_by_natural_key() {
    // D4 invariant 2: community_id is DETERMINISTIC — communities are ranked
    // 1..k by the natural key (file_path, line_start, …) of their smallest
    // member, so an identical graph always yields identical ids. Two disjoint
    // triangles ⇒ two communities; the one holding the earliest node is #1.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commdet_{}", uuid::Uuid::new_v4())).await;
    let mut n = std::collections::HashMap::new();
    for (name, line) in [("a", 10), ("b", 20), ("c", 30), ("d", 40), ("e", 50), ("f", 60)] {
        let id = s
            .seed_node(&fid, "function", name, "a.rs", None, Some("()"), Some(line), Some(line + 1))
            .await
            .unwrap();
        n.insert(name, id);
    }
    // Two disjoint triangles: {a,b,c} and {d,e,f} (resolved calls).
    for (x, y) in [("a", "b"), ("b", "c"), ("c", "a"), ("d", "e"), ("e", "f"), ("f", "d")] {
        s.insert_edge(&fid, &n[x], crate::db::pg_store::EdgeTarget::Node(&n[y]), "calls")
            .await
            .unwrap();
    }

    let read_ids = |s: &PgStore, n: &std::collections::HashMap<&str, uuid::Uuid>| {
        let ids: Vec<uuid::Uuid> = ["a", "b", "c", "d", "e", "f"].iter().map(|k| n[*k]).collect();
        let pool = s.pool().clone();
        async move {
            let mut out = Vec::new();
            for id in ids {
                let (v,): (Option<i32>,) =
                    query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                        .bind(id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                out.push(v);
            }
            out
        }
    };

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    let first: Vec<Option<i32>> = read_ids(&s, &n).await;
    // {a,b,c} share a community; {d,e,f} share another; triangle-a (earliest
    // natural key) is community 1, triangle-d is 2.
    assert_eq!(&first[0..3], &[Some(1), Some(1), Some(1)], "earliest triangle is community 1");
    assert_eq!(&first[3..6], &[Some(2), Some(2), Some(2)], "later triangle is community 2");

    // Invariant 5 after a REAL detect run (not just the hand-built replace):
    // claimed sum(node_count) == real nodes carrying a community_id.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let (real,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, real, "per-folder claimed == real after detect_communities (invariant 5)");

    // Re-running over the identical graph yields the identical assignment.
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    let second: Vec<Option<i32>> = read_ids(&s, &n).await;
    assert_eq!(first, second, "identical graph ⇒ identical community ids (deterministic)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn detect_communities_clears_stale_on_empty_folder() {
    // D4 invariant 5: running detection on a folder that has become empty
    // clears its stale community rows (the nodes.is_empty() → replace([]) path).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commempty_{}", uuid::Uuid::new_v4())).await;
    s.upsert_community(&fid, 1, "stale", 3).await.unwrap();
    assert_eq!(s.list_communities(&fid).await.unwrap().len(), 1, "seeded a stale community");

    // No nodes exist for this folder → detection must clear the stale rows.
    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();
    assert!(
        s.list_communities(&fid).await.unwrap().is_empty(),
        "an empty folder's stale communities are cleared"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_coverage_full_singletons_inherit_file_community() {
    // D4.4: every node gets a community_id. A file's symbols with NO call/
    // import edge still land in a community via `parent_id` containment
    // (they cluster under the file), and any residual singleton inherits its
    // enclosing file community — so coverage is ~100% (invariant 5), not just
    // the nodes that happen to carry a resolved semantic edge.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commcov_{}", uuid::Uuid::new_v4())).await;
    // A file with a struct + two methods, and NO edges between any of them.
    let file = s
        .seed_node(&fid, "file", "widget.rs", "src/widget.rs", None, None, Some(1), Some(99))
        .await
        .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "struct",
        "Widget",
        "src/widget.rs",
        Some(&file),
        None,
        Some(2),
        Some(2),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "method",
        "new",
        "src/widget.rs",
        Some(&file),
        Some("() -> Self"),
        Some(3),
        Some(5),
    )
    .await
    .unwrap();
    crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "method",
        "render",
        "src/widget.rs",
        Some(&file),
        Some("(&self)"),
        Some(7),
        Some(20),
    )
    .await
    .unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1")
        .bind(fid)
        .fetch_one(s.pool())
        .await
        .unwrap();
    let (covered,): (i64,) = query_as(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND community_id IS NOT NULL",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(total, 4, "seeded 4 nodes");
    assert_eq!(
        covered, total,
        "every node carries a community_id (singletons inherit the file community)"
    );

    // per-folder integrity still holds with the broadened coverage.
    let (claimed,): (i64,) = query_as(
        "SELECT COALESCE(sum(node_count),0) FROM inference.communities WHERE folder_id=$1",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(claimed, covered, "claimed == real (invariant 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_adjacency_includes_extends() {
    // D4.4: the adjacency set is broadened to calls,imports,extends,references
    // (the dead `implements` is dropped). Two classes in DIFFERENT files
    // (so `parent_id` containment does NOT group them) linked only by an
    // `extends` edge land in the SAME community — before D4b, `extends` was
    // ignored and they would be separate singletons.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commext_{}", uuid::Uuid::new_v4())).await;
    let base = s
        .seed_node(&fid, "class", "Base", "src/base.rs", None, Some("class Base"), Some(1), Some(5))
        .await
        .unwrap();
    let derived = s
        .seed_node(
            &fid,
            "class",
            "Derived",
            "src/derived.rs",
            None,
            Some("class Derived"),
            Some(1),
            Some(5),
        )
        .await
        .unwrap();
    s.insert_edge(&fid, &derived, crate::db::pg_store::EdgeTarget::Node(&base), "extends")
        .await
        .unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let cid = |id: uuid::Uuid| {
        let pool = s.pool().clone();
        async move {
            let (v,): (Option<i32>,) =
                query_as("SELECT community_id FROM sensei.nodes WHERE id=$1")
                    .bind(id)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            v
        }
    };
    let cb = cid(base).await;
    let cd = cid(derived).await;
    assert!(cb.is_some(), "extends-linked class carries a community");
    assert_eq!(cb, cd, "extends-linked classes share a community (broadened adjacency)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn god_node_ids_rank_by_adjacency_not_by_a_stored_column() {
    // A community's god_node_ids are its top-5 members by degree. Degree is
    // counted from the adjacency `detect_communities_for_folder` has already
    // built from the edges — NOT read back from a `nodes.degree` cache that a
    // separate barrier had to populate first (56 of 430,988 rows were measurably
    // stale, and that drift produced wrong god nodes in 4 live communities).
    //
    // Two things give this test teeth:
    // The hub is LAST in natural-key order, so a ranking that fell back to the
    // tie-break alone — which is what a missing degree signal degrades to — would
    // put it last instead of first.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("godnode_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b = s
        .seed_node(&fid, "function", "b", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let c = s
        .seed_node(&fid, "function", "c", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    let hub = s
        .seed_node(&fid, "function", "hub", "a.rs", None, Some("()"), Some(9), Some(10))
        .await
        .unwrap();
    // a→hub, b→hub, c→hub, a→b. Adjacency degrees: hub=3, a=2, b=2, c=1.
    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&hub), "calls").await.unwrap();
    s.insert_edge(&fid, &b, crate::db::pg_store::EdgeTarget::Node(&hub), "calls").await.unwrap();
    s.insert_edge(&fid, &c, crate::db::pg_store::EdgeTarget::Node(&hub), "calls").await.unwrap();
    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap();

    crate::indexer::community::detect_communities_for_folder(&s, &fid).await.unwrap();

    let (god,): (Vec<uuid::Uuid>,) = query_as(
            "SELECT god_node_ids FROM inference.communities WHERE folder_id=$1 ORDER BY community_id LIMIT 1")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(
        god.first(),
        Some(&hub),
        "the hub ranks first from the adjacency, despite being last by natural key"
    );
    assert!(god.len() <= 5, "at most 5 god nodes per community");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn community_description_authoritative_write_is_honest_null() {
    // D4.5 never-fabricate: the authoritative detection write leaves every
    // community's description NULL with props.source='null' — honest-empty,
    // NEVER a static template. (Model prose is stamped later, off-barrier, by
    // enrich_community_descriptions.) The Done-gate keys on
    // props.source ∈ {'narration-cache','null'}.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("commdesc_{}", uuid::Uuid::new_v4())).await;
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

    let rows: Vec<(Option<String>, serde_json::Value)> =
        query_as("SELECT description, props FROM inference.communities WHERE folder_id=$1")
            .bind(fid)
            .fetch_all(s.pool())
            .await
            .unwrap();
    assert!(!rows.is_empty(), "at least one community was written");
    for (desc, props) in &rows {
        assert_eq!(*desc, None, "description is honest-NULL without a gateway");
        let source = props.get("source").and_then(|v| v.as_str());
        assert_eq!(source, Some("null"), "props.source records the honest-empty provenance");
        assert_ne!(source, Some("template"), "never a templated description (never-fabricate)");
    }

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn graph_nodes_returns_community_and_structural_edges() {
    // 7.1: get_nodes_scoped exposes community_id, and get_edges_scoped_kinds
    // returns the full layout set calls,imports,extends — NOT just calls, and
    // NOT overlay kinds like covers.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("gscope_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let b =
        s.seed_node(&fid, "class", "B", "a.rs", None, Some("()"), Some(3), Some(4)).await.unwrap();
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=5 WHERE id=$1")
        .bind(a)
        .execute(s.pool())
        .await
        .unwrap();
    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "calls").await.unwrap();
    s.insert_edge(
        &fid,
        &a,
        crate::db::pg_store::EdgeTarget::Unresolved { name: Some("lib"), file: None },
        "imports",
    )
    .await
    .unwrap();
    s.insert_edge(&fid, &b, crate::db::pg_store::EdgeTarget::Node(&a), "extends").await.unwrap();
    s.insert_edge(&fid, &a, crate::db::pg_store::EdgeTarget::Node(&b), "covers").await.unwrap(); // overlay — excluded

    let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
    let a_node = nodes.iter().find(|n| n["name"] == "a").unwrap();
    assert_eq!(a_node["community_id"].as_i64(), Some(5), "get_nodes_scoped exposes community_id");

    let edges = s.get_edges_scoped_kinds(&[fid], &["calls", "imports", "extends"]).await.unwrap();
    let kinds: std::collections::HashSet<&str> =
        edges.iter().filter_map(|e| e["kind"].as_str()).collect();
    assert_eq!(edges.len(), 3, "exactly the 3 layout edges (covers excluded)");
    assert!(
        kinds.contains("calls") && kinds.contains("imports") && kinds.contains("extends"),
        "all 3 layout kinds present: {kinds:?}"
    );
    assert!(!kinds.contains("covers"), "covers (overlay) is not a layout edge");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn communities_info_uses_live_membership() {
    // 7.3: list_communities_live_scoped counts from the real nodes.community_id
    // join, NOT the denormalized communities.node_count — so a stale count
    // doesn't drive the overview.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("livecomm_{}", uuid::Uuid::new_v4())).await;
    let n1 = s
        .seed_node(&fid, "function", "n1", "a.rs", None, Some("()"), Some(1), Some(2))
        .await
        .unwrap();
    let n2 = s
        .seed_node(&fid, "function", "n2", "a.rs", None, Some("()"), Some(3), Some(4))
        .await
        .unwrap();
    let n3 = s
        .seed_node(&fid, "function", "n3", "a.rs", None, Some("()"), Some(5), Some(6))
        .await
        .unwrap();
    // Community 1 has 2 live members, community 2 has 1 — but seed a STALE count.
    s.upsert_community(&fid, 1, "c1", 99).await.unwrap(); // stale node_count = 99
    s.upsert_community(&fid, 2, "c2", 0).await.unwrap(); // stale node_count = 0
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=1 WHERE id = ANY($1)")
        .bind(vec![n1, n2])
        .execute(s.pool())
        .await
        .unwrap();
    sqlx_core::query::query("UPDATE sensei.nodes SET community_id=2 WHERE id=$1")
        .bind(n3)
        .execute(s.pool())
        .await
        .unwrap();

    let live = s.list_communities_live_scoped(&[fid]).await.unwrap();
    let count_of = |label: &str| {
        live.iter().find(|c| c["label"] == label).and_then(|c| c["node_count"].as_i64())
    };
    assert_eq!(count_of("c1"), Some(2), "c1 sized by 2 LIVE members, not the stale 99");
    assert_eq!(count_of("c2"), Some(1), "c2 sized by 1 LIVE member, not the stale 0");
    // Ordered by live count desc → c1 first.
    assert_eq!(live.first().and_then(|c| c["label"].as_str()), Some("c1"));

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn upsert_node_at_same_line_keeps_id_and_renulls_embedding_on_sig_change() {
    // D3: a re-upsert at the SAME identity (line_start is part of the key)
    // keeps the id — preserving community_id and degree. The embedding is
    // PRESERVED when the signature is unchanged, and RE-NULLED (re-embed) when
    // the signature changed — signature being the only embed input that can
    // change on a same-identity conflict. A DIFFERENT line is a new identity.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("nodeid_{}", uuid::Uuid::new_v4())).await;
    let id1 = s
        .seed_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(10),
            Some(20),
        )
        .await
        .unwrap();

    // Simulate a prior enrich + embed pass on this node.
    let zeros = format!("[{}]", vec!["0"; 384].join(","));
    sqlx_core::query::query(
        "UPDATE sensei.nodes SET community_id = 7, embedding = $2::vector WHERE id = $1",
    )
    .bind(id1)
    .bind(&zeros)
    .execute(s.pool())
    .await
    .unwrap();

    // Re-upsert SAME line, SAME signature, only line_end grew → id kept, all preserved.
    let id2 = s
        .seed_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(10),
            Some(25),
        )
        .await
        .unwrap();
    assert_eq!(id1, id2, "a re-upsert at the same identity keeps its id");
    let (community, has_emb): (Option<i32>, bool) =
        query_as("SELECT community_id, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1")
            .bind(id1)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(
        (community, has_emb),
        (Some(7), true),
        "community_id/degree/embedding preserved when signature is unchanged"
    );

    // Re-upsert SAME line, CHANGED signature → id kept, community kept, embedding RE-NULLED.
    let id3 = s
        .seed_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i64)"),
            Some(10),
            Some(25),
        )
        .await
        .unwrap();
    assert_eq!(id1, id3, "same identity (line) keeps the id even when signature changes");
    let (community2, has_emb2): (Option<i32>, bool) =
        query_as("SELECT community_id, embedding IS NOT NULL FROM sensei.nodes WHERE id = $1")
            .bind(id1)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(community2, Some(7), "community_id is still preserved");
    assert!(!has_emb2, "embedding is re-nulled for re-embedding when the signature changed");

    // A DIFFERENT line is a new identity ⇒ a new node (a moved symbol churns).
    let id4 = s
        .seed_node(
            &fid,
            "function",
            "foo",
            "a.rs",
            None,
            Some("fn foo(x: i32)"),
            Some(99),
            Some(105),
        )
        .await
        .unwrap();
    assert_ne!(
        id1, id4,
        "a different line_start is a new identity (moved symbol re-mints until D5c nesting)"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A node whose fqn SHAPE changes must be adopted, not rejected.
///
/// Reproduces the production failure: `MetricSparkline.svelte` was indexed
/// with the module fqn `typescript·@sensei/desktop·lib/components/
/// MetricSparkline`; a later parse yielded no top-level defs, so the fqn's
/// language segment changed. `ON CONFLICT (folder_id, fqn)` cannot see the old
/// row, so the statement fell through to a raw INSERT and hit
/// `nodes_unique_identity` — the same file/kind/name/parent/line. That made
/// process_file fail, which withheld the `files` row, which made the reconcile
/// re-drive the folder every 5 minutes forever with the folder stuck `failed`.
#[tokio::test]
async fn upsert_node_by_fqn_adopts_row_when_only_the_fqn_changed() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("adopt_{}", uuid::Uuid::new_v4())).await;

    let file_path = "src/lib/components/MetricSparkline.svelte";
    let name = "lib/components/MetricSparkline";
    // NULL line_start / parent_id on purpose — exercises the NULLS NOT
    // DISTINCT semantics the identity index uses, which is exactly the shape
    // a module container node has.
    let def = || FqnDef {
        file_path,
        signature: None,
        line_start: None,
        line_end: None,
        is_exported: false,
        parent_id: None,
    };

    // Indexed once under the original fqn.
    let first = s
        .seed_node_by_fqn(
            &fid,
            "typescript·@sensei/desktop·lib/components/MetricSparkline",
            "module",
            name,
            Some("svelte"),
            Some(def()),
        )
        .await
        .unwrap();

    // Same structural identity, DIFFERENT fqn — this used to be a hard error.
    let second = s
        .seed_node_by_fqn(
            &fid,
            "svelte·@sensei/desktop·lib/components/MetricSparkline",
            "module",
            name,
            Some("svelte"),
            Some(def()),
        )
        .await
        .expect("an fqn change on an existing identity must adopt the row, not fail");

    assert_eq!(
        first, second,
        "the existing node is adopted, keeping its id (and so its edges/embedding)"
    );

    // Exactly one row, now carrying the new fqn.
    let (count, fqn): (i64, Option<String>) = query_as(
        "SELECT count(*) OVER (), n.fqn FROM sensei.nodes n
             JOIN sensei.node_paths np ON np.node_id = n.id
             WHERE n.folder_id=$1 AND np.file_path=$2 AND n.kind='module'::sensei.node_kind",
    )
    .bind(fid)
    .bind(file_path)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(count, 1, "adoption must not leave a duplicate module node behind");
    assert_eq!(
        fqn.as_deref(),
        Some("svelte·@sensei/desktop·lib/components/MetricSparkline"),
        "the adopted row is re-pointed at the new fqn"
    );
}

/// TWO FILES declaring ONE fqn must resolve to one node, not fail the folder.
///
/// An fqn is a LOOKUP KEY: a reference mints it from what the call site can see
/// — for kotlin, the package and the name — with no knowledge of which file
/// holds the definition. So the definition side cannot add file or path
/// information to disambiguate, or the two sides would never produce the same
/// string and every reference would miss. It follows that N files legitimately
/// map to one fqn, and one fqn must mean one node.
///
/// An Android workspace makes this concrete: `app/src/panama/java/…/Color.kt` and
/// `app/src/ecuador/java/…/Color.kt` each declare `val colorPrimary` in one
/// package. MEASURED over that 245-file repo: 423 distinct top-level
/// declarations, 26 colliding, all 26 `val`.
///
/// The failure needed three rows, which is why it survived: row A holds the fqn
/// for file A; file B already has a row under a DIFFERENT fqn (its state from an
/// earlier index); upserting B with A's fqn matches `ON CONFLICT (folder_id,
/// fqn)`, re-points row A's file_path to B, collides with row B on
/// `nodes_unique_identity`, and the `adopt_node_by_identity` recovery then tries
/// to give row B the fqn row A already owns — violating `nodes_unique_fqn`.
/// `process_file` returned Err, `fail_folder` withheld the `files` row, the reconcile
/// re-drove the folder every tick, and the repo never finished indexing: 112
/// retries per 200KB of daemon log, folder stuck `failed`.
///
/// Breaking mutation: drop the fqn-conflict fallback in `upsert_node_by_fqn` —
/// the third upsert returns Err and the poison pill is back.
#[tokio::test]
async fn upsert_node_by_fqn_resolves_two_files_declaring_one_fqn() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("variant_{}", uuid::Uuid::new_v4())).await;

    let shared = "kotlin·com.acme.theme·colorPrimary";
    let panama = "app/src/panama/java/com/acme/theme/Color.kt";
    let ecuador = "app/src/ecuador/java/com/acme/theme/Color.kt";
    let def = |file_path: &'static str| FqnDef {
        file_path,
        signature: None,
        line_start: Some(3),
        line_end: Some(3),
        is_exported: true,
        parent_id: None,
    };

    // Variant A claims the fqn.
    let a = s
        .seed_node_by_fqn(&fid, shared, "const", "colorPrimary", Some("kotlin"), Some(def(panama)))
        .await
        .unwrap();

    // Variant B already exists under its own (earlier) fqn — the state that
    // turns the collision into a hard error rather than a plain ON CONFLICT.
    s.seed_node_by_fqn(
        &fid,
        "kotlin·com.acme.theme·colorPrimary·ecuador",
        "const",
        "colorPrimary",
        Some("kotlin"),
        Some(def(ecuador)),
    )
    .await
    .unwrap();

    // Now B is re-derived onto the SHARED fqn. This used to be Err.
    let b = s
        .seed_node_by_fqn(&fid, shared, "const", "colorPrimary", Some("kotlin"), Some(def(ecuador)))
        .await
        .expect("a second file declaring one fqn must resolve, not fail the folder");

    assert_eq!(
        a, b,
        "one fqn is one node — that is what makes it a key the caller can mint from (package, name)"
    );

    // And exactly one row holds it, so a reference cannot resolve ambiguously.
    let holders: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2",
    )
    .bind(fid)
    .bind(shared)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(holders, 1, "an fqn must name exactly one node");
}

/// **THE ROW MOVES ON ONE RULE AND THE RECOVERY SEARCHES BY ANOTHER.**
///
/// MEASURED on the live daemon, 44 files in one day, every one failing WHOLE
/// rather than losing a single edge:
///
/// ```text
/// adopt node by identity (generatePrescriptionSchedule): no rows returned…  14
/// adopt node by identity (uploadPrescriptionImage): no rows returned…        9
/// ```
///
/// `upsert_node_ex`'s `ON CONFLICT (folder_id, fqn) DO UPDATE` sets
///
/// ```sql
/// parent_id = COALESCE(EXCLUDED.parent_id, nodes.parent_id)
/// ```
///
/// — KEEP THE OLD PARENT when the incoming one is NULL. `parent_id` is part of
/// `nodes_unique_identity`, so the row the update becomes, and therefore the
/// row it collides with, carries the OLD parent. `adopt_node_by_identity` then
/// searches with the INCOMING parent (NULL), matches nothing, and `fetch_one`
/// turns that into `RowNotFound` → the file fails → `fail_folder` withholds the
/// `files` row → the reconcile re-drives the folder forever.
///
/// It is the poison pill the arm directly above already names, on a path that
/// did not get the same treatment. Deterministic — no concurrency, which is
/// what I assumed twice before reading the two update rules side by side.
///
/// The fix does not depend on which column disagrees: the `ON CONFLICT` already
/// matched a row BY FQN, so when the identity search comes back empty that row
/// is the answer. Breaking mutation: restore `fetch_one` and this returns Err.
#[tokio::test]
async fn an_upsert_whose_identity_moved_does_not_fail_the_file() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("adopt_miss_{}", uuid::Uuid::new_v4())).await;
    let file = "src/svc.ts";

    // A parent to hang the first revision off.
    let parent = s
        .seed_node(&fid, "class", "PrescriptionService", file, None, None, Some(1), Some(99))
        .await
        .unwrap();

    let held = "ts·app·svc·generatePrescriptionSchedule";

    // X holds the fqn, nested under the parent at line 10.
    let x = s
        .seed_node_by_fqn(
            &fid,
            held,
            "function",
            "generatePrescriptionSchedule",
            Some("typescript"),
            Some(FqnDef {
                file_path: file,
                signature: None,
                line_start: Some(10),
                line_end: Some(10),
                is_exported: true,
                parent_id: Some(&parent),
            }),
        )
        .await
        .unwrap();

    // Y already occupies the identity X MOVES INTO when re-declared at line 20
    // with no parent: the COALESCE keeps X's old parent, so the collision is at
    // (file, function, name, PARENT, 20) — while the recovery will look for
    // that same row with parent NULL.
    s.seed_node(
        &fid,
        "function",
        "generatePrescriptionSchedule",
        file,
        Some(&parent),
        None,
        Some(20),
        Some(20),
    )
    .await
    .unwrap();

    // The upsert that used to fail: same fqn, now top-level (no parent), line 20.
    let got = s
        .upsert_node_by_fqn(crate::db::pg_store::FqnNode {
            folder_id: &fid,
            fqn: held,
            kind: "function",
            name: "generatePrescriptionSchedule",
            language: Some("typescript"),
            def: Some(FqnDef {
                file_path: file,
                signature: None,
                line_start: Some(20),
                line_end: Some(20),
                is_exported: true,
                parent_id: None,
            }),
        })
        .await
        .expect("an upsert whose identity moved must not fail the whole file");

    assert_eq!(got, x, "the fqn still names the node that held it");

    let holders: i64 = sqlx_core::query_scalar::query_scalar(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2",
    )
    .bind(fid)
    .bind(held)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(holders, 1, "an fqn must still name exactly one node");
}

#[tokio::test]
async fn upsert_node_by_fqn_merges_ref_and_def() {
    // FQN get-or-create (SCIP/LSIF moniker model): a REFERENCE creates an
    // unresolved stub (resolved=false, NULL file_path); a later DEFINITION
    // with the same (folder_id, fqn) returns the SAME id, flips resolved=true
    // and fills file/line/signature; a second reference shares the one node
    // and never downgrades the resolved definition. No "unresolved" state —
    // the node exists from its first mention.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("fqn_{}", uuid::Uuid::new_v4())).await;
    let fqn = "rust·senseid·widget·Widget·new";

    // 1. Reference-first → a stub.
    let stub = s.seed_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
    let (resolved, fp): (bool, Option<String>) =
        query_as("SELECT n.resolved, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id=$1")
            .bind(stub)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(!resolved, "a reference-first node is an unresolved stub");
    assert_eq!(fp, None, "a stub has no known file");

    // 2. The definition enriches the SAME node in place.
    let def = s
        .seed_node_by_fqn(
            &fid,
            fqn,
            "method",
            "new",
            Some("rust"),
            Some(FqnDef {
                file_path: "src/widget.rs",
                signature: Some("fn new() -> Self"),
                line_start: Some(10),
                line_end: Some(12),
                is_exported: true,
                parent_id: None,
            }),
        )
        .await
        .unwrap();
    assert_eq!(stub, def, "the definition get-or-creates the SAME node as the reference");
    let (resolved2, fp2, sig, ls, exported): (
        bool,
        Option<String>,
        Option<String>,
        Option<i32>,
        bool,
    ) = query_as(
        "SELECT n.resolved, np.file_path, n.signature, n.line_start, n.is_exported \
                        FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id \
                       WHERE n.id=$1",
    )
    .bind(def)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(resolved2, "the node is resolved once its definition is seen");
    assert_eq!(fp2.as_deref(), Some("src/widget.rs"));
    assert_eq!(sig.as_deref(), Some("fn new() -> Self"));
    assert_eq!(ls, Some(10));
    assert!(exported, "the definition's is_exported is written");

    // 3. A second reference shares the one node and does NOT downgrade it.
    let ref2 = s.seed_node_by_fqn(&fid, fqn, "method", "new", Some("rust"), None).await.unwrap();
    assert_eq!(ref2, def, "a later reference resolves to the same node");
    let (still_resolved, still_fp): (bool, Option<String>) =
        query_as("SELECT n.resolved, np.file_path FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id=$1")
            .bind(def)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(still_resolved, "a reference must not downgrade an already-resolved node");
    assert_eq!(
        still_fp.as_deref(),
        Some("src/widget.rs"),
        "a reference must not clear the definition's file"
    );

    // Exactly one node for this fqn.
    let (n,): (i64,) = query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND fqn=$2")
        .bind(fid)
        .bind(fqn)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(n, 1, "ref + def + ref = exactly one node");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A definition's RETURN TYPE has to reach the node row, because it is the one
/// hop of the transitive receiver chain that no call site can see for itself:
/// `ctx.pg().method()` can only name WHICH `method` once something records that
/// `pg` returns a `PgStore`. `signature` is not a substitute — it stores the
/// declaration LINE only, and MEASURED on the live graph 1,092 of 3,841 rust
/// methods (28%) wrap their signature so the `->` never appears in it.
///
/// The ORDERING is the load-bearing part of this test. A call site is routinely
/// scanned before the file defining its callee, so the row starts life as a
/// reference-minted stub and the definition merges into it later. The return
/// type must therefore survive `upsert_node_by_fqn`'s DO UPDATE in both
/// directions — the definition merging over a stub, and a later reference
/// re-touching a resolved node — and must sit alongside props another writer
/// already owns.
///
/// Breaking mutation: name `props` in that DO UPDATE set-list. The merge then
/// replaces the jsonb with EXCLUDED's default `'{}'` and the type is gone.
#[tokio::test]
async fn node_return_type_survives_the_stub_then_definition_merge() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("rettype_{}", uuid::Uuid::new_v4())).await;
    let fqn = "rust·senseid·tasks::executor·TaskContext·pg";
    let def = || FqnDef {
        file_path: "src/tasks/executor.rs",
        signature: Some("    pub fn pg("),
        line_start: Some(19),
        line_end: Some(21),
        is_exported: true,
        parent_id: None,
    };

    // 1. The CALL SITE is scanned first → an unresolved stub carrying nothing.
    let stub = s.seed_node_by_fqn(&fid, fqn, "method", "pg", Some("rust"), None).await.unwrap();
    assert_eq!(
        s.node_return_type(&stub).await.unwrap(),
        None,
        "a node nobody wrote a return type onto reports None — never a placeholder"
    );

    // 2. Another writer already owns a props key on this row (the section and
    //    rationale paths both do exactly this through `set_node_props`).
    s.set_node_props(&stub, &serde_json::json!({"marker": "pre-existing"})).await.unwrap();

    // 3. The return type is stamped, VERBATIM — the module path is what says
    //    which `PgStore` is meant, so normalising here would throw it away.
    s.set_node_return_type(&stub, "&crate::db::pg_store::PgStore").await.unwrap();
    assert_eq!(
        s.node_return_type(&stub).await.unwrap().as_deref(),
        Some("&crate::db::pg_store::PgStore"),
        "the return type round-trips verbatim"
    );

    // 4. THE MERGE: the defining file is scanned and enriches the same row.
    let node =
        s.seed_node_by_fqn(&fid, fqn, "method", "pg", Some("rust"), Some(def())).await.unwrap();
    assert_eq!(stub, node, "the definition enriches the SAME node the reference stubbed");
    let (resolved, marker): (bool, Option<String>) =
        query_as("SELECT resolved, props->>'marker' FROM sensei.nodes WHERE id=$1")
            .bind(node)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert!(resolved, "the definition still resolves the node");
    assert_eq!(marker.as_deref(), Some("pre-existing"), "the merge must not drop other props");
    assert_eq!(
        s.node_return_type(&node).await.unwrap().as_deref(),
        Some("&crate::db::pg_store::PgStore"),
        "the definition merge must not clobber the return type"
    );

    // 5. A LATER reference (another caller file) must not erase it either.
    s.seed_node_by_fqn(&fid, fqn, "method", "pg", Some("rust"), None).await.unwrap();
    assert_eq!(
        s.node_return_type(&node).await.unwrap().as_deref(),
        Some("&crate::db::pg_store::PgStore"),
        "a reference must not clear the definition's return type"
    );

    // 6. A re-scan where the signature CHANGED overwrites rather than
    //    accumulating — a stale type is a fact the resolver would chase.
    s.set_node_return_type(&node, "Arc<PgStore>").await.unwrap();
    assert_eq!(
        s.node_return_type(&node).await.unwrap().as_deref(),
        Some("Arc<PgStore>"),
        "a re-scan replaces the previous type"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// A function that returns NOTHING must leave no return type behind.
///
/// Absence already means "returns nothing / not extracted", so storing `()` or a
/// blank would add a second encoding of the same fact that no resolver can turn
/// into a type. The write REMOVES the key rather than skipping, because skipping
/// is the fabrication case: a function that used to return `-> PgStore` and now
/// returns unit would keep the previous scan's type, and the receiver chain
/// would go on resolving calls against a type the function no longer returns.
///
/// Breaking mutation: make the no-type branch return `Ok(())` without touching
/// the row — the stale `PgStore` survives the re-scan.
#[tokio::test]
async fn set_node_return_type_clears_the_key_when_the_function_returns_nothing() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("rettype_unit_{}", uuid::Uuid::new_v4())).await;
    let fqn = "rust·senseid·tasks::executor·TaskContext·reset";

    let node = s.seed_node_by_fqn(&fid, fqn, "method", "reset", Some("rust"), None).await.unwrap();
    s.set_node_props(&node, &serde_json::json!({"marker": "pre-existing"})).await.unwrap();
    s.set_node_return_type(&node, "&crate::db::pg_store::PgStore").await.unwrap();

    for gone in ["()", "", "   "] {
        s.set_node_return_type(&node, gone).await.unwrap();
        assert_eq!(
            s.node_return_type(&node).await.unwrap(),
            None,
            "{gone:?} names no type, so the node must carry no return type"
        );
        // Absent, not a stored jsonb null: `props->>'k'` reads both as NULL, so
        // only the key's existence distinguishes "no return type" from
        // "we looked and recorded nothing".
        let (present, marker): (bool, Option<String>) = query_as(
            "SELECT jsonb_exists(props, 'return_type'), props->>'marker' FROM sensei.nodes WHERE id=$1",
        )
        .bind(node)
        .fetch_one(s.pool())
        .await
        .unwrap();
        assert!(!present, "{gone:?} must remove the key, not store a null under it");
        assert_eq!(marker.as_deref(), Some("pre-existing"), "clearing must not drop other props");
    }

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

/// Writing a return type onto a node id that names no row is a lost write, and a
/// lost write on the emit path is indistinguishable from a function the parser
/// found no return type for. An UPDATE matching zero rows reports success, so the
/// row count has to be checked explicitly.
///
/// Breaking mutation: drop the `rows_affected` check — the call reports Ok and
/// the fact vanishes.
#[tokio::test]
async fn set_node_return_type_errors_when_the_node_does_not_exist() {
    let s = pg_store().await;
    let ghost = uuid::Uuid::new_v4();
    let err = s
        .set_node_return_type(&ghost, "PgStore")
        .await
        .expect_err("writing to a node that does not exist must not report success");
    assert!(err.contains(&ghost.to_string()), "the error names the node it could not write: {err}");
}

/// The fqn-shape adoption path (`adopt_node_by_identity`) is a SECOND write path
/// that fires on re-index whenever a file's fqn shape changes. It re-points an
/// existing row's fqn, and it must carry the return type across with everything
/// else — losing it there would be invisible in a fresh database and show up
/// only as receiver calls that resolve on the first index and stop resolving
/// after the second.
///
/// Breaking mutation: add `props = '{}'::jsonb` to that UPDATE's set-list.
#[tokio::test]
async fn node_return_type_survives_the_fqn_shape_adoption() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("rettype_adopt_{}", uuid::Uuid::new_v4())).await;
    let def = || FqnDef {
        file_path: "src/db/pg_store/graph.rs",
        signature: Some("    pub fn pool(&self) -> &PgPool {"),
        line_start: Some(7),
        line_end: Some(9),
        is_exported: true,
        parent_id: None,
    };

    let first = s
        .seed_node_by_fqn(
            &fid,
            "rust·senseid·db::pg_store·PgStore·pool",
            "method",
            "pool",
            Some("rust"),
            Some(def()),
        )
        .await
        .unwrap();
    s.set_node_return_type(&first, "&PgPool").await.unwrap();

    // Same structural identity, DIFFERENT fqn — the adoption path.
    let second = s
        .seed_node_by_fqn(
            &fid,
            "rust·senseid·db::pg_store::graph·PgStore·pool",
            "method",
            "pool",
            Some("rust"),
            Some(def()),
        )
        .await
        .expect("an fqn change on an existing identity must adopt the row, not fail");
    assert_eq!(first, second, "the adoption keeps the row id");
    assert_eq!(
        s.node_return_type(&second).await.unwrap().as_deref(),
        Some("&PgPool"),
        "adoption must carry the return type across with the rest of the row"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn lib_node_by_fqn() {
    // An external reference get-or-creates a first-class external node:
    // resolved=true (the external symbol IS its own definition — nothing to
    // enrich), NULL file_path (no local file), grouped by package in props.
    // Stable id across repeated references.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("lib_{}", uuid::Uuid::new_v4())).await;
    let fqn = "lib·serde_json·serde_json·from_str";

    let a =
        s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json", Some("rust")).await.unwrap();
    let (kind, resolved, fp, pkg): (String, bool, Option<String>, Option<String>) = query_as(
        "SELECT n.kind::text, n.resolved, np.file_path, n.props->>'package' FROM sensei.nodes n LEFT JOIN sensei.node_paths np ON np.node_id = n.id WHERE n.id=$1",
    )
    .bind(a)
    .fetch_one(s.pool())
    .await
    .unwrap();
    // D12: the kind says WHAT, and an import names a thing without saying what
    // KIND of thing — so `unknown`. The `lib·` fqn prefix is what says external.
    assert_eq!(kind, "unknown");
    assert!(resolved, "a lib symbol is its own definition — resolved");
    assert_eq!(fp, None, "a lib symbol has no local file");
    assert_eq!(pkg.as_deref(), Some("serde_json"), "grouped by package in props");

    // A second reference to the same external fqn shares the one node.
    let b =
        s.upsert_lib_node_by_fqn(&fid, fqn, "from_str", "serde_json", Some("rust")).await.unwrap();
    assert_eq!(a, b, "repeated external references share one lib node");
    let (n,): (i64,) = query_as(&format!(
        "SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 \
              AND {ext} AND kind <> 'package'::sensei.node_kind",
        ext = crate::languages::fqn::sql_is_external("fqn")
    ))
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(n, 1);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn graph_nodes_and_tree_expose_fqn_and_containers() {
    // Phase 7.2: the graph/nodes projection (get_nodes_scoped) carries `fqn` +
    // `resolved` so the Atlas can key symbols by moniker, and /tree (build_tree)
    // nests the Phase-5 type/module containers (file → type → method) via
    // parent_id. These are the two projections the retrieval endpoints delegate to.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("retr_{}", uuid::Uuid::new_v4())).await;

    // file → struct container → method(fqn), nested by parent_id (Phase 5 shape).
    let file_id = s
        .seed_node(&fid, "file", "lib.rs", "src/lib.rs", None, None, Some(1), Some(9))
        .await
        .unwrap();
    let type_id = s
        .seed_node(
            &fid,
            "struct",
            "Widget",
            "src/lib.rs",
            Some(&file_id),
            Some("struct Widget"),
            Some(2),
            Some(6),
        )
        .await
        .unwrap();
    let method_fqn = "rust·pkg·lib·Widget·render";
    let method_id = s
        .seed_node_by_fqn(
            &fid,
            method_fqn,
            "method",
            "render",
            Some("rust"),
            Some(super::FqnDef {
                file_path: "src/lib.rs",
                signature: Some("fn render(&self)"),
                line_start: Some(3),
                line_end: Some(5),
                is_exported: true,
                parent_id: Some(&type_id),
            }),
        )
        .await
        .unwrap();

    // ── graph/nodes exposes fqn + resolved ──
    let nodes = s.get_nodes_scoped(&[fid]).await.unwrap();
    let method = nodes.iter().find(|n| n["name"] == "render").expect("method node in projection");
    assert_eq!(crate::api::util::json_uuid(&method["id"]), Some(method_id), "same method node");
    assert_eq!(
        method["fqn"].as_str(),
        Some(method_fqn),
        "get_nodes_scoped workspaces the node's fqn"
    );
    assert_eq!(method["resolved"].as_bool(), Some(true), "and its resolved flag");
    assert_eq!(
        method["is_test"].as_bool(),
        Some(false),
        "and its is_test flag (default false here)"
    );

    // ── /tree nests the type container → method (Phase 5 parent_id) ──
    let folders = s.get_folders_scoped(&[fid]).await.unwrap();
    let tree = crate::api::handlers::codebase::build_tree_pub(&folders, &nodes);
    let files = tree["tree"][0]["nodes"].as_array().expect("folder root nodes");
    let file = files.iter().find(|n| n["name"] == "lib.rs").expect("file node under folder");
    let widget = file["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "Widget")
        .expect("type container nested under file");
    assert_eq!(widget["kind"], "struct", "the type container carries its kind");
    let render = widget["children"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["name"] == "render")
        .expect("method nested under the type container");
    assert_eq!(render["kind"], "method", "the method nests under the type container (Phase 5)");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn two_same_name_stubs_do_not_merge() {
    // Phase 3.0: with the partial identity index (`where file_path is not null`),
    // reference stubs (file_path NULL) are governed ONLY by nodes_unique_fqn.
    // Two references with the same simple name but DIFFERENT fqns must stay two
    // distinct nodes — the false-merge this rebuild exists to kill. (Under the
    // old non-partial identity constraint these would collide on
    // (folder, NULL, kind, name, NULL, NULL).)
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("stubmerge_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node_by_fqn(&fid, "rust·pkg·a·A·foo", "method", "foo", Some("rust"), None)
        .await
        .unwrap();
    let b = s
        .seed_node_by_fqn(&fid, "rust·pkg·b·B·foo", "method", "foo", Some("rust"), None)
        .await
        .unwrap();
    assert_ne!(a, b, "same simple name, different fqn → two distinct stub nodes");
    let (n,): (i64,) =
        query_as("SELECT count(*) FROM sensei.nodes WHERE folder_id=$1 AND resolved=false")
            .bind(fid)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(n, 2, "both stubs coexist under the fqn index");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn legacy_upsert_sets_language_from_extension() {
    // Every node written via the legacy upsert_node/_ex path (all non-Rust +
    // file/section/rationale nodes, for the whole FQN transition) must carry
    // `language` derived from its file extension — otherwise the same-language
    // bare-name fallback filter (plan 0.8) has nothing to match on. Compound
    // extensions resolve too (.svelte.ts → typescript).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("lang_{}", uuid::Uuid::new_v4())).await;
    let cases = [
        ("src/a.rs", "function", Some("fn f()"), "rust"),
        ("pkg/b.py", "function", Some("def g()"), "python"),
        ("app/c.svelte.ts", "function", None, "typescript"), // compound ext
        ("docs/e.md", "doc", None, "markdown"),
    ];
    for (path, kind, sig, want) in cases {
        let id = crate::tasks::test_support::seed_node(
            &s,
            &fid,
            kind,
            "n",
            path,
            None,
            sig,
            Some(1),
            Some(2),
        )
        .await
        .unwrap();
        let (lang,): (Option<String>,) = query_as("SELECT language FROM sensei.nodes WHERE id=$1")
            .bind(id)
            .fetch_one(s.pool())
            .await
            .unwrap();
        assert_eq!(lang.as_deref(), Some(want), "{path} → language {want}");
    }

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn node_locations_tolerates_stub_rows() {
    // file_id is nullable (reference stubs + external `lib·` nodes have none).
    // node_locations decodes file_path as a required String, so a stub id among
    // the requested ids must NOT error the whole fetch — the stub (no location)
    // is simply omitted while the real node still resolves.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("nodeloc_{}", uuid::Uuid::new_v4())).await;
    let real = s
        .seed_node(&fid, "function", "real", "a.rs", None, Some("fn real()"), Some(3), Some(9))
        .await
        .unwrap();
    let stub = s
        .seed_node_by_fqn(&fid, "rust·pkg·m·Missing·gone", "method", "gone", Some("rust"), None)
        .await
        .unwrap();

    let locs = s.node_locations(&[real, stub]).await.unwrap();
    assert_eq!(locs.len(), 1, "the stub (NULL file_path) is omitted, not an error");
    assert_eq!(locs[0].0, real, "the real node still resolves");
    assert_eq!(locs[0].2, "a.rs", "with its file_path");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn prune_file_nodes_deletes_vanished_and_unresolves_inbound() {
    // D3: a symbol that vanished from the parse is pruned; an inbound
    // cross-file edge to it is UNRESOLVED (target_id→NULL, target_name kept),
    // not cascade-deleted (invariant 3). A kept node is untouched.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("prune_{}", uuid::Uuid::new_v4())).await;
    let keep = s
        .seed_node(&fid, "function", "keep", "a.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    let gone = s
        .seed_node(&fid, "function", "gone", "a.rs", None, Some("()"), Some(6), Some(9))
        .await
        .unwrap();
    let caller = s
        .seed_node(&fid, "function", "caller", "b.rs", None, Some("()"), Some(1), Some(3))
        .await
        .unwrap();
    // A resolved inbound edge b.rs::caller → a.rs::gone, carrying target_name.
    let e = s
        .insert_edge(
            &fid,
            &caller,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("gone"), file: None },
            "calls",
        )
        .await
        .unwrap();
    s.resolve_edge(&e, &gone).await.unwrap();

    // Re-index of a.rs keeps only `keep`.
    let pruned = s.prune_file_nodes(&fid, "a.rs", &[keep]).await.unwrap();
    assert_eq!(pruned, 1, "the vanished `gone` node is pruned");

    let (keep_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
        .bind(keep)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(keep_exists, "the surviving node is untouched");
    let (gone_exists,): (bool,) = query_as("SELECT EXISTS(SELECT 1 FROM sensei.nodes WHERE id=$1)")
        .bind(gone)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert!(!gone_exists, "the vanished node is deleted");
    // The inbound edge survived, unresolved (target_id NULL, target_name kept).
    let (tid, tname): (Option<uuid::Uuid>, Option<String>) =
        query_as("SELECT target_id, target_name FROM sensei.edges WHERE id = $1")
            .bind(e)
            .fetch_one(s.pool())
            .await
            .unwrap();
    assert_eq!(tid, None, "inbound edge to the pruned node is unresolved, not deleted");
    assert_eq!(tname.as_deref(), Some("gone"), "target_name is kept for re-resolution");

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn delete_edges_from_sources_clears_a_files_out_edges() {
    // D3 per-file reconcile: a surviving symbol's stale out-edges are cleared
    // before re-inserting the current set (they don't cascade — the node
    // lives). Only edges FROM the given sources are removed.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("outedge_{}", uuid::Uuid::new_v4())).await;
    let a = s
        .seed_node(&fid, "function", "a", "a.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    let b = s
        .seed_node(&fid, "function", "b", "b.rs", None, Some("()"), Some(1), Some(5))
        .await
        .unwrap();
    s.insert_edge(
        &fid,
        &a,
        crate::db::pg_store::EdgeTarget::Unresolved { name: Some("x"), file: None },
        "calls",
    )
    .await
    .unwrap(); // a's out-edge
    s.insert_edge(
        &fid,
        &b,
        crate::db::pg_store::EdgeTarget::Unresolved { name: Some("y"), file: None },
        "calls",
    )
    .await
    .unwrap(); // b's out-edge (must survive)

    let n = s.delete_edges_from_sources(&fid, &[a]).await.unwrap();
    assert_eq!(n, 1, "only a's out-edge is deleted");
    assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 1, "b's out-edge survives");
    // Empty sources is a cheap no-op.
    assert_eq!(s.delete_edges_from_sources(&fid, &[]).await.unwrap(), 0);

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_swaps_the_full_set() {
    // D2: replace_edges_of_kind removes STALE edges of a kind and inserts the
    // current set atomically — the "replaced, not appended" guarantee that
    // makes a derived kind (covers) a pure function of the current tree.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replkind_{}", uuid::Uuid::new_v4())).await;
    let doc =
        crate::tasks::test_support::seed_node(&s, &fid, "doc", "d", "d.md", None, None, None, None)
            .await
            .unwrap();
    let f1 = crate::tasks::test_support::seed_node(
        &s, &fid, "file", "f1", "f1.rs", None, None, None, None,
    )
    .await
    .unwrap();
    let f2 = crate::tasks::test_support::seed_node(
        &s, &fid, "file", "f2", "f2.rs", None, None, None, None,
    )
    .await
    .unwrap();

    // A STALE covers edge doc→f1 (as if f1 was the covered file last scan).
    s.insert_edge(&fid, &doc, crate::db::pg_store::EdgeTarget::Node(&f1), "covers").await.unwrap();
    assert_eq!(s.get_edges_by_kind(&fid, "covers").await.unwrap().len(), 1);

    // Replace the covers set with {doc→f2}: the stale doc→f1 must vanish.
    s.replace_edges_of_kind(
        &fid,
        "covers",
        &[EdgeSpec { source_id: doc, target_id: Some(f2), target_name: None, target_file: None }],
    )
    .await
    .unwrap();

    let (tid,): (Option<uuid::Uuid>,) = query_as(
        "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().len(),
        1,
        "exactly the current set — stale edge removed, not appended"
    );
    assert_eq!(tid, Some(f2), "the surviving covers edge is the new target");

    // Replacing with an EMPTY set clears the kind for the folder.
    s.replace_edges_of_kind(&fid, "covers", &[]).await.unwrap();
    assert!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().is_empty(),
        "an empty set clears every edge of the kind"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_handles_unresolved_edges() {
    // The unresolved branch (target_id=None) — the path the per-file reconcile
    // (D3) will use. Replaces by (target_name, target_file).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replun_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    s.insert_edge(
        &fid,
        &a,
        crate::db::pg_store::EdgeTarget::Unresolved { name: Some("old"), file: None },
        "calls",
    )
    .await
    .unwrap(); // stale unresolved a→"old"

    s.replace_edges_of_kind(
        &fid,
        "calls",
        &[EdgeSpec {
            source_id: a,
            target_id: None,
            target_name: Some("new".into()),
            target_file: Some("x.rs".into()),
        }],
    )
    .await
    .unwrap();

    let (name, file): (Option<String>, Option<String>) = query_as(
            "SELECT target_name, target_file FROM sensei.edges WHERE folder_id=$1 AND kind='calls'::sensei.edge_kind")
            .bind(fid).fetch_one(s.pool()).await.unwrap();
    assert_eq!(
        (name.as_deref(), file.as_deref()),
        (Some("new"), Some("x.rs")),
        "unresolved edge replaced by (target_name, target_file)"
    );
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "stale unresolved edge removed"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn replace_edges_of_kind_is_atomic_and_rolls_back_on_failure() {
    // The "one transaction" guarantee: if an insert in the batch fails (a bad
    // source_id → FK violation), the whole replace rolls back — the OLD set is
    // intact, never half-deleted (no zero-covers window).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("replatomic_{}", uuid::Uuid::new_v4())).await;
    let doc =
        crate::tasks::test_support::seed_node(&s, &fid, "doc", "d", "d.md", None, None, None, None)
            .await
            .unwrap();
    let f1 = crate::tasks::test_support::seed_node(
        &s, &fid, "file", "f1", "f1.rs", None, None, None, None,
    )
    .await
    .unwrap();
    s.insert_edge(&fid, &doc, crate::db::pg_store::EdgeTarget::Node(&f1), "covers").await.unwrap();

    // A batch whose second edge has a bogus source_id (no such node) → the
    // FK on edges.source_id fails the insert mid-batch.
    let bogus = uuid::Uuid::new_v4();
    let res = s
        .replace_edges_of_kind(
            &fid,
            "covers",
            &[
                EdgeSpec {
                    source_id: doc,
                    target_id: Some(f1),
                    target_name: None,
                    target_file: None,
                },
                EdgeSpec {
                    source_id: bogus,
                    target_id: Some(f1),
                    target_name: None,
                    target_file: None,
                },
            ],
        )
        .await;
    assert!(res.is_err(), "a bad edge fails the replace");

    assert_eq!(
        s.get_edges_by_kind(&fid, "covers").await.unwrap().len(),
        1,
        "the DELETE rolled back with the failed insert — old set intact, not half-deleted"
    );
    let (tid,): (Option<uuid::Uuid>,) = query_as(
        "SELECT target_id FROM sensei.edges WHERE folder_id=$1 AND kind='covers'::sensei.edge_kind",
    )
    .bind(fid)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert_eq!(tid, Some(f1), "the surviving edge is the original (rollback)");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn insert_edge_unresolved_dedups_by_target_file() {
    // D1: the unresolved identity is (folder, source, target_name, target_file,
    // kind). Same target_name in DIFFERENT files are distinct edges; same
    // name + same file (incl. nulls-not-distinct) upserts to one row. This is
    // the whole point of the target_file column — a same-named symbol in two
    // files must not collapse to one edge.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("edgetf_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();

    let e1 = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved {
                name: Some("helper"),
                file: Some("x.rs"),
            },
            "calls",
        )
        .await
        .unwrap();
    let e2 = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved {
                name: Some("helper"),
                file: Some("y.rs"),
            },
            "calls",
        )
        .await
        .unwrap();
    assert_ne!(e1, e2, "same target_name in different files are distinct unresolved edges");
    assert_eq!(s.get_edges_by_kind(&fid, "calls").await.unwrap().len(), 2, "two distinct rows");

    let e3 = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved {
                name: Some("helper"),
                file: Some("x.rs"),
            },
            "calls",
        )
        .await
        .unwrap();
    assert_eq!(e1, e3, "same (target_name, target_file) upserts to the same row");
    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        2,
        "no new row on re-insert"
    );

    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_second_call_is_safe() {
    // resolve_edge is idempotent: resolving the same edge to the same target
    // twice must be a safe no-op (one edge), not a unique-violation throw.
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolve2x_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "b",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let u = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("b"), file: None },
            "calls",
        )
        .await
        .unwrap();

    s.resolve_edge(&u, &b).await.unwrap();
    s.resolve_edge(&u, &b).await.unwrap(); // second call — must not throw

    assert_eq!(
        s.get_edges_by_kind(&fid, "calls").await.unwrap().len(),
        1,
        "resolving twice keeps exactly one edge"
    );
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

#[tokio::test]
async fn resolve_edge_updates_in_place_when_no_conflict() {
    // The common case: no existing resolved dup → the unresolved edge is
    // updated in place to the resolved target (not deleted).
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("resolveok_{}", uuid::Uuid::new_v4())).await;
    let a = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "a",
        "a.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let b = crate::tasks::test_support::seed_node(
        &s,
        &fid,
        "function",
        "b",
        "b.rs",
        None,
        None,
        Some(1),
        Some(5),
    )
    .await
    .unwrap();
    let u = s
        .insert_edge(
            &fid,
            &a,
            crate::db::pg_store::EdgeTarget::Unresolved { name: Some("b"), file: None },
            "calls",
        )
        .await
        .unwrap();

    s.resolve_edge(&u, &b).await.unwrap();

    let (tid,): (Option<uuid::Uuid>,) = query_as("SELECT target_id FROM sensei.edges WHERE id=$1")
        .bind(u)
        .fetch_one(s.pool())
        .await
        .unwrap();
    assert_eq!(tid, Some(b), "the edge is resolved in place to the target");
    s.delete_nodes_by_folder(&fid).await.unwrap();
}

// ── Communities tests ────────────────────────────────────────────

#[tokio::test]
async fn community_upsert_and_list() {
    let s = pg_store().await;
    let fid = create_test_folder(&s, &format!("comm_{}", uuid::Uuid::new_v4())).await;
    let cid = s.upsert_community(&fid, 1, "_test:auth_cluster", 3).await.unwrap();
    let comms = s.list_communities(&fid).await.unwrap();
    assert!(comms.iter().any(|c| c["label"] == "_test:auth_cluster" && c["node_count"] == 3));
    sqlx_core::query::query("DELETE FROM inference.communities WHERE id = $1")
        .bind(cid)
        .execute(s.pool())
        .await
        .unwrap();
}
