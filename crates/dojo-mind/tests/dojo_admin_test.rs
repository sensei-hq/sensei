//! Integration tests for R7 — the tenant-scoped Dōjō ADMIN console backend:
//! `/v1/t/{tenant_key}/{members,identities,policies,health,audit}`.
//!
//! Covers the done-gates from `docs/spec/screen/dojo-admin-console.md`:
//! member provisioning derives the git role (a GitHub `write` role lands as
//! `contributor`); identity + policy CRUD; the health strip rollups computed
//! from `dojo.events` (+ triage queue); and non-repudiation — every admin
//! mutation persists a `dojo.audit_events` row. Also the ADMIN role-floor 403s:
//! a maintainer JWT, a contributor API key, and even an `admin` API key (the
//! API-key plane tops out at Maintainer) are all forbidden — the console is
//! reachable only by a human with the `admin` dojo membership role via SSO/JWT.
//!
//! Embedded PG + synthetic JWTs — no Docker, no running Supabase (mirrors the
//! `dojo_promote_test` / `dojo_artifacts_test` harness).

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

// ── harness ──────────────────────────────────────────────────────────────────

fn now() -> usize {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
}

/// Sign a synthetic Supabase-shaped access token for `sub` (the JWT plane a real
/// SSO login would mint) with the default test secret + audience.
fn sign_jwt(sub: &str) -> String {
    let claims = json!({
        "sub": sub,
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() + 3600,
        "email": "admin@example.com",
        "role": "authenticated",
    });
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(DEFAULT_SUPABASE_JWT_SECRET.as_bytes()),
    )
    .unwrap()
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

/// One flexible request helper for every verb the admin routes use.
async fn call(
    app: &Router,
    method: &str,
    path: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(path);
    if let Some(tok) = bearer {
        b = b.header("authorization", format!("Bearer {tok}"));
    }
    let body = match body {
        Some(v) => {
            b = b.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    send(app, b.body(body).unwrap()).await
}

const KEY: &str = "github%2Facme"; // url-encoded tenant discovery key

/// Stand up the `github/acme` tenant with one JWT-authenticated `admin`
/// membership. Returns `(tenant_id, admin_user_id, admin_bearer_token)`.
async fn admin_tenant(store: &DojoStore) -> (Uuid, Uuid, String) {
    let t = store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let admin_user = Uuid::new_v4();
    store
        .create_membership(&t, &admin_user, "employer", "sso", "admin")
        .await
        .unwrap();
    (t, admin_user, sign_jwt(&admin_user.to_string()))
}

async fn key_for(store: &DojoStore, name: &str, role: &str) -> String {
    let m = store.create_member(name, None, role).await.unwrap();
    store.issue_key(&m, None).await.unwrap().plaintext
}

async fn count_audit_for(pool: &PgPool, tenant_id: Uuid, actor: Uuid) -> i64 {
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*) FROM dojo.audit_events WHERE tenant_id = $1 AND actor_id = $2",
    )
    .bind(tenant_id)
    .bind(actor)
    .fetch_one(pool)
    .await
    .unwrap();
    n
}

async fn insert_event(pool: &PgPool, tenant_id: Uuid, actor: Option<Uuid>, action: &str) {
    sqlx_core::query::query("INSERT INTO dojo.events(tenant_id, actor_id, action) VALUES($1, $2, $3)")
        .bind(tenant_id)
        .bind(actor)
        .bind(action)
        .execute(pool)
        .await
        .unwrap();
}

/// Land one `queued` triage row (with its backing artifact) so `queue_depth`
/// has something to count — without going through the promote flow.
async fn insert_queued(pool: &PgPool, tenant_id: Uuid, signature: &str) {
    let (artifact_id,): (Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO dojo.artifacts(tenant_id, kind, title, body, signature)
         VALUES($1, 'principle'::dojo.artifact_kind, 't', 'b', $2) RETURNING id",
    )
    .bind(tenant_id)
    .bind(signature)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO dojo.triage_queue(tenant_id, artifact_id, signature) VALUES($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind(artifact_id)
    .bind(signature)
    .execute(pool)
    .await
    .unwrap();
}

fn find_member<'a>(page: &'a Value, user_id: &str) -> Option<&'a Value> {
    page["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["user_id"] == user_id)
}

// ── members: list + set-role ─────────────────────────────────────────────────

#[tokio::test]
async fn admin_lists_members_and_sets_role() {
    let (app, store, pool) = boot().await;
    let (t, admin_user, admin) = admin_tenant(&store).await;

    // A second (contributor) membership to re-role.
    let dev = Uuid::new_v4();
    store
        .create_membership(&t, &dev, "employer", "sso", "contributor")
        .await
        .unwrap();

    let (s, page) = call(&app, "GET", &format!("/v1/t/{KEY}/members"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        find_member(&page, &dev.to_string()).unwrap()["role"],
        "contributor"
    );

    // Admin override: contributor → maintainer.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/members/{dev}/role"),
        Some(&admin),
        Some(json!({ "role": "maintainer" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/members"), Some(&admin), None).await;
    assert_eq!(
        find_member(&page, &dev.to_string()).unwrap()["role"],
        "maintainer"
    );

    // The re-role stamped exactly one audit event, attributed to the admin.
    assert_eq!(count_audit_for(&pool, t, admin_user).await, 1);

    // An unknown member → 404.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/members/{}/role", Uuid::new_v4()),
        Some(&admin),
        Some(json!({ "role": "maintainer" })),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

/// Done-gate: adding a member auto-provisions with the git-derived role —
/// creating a member with a GitHub `write` role lands as `contributor`.
#[tokio::test]
async fn add_member_with_git_write_role_lands_as_contributor() {
    let (app, store, _pool) = boot().await;
    let (_t, _admin_user, admin) = admin_tenant(&store).await;

    // Seed the git-provider-role → dojo-role map (a global default): write → contributor.
    store
        .upsert_role_mapping(None, "contributor", "write", Some("github write"))
        .await
        .unwrap();

    let dev = Uuid::new_v4();
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/members"),
        Some(&admin),
        Some(json!({
            "user_id": dev.to_string(),
            "kind": "employer",
            "authenticated_via": "github_oauth",
            "git_provider_role": "write"
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["role"], "contributor", "write git role derives to contributor");

    // And the membership row reflects it.
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/members"), Some(&admin), None).await;
    assert_eq!(
        find_member(&page, &dev.to_string()).unwrap()["role"],
        "contributor"
    );
}

/// An explicit `role` overrides git derivation; no git role + no override falls
/// back to least-privilege `contributor`.
#[tokio::test]
async fn add_member_explicit_override_and_default() {
    let (app, store, _pool) = boot().await;
    let (_t, _admin_user, admin) = admin_tenant(&store).await;

    let a = Uuid::new_v4();
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/members"),
        Some(&admin),
        Some(json!({ "user_id": a.to_string(), "kind": "employer", "authenticated_via": "sso", "role": "maintainer" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["role"], "maintainer", "explicit override wins");

    let b = Uuid::new_v4();
    let (_, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/members"),
        Some(&admin),
        Some(json!({ "user_id": b.to_string(), "kind": "employer", "authenticated_via": "sso" })),
    )
    .await;
    assert_eq!(body["role"], "contributor", "no git role, no override → contributor");
}

// ── role floor: only a JWT admin reaches the console ─────────────────────────

#[tokio::test]
async fn non_admin_cannot_reach_admin_console() {
    let (app, store, _pool) = boot().await;
    let (t, _admin_user, admin) = admin_tenant(&store).await;

    // A maintainer JWT membership (Maintainer < Admin).
    let maint_user = Uuid::new_v4();
    store
        .create_membership(&t, &maint_user, "employer", "sso", "maintainer")
        .await
        .unwrap();
    let maint = sign_jwt(&maint_user.to_string());

    // API-key callers top out at Maintainer — even an `admin` member role.
    let contributor_key = key_for(&store, "Con", "publisher").await;
    let admin_key = key_for(&store, "Adm", "admin").await;

    // Positive control: the JWT admin is allowed.
    let (s, _) = call(&app, "GET", &format!("/v1/t/{KEY}/members"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK, "the admin JWT reaches the console");

    // Every non-admin caller is 403 on a read AND a mutation AND health/audit.
    for bearer in [&maint, &contributor_key, &admin_key] {
        for (method, path, body) in [
            ("GET", format!("/v1/t/{KEY}/members"), None),
            (
                "POST",
                format!("/v1/t/{KEY}/policies"),
                Some(json!({ "scope_key": "all-org" })),
            ),
            ("GET", format!("/v1/t/{KEY}/identities"), None),
            ("GET", format!("/v1/t/{KEY}/health"), None),
            ("GET", format!("/v1/t/{KEY}/audit"), None),
        ] {
            let (s, _) = call(&app, method, &path, Some(bearer), body).await;
            assert_eq!(
                s,
                StatusCode::FORBIDDEN,
                "non-admin {method} {path} must be 403"
            );
        }
    }
}

#[tokio::test]
async fn admin_route_unknown_tenant_404_and_no_credential_401() {
    let (app, store, _pool) = boot().await;
    let (_t, _admin_user, admin) = admin_tenant(&store).await;

    // Unknown tenant resolves to 404 before auth.
    let (s, _) = call(
        &app,
        "GET",
        "/v1/t/github%2Fnope/members",
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // No bearer → 401.
    let (s, _) = call(&app, "GET", &format!("/v1/t/{KEY}/members"), None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

// ── identities CRUD ──────────────────────────────────────────────────────────

#[tokio::test]
async fn identities_crud_roundtrip() {
    let (app, store, pool) = boot().await;
    let (t, admin_user, admin) = admin_tenant(&store).await;

    // Bad provider → 400.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/identities"),
        Some(&admin),
        Some(json!({ "user_id": Uuid::new_v4().to_string(), "provider": "ldap", "subject": "x" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Create.
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/identities"),
        Some(&admin),
        Some(json!({
            "user_id": Uuid::new_v4().to_string(),
            "provider": "sso",
            "subject": "okta|abc",
            "email": "a@acme.io",
            "display_name": "Ann"
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let id = body["id"].as_str().unwrap().to_string();

    // List.
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/identities"), Some(&admin), None).await;
    let rows = page["identities"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["provider"], "sso");
    assert_eq!(rows[0]["display_name"], "Ann");

    // Patch display_name; email left unchanged.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/identities/{id}"),
        Some(&admin),
        Some(json!({ "display_name": "Annie" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/identities"), Some(&admin), None).await;
    assert_eq!(page["identities"][0]["display_name"], "Annie");
    assert_eq!(page["identities"][0]["email"], "a@acme.io");

    // Delete.
    let (s, body) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/identities/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["deleted"], true);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/identities"), Some(&admin), None).await;
    assert_eq!(page["identities"].as_array().unwrap().len(), 0);

    // Deleting a gone identity → 404.
    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/identities/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // add + update + delete each stamped an audit event.
    assert_eq!(count_audit_for(&pool, t, admin_user).await, 3);
}

// ── policies CRUD + edit-in-place ────────────────────────────────────────────

#[tokio::test]
async fn policies_crud_and_edit_takes_effect() {
    let (app, store, _pool) = boot().await;
    let (_t, _admin_user, admin) = admin_tenant(&store).await;

    // Bad attribution → 400.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/policies"),
        Some(&admin),
        Some(json!({ "scope_key": "all-org", "attribution_default": "weird" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Create.
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/policies"),
        Some(&admin),
        Some(json!({
            "scope_key": "all-org",
            "attribution_default": "named",
            "confidentiality": { "pack": "SOC2" },
            "retention_days": 90
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let id = body["id"].as_str().unwrap().to_string();

    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/policies"), Some(&admin), None).await;
    assert_eq!(page["policies"].as_array().unwrap().len(), 1);
    assert_eq!(page["policies"][0]["attribution_default"], "named");
    assert_eq!(page["policies"][0]["retention_days"], 90);
    assert_eq!(page["policies"][0]["confidentiality"]["pack"], "SOC2");

    // Edit-in-place by scope_key (upsert) — the change is visible on next read.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/policies"),
        Some(&admin),
        Some(json!({ "scope_key": "all-org", "attribution_default": "anonymous", "retention_days": 90 })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/policies"), Some(&admin), None).await;
    assert_eq!(page["policies"].as_array().unwrap().len(), 1, "upsert edits in place");
    assert_eq!(page["policies"][0]["attribution_default"], "anonymous");

    // Patch by id (retention only; attribution left unchanged).
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/policies/{id}"),
        Some(&admin),
        Some(json!({ "retention_days": 30 })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/policies"), Some(&admin), None).await;
    assert_eq!(page["policies"][0]["retention_days"], 30);
    assert_eq!(page["policies"][0]["attribution_default"], "anonymous");

    // Delete.
    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/policies/{id}"),
        Some(&admin),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/policies"), Some(&admin), None).await;
    assert_eq!(page["policies"].as_array().unwrap().len(), 0);
}

// ── health rollups from dojo.events ──────────────────────────────────────────

#[tokio::test]
async fn health_rollup_reports_events_and_queue() {
    let (app, store, pool) = boot().await;
    let (t, _admin_user, admin) = admin_tenant(&store).await;

    // Another tenant's activity must NOT leak into these rollups.
    let other = store
        .create_tenant("github/beta", "github", "beta", None, "Beta", "url")
        .await
        .unwrap();
    insert_event(&pool, other, Some(Uuid::new_v4()), "approved").await;
    insert_queued(&pool, other, "beta-sig").await;

    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    insert_event(&pool, t, Some(a), "approved").await; // connection + publish
    insert_event(&pool, t, Some(b), "error").await; // connection + error
    insert_event(&pool, t, Some(a), "queued").await; // same actor; neither rate
    insert_event(&pool, t, None, "distributed").await; // publish; null actor (no conn)
    insert_queued(&pool, t, "sig1").await;
    insert_queued(&pool, t, "sig2").await;

    let (s, h) = call(&app, "GET", &format!("/v1/t/{KEY}/health"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(h["connections"], 2, "distinct actors active in last 5min");
    assert_eq!(h["queue_depth"], 2, "queued triage rows");
    assert_eq!(h["publish_rate_1h"], 2, "approved + distributed in last hour");
    assert_eq!(h["error_rate_1h"], 1, "error events in last hour");
}

// ── audit: non-repudiation (every admin action persists) ─────────────────────

/// Done-gate: `select count(*) from dojo.audit_events where actor_id = {admin}`
/// equals the number of admin mutations in the session — and reads add none.
#[tokio::test]
async fn audit_log_persists_every_admin_action() {
    let (app, store, pool) = boot().await;
    let (t, admin_user, admin) = admin_tenant(&store).await;

    // 1) create a policy
    let (_, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/policies"),
        Some(&admin),
        Some(json!({ "scope_key": "team:mobile" })),
    )
    .await;
    let policy_id = body["id"].as_str().unwrap().to_string();

    // 2) patch the policy
    call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/policies/{policy_id}"),
        Some(&admin),
        Some(json!({ "retention_days": 10 })),
    )
    .await;

    // 3) wire an identity
    call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/identities"),
        Some(&admin),
        Some(json!({ "user_id": Uuid::new_v4().to_string(), "provider": "github_oauth", "subject": "gh|1" })),
    )
    .await;

    // 4) provision a member
    let dev = Uuid::new_v4();
    call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/members"),
        Some(&admin),
        Some(json!({ "user_id": dev.to_string(), "kind": "employer", "authenticated_via": "sso", "role": "contributor" })),
    )
    .await;

    // 5) re-role the member
    call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/members/{dev}/role"),
        Some(&admin),
        Some(json!({ "role": "maintainer" })),
    )
    .await;

    // Exactly five mutations → five audit rows attributed to the admin.
    assert_eq!(count_audit_for(&pool, t, admin_user).await, 5);

    // The audit list returns them, most recent first.
    let (s, page) = call(&app, "GET", &format!("/v1/t/{KEY}/audit"), Some(&admin), None).await;
    assert_eq!(s, StatusCode::OK);
    let events = page["events"].as_array().unwrap();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["action"], "role_changed", "newest first");

    // Reads must NOT add audit rows (non-repudiation counts actions, not views).
    call(&app, "GET", &format!("/v1/t/{KEY}/members"), Some(&admin), None).await;
    call(&app, "GET", &format!("/v1/t/{KEY}/health"), Some(&admin), None).await;
    assert_eq!(count_audit_for(&pool, t, admin_user).await, 5, "reads add no audit");
}
