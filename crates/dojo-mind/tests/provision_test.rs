//! Integration tests for `sensei-dojo provision` — the tenant/membership/key
//! bootstrap. Proves a provisioned API key authenticates through the REAL dojo
//! auth path (`authenticate_dojo`) with the mapped authority, that the tenant
//! upsert is idempotent, that a membership is created, and that `--scope` is
//! applied. Embedded PG, no Docker, no running Supabase.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dojo_mind::api::{build_router, SharedState};
use dojo_mind::db::DojoDb;
use dojo_mind::provision::provision;
use dojo_mind::store::DojoStore;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx_postgres::PgPool;
use std::sync::Arc;
use tower::ServiceExt; // oneshot

async fn boot() -> (Router, DojoStore, PgPool) {
    let db = DojoDb::bootstrap_temp().await.unwrap();
    let pool = db.pool().clone();
    let store = DojoStore::new(pool.clone());
    Box::leak(Box::new(db)); // keep the embedded PG alive for the test
    let app = build_router(Arc::new(SharedState { store: store.clone() }));
    (app, store, pool)
}

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, body)
}

/// A minimal valid `PublishedArtifact` body (principle kind).
fn artifact_body(sig: &str, title: &str) -> String {
    json!({
        "signature": sig,
        "tenant_key": "ignored/by-server",
        "kind": "principle",
        "title": title,
        "body": "prefer clarity over cleverness",
        "payload": { "kind": "principle", "rationale": "keeps the system honest" },
        "scope": { "stack": "rust" },
        "attribution": { "mode": "named", "author": "tester", "org": "acme", "dereferenced": false },
        "dereferenced": false
    })
    .to_string()
}

/// The provisioned key must resolve through the store's API-key path with the
/// mapped member role — `maintainer` → member `admin`.
#[tokio::test]
async fn provisioned_maintainer_key_verifies_via_auth_path() {
    let (_app, store, _pool) = boot().await;
    let p = provision(&store, "github", "acme", None, "boss@acme.io", "maintainer", "private")
        .await
        .unwrap();
    assert_eq!(p.tenant_key, "github/acme");
    assert_eq!(p.member_role, "admin", "maintainer maps to the admin member role");

    let caller = store.find_member_by_key(&p.token).await.unwrap().unwrap();
    assert_eq!(caller.role, "admin", "the key verifies via the shipped auth path");
    assert_eq!(caller.name, "boss@acme.io");
}

/// Re-provisioning the same origin/org reuses the tenant (idempotent), and each
/// call creates a distinct membership row.
#[tokio::test]
async fn provision_is_idempotent_on_tenant_and_creates_membership() {
    let (_app, store, pool) = boot().await;
    let p1 = provision(&store, "github", "acme", None, "a@acme.io", "contributor", "private")
        .await
        .unwrap();
    let p2 = provision(&store, "github", "acme", None, "b@acme.io", "maintainer", "private")
        .await
        .unwrap();
    assert_eq!(p1.tenant_id, p2.tenant_id, "same tenant reused across provisions");
    assert_ne!(p1.membership_id, p2.membership_id, "each provision creates a membership");

    let (tenants,): (i64,) =
        sqlx_core::query_as::query_as("SELECT count(*)::bigint FROM dojo.tenants WHERE key = $1")
            .bind("github/acme")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tenants, 1, "tenant is not duplicated");

    let (members,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*)::bigint FROM dojo.memberships WHERE tenant_id = $1",
    )
    .bind(p1.tenant_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(members, 2, "two memberships created");
}

/// `--scope global` is reflected on the tenant row (the DDL default is private).
#[tokio::test]
async fn provision_applies_requested_scope() {
    let (_app, store, pool) = boot().await;
    let p = provision(&store, "org", "global-dojo", None, "c@x.io", "contributor", "global")
        .await
        .unwrap();
    let (scope,): (String,) =
        sqlx_core::query_as::query_as("SELECT scope::text FROM dojo.tenants WHERE id = $1")
            .bind(p.tenant_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(scope, "global");
}

/// End-to-end via the router: a provisioned contributor key may publish, a
/// provisioned maintainer key may run the promote sweep, and the contributor
/// key is 403 on that maintainer-only route (proves the role mapping).
#[tokio::test]
async fn provisioned_keys_carry_the_mapped_dojo_authority() {
    let (app, store, _pool) = boot().await;
    let maint = provision(&store, "github", "acme", None, "m@acme.io", "maintainer", "private")
        .await
        .unwrap();
    let contrib = provision(&store, "github", "acme", None, "c@acme.io", "contributor", "private")
        .await
        .unwrap();

    // Contributor publishes → 200.
    let (s_pub, r_pub) = send(
        &app,
        Request::post("/v1/t/github%2Facme/artifacts")
            .header("authorization", format!("Bearer {}", contrib.token))
            .header("content-type", "application/json")
            .body(Body::from(artifact_body("prov-sig", "clarity")))
            .unwrap(),
    )
    .await;
    assert_eq!(s_pub, StatusCode::OK, "contributor key may publish: {r_pub:?}");

    // Contributor on the maintainer-only promote sweep → 403.
    let (s_forbidden, _) = send(
        &app,
        Request::post("/v1/t/github%2Facme/triage/promote")
            .header("authorization", format!("Bearer {}", contrib.token))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(s_forbidden, StatusCode::FORBIDDEN, "contributor may not triage");

    // Maintainer on the promote sweep → 200.
    let (s_ok, _) = send(
        &app,
        Request::post("/v1/t/github%2Facme/triage/promote")
            .header("authorization", format!("Bearer {}", maint.token))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(s_ok, StatusCode::OK, "maintainer key may run the promote sweep");
}
