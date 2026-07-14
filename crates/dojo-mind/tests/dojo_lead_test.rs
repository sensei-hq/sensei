//! Integration tests for R8 — the tenant-scoped Dōjō CLIENT-LEAD console backend:
//! `/v1/t/{tenant_key}/{engagements,incidents,audit/artifacts,compliance/export}`.
//!
//! Covers the done-gates from `docs/llm-spec/screen/dojo-client-lead-console.md`:
//! engagements CRUD + project bind; incident CRUD with the open-count gate
//! (`resolved_at is null`); the artifact audit view with the **`non_dereferenced
//! == 0`** done-gate (and the `dereferenced=true` filter); and the compliance
//! export — BLOCKED when any in-scope artifact is non-dereferenced (the red-fail
//! chip), and, on success, provably **leaking no source references** (the export
//! never carries `attribution` / `signature` / `contributed_by`). Also the LEAD
//! role-floor: a plain contributor JWT and an API-key caller (which never carries
//! `Lead`) are 403; a `client_lead` JWT — and, by the linear floor, a maintainer
//! / admin JWT — are allowed.
//!
//! Embedded PG + synthetic JWTs — no Docker, no running Supabase (mirrors the
//! `dojo_admin_test` / `dojo_artifacts_test` harness).

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

/// Sign a synthetic Supabase-shaped access token for `sub` with the default test
/// secret + audience — the token a real SSO login would mint.
fn sign_jwt(sub: &str) -> String {
    let claims = json!({
        "sub": sub,
        "aud": DEFAULT_SUPABASE_JWT_AUD,
        "exp": now() + 3600,
        "email": "lead@example.com",
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

/// One flexible request helper for every verb the client-lead routes use.
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

/// A raw GET returning `(status, body_text, content_type)` — for the CSV export,
/// whose body is not JSON.
async fn get_raw(app: &Router, path: &str, bearer: &str) -> (StatusCode, String, String) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {bearer}"))
        .body(Body::empty())
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let ctype = res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, String::from_utf8_lossy(&bytes).to_string(), ctype)
}

const KEY: &str = "github%2Facme"; // url-encoded tenant discovery key

/// Stand up the `github/acme` tenant with one JWT-authenticated `client_lead`
/// membership. Returns `(tenant_id, lead_user_id, lead_bearer_token)`.
async fn lead_tenant(store: &DojoStore) -> (Uuid, Uuid, String) {
    let t = store
        .create_tenant("github/acme", "github", "acme", None, "Acme", "url")
        .await
        .unwrap();
    let lead_user = Uuid::new_v4();
    store
        .create_membership(&t, &lead_user, "employer", "sso", "client_lead")
        .await
        .unwrap();
    (t, lead_user, sign_jwt(&lead_user.to_string()))
}

/// A JWT for a fresh membership with `role` in the tenant `t`.
async fn member_jwt(store: &DojoStore, t: &Uuid, role: &str) -> String {
    let u = Uuid::new_v4();
    store
        .create_membership(t, &u, "employer", "sso", role)
        .await
        .unwrap();
    sign_jwt(&u.to_string())
}

/// An API key bound to a federation member with the given API-key `role`.
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

/// Insert one artifact directly (bypassing publish/triage) so a test controls its
/// engagement binding, strip status, attribution + signature exactly.
#[allow(clippy::too_many_arguments)]
async fn insert_artifact(
    pool: &PgPool,
    tenant_id: Uuid,
    engagement_id: Option<Uuid>,
    title: &str,
    signature: &str,
    attribution: Value,
    dereferenced: bool,
    contributed_by: Option<Uuid>,
) {
    sqlx_core::query::query(
        "INSERT INTO dojo.artifacts
           (tenant_id, engagement_id, kind, title, body, signature, attribution,
            dereferenced, contributed_by)
         VALUES($1, $2, 'principle'::dojo.artifact_kind, $3, 'b', $4, $5::jsonb, $6, $7)",
    )
    .bind(tenant_id)
    .bind(engagement_id)
    .bind(title)
    .bind(signature)
    .bind(attribution)
    .bind(dereferenced)
    .bind(contributed_by)
    .execute(pool)
    .await
    .unwrap();
}

// ── engagements: CRUD + project bind ─────────────────────────────────────────

#[tokio::test]
async fn engagements_crud_and_bind() {
    let (app, store, pool) = boot().await;
    let (t, lead_user, lead) = lead_tenant(&store).await;

    // Missing client → 400.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/engagements"),
        Some(&lead),
        Some(json!({ "client": "  " })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Create (done-gate: engagements can be created and bound to projects).
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/engagements"),
        Some(&lead),
        Some(json!({
            "client": "Globex",
            "description": "Q3 platform work",
            "starts_on": "2026-01-01",
            "policy_overrides": { "pack": "HIPAA" }
        })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let eng_id = body["id"].as_str().unwrap().to_string();

    // List.
    let (s, page) = call(&app, "GET", &format!("/v1/t/{KEY}/engagements"), Some(&lead), None).await;
    assert_eq!(s, StatusCode::OK);
    let rows = page["engagements"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["client"], "Globex");
    assert_eq!(rows[0]["status"], "active");
    assert_eq!(rows[0]["starts_on"], "2026-01-01");
    assert_eq!(rows[0]["policy_overrides"]["pack"], "HIPAA");
    assert_eq!(rows[0]["project_bindings"].as_array().unwrap().len(), 0);

    // Bind a project.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/engagements/{eng_id}/bind"),
        Some(&lead),
        Some(json!({ "project_id": "proj-123", "name": "platform" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/engagements"), Some(&lead), None).await;
    let bindings = page["engagements"][0]["project_bindings"].as_array().unwrap();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0]["project_id"], "proj-123");
    assert_eq!(bindings[0]["name"], "platform");

    // Bad status → 400; a good PATCH closes it (status = ended).
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/engagements/{eng_id}"),
        Some(&lead),
        Some(json!({ "status": "weird" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/engagements/{eng_id}"),
        Some(&lead),
        Some(json!({ "status": "ended", "ends_on": "2026-09-30" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/engagements"), Some(&lead), None).await;
    assert_eq!(page["engagements"][0]["status"], "ended");
    assert_eq!(page["engagements"][0]["ends_on"], "2026-09-30");

    // Delete → gone; a second delete → 404.
    let (s, body) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/engagements/{eng_id}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["deleted"], true);
    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/engagements/{eng_id}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // create + bind + update + delete each stamped an audit event (4).
    assert_eq!(count_audit_for(&pool, t, lead_user).await, 4);
}

// ── incidents: CRUD + open-count gate ────────────────────────────────────────

#[tokio::test]
async fn incidents_crud_and_open_count() {
    let (app, store, _pool) = boot().await;
    let (_t, _lead_user, lead) = lead_tenant(&store).await;

    // Bad severity → 400.
    let (s, _) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/incidents"),
        Some(&lead),
        Some(json!({ "title": "leak?", "severity": "spicy" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Two open incidents (one high, one default medium).
    let (s, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/incidents"),
        Some(&lead),
        Some(json!({ "title": "near-leak in export", "severity": "high" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(body["severity"], "high");
    let hi_id = body["id"].as_str().unwrap().to_string();

    let (_, body) = call(
        &app,
        "POST",
        &format!("/v1/t/{KEY}/incidents"),
        Some(&lead),
        Some(json!({ "title": "second look" })),
    )
    .await;
    assert_eq!(body["severity"], "medium", "severity defaults to medium");

    // Open count = 2; worst-first ordering surfaces the high one.
    let (s, page) = call(&app, "GET", &format!("/v1/t/{KEY}/incidents"), Some(&lead), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(page["open_count"], 2, "both are open (resolved_at is null)");
    assert_eq!(page["incidents"][0]["severity"], "high", "worst severity first");

    // Resolve the high one → open count drops to 1 and resolved_at is set.
    let (s, _) = call(
        &app,
        "PATCH",
        &format!("/v1/t/{KEY}/incidents/{hi_id}"),
        Some(&lead),
        Some(json!({ "resolved": true, "resolution": "false alarm" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, page) = call(&app, "GET", &format!("/v1/t/{KEY}/incidents"), Some(&lead), None).await;
    assert_eq!(page["open_count"], 1, "one resolved → one open");
    let resolved = page["incidents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == hi_id)
        .unwrap();
    assert_eq!(resolved["status"], "resolved");
    assert!(!resolved["resolved_at"].is_null(), "resolved_at stamped");
    assert_eq!(resolved["resolution"], "false alarm");

    // Delete the resolved one; deleting again → 404.
    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/incidents/{hi_id}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = call(
        &app,
        "DELETE",
        &format!("/v1/t/{KEY}/incidents/{hi_id}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

// ── artifact audit view: the non_dereferenced == 0 done-gate ─────────────────

/// Done-gate: for a client-work engagement whose artifacts are all stripped, the
/// audit view returns exactly the dereferenced rows and `non_dereferenced == 0`
/// (mirrors the spec's `jq` curl); the `dereferenced=true` filter returns the
/// same set.
#[tokio::test]
async fn audit_view_non_dereferenced_is_zero() {
    let (app, store, pool) = boot().await;
    let (t, _lead_user, lead) = lead_tenant(&store).await;

    let eng = store
        .create_engagement(&t, "Globex", None, None, None, None, None)
        .await
        .unwrap();

    // Three dereferenced client-work artifacts under the engagement.
    for i in 0..3 {
        insert_artifact(
            &pool,
            t,
            Some(eng),
            &format!("lesson {i}"),
            &format!("sig-{i}"),
            json!({ "dereferenced": true }),
            true,
            None,
        )
        .await;
    }

    // Unfiltered: every row present, all with strip info, non_dereferenced == 0.
    let (s, page) = call(
        &app,
        "GET",
        &format!("/v1/t/{KEY}/audit/artifacts?engagement={eng}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let rows = page.as_array().expect("audit view is a bare array");
    assert_eq!(rows.len(), 3);
    // Exactly the spec's `jq` gate.
    let non_dereferenced = rows
        .iter()
        .filter(|r| r["dereferenced"] == Value::Bool(false))
        .count();
    assert_eq!(non_dereferenced, 0, "done-gate: non_dereferenced == 0");
    // Every row carries strip verification (attribution) — no row without it.
    assert!(rows.iter().all(|r| r.get("attribution").is_some()));

    // The dereferenced=true filter returns the same three.
    let (_, page) = call(
        &app,
        "GET",
        &format!("/v1/t/{KEY}/audit/artifacts?engagement={eng}&dereferenced=true"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(page.as_array().unwrap().len(), 3);
}

/// A non-dereferenced client-work artifact surfaces in the audit view (so the
/// console renders the red-fail chip) AND blocks the compliance export.
#[tokio::test]
async fn non_dereferenced_artifact_surfaces_and_blocks_export() {
    let (app, store, pool) = boot().await;
    let (t, _lead_user, lead) = lead_tenant(&store).await;

    let eng = store
        .create_engagement(&t, "Globex", None, None, None, None, None)
        .await
        .unwrap();
    // Two clean + one leaking (dereferenced = false).
    insert_artifact(&pool, t, Some(eng), "ok-1", "s1", json!({}), true, None).await;
    insert_artifact(&pool, t, Some(eng), "ok-2", "s2", json!({}), true, None).await;
    insert_artifact(&pool, t, Some(eng), "leak", "s3", json!({}), false, None).await;

    // Audit view shows all three; one is non-dereferenced (the fail chip).
    let (_, page) = call(
        &app,
        "GET",
        &format!("/v1/t/{KEY}/audit/artifacts?engagement={eng}"),
        Some(&lead),
        None,
    )
    .await;
    let rows = page.as_array().unwrap();
    assert_eq!(rows.len(), 3);
    let non_dereferenced = rows
        .iter()
        .filter(|r| r["dereferenced"] == Value::Bool(false))
        .count();
    assert_eq!(non_dereferenced, 1);

    // Export is blocked (409) and names the offending count.
    let (s, body) = call(
        &app,
        "GET",
        &format!("/v1/t/{KEY}/compliance/export?engagement={eng}"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "a broken strip blocks export");
    assert_eq!(body["non_dereferenced"], 1);
}

// ── compliance export: emits the covered subset, leaks no source refs ────────

/// Done-gate: a clean engagement exports a CSV whose columns are the source-ref-
/// free subset — the export leaks NO source reference (`attribution` author/org,
/// `signature`, `contributed_by` are all absent), while the covered fields
/// (kind, title, dereferenced proof, engagement client) are present.
#[tokio::test]
async fn compliance_export_leaks_no_source_refs() {
    let (app, store, pool) = boot().await;
    let (t, _lead_user, lead) = lead_tenant(&store).await;

    let eng = store
        .create_engagement(&t, "Globex", None, None, None, None, None)
        .await
        .unwrap();
    let contributor = Uuid::new_v4();
    // A dereferenced artifact whose SOURCE fields carry unmistakable markers.
    insert_artifact(
        &pool,
        t,
        Some(eng),
        "General lesson: prefer clarity",
        "SECRETSIG",
        json!({ "author": "SECRETAUTHOR", "org": "SECRETORG", "dereferenced": true }),
        true,
        Some(contributor),
    )
    .await;

    let (s, csv, ctype) = get_raw(
        &app,
        &format!("/v1/t/{KEY}/compliance/export?engagement={eng}"),
        &lead,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(ctype.starts_with("text/csv"), "CSV content type, got {ctype}");

    // No source reference leaks — neither the values nor the column names.
    for leaked in [
        "SECRETAUTHOR",
        "SECRETORG",
        "SECRETSIG",
        &contributor.to_string(),
        "attribution",
        "signature",
        "contributed_by",
    ] {
        assert!(
            !csv.contains(leaked),
            "export must not leak source reference '{leaked}':\n{csv}"
        );
    }

    // The covered subset IS present (evidence of what was shared + that it held).
    assert!(csv.contains("General lesson: prefer clarity"), "title present");
    assert!(csv.contains("principle"), "kind present");
    assert!(csv.contains("Globex"), "engagement client present");
    assert!(csv.contains("dereferenced"), "strip-proof column present");

    // The JSON variant is likewise source-ref-free.
    let (s, page) = call(
        &app,
        "GET",
        &format!("/v1/t/{KEY}/compliance/export?engagement={eng}&format=json"),
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let raw = page.to_string();
    for leaked in ["SECRETAUTHOR", "SECRETORG", "SECRETSIG", "attribution", "signature"] {
        assert!(!raw.contains(leaked), "json export must not leak '{leaked}'");
    }
    assert_eq!(page["rows"][0]["dereferenced"], true);
}

// ── role floor: only client-lead+ reaches the console ────────────────────────

#[tokio::test]
async fn non_client_lead_rejected_and_lead_admin_allowed() {
    let (app, store, _pool) = boot().await;
    let (t, _lead_user, lead) = lead_tenant(&store).await;

    // Below the floor: a plain contributor JWT and an API-key caller (the API
    // plane never carries `Lead`, even an `admin` member key tops out at
    // Maintainer < Admin but > Lead — so exclude keys entirely, test a member
    // key which is below Lead).
    let contributor = member_jwt(&store, &t, "contributor").await;
    let member_key = key_for(&store, "Pull", "member").await;

    // At/above the floor: client_lead (exact), and — by the linear floor —
    // maintainer and admin also pass (documented consequence: `Lead < Maintainer
    // < Admin`, so higher-trust roles inherit read access to this console).
    let maintainer = member_jwt(&store, &t, "maintainer").await;
    let admin = member_jwt(&store, &t, "admin").await;

    let read_routes = [
        format!("/v1/t/{KEY}/engagements"),
        format!("/v1/t/{KEY}/incidents"),
        format!("/v1/t/{KEY}/audit/artifacts"),
        format!("/v1/t/{KEY}/compliance/export"),
    ];

    // Every below-floor caller is 403 on every read AND on a mutation.
    for bearer in [&contributor, &member_key] {
        for path in &read_routes {
            let (s, _) = call(&app, "GET", path, Some(bearer), None).await;
            assert_eq!(s, StatusCode::FORBIDDEN, "below-floor GET {path} must be 403");
        }
        let (s, _) = call(
            &app,
            "POST",
            &format!("/v1/t/{KEY}/engagements"),
            Some(bearer),
            Some(json!({ "client": "x" })),
        )
        .await;
        assert_eq!(s, StatusCode::FORBIDDEN, "below-floor POST engagements must be 403");
    }

    // client_lead + maintainer + admin all reach the console (reads succeed).
    for bearer in [&lead, &maintainer, &admin] {
        let (s, _) = call(&app, "GET", &format!("/v1/t/{KEY}/engagements"), Some(bearer), None).await;
        assert_eq!(s, StatusCode::OK, "at/above-floor caller reaches the console");
    }
}

#[tokio::test]
async fn client_lead_unknown_tenant_404_and_no_credential_401() {
    let (app, store, _pool) = boot().await;
    let (_t, _lead_user, lead) = lead_tenant(&store).await;

    // Unknown tenant → 404 (before auth).
    let (s, _) = call(
        &app,
        "GET",
        "/v1/t/github%2Fnope/engagements",
        Some(&lead),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    // No bearer → 401.
    let (s, _) = call(&app, "GET", &format!("/v1/t/{KEY}/engagements"), None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}
