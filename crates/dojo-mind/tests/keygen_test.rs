use dojo_mind::db::DojoDb;
use dojo_mind::keygen::generate_key;
use dojo_mind::store::DojoStore;

#[tokio::test]
async fn keygen_creates_resolvable_admin_key() {
    let db = DojoDb::bootstrap_temp().await.unwrap();
    let store = DojoStore::new(db.pool().clone());
    let key = generate_key(&store, "bootstrap admin", "admin", Some("initial")).await.unwrap();
    let caller = store.find_member_by_key(&key).await.unwrap().unwrap();
    assert_eq!(caller.role, "admin");
}
