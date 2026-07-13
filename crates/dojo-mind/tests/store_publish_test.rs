use dojo_mind::db::DojoDb;
use dojo_mind::store::DojoStore;
use dojo_protocol::{content_hash, PublishedRule};

fn rule(content: &str, title: &str) -> PublishedRule {
    PublishedRule {
        content_hash: content_hash(content),
        scope_key: "organization".into(), namespace_slug: "sensei-hq".into(), namespace_name: "Sensei HQ".into(),
        rule_type: "convention".into(), title: title.into(), content: content.into(),
        impact: None, enforcement: "mandatory".into(),
        origin_repo: Some("sensei/daemon".into()), published_by: "jerry".into(), published_at: "2026-06-11T00:00:00Z".into(),
    }
}

#[tokio::test]
async fn publish_creates_then_republish_bumps_version_and_seq() {
    let db = DojoDb::bootstrap_temp().await.unwrap();
    let store = DojoStore::new(db.pool().clone());
    let r1 = store.publish(&rule("always use tdd", "TDD")).await.unwrap();
    assert_eq!(r1.version, 1);
    let mut again = rule("always use tdd", "TDD (revised)");
    again.content_hash = content_hash("always use tdd");
    let r2 = store.publish(&again).await.unwrap();
    assert_eq!(r2.id, r1.id, "same (namespace, content_hash) → same row");
    assert_eq!(r2.version, 2);
    assert!(r2.seq > r1.seq, "seq must advance on republish");
    let r3 = store.publish(&rule("prefer pure functions", "Purity")).await.unwrap();
    assert_ne!(r3.id, r1.id);
    assert!(r3.seq > r2.seq);
}
