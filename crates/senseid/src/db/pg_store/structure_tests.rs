//! What a PACKAGE-ROOT symbol's module is (#231).
//!
//! Every structure view decomposes an fqn positionally and reads the third
//! segment as the module. That segment used to be dropped when the symbol sat
//! at a package root, which slid the symbol's NAME into the module's place —
//! `c·senseid·ACCEPT_INPUT·item`, a C macro read as a module. Measured
//! 2026-10-05: 13.7% of project `sensei`'s module dependencies named an endpoint
//! absent from its own node universe, and 78% on the largest client project.
//!
//! `indexer::fqn` now occupies the slot, so the third segment is the module
//! whether or not there is one. That leaves exactly one question for SQL, and it
//! is this file's subject: what module does a symbol with no module belong to?
use super::*;

/// A PACKAGE-ROOT SYMBOL BELONGS TO ITS PACKAGE, and that is a module identity
/// rather than a hole.
///
/// Three answers were available and two are wrong. `NULL` drops the symbol from
/// every structure diagram, which is how a C header's whole contents would
/// vanish rather than appear at the root. `pkg || '/'` is a distinct identity
/// per package but reads as a truncation and sorts oddly beside `senseid/tasks`.
/// The package's own name is the thing a reader would say out loud: everything
/// at the root of `senseid` is in `senseid`.
///
/// NULL IN, NULL OUT survives, and it is a DIFFERENT case: a node carrying no
/// fqn at all yields NULL for both arguments, and a symbol with no package has
/// no module identity rather than a plausible-looking one rooted at the empty
/// string. An empty module is a fact; a missing package is an absence.
///
/// Mutation that must break this test: drop the empty-module arm so the root
/// case falls through to `pkg || '/' || ''`, or make it return NULL.
#[tokio::test]
async fn a_package_root_symbol_belongs_to_its_package() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    async fn answer(pg: &PgStore, pkg: Option<&str>, module: Option<&str>) -> Option<String> {
        let (m,): (Option<String>,) =
            sqlx_core::query_as::query_as("SELECT sensei.module_of($1, $2)")
                .bind(pkg)
                .bind(module)
                .fetch_one(pg.pool())
                .await
                .unwrap();
        m
    }

    assert_eq!(
        answer(&pg, Some("senseid"), Some("")).await,
        Some("senseid".to_string()),
        "a symbol at the package root is in the package, not in a module called nothing"
    );
    // The ordinary case is untouched: the FIRST segment, both separators.
    assert_eq!(
        answer(&pg, Some("senseid"), Some("tasks::handlers")).await,
        Some("senseid/tasks".into())
    );
    assert_eq!(answer(&pg, Some("app"), Some("lib/triage")).await, Some("app/lib".into()));
    // ...and the absence stays an absence.
    assert_eq!(answer(&pg, None, Some("tasks")).await, None, "no package is no module identity");
    assert_eq!(
        answer(&pg, Some("senseid"), None).await,
        None,
        "no fqn at all is no module identity"
    );
}

/// A PROJECT WITH NOTHING INDEXED HAS A VERSION, AND IT IS `None` (#233).
///
/// Not an error and not a substituted "now". Every diagram payload is cached
/// against this value, so an `Err` here would 500 the screen for a project that
/// is merely empty, and a fabricated timestamp would make two empty projects
/// look like they had changed between one request and the next — a cache that
/// never hits for exactly the cheapest payload.
///
/// The second half is the one with teeth: the version MOVES when a file is
/// indexed. A version that stood still would serve a stale diagram forever.
///
/// Mutation that must break this test: read `max(folders.modified_at)` instead,
/// or `coalesce(max(fi.indexed_at), now())`.
#[tokio::test]
async fn a_projects_graph_version_is_none_until_a_file_is_indexed() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let tag = uuid::Uuid::new_v4();
    let root = format!("/_test/version/{tag}");
    let root_id = pg.add_watch_root(&root, "ver-wt", &serde_json::json!([])).await.unwrap();
    let folder_id = pg.upsert_repo(&root_id, "ver", &root).await.unwrap();
    let project_id = pg.create_project(&format!("_test:version:{tag}"), None, None).await.unwrap();
    crate::tasks::test_support::place_folder_in_project(
        &pg,
        &folder_id,
        &project_id,
        &format!("ver-{tag}"),
    )
    .await
    .unwrap();

    assert_eq!(
        pg.project_graph_version(&project_id).await.unwrap(),
        None,
        "a project whose folders hold no indexed file has no graph yet"
    );

    pg.upsert_file_row(&folder_id, "a.rs", 1, "seed", None).await.unwrap();
    let first = pg.project_graph_version(&project_id).await.unwrap();
    assert!(first.is_some(), "an indexed file gives the project a version");

    // Re-indexing that same file MOVES it — which is the whole contract. A
    // version that only appeared once would pin the first answer forever.
    sqlx_core::query::query("UPDATE sensei.files SET indexed_at = now() WHERE folder_id = $1")
        .bind(folder_id)
        .execute(pg.pool())
        .await
        .unwrap();
    let second = pg.project_graph_version(&project_id).await.unwrap();
    assert!(second > first, "re-indexing moves the version: {first:?} -> {second:?}");

    crate::tasks::test_support::cleanup_metrics_fixture(&pg, &project_id, Some(&folder_id), &[])
        .await;
}
