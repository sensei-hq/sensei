use hive_mind::api::{build_router, SharedState};
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt; // oneshot

async fn app_with_keys() -> (axum::Router, String, String) {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    let pub_member = store.create_member("Pub", None, "publisher").await.unwrap();
    let mem_member = store.create_member("Mem", None, "member").await.unwrap();
    let pub_key = store.issue_key(&pub_member, None).await.unwrap().plaintext;
    let mem_key = store.issue_key(&mem_member, None).await.unwrap().plaintext;
    Box::leak(Box::new(db)); // keep embedded PG alive for the test
    let router = build_router(Arc::new(SharedState { store }));
    (router, pub_key, mem_key)
}

fn publish_body() -> String {
    serde_json::json!({
        "content_hash": hive_protocol::content_hash("always tdd"),
        "scope_key": "organization", "namespace_slug": "sensei-hq", "namespace_name": "Sensei HQ",
        "rule_type": "convention", "title": "TDD", "content": "always tdd",
        "impact": null, "enforcement": "mandatory",
        "origin_repo": "sensei/daemon", "published_by": "jerry", "published_at": "2026-06-11T00:00:00Z"
    }).to_string()
}

#[tokio::test]
async fn health_is_unauthenticated() {
    let (app, _, _) = app_with_keys().await;
    let res = app.oneshot(Request::get("/v1/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn publish_requires_publisher_then_pull_returns_it() {
    let (app, pub_key, mem_key) = app_with_keys().await;
    let res = app.clone().oneshot(Request::post("/v1/rules")
        .header("authorization", format!("Bearer {mem_key}")).header("content-type","application/json")
        .body(Body::from(publish_body())).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::FORBIDDEN);
    let res = app.clone().oneshot(Request::post("/v1/rules")
        .header("authorization", format!("Bearer {pub_key}")).header("content-type","application/json")
        .body(Body::from(publish_body())).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let res = app.oneshot(Request::get("/v1/rules?since=0")
        .header("authorization", format!("Bearer {mem_key}")).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["rules"].as_array().unwrap().len(), 1);
    assert_eq!(v["rules"][0]["enforcement"], "mandatory");
}

#[tokio::test]
async fn no_key_is_unauthorized() {
    let (app, _, _) = app_with_keys().await;
    let res = app.oneshot(Request::get("/v1/rules?since=0").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
