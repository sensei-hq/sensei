use super::*;

#[tokio::test]
async fn connect_to_pg() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (i32,) = query_as("SELECT 1").fetch_one(store.pool()).await.unwrap();
    assert_eq!(row.0, 1);
}

#[tokio::test]
async fn execute_raw_works() {
    let store = PgStore::connect_test().await.unwrap();
    store.execute_raw("SELECT 1").await.unwrap();
}

#[tokio::test]
async fn schema_exists() {
    let store = PgStore::connect_test().await.unwrap();
    let row: (bool,) = query_as(
        "SELECT EXISTS(SELECT 1 FROM information_schema.schemata WHERE schema_name = 'sensei')",
    )
    .fetch_one(store.pool())
    .await
    .unwrap();
    assert!(row.0, "sensei schema must exist — run `dbd apply` first");
}

// ── Config tests ───────────────────────────────────────────────

#[tokio::test]
async fn config_set_and_get() {
    let s = pg_store().await;
    let k = tkey("set_get", "theme");
    s.set_config(&k, "dark").await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), Some("dark".into()));
    s.delete_config(&k).await.unwrap(); // cleanup
}

#[tokio::test]
async fn config_get_missing_returns_none() {
    let s = pg_store().await;
    assert_eq!(s.get_config("_test:missing:nonexistent").await.unwrap(), None);
}

#[tokio::test]
async fn config_set_overwrites() {
    let s = pg_store().await;
    let k = tkey("overwrite", "k");
    s.set_config(&k, "v1").await.unwrap();
    s.set_config(&k, "v2").await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), Some("v2".into()));
    s.delete_config(&k).await.unwrap();
}

#[tokio::test]
async fn config_delete() {
    let s = pg_store().await;
    let k = tkey("delete", "k");
    s.set_config(&k, "v").await.unwrap();
    s.delete_config(&k).await.unwrap();
    assert_eq!(s.get_config(&k).await.unwrap(), None);
}

#[tokio::test]
async fn config_delete_nonexistent_is_noop() {
    let s = pg_store().await;
    s.delete_config("_test:noop:nope").await.unwrap();
}

#[tokio::test]
async fn config_get_all() {
    let s = pg_store().await;
    let k1 = tkey("getall", "a");
    let k2 = tkey("getall", "b");
    s.set_config(&k1, "1").await.unwrap();
    s.set_config(&k2, "2").await.unwrap();
    let all = s.get_all_config().await.unwrap();
    assert_eq!(all[&k1], "1");
    assert_eq!(all[&k2], "2");
    s.delete_config(&k1).await.unwrap();
    s.delete_config(&k2).await.unwrap();
}
