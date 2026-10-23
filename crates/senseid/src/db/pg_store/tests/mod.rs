use super::graph_seed::SeedGraph;
use super::*;
use crate::languages::fqn::ReceiverHint;
use crate::tasks::test_support::{SCHEDULE_EDIT_GATE, TEST_SCHEDULE_PREFIX, test_schedule_name};
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
mod workspaces_tests;

// Fixtures one group defines and others reuse.
use dojo_tests::{seed_sync_fixture, seed_sync_fixture_at};
use libraries_tests::{REPAIR_SWEEP_GATE, clear_test_session, session_row_folder};
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
