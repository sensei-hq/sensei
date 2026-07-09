//! Restart-persistence: a NON-temporary embedded cluster must reopen across
//! `HiveDb` lifecycles. Before the pinned superuser password, the second
//! bootstrap over the same data dir failed with "password authentication
//! failed" because `Settings::default()` randomised the password per process.

use std::path::PathBuf;

use hive_mind::db::HiveDb;

fn workspace_database_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database")
}

/// A unique, self-cleaning data dir (persistent — bootstrap is non-temporary,
/// so nothing wipes it; the test removes it at the end).
fn unique_data_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("sensei-hive-persist-{}-{n}", std::process::id()))
}

#[tokio::test(flavor = "multi_thread")]
async fn a_persisted_cluster_reopens_after_the_db_is_dropped() {
    let data_dir = unique_data_dir();
    let db_dir = workspace_database_dir();

    // First boot: fresh initdb (with the pinned password), deploy, then insert
    // a row so we can prove the data survives — not just that connect works.
    {
        let db = HiveDb::bootstrap(data_dir.clone(), db_dir.clone())
            .await
            .expect("first bootstrap should initialise the cluster");
        sqlx_core::query::query(
            "INSERT INTO dojo.tenants (key, origin, org, scope, name, dojo_url) \
             VALUES ('github/persist-probe', 'github', 'persist-probe', 'private', 'probe', \
             'http://localhost:7755/github/persist-probe')",
        )
        .execute(db.pool())
        .await
        .expect("insert a probe tenant");
    } // db dropped here → the embedded server stops, but temporary=false keeps the dir

    // Second boot over the SAME data dir: this is the regression — it must
    // REOPEN (pinned password matches what initdb baked in), and the earlier
    // row must still be there.
    let db2 = HiveDb::bootstrap(data_dir.clone(), db_dir.clone())
        .await
        .expect("second bootstrap must reopen the persisted cluster (pinned password)");
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*)::bigint FROM dojo.tenants WHERE key = 'github/persist-probe'",
    )
    .fetch_one(db2.pool())
    .await
    .expect("probe query");
    assert_eq!(n, 1, "the probe tenant must survive a restart");

    drop(db2);
    let _ = std::fs::remove_dir_all(&data_dir);
}
