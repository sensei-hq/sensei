use dojo_mind::db::DojoDb;

#[tokio::test]
async fn bootstrap_creates_dojo_schema_seeds_scopes_and_excludes_daemon_tables() {
    let db = DojoDb::bootstrap_temp().await.expect("bootstrap embedded dojo");
    let pool = db.pool();

    let (rules_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('dojo.shared_rules') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(rules_exists, "dojo.shared_rules should exist");

    let (n,): (i64,) = sqlx_core::query_as::query_as("SELECT count(*) FROM sensei.scopes")
        .fetch_one(pool).await.unwrap();
    assert_eq!(n, 8, "scopes ladder should be seeded");

    let (memories_exists,): (bool,) = sqlx_core::query_as::query_as(
        "SELECT to_regclass('sensei.memories') IS NOT NULL")
        .fetch_one(pool).await.unwrap();
    assert!(!memories_exists, "daemon-only sensei.memories must NOT exist in the dojo DB");
}
