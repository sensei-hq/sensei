//! Integration tests for the tenant-scoped Dōjō artifact routes:
//! `POST/GET /v1/t/{tenant_key}/artifacts`. Covers tenant isolation, dual auth
//! (API-key + Supabase-JWT with synthetic tokens), the publish→pull seq-cursor
//! round-trip, and the 404 / 401 / 403 failure paths. No running Supabase and
//! no Docker — embedded PG + synthetic JWTs.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use hive_mind::api::{build_router, SharedState};
use hive_mind::auth::{DEFAULT_SUPABASE_JWT_AUD, DEFAULT_SUPABASE_JWT_SECRET};
use hive_mind::db::HiveDb;
use hive_mind::store::HiveStore;
use http_body_util::BodyExt;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // oneshot
use uuid::Uuid;

// ── helpers ──────────────────────────────────────────────────────────────────

fn now() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

/// Sign a synthetic Supabase-shaped access token for `sub` with the default
/// (test) secret and audience — the token a real Supabase login would mint.
fn sign_jwt(sub: &str) -> String {
    let claims = json!({
        "sub": sub,
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() + 3600,
        "email": "human@example.com",
        "role": "authenticated",
    });
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(DEFAULT_SUPABASE_JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

/// A valid `PublishedArtifact` body (principle kind) — `contributed_by` in the
/// body is deliberately a bogus value to prove the server overrides it.
fn artifact_body(title: &str) -> String {
    json!({
        "signature": format!("sig-{title}"),
        "tenant_key": "ignored/by-server",
        "kind": "principle",
        "title": title,
        "body": "prefer clarity over cleverness",
        "payload": { "kind": "principle", "rationale": "keeps the system honest" },
        "scope": { "stack": "rust" },
        "attribution": { "mode": "named", "author": "jerry", "org": "acme", "dereferenced": false },
        "dereferenced": false,
        "contributed_by": "spoofed-not-a-real-id"
    })
    .to_string()
}

/// Boot embedded PG (deploys hive + dojo scopes) and build the router. Returns
/// a store sharing the same pool as the router's store, so a test seeds tenants
/// / members / memberships through it and the router sees them.
async fn boot() -> (Router, HiveStore) {
    let db = HiveDb::bootstrap_temp().await.unwrap();
    let store = HiveStore::new(db.pool().clone());
    Box::leak(Box::new(db)); // keep embedded PG alive for the test
    let app = build_router(Arc::new(SharedState {
        store: store.clone(),
    }));
    (app, store)
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

async fn publish(app: &Router, enc_key: &str, bearer: &str, title: &str) -> (StatusCode, Value) {
    let req = Request::post(format!("/v1/t/{enc_key}/artifacts"))
        .header("authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json")
        .body(Body::from(artifact_body(title)))
        .unwrap();
    send(app, req).await
}

async fn pull(app: &Router, enc_key: &str, bearer: Option<&str>, since: i64) -> (StatusCode, Value) {
    let mut b = Request::get(format!("/v1/t/{enc_key}/artifacts?since={since}"));
    if let Some(tok) = bearer {
        b = b.header("authorization", format!("Bearer {tok}"));
    }
    send(app, b.body(Body::empty()).unwrap()).await
}

async fn publisher_key(store: &HiveStore) -> String {
    let m = store.create_member("Pub", None, "publisher").await.unwrap();
    store.issue_key(&m, None).await.unwrap().plaintext
}

// ── tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_tenant_is_404() {
    let (app, store) = boot().await;
    let key = publisher_key(&store).await;
    let (status, _) = pull(&app, "github%2Fnope", Some(&key), 0).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "unknown tenant_key → 404");
}

#[tokio::test]
async fn no_credential_is_401() {
    let (app, store) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let (status, _) = pull(&app, "github%2Facme", None, 0).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "no bearer → 401");
}

#[tokio::test]
async fn api_key_publish_pull_roundtrip_with_seq_cursor() {
    let (app, store) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let key = publisher_key(&store).await;

    let (s1, r1) = publish(&app, "github%2Facme", &key, "first").await;
    assert_eq!(s1, StatusCode::OK);
    let (s2, r2) = publish(&app, "github%2Facme", &key, "second").await;
    assert_eq!(s2, StatusCode::OK);
    let seq1 = r1["seq"].as_i64().unwrap();
    let seq2 = r2["seq"].as_i64().unwrap();
    assert!(seq2 > seq1, "seq must advance across publishes ({seq1} -> {seq2})");

    // Pull from the start → both artifacts, ordered by seq, cursor = max seq.
    let (sp, page) = pull(&app, "github%2Facme", Some(&key), 0).await;
    assert_eq!(sp, StatusCode::OK);
    let arts = page["artifacts"].as_array().unwrap();
    assert_eq!(arts.len(), 2);
    assert_eq!(arts[0]["seq"].as_i64().unwrap(), seq1);
    assert_eq!(arts[1]["seq"].as_i64().unwrap(), seq2);
    assert_eq!(page["cursor"].as_i64().unwrap(), seq2);
    assert_eq!(arts[0]["title"], "first");
    assert_eq!(arts[0]["status"], "submitted");
    assert_eq!(arts[0]["kind"], "principle");

    // Attribution is server-controlled: the bogus body `contributed_by` is
    // replaced with the authenticated caller's id (a real uuid).
    let attributed = arts[0]["contributed_by"].as_str().unwrap();
    assert_ne!(attributed, "spoofed-not-a-real-id");
    assert!(Uuid::parse_str(attributed).is_ok(), "contributed_by is the caller uuid");

    // Resuming from the cursor yields nothing new.
    let (_, empty) = pull(&app, "github%2Facme", Some(&key), seq2).await;
    assert_eq!(empty["artifacts"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn member_api_key_can_pull_but_not_publish() {
    let (app, store) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let m = store.create_member("Mem", None, "member").await.unwrap();
    let member_key = store.issue_key(&m, None).await.unwrap().plaintext;

    let (sp, _) = publish(&app, "github%2Facme", &member_key, "x").await;
    assert_eq!(sp, StatusCode::FORBIDDEN, "member role may not publish");
    let (sg, _) = pull(&app, "github%2Facme", Some(&member_key), 0).await;
    assert_eq!(sg, StatusCode::OK, "member role may pull");
}

#[tokio::test]
async fn tenant_isolation_a_not_visible_to_b() {
    let (app, store) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url-a")
        .await
        .unwrap();
    store
        .create_tenant("github/beta", "github", "beta", None, "Beta", "url-b")
        .await
        .unwrap();
    let key = publisher_key(&store).await;

    let (s1, _) = publish(&app, "github%2Facme", &key, "secret-to-acme").await;
    assert_eq!(s1, StatusCode::OK);

    // Tenant B must NOT see A's artifact.
    let (_, b_page) = pull(&app, "github%2Fbeta", Some(&key), 0).await;
    assert_eq!(
        b_page["artifacts"].as_array().unwrap().len(),
        0,
        "tenant B must not see tenant A's artifacts"
    );
    // Tenant A sees its own.
    let (_, a_page) = pull(&app, "github%2Facme", Some(&key), 0).await;
    assert_eq!(a_page["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(a_page["artifacts"][0]["title"], "secret-to-acme");
}

#[tokio::test]
async fn jwt_membership_can_publish_and_pull() {
    let (app, store) = boot().await;
    let t = store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let user_id = Uuid::new_v4();
    store
        .create_membership(&t, &user_id, "employer", "sso", "contributor")
        .await
        .unwrap();

    let token = sign_jwt(&user_id.to_string());
    let (sp, _) = publish(&app, "github%2Facme", &token, "via-jwt").await;
    assert_eq!(sp, StatusCode::OK, "a valid JWT bound to a membership may publish");
    let (_, page) = pull(&app, "github%2Facme", Some(&token), 0).await;
    assert_eq!(page["artifacts"].as_array().unwrap().len(), 1);
    assert_eq!(page["artifacts"][0]["title"], "via-jwt");
}

#[tokio::test]
async fn bad_jwt_is_rejected_401() {
    let (app, store) = boot().await;
    let t = store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let user_id = Uuid::new_v4();
    store
        .create_membership(&t, &user_id, "employer", "sso", "contributor")
        .await
        .unwrap();

    // A token signed with the wrong secret.
    let forged = encode(
        &Header::new(Algorithm::HS256),
        &json!({ "sub": user_id.to_string(), "aud": DEFAULT_SUPABASE_JWT_AUD, "exp": now() + 3600 }),
        &EncodingKey::from_secret(b"a-totally-different-secret-key-value-xx"),
    )
    .unwrap();
    let (s1, _) = pull(&app, "github%2Facme", Some(&forged), 0).await;
    assert_eq!(s1, StatusCode::UNAUTHORIZED, "wrong-secret JWT → 401");

    // Pure garbage bearer.
    let (s2, _) = pull(&app, "github%2Facme", Some("not.a.jwt"), 0).await;
    assert_eq!(s2, StatusCode::UNAUTHORIZED, "garbage bearer → 401");
}

#[tokio::test]
async fn valid_jwt_without_membership_is_403() {
    let (app, store) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    // Valid signature, but this subject has no membership in the tenant.
    let token = sign_jwt(&Uuid::new_v4().to_string());
    let (status, _) = pull(&app, "github%2Facme", Some(&token), 0).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "a valid JWT with no membership in the tenant → 403"
    );
}
