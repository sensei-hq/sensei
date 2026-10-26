use super::graph_seed::SeedGraph;
use super::*;
use crate::languages::fqn::ReceiverHint;
use crate::tasks::test_support::{TEST_SCHEDULE_PREFIX, test_schedule_name};
use sqlx_core::query_as::query_as;

// Every test here connects via `PgStore::connect_test()` — the tiny floorless
// pool. Do NOT reach for `PgStore::connect()`: that is the daemon's pool with a
// warm floor of `DB_POOL_MIN_CONNECTIONS`, and cargo runs these tests in
// parallel, so each one would hold 8 connections open and the suite would blow
// past Postgres's `max_connections` (it did: 80 of 223 tests failed to connect
// at default parallelism). `connect_test` also owns the URL default, including
// the never-point-at-`sensei` guard.

mod activity_tests;
mod api_cost_tests;
mod config_tests;
mod dojo_tests;
mod gateway_tests;
mod graph_tests;
mod graph_views_tests;
mod libraries_tests;
mod memories_tests;
mod metric_rows_tests;
mod misc_tests;
mod patterns_tests;
mod personas_tests;
mod receivers_tests;
mod repo_anchor_tests;
mod scan_tests;
mod stub_gc_tests;
mod tasks_tests;
mod turn_usage_tests;
mod typed_rows_tests;
mod workspaces_tests;

// Fixtures one group defines and others reuse.
use dojo_tests::{seed_sync_fixture, seed_sync_fixture_at};
use libraries_tests::{clear_test_session, session_row_folder};
use metric_rows_tests::seed_metric;

async fn pg_store() -> PgStore {
    PgStore::connect_test().await.unwrap()
}

/// Generate a unique key prefix for test isolation.
fn tkey(test: &str, key: &str) -> String {
    format!("_test:{}:{}", test, key)
}

/// Create a unique test folder for FK tests. Uses suffix for isolation.
///
/// CARRIES A REPOSITORY, because every folder does. Since da504332 a folder
/// inherits `repository_id` from its anchor at write time — measured 13,722 of
/// 13,722 on the live DB — so a folder without one is a state production can no
/// longer produce. Modelling it here is not a harmless simplification: it is
/// what let `version_conflicts_view_flags_multi_version_pins_and_excludes_local`
/// keep asserting on a view that, resolved through `folder_workspaces`, could
/// never have matched its fixture.
///
/// The repository is keyed on `suffix`, so the same suffix returns the same
/// repository exactly as it returns the same folder.
pub(crate) async fn create_test_folder(s: &PgStore, suffix: &str) -> uuid::Uuid {
    use sqlx_core::query_as::query_as;
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let abs_path = format!("/_test/{}", suffix);
    let (rid,): (uuid::Uuid,) = query_as(
        "INSERT INTO sensei.repositories(repo_key, name) VALUES($1, $2) \
         ON CONFLICT(repo_key) DO UPDATE SET name = EXCLUDED.name RETURNING id",
    )
    .bind(format!("test/{suffix}"))
    .bind(suffix)
    .fetch_one(s.pool())
    .await
    .unwrap();
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path, repository_id) VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2, $3) ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name, repository_id = EXCLUDED.repository_id RETURNING id"
        ).bind(suffix).bind(&abs_path).bind(rid).fetch_one(s.pool()).await.unwrap();
    row.0
}

/// A folder with NO repository — the state [`create_test_folder`] deliberately
/// cannot produce.
///
/// `folders.repository_id` is NULLABLE and the production writer guards for it
/// (`WHERE f.repository_id IS NOT NULL`): a folder whose anchor has no
/// repository inherits none. So the unattributed bucket stays representable
/// even though it is currently empty on the live DB, and the views' honest-NULL
/// behaviour over it is a real property that needs a fixture to assert on.
///
/// Use this ONLY to test that behaviour. Everything else wants the default,
/// which carries a repository exactly as every live folder does.
pub(crate) async fn create_test_folder_unattributed(s: &PgStore, suffix: &str) -> uuid::Uuid {
    use sqlx_core::query_as::query_as;
    s.execute_raw(
            "INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001', '/_test', '_test', 'watching'::sensei.watch_status) ON CONFLICT DO NOTHING"
        ).await.unwrap();
    let abs_path = format!("/_test/{}", suffix);
    let row: (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.folders(root_id, kind, name, path, abs_path) VALUES('00000000-0000-0000-0000-000000000001', 'git'::sensei.folder_kind, $1, $1, $2) ON CONFLICT(abs_path) DO UPDATE SET name = EXCLUDED.name, repository_id = NULL RETURNING id"
        ).bind(suffix).bind(&abs_path).fetch_one(s.pool()).await.unwrap();
    row.0
}

/// Create a unique (workspace, folder) pair for FK tests that need both,
/// wiring the folder to the workspace. Used by the pattern tests since
/// detected_patterns is workspace-scoped (#82) and needs a non-null
/// workspace_id, while `list_patterns_by_folder` still keys on folder.
///
/// Goes through `set_folder_workspace`, the production API, so the membership
/// reaches `repositories_in_workspaces` as well as `folders.workspace_id`. A raw
/// `UPDATE … SET workspace_id` writes only the half that `folder_workspaces` does
/// not read.
async fn create_test_workspace_and_folder(s: &PgStore, suffix: &str) -> (uuid::Uuid, uuid::Uuid) {
    let pid = s.create_workspace(&format!("_test:{}", suffix), None, None).await.unwrap();
    let fid = create_test_folder(s, suffix).await;
    s.set_folder_workspace(&fid, &pid, "root", None).await.unwrap();
    (pid, fid)
}

/// #204: a test that needs `sensei_test` and cannot connect FAILS; it never
/// passes by returning early.
///
/// 198 tests opened with `let Ok(pg) = PgStore::connect_test().await else {
/// return };`. Every place these tests run has the database (rust.yml
/// provisions it; nothing else runs them), so the early return only ever hid a
/// failure: when the suite exhausts Postgres's connection budget — the
/// subject of #204 — those tests report a pass having run nothing.
#[test]
fn no_test_passes_by_skipping_a_failed_test_db_connect() {
    fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                rust_files(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    // Spelled at runtime so this test's own text never matches itself.
    let opener = format!("{}::connect_test().await else", "PgStore");
    let mut skips = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        for (i, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("let Ok(") && line.contains(&opener) {
                skips.push(format!("{}:{}", f.strip_prefix(&src).unwrap().display(), i + 1));
            }
        }
    }
    assert!(skips.is_empty(), "{} tests skip on a failed connect: {skips:#?}", skips.len());
}

/// #204: the suite cannot exhaust Postgres's connection budget, whatever the
/// machine's core count.
///
/// `cargo test` runs one thread per core by default and each test's pool holds
/// up to `TEST_POOL_MAX_CONNECTIONS`; with the daemon's pool on top, a big
/// enough machine overran `max_connections` and the suite hung or — before
/// `test_store` — passed tests that never ran. `.cargo/config.toml` caps the
/// threads; this checks the cap against the server the tests actually use,
/// leaving room for psql and other admin sessions.
#[tokio::test]
async fn the_test_suite_fits_postgres_connection_budget() {
    let config = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.cargo/config.toml"),
    )
    .expect(".cargo/config.toml");
    let threads: u32 = config
        .lines()
        .find(|l| l.trim_start().starts_with("RUST_TEST_THREADS"))
        .and_then(|l| l.split('"').nth(1))
        .and_then(|v| v.parse().ok())
        .expect(".cargo/config.toml must cap RUST_TEST_THREADS");

    let pg = PgStore::test_store().await;
    let (max,): (String,) =
        sqlx_core::query_as::query_as("SHOW max_connections").fetch_one(pg.pool()).await.unwrap();
    let max: u32 = max.parse().unwrap();

    const ADMIN_HEADROOM: u32 = 10;
    let worst =
        threads * super::TEST_POOL_MAX_CONNECTIONS + sensei_bootstrap::DB_POOL_MAX_CONNECTIONS;
    assert!(
        worst + ADMIN_HEADROOM <= max,
        "{threads} threads × {} + daemon {} = {worst}, + {ADMIN_HEADROOM} headroom > max_connections {max}",
        super::TEST_POOL_MAX_CONNECTIONS,
        sensei_bootstrap::DB_POOL_MAX_CONNECTIONS
    );
}

/// #183: isolation BY CONSTRUCTION — a test's writes are invisible to every
/// other test.
///
/// Every DB test shared one `sensei_test`, so a database-wide operation in one
/// test (a heal, a prune, a rescan sweep) destroyed a sibling's fixture
/// mid-flight. Five gates were added after five confusing failures, each by
/// convention. Each test now gets its own database, cloned from `sensei_test`.
///
/// Libtest runs each test on its own thread, so "another test" is modelled as
/// another thread: a row written there must not be visible here.
#[tokio::test]
async fn a_tests_rows_are_invisible_to_every_other_test() {
    let here = PgStore::test_store().await;
    let marker = format!("/_test/isolation/{}", uuid::Uuid::new_v4());
    let elsewhere = marker.clone();
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(
            async move {
                let other = PgStore::test_store().await;
                other
                    .add_watch_root(&elsewhere, "isolation", &serde_json::json!([]))
                    .await
                    .unwrap();
            },
        )
    })
    .join()
    .unwrap();

    let roots = here.list_watch_roots().await.unwrap();
    assert!(
        roots.iter().all(|r| r["path"] != marker.as_str()),
        "another test's row is visible here: the tests share a database"
    );

    // And this test's own connections all see ONE database: a second
    // connect within the same test is the same database, not a fresh clone.
    let again = PgStore::test_store().await;
    here.add_watch_root(&marker, "isolation-here", &serde_json::json!([])).await.unwrap();
    let seen = again.list_watch_roots().await.unwrap();
    assert!(seen.iter().any(|r| r["path"] == marker.as_str()), "one test, one database");
}
