use hive_mind::db::HiveDb;
use hive_mind::keygen::generate_key;
use hive_mind::store::HiveStore;

#[tokio::test]
async fn keygen_creates_resolvable_admin_key() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    let key = generate_key(&store, "bootstrap admin", "admin", Some("initial")).await.unwrap();
    let caller = store.find_member_by_key(&key).await.unwrap().unwrap();
    assert_eq!(caller.role, "admin");
}
