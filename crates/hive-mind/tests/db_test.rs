use hive_mind::db::HiveDb;

#[tokio::test]
async fn bootstrap_creates_hive_schema_seeds_scopes_and_excludes_daemon_tables() {
    let db = HiveDb::bootstrap_temp().await.expect("bootstrap embedded hive");
    let pool = db.pool();

    let (rules_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('hive.shared_rules') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(rules_exists, "hive.shared_rules should exist");

    let (n,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.scopes")
        .fetch_one(pool).await.unwrap();
    assert_eq!(n, 8, "scopes ladder should be seeded");

    let (memories_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('sensei.memories') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(!memories_exists, "daemon-only sensei.memories must NOT exist in the hive DB");
}
