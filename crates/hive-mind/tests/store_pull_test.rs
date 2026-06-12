use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use hive_protocol::{content_hash, PublishedRule};

fn rule(content: &str) -> PublishedRule {
    PublishedRule { content_hash: content_hash(content), scope_key: "organization".into(),
        namespace_slug: "sensei-hq".into(), namespace_name: "Sensei HQ".into(), rule_type: "convention".into(),
        title: "t".into(), content: content.into(), impact: None, enforcement: "recommended".into(),
        origin_repo: None, published_by: "jerry".into(), published_at: "2026-06-11T00:00:00Z".into() }
}

#[tokio::test]
async fn pull_since_returns_deltas_and_tombstones() {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    let a = store.publish(&rule("rule a")).await.unwrap();
    let _b = store.publish(&rule("rule b")).await.unwrap();
    let page = store.pull_since(0).await.unwrap();
    assert_eq!(page.rules.len(), 2);
    assert!(page.cursor >= 2);
    assert_eq!(page.rules[0].rule.namespace_slug, "sensei-hq");
    assert_eq!(page.rules[0].rule.scope_key, "organization");
    let empty = store.pull_since(page.cursor).await.unwrap();
    assert_eq!(empty.rules.len(), 0);
    store.retract(&a.id).await.unwrap();
    let after = store.pull_since(page.cursor).await.unwrap();
    assert_eq!(after.rules.len(), 1);
    assert_eq!(after.rules[0].status, "tombstoned");
    assert_eq!(after.rules[0].id, a.id);
}
