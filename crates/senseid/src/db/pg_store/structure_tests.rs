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
