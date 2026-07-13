use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;

#[tokio::test]
async fn member_key_issue_lookup_and_revoke() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    let member = store.create_member("Jerry", Some("jerry@x.io"), "publisher").await.unwrap();
    let issued = store.issue_key(&member, Some("laptop")).await.unwrap();
    assert!(issued.plaintext.len() >= 40);
    let who = store.find_member_by_key(&issued.plaintext).await.unwrap();
    assert!(who.is_some());
    let who = who.unwrap();
    assert_eq!(who.role, "publisher");
    assert_eq!(who.member_id, member);
    assert!(store.find_member_by_key("not-a-real-key").await.unwrap().is_none());
    store.revoke_key(&issued.key_id).await.unwrap();
    assert!(store.find_member_by_key(&issued.plaintext).await.unwrap().is_none());
}
