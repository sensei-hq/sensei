use dojo_mind::api::{build_router, SharedState};
use dojo_mind::db::DojoDb;
use dojo_mind::store::DojoStore;
use std::sync::Arc;

#[tokio::test]
async fn server_serves_health_on_ephemeral_port() {
    let db = DojoDb::bootstrap_temp().await.unwrap();
    let store = DojoStore::new(db.pool().clone());
    Box::leak(Box::new(db));
    let app = build_router(Arc::new(SharedState { store }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap(); });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let body = reqwest::get(format!("http://{addr}/v1/health")).await.unwrap().text().await.unwrap();
    assert!(body.contains("\"status\":\"ok\""), "got: {body}");
}
