//! Integration tests for C8 — collective triage/promote that CLOSES the loop.
//!
//! Covers: a high-bar cluster auto-publishes (status→published + a decisions
//! row) so a subsequent pull returns it; a low-bar cluster stays submitted and
//! lands in triage_queue (not published); a global-dojo cluster under K
//! contributors is NOT published (k-anonymity); the maintainer endpoints
//! (list/decide/promote) incl. dual-auth role checks and the 400 safe-default
//! gates; and idempotency (re-running never double-publishes/double-decides).
//! Embedded PG + synthetic JWTs — no Docker, no running Supabase.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use dojo_mind::api::{build_router, SharedState};
use dojo_mind::auth::{DEFAULT_SUPABASE_JWT_AUD, DEFAULT_SUPABASE_JWT_SECRET};
use dojo_mind::db::DojoDb;
use dojo_mind::store::DojoStore;
use http_body_util::BodyExt;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use sqlx_postgres::PgPool;
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

/// A high-bar `pattern` artifact: an observed FTR lift of `0.20` (= full
/// efficacy credit) → score clears the auto-approve bar on a single contributor.
fn pattern_body(signature: &str, title: &str) -> String {
    json!({
        "signature": signature,
        "tenant_key": "ignored/by-server",
        "kind": "pattern",
        "title": title,
        "body": "inject the clock instead of calling now()",
        "payload": {
            "kind": "pattern", "family": "design",
            "pattern_id": "codebase.di", "ftr_delta_observed": 0.20
        },
        "scope": { "stack": "rust" },
        "attribution": { "mode": "named", "author": "jerry", "org": "acme", "dereferenced": false },
        "dereferenced": false
    })
    .to_string()
}

/// A low-bar `principle` artifact: one contributor, no efficacy signal → score
/// below the bar → queued for a maintainer.
fn principle_body(signature: &str, title: &str) -> String {
    json!({
        "signature": signature,
        "tenant_key": "ignored/by-server",
        "kind": "principle",
        "title": title,
        "body": "prefer clarity over cleverness",
        "payload": { "kind": "principle", "rationale": "keeps the system honest" },
        "scope": { "stack": "rust" },
        "attribution": { "mode": "named", "author": "jerry", "org": "acme", "dereferenced": false },
        "dereferenced": false
    })
    .to_string()
}

async fn boot() -> (Router, DojoStore, PgPool) {
    let db = DojoDb::bootstrap_temp().await.unwrap();
    let pool = db.pool().clone();
    let store = DojoStore::new(pool.clone());
    Box::leak(Box::new(db)); // keep embedded PG alive for the test
    let app = build_router(Arc::new(SharedState {
        store: store.clone(),
    }));
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

async fn publish(app: &Router, enc_key: &str, bearer: &str, body: String) -> (StatusCode, Value) {
    let req = Request::post(format!("/v1/t/{enc_key}/artifacts"))
        .header("authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();
    send(app, req).await
}

async fn pull(app: &Router, enc_key: &str, bearer: &str, since: i64) -> (StatusCode, Value) {
    let req = Request::get(format!("/v1/t/{enc_key}/artifacts?since={since}"))
        .header("authorization", format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap();
    send(app, req).await
}

async fn get_triage(app: &Router, enc_key: &str, bearer: &str) -> (StatusCode, Value) {
    let req = Request::get(format!("/v1/t/{enc_key}/triage"))
        .header("authorization", format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap();
    send(app, req).await
}

async fn decide(
    app: &Router,
    enc_key: &str,
    bearer: &str,
    signature: &str,
    body: Value,
) -> (StatusCode, Value) {
    let req = Request::post(format!("/v1/t/{enc_key}/triage/{signature}/decide"))
        .header("authorization", format!("Bearer {bearer}"))
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    send(app, req).await
}

async fn key_for(store: &DojoStore, name: &str, role: &str) -> String {
    let m = store.create_member(name, None, role).await.unwrap();
    store.issue_key(&m, None).await.unwrap().plaintext
}

async fn count(pool: &PgPool, sql: &str) -> i64 {
    let (n,): (i64,) = sqlx_core::query_as::query_as(sql)
        .fetch_one(pool)
        .await
        .unwrap();
    n
}

/// Number of `published`-status artifacts in a pull page.
fn published_in(page: &Value) -> usize {
    page["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["status"] == "published")
        .count()
}

// ── auto-approve (loop closes) ───────────────────────────────────────────────

#[tokio::test]
async fn high_bar_cluster_auto_publishes_and_pull_returns_it() {
    let (app, store, pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let key = key_for(&store, "Pub", "publisher").await;

    let (s, _) = publish(&app, "github%2Facme", &key, pattern_body("hi-sig", "di")).await;
    assert_eq!(s, StatusCode::OK);

    // A subsequent pull returns it as PUBLISHED (the loop closed → C7 can pull).
    let (sp, page) = pull(&app, "github%2Facme", &key, 0).await;
    assert_eq!(sp, StatusCode::OK);
    assert_eq!(published_in(&page), 1, "high-bar cluster must auto-publish");

    // An automated approve decision was recorded.
    let approvals = count(
        &pool,
        "SELECT count(*) FROM dojo.decisions WHERE status = 'approve' AND automated = true",
    )
    .await;
    assert_eq!(approvals, 1, "auto-approve must record a decisions row");
}

// ── below-bar (queue, do not publish) ────────────────────────────────────────

#[tokio::test]
async fn low_bar_cluster_stays_submitted_and_lands_in_triage() {
    let (app, store, pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let key = key_for(&store, "Pub", "publisher").await;

    let (s, _) = publish(&app, "github%2Facme", &key, principle_body("lo-sig", "clarity")).await;
    assert_eq!(s, StatusCode::OK);

    // Pull returns it, but it is NOT published (still submitted).
    let (_, page) = pull(&app, "github%2Facme", &key, 0).await;
    assert_eq!(published_in(&page), 0, "low-bar cluster must NOT publish");
    assert_eq!(page["artifacts"][0]["status"], "submitted");

    // It landed in the triage queue, queued; no decision was recorded.
    let queued = count(
        &pool,
        "SELECT count(*) FROM dojo.triage_queue WHERE state = 'queued'",
    )
    .await;
    assert_eq!(queued, 1, "low-bar cluster must land in triage_queue queued");
    let decisions = count(&pool, "SELECT count(*) FROM dojo.decisions").await;
    assert_eq!(decisions, 0, "queueing must not record a decision");
}

// ── k-anonymity on the global-dojo collective ────────────────────────────────

#[tokio::test]
async fn global_dojo_under_k_contributors_is_not_published() {
    // The global-dojo tenant is seeded on boot (scope = global).
    let (app, store, _pool) = boot().await;
    let g = "org%2Fglobal-dojo";

    // Three DISTINCT contributors, each contributing the SAME high-bar signature.
    let k1 = key_for(&store, "C1", "publisher").await;
    let k2 = key_for(&store, "C2", "publisher").await;
    let k3 = key_for(&store, "C3", "publisher").await;
    let body = || pattern_body("global-sig", "shared insight");

    // 1 contributor — score clears the bar but 1 < K → k-anonymity block.
    publish(&app, g, &k1, body()).await;
    let (_, p1) = pull(&app, g, &k1, 0).await;
    assert_eq!(published_in(&p1), 0, "1 < K contributors must not publish");

    // 2 contributors — still under K = 3.
    publish(&app, g, &k2, body()).await;
    let (_, p2) = pull(&app, g, &k2, 0).await;
    assert_eq!(published_in(&p2), 0, "2 < K contributors must not publish");

    // 3rd distinct contributor reaches K → the collective publishes exactly one.
    publish(&app, g, &k3, body()).await;
    let (_, p3) = pull(&app, g, &k3, 0).await;
    assert_eq!(
        published_in(&p3),
        1,
        "reaching K distinct contributors publishes the representative"
    );
}

// ── maintainer decide (human approve) ────────────────────────────────────────

#[tokio::test]
async fn maintainer_approve_publishes_the_queued_cluster() {
    let (app, store, pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let pubk = key_for(&store, "Pub", "publisher").await;
    let admin = key_for(&store, "Maint", "admin").await; // admin → Maintainer access

    // A low-bar contribution lands in the queue.
    publish(&app, "github%2Facme", &pubk, principle_body("todo-sig", "todo")).await;

    // The maintainer sees it in the queue.
    let (sg, q) = get_triage(&app, "github%2Facme", &admin).await;
    assert_eq!(sg, StatusCode::OK);
    let rows = q["queue"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["signature"], "todo-sig");
    assert_eq!(rows[0]["kind"], "principle");

    // Approve with a distribution scope → published.
    let (sd, dd) = decide(
        &app,
        "github%2Facme",
        &admin,
        "todo-sig",
        json!({ "status": "approve", "distribution_scope": { "team": "core" } }),
    )
    .await;
    assert_eq!(sd, StatusCode::OK);
    assert_eq!(dd["status"], "approved");

    // The artifact is now published, and the queue is empty again.
    let (_, page) = pull(&app, "github%2Facme", &admin, 0).await;
    assert_eq!(published_in(&page), 1);
    let (_, q2) = get_triage(&app, "github%2Facme", &admin).await;
    assert_eq!(q2["queue"].as_array().unwrap().len(), 0);

    // A human (non-automated) approve decision was recorded.
    let human_approvals = count(
        &pool,
        "SELECT count(*) FROM dojo.decisions WHERE status = 'approve' AND automated = false",
    )
    .await;
    assert_eq!(human_approvals, 1);
}

#[tokio::test]
async fn jwt_maintainer_membership_can_decide() {
    let (app, store, _pool) = boot().await;
    let t = store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let pubk = key_for(&store, "Pub", "publisher").await;
    publish(&app, "github%2Facme", &pubk, principle_body("j-sig", "jj")).await;

    // A membership with the maintainer role, authenticated via a synthetic JWT.
    let user = Uuid::new_v4();
    store
        .create_membership(&t, &user, "employer", "sso", "maintainer")
        .await
        .unwrap();
    let token = sign_jwt(&user.to_string());

    let (sd, _) = decide(
        &app,
        "github%2Facme",
        &token,
        "j-sig",
        json!({ "status": "approve", "distribution_scope": { "company": "acme" } }),
    )
    .await;
    assert_eq!(sd, StatusCode::OK, "a JWT maintainer membership may decide");
}

// ── role gate: non-maintainer is 403 ─────────────────────────────────────────

#[tokio::test]
async fn non_maintainer_cannot_list_or_decide() {
    let (app, store, _pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let pubk = key_for(&store, "Pub", "publisher").await; // publisher → Contributor
    publish(&app, "github%2Facme", &pubk, principle_body("x-sig", "x")).await;

    // A contributor may NOT view the triage queue…
    let (sg, _) = get_triage(&app, "github%2Facme", &pubk).await;
    assert_eq!(sg, StatusCode::FORBIDDEN);

    // …nor decide.
    let (sd, _) = decide(
        &app,
        "github%2Facme",
        &pubk,
        "x-sig",
        json!({ "status": "approve", "distribution_scope": { "team": "core" } }),
    )
    .await;
    assert_eq!(sd, StatusCode::FORBIDDEN, "non-maintainer decide → 403");
}

// ── decide safe-default gates (400) ──────────────────────────────────────────

#[tokio::test]
async fn approve_needs_distribution_scope_and_decline_needs_reason() {
    let (app, store, _pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let pubk = key_for(&store, "Pub", "publisher").await;
    let admin = key_for(&store, "Maint", "admin").await;
    publish(&app, "github%2Facme", &pubk, principle_body("g-sig", "g")).await;

    // Approve without a distribution scope is rejected (unsafe broadcast).
    let (s1, _) = decide(
        &app,
        "github%2Facme",
        &admin,
        "g-sig",
        json!({ "status": "approve" }),
    )
    .await;
    assert_eq!(s1, StatusCode::BAD_REQUEST);

    // Decline without a reason is rejected.
    let (s2, _) = decide(
        &app,
        "github%2Facme",
        &admin,
        "g-sig",
        json!({ "status": "decline", "reason": "  " }),
    )
    .await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);

    // An unknown signature yields 404 (nothing submitted to decide on).
    let (s3, _) = decide(
        &app,
        "github%2Facme",
        &admin,
        "no-such-sig",
        json!({ "status": "approve", "distribution_scope": { "team": "core" } }),
    )
    .await;
    assert_eq!(s3, StatusCode::NOT_FOUND);
}

// ── idempotency: re-running never double-publishes/double-decides ────────────

#[tokio::test]
async fn promotion_is_idempotent() {
    let (app, store, pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let tid = store
        .resolve_tenant("github/acme")
        .await
        .unwrap()
        .unwrap();
    let key = key_for(&store, "Pub", "publisher").await;

    // Publish a high-bar cluster → auto-published inline.
    publish(&app, "github%2Facme", &key, pattern_body("idem-sig", "di")).await;

    let published0 = count(
        &pool,
        "SELECT count(*) FROM dojo.artifacts WHERE status = 'published'",
    )
    .await;
    let decisions0 = count(&pool, "SELECT count(*) FROM dojo.decisions").await;
    assert_eq!(published0, 1);
    assert_eq!(decisions0, 1);

    // Re-run promotion twice (tenant sweep + direct cluster) — no-ops.
    store.promote_tenant(&tid).await.unwrap();
    let outcome = store.promote_cluster(&tid, "idem-sig").await.unwrap();
    assert!(
        matches!(
            outcome,
            dojo_mind::collective::promote::PromoteOutcome::NoOp
        ),
        "re-promoting a published cluster is a no-op, got {outcome:?}"
    );

    let published1 = count(
        &pool,
        "SELECT count(*) FROM dojo.artifacts WHERE status = 'published'",
    )
    .await;
    let decisions1 = count(&pool, "SELECT count(*) FROM dojo.decisions").await;
    assert_eq!(published1, 1, "must not double-publish");
    assert_eq!(decisions1, 1, "must not double-decide");
}

// ── maintainer promote sweep endpoint ────────────────────────────────────────

#[tokio::test]
async fn maintainer_promote_sweep_endpoint_runs() {
    let (app, store, _pool) = boot().await;
    store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let admin = key_for(&store, "Maint", "admin").await;
    let pubk = key_for(&store, "Pub", "publisher").await;
    publish(&app, "github%2Facme", &pubk, principle_body("s-sig", "s")).await;

    let req = Request::post("/v1/t/github%2Facme/triage/promote")
        .header("authorization", format!("Bearer {admin}"))
        .body(Body::empty())
        .unwrap();
    let (s, body) = send(&app, req).await;
    assert_eq!(s, StatusCode::OK);
    assert!(body["promoted"].is_array());
}
