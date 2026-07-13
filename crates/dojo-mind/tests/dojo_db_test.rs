//! Verifies the service boots the unified `dojo` scope in its embedded PG: BOTH
//! the artifact tables (with the `dojo.artifacts.seq` cursor) AND the
//! governance-federation tables land, and the global-dojo tenant is seeded
//! idempotently.

use dojo_mind::db::DojoDb;

#[tokio::test]
async fn bootstrap_deploys_unified_dojo_scope_and_seeds_global_dojo() {
    let db = DojoDb::bootstrap_temp().await.expect("bootstrap embedded dojo");
    let pool = db.pool();

    // dojo.* tables landed.
    for tbl in [
        "dojo.tenants",
        "dojo.memberships",
        "dojo.identities",
        "dojo.artifacts",
    ] {
        let (exists,): (bool,) = sqlx_core::query_as::query_as(&format!(
            "SELECT to_regclass('{tbl}') IS NOT NULL"
        ))
        .fetch_one(pool)
        .await
        .unwrap();
        assert!(exists, "{tbl} should exist in the dojo scope");
    }

    // The seq cursor column + its sequence exist on dojo.artifacts.
    let (has_seq,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.columns
         WHERE table_schema='dojo' AND table_name='artifacts' AND column_name='seq')",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(has_seq, "dojo.artifacts.seq column must exist");

    let (has_sequence,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('dojo.artifacts_seq') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(has_sequence, "dojo.artifacts_seq sequence must exist");

    // The global-dojo tenant is seeded (and the seed is idempotent).
    let (globals,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM dojo.tenants WHERE key = 'org/global-dojo' AND scope = 'global'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(globals, 1, "the global-dojo tenant must be seeded exactly once");

    // The governance-federation tables (shared_rules/members/api_keys/audit_log)
    // land in the same unified `dojo` scope.
    let (rules_ok,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('dojo.shared_rules') IS NOT NULL",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(rules_ok, "the governance-federation tables must deploy in the dojo scope");
}
