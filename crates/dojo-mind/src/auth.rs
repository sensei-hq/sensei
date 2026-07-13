//! Bearer-token auth + role-floor enforcement.
//!
//! Two authentication planes coexist here (all additive to the shipped
//! API-key path):
//!
//! * **API-key plane** (`require`, [`Role`]) — the original sha256 bearer-key
//!   middleware, unchanged. Guards the `/v1/rules` / `/v1/members` federation
//!   routes AND is accepted on the new tenant-scoped `/v1/t/…` dojo routes
//!   (daemon / federation traffic).
//! * **Supabase-JWT plane** ([`verify_supabase_jwt`]) — human / console traffic
//!   on the dojo routes. A request to a dojo route authenticates if EITHER a
//!   valid API key OR a valid Supabase JWT is presented; the dual check lives in
//!   [`authenticate_dojo`].

use crate::api::AppState;
use crate::store::DojoStore;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Member = 0,
    Publisher = 1,
    Admin = 2,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "member" => Some(Role::Member),
            "publisher" => Some(Role::Publisher),
            "admin" => Some(Role::Admin),
            _ => None,
        }
    }
}

/// True when `have` meets-or-exceeds the required `floor`.
pub fn role_satisfies(have: Role, floor: Role) -> bool {
    have >= floor
}

/// The resolved caller, attached to the request by `require`.
#[derive(Clone)]
pub struct AuthCaller {
    pub member_id: uuid::Uuid,
    pub name: String,
    pub role: Role,
}

/// Extract the `Bearer <token>` value from an Authorization header map.
/// Shared by the API-key middleware and the dojo dual-auth path (DRY).
pub fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

fn bearer(req: &Request) -> Option<String> {
    token_from_headers(req.headers())
}

/// Middleware: resolve the bearer key → 401 if invalid; attach `AuthCaller`.
pub async fn require(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let key = bearer(&req).ok_or(StatusCode::UNAUTHORIZED)?;
    let caller = state
        .store
        .find_member_by_key(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let role = Role::parse(&caller.role).ok_or(StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(AuthCaller {
        member_id: caller.member_id,
        name: caller.name,
        role,
    });
    Ok(next.run(req).await)
}

// ── Supabase-JWT plane (dojo routes) ─────────────────────────────────────────

/// The Supabase local-development JWT secret (the value the Supabase CLI ships
/// in `config.toml`). Used as the default when `SUPABASE_JWT_SECRET` is unset —
/// tests sign synthetic tokens with it, so no running Supabase is needed.
pub const DEFAULT_SUPABASE_JWT_SECRET: &str =
    "super-secret-jwt-token-with-at-least-32-characters-long";

/// The audience Supabase stamps on user access tokens.
pub const DEFAULT_SUPABASE_JWT_AUD: &str = "authenticated";

/// Verification parameters for the Supabase-JWT plane. Injected into the router
/// (see `api::build_router_with_jwt`); production overrides the secret/audience
/// from env, tests inject a known secret to sign synthetic tokens.
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: DEFAULT_SUPABASE_JWT_SECRET.to_string(),
            audience: DEFAULT_SUPABASE_JWT_AUD.to_string(),
        }
    }
}

/// The subset of a Supabase access token we consume. `sub` is the stable user
/// id (matched against `dojo.memberships.user_id`); `email` / `role` are carried
/// through when present. `exp` / `aud` back the signature-independent checks
/// (expiry + audience) enforced by [`verify_supabase_jwt`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupabaseClaims {
    pub sub: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    pub exp: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
}

/// Verify a Supabase HS256 JWT against `secret`, enforcing expiry and the
/// expected `audience`. Returns the decoded claims on success; any failure
/// (bad signature, expired, wrong audience, missing `sub`/`exp`/`aud`,
/// malformed) is a single `Err` the caller maps to 401. Pure + synchronous, so
/// it is unit-testable with synthetic tokens (no running Supabase).
pub fn verify_supabase_jwt(
    token: &str,
    secret: &str,
    audience: &str,
) -> Result<SupabaseClaims, jsonwebtoken::errors::Error> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&[audience]);
    // Require the registered claims we rely on to be present (exp/aud validated
    // above; sub is what we resolve the membership by).
    validation.set_required_spec_claims(&["exp", "aud", "sub"]);
    let data = decode::<SupabaseClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

// ── Dojo dual-auth (tenant-scoped) ───────────────────────────────────────────

/// Authority a caller carries on a tenant's dojo routes. `Member` may pull;
/// `Contributor` may additionally publish; `Lead` may additionally run the
/// client-lead console (engagements / audit / incidents / compliance export);
/// `Maintainer` may additionally triage (list the queue, decide, run the
/// promotion sweep); `Admin` may additionally run the admin console (members /
/// identities / policies / health / audit).
/// Ordered so a floor check is a comparison, mirroring [`Role`]/[`role_satisfies`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DojoAccess {
    /// Read-only: pull artifacts (`member+`).
    Member = 0,
    /// Read-write: publish + pull (`contributor+`).
    Contributor = 1,
    /// Client-confidentiality authority: run the client-lead console —
    /// register/bind engagements, audit the universal dereference strip, handle
    /// incidents, and export compliance evidence (`client_lead`). Sits ABOVE
    /// `Contributor` (a plain contributor cannot register engagements or export
    /// compliance data — the console's role-floor 403) and deliberately BELOW
    /// `Maintainer` (a client-lead is NOT a triage role — it never gains
    /// queue/decide/promote authority; see `dojo_role_to_access` + the
    /// `dojo.member_role` DDL comment). Reachable only via the Supabase-JWT plane
    /// with the `client_lead` dojo membership role — the console is human / SSO
    /// traffic (the API-key plane never mints it, mirroring `Admin`).
    Lead = 2,
    /// Triage authority: list/decide/promote the queue (`maintainer+`). Maps
    /// from the `maintainer` dojo membership role and the `admin` API-key member
    /// role (the API-key plane's ceiling — see `member_role_to_access`).
    Maintainer = 3,
    /// Admin-console authority: provision members, wire identities, set
    /// policies, read health + audit (`admin`). Reachable ONLY via the
    /// Supabase-JWT plane with the `admin` dojo membership role — the admin
    /// console is human / SSO traffic. The API-key plane deliberately tops out
    /// at `Maintainer` (a daemon key never carries console-admin authority, and
    /// provisioning squashes `maintainer`/`admin` onto one API-key role).
    Admin = 4,
}

/// Which plane authenticated a dojo caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DojoAuthVia {
    /// A service API key (daemon / federation traffic).
    ApiKey,
    /// A Supabase JWT resolved to a tenant membership (console traffic).
    Supabase,
}

/// The resolved caller on a tenant-scoped dojo route.
#[derive(Debug, Clone)]
pub struct DojoCaller {
    /// The tenant the caller is acting within (resolved from the path).
    pub tenant_id: Uuid,
    /// Stable subject id — the member id (API-key plane) or the Supabase
    /// `sub` (JWT plane). Recorded as the artifact's `contributed_by`.
    pub subject: String,
    /// Human-facing label (member name or JWT email/sub), for audit.
    pub display: String,
    pub access: DojoAccess,
    pub via: DojoAuthVia,
}

/// Why a dojo auth attempt failed — mapped to a status code by the handler.
#[derive(Debug)]
pub enum DojoAuthError {
    /// No usable credential: missing bearer, or neither a valid API key nor a
    /// valid JWT. → 401.
    Unauthenticated,
    /// A valid JWT, but the subject has no membership in this tenant. → 403.
    Forbidden,
    /// A backend failure while resolving the credential. → 500 (never silent).
    Internal(String),
}

/// Map an API-key member role onto dojo access: `member` pulls; `publisher`
/// contributes; `admin` additionally maintains (triage). An unparseable role
/// falls back to the least-privilege `Member`. The API-key plane tops out at
/// `Maintainer` on purpose — console-admin authority (`DojoAccess::Admin`) is
/// human / SSO traffic, reached only through the JWT plane (see
/// `dojo_role_to_access`). Provisioning also squashes the `maintainer`/`admin`
/// dojo roles onto this one API-key role, so an API key can't be trusted to
/// distinguish them.
fn member_role_to_access(role: &str) -> DojoAccess {
    match Role::parse(role) {
        Some(Role::Admin) => DojoAccess::Maintainer,
        Some(Role::Publisher) => DojoAccess::Contributor,
        Some(Role::Member) | None => DojoAccess::Member,
    }
}

/// Map a `dojo.member_role` string (JWT plane) onto dojo access: `admin` runs
/// the admin console (`Admin`); `maintainer` triages (`Maintainer`);
/// `client_lead` runs the client-lead console (`Lead`); `contributor`
/// contributes; anything unrecognised falls back to least-privilege `Member`.
/// (`client_lead` guards client confidentiality — it is a dedicated console
/// role, NOT a triage role: `Lead` sits below `Maintainer` so it never inherits
/// queue/promote authority.) Only this JWT plane can grant `Lead`/`Admin`, so
/// both consoles are gated to a human with the matching dojo membership role.
fn dojo_role_to_access(role: &str) -> DojoAccess {
    match role {
        "admin" => DojoAccess::Admin,
        "maintainer" => DojoAccess::Maintainer,
        "client_lead" => DojoAccess::Lead,
        "contributor" => DojoAccess::Contributor,
        _ => DojoAccess::Member,
    }
}

/// Dual-auth for the dojo routes: accept EITHER a valid service API key OR a valid
/// Supabase JWT bound to a membership in `tenant_id`.
///
/// Order: API-key first (federation traffic), then Supabase JWT. A JWT that
/// verifies but resolves no membership in this tenant is `Forbidden` (the human
/// is authenticated but not a member here). Any other missing/invalid
/// credential is `Unauthenticated`.
pub async fn authenticate_dojo(
    store: &DojoStore,
    jwt: &JwtConfig,
    headers: &HeaderMap,
    tenant_id: Uuid,
) -> Result<DojoCaller, DojoAuthError> {
    let token = token_from_headers(headers).ok_or(DojoAuthError::Unauthenticated)?;

    // Plane 1 — API key (daemon / federation). Same sha256 lookup as `require`.
    if let Some(c) = store
        .find_member_by_key(&token)
        .await
        .map_err(DojoAuthError::Internal)?
    {
        return Ok(DojoCaller {
            tenant_id,
            subject: c.member_id.to_string(),
            display: c.name,
            access: member_role_to_access(&c.role),
            via: DojoAuthVia::ApiKey,
        });
    }

    // Plane 2 — Supabase JWT (console). Verify, then resolve membership+role.
    let claims = verify_supabase_jwt(&token, &jwt.secret, &jwt.audience)
        .map_err(|_| DojoAuthError::Unauthenticated)?;
    match store
        .find_membership_role(&tenant_id, &claims.sub)
        .await
        .map_err(DojoAuthError::Internal)?
    {
        // Every dojo membership role (contributor+) may contribute; maintainer/
        // admin additionally carry triage authority (see `dojo_role_to_access`).
        Some(role) => Ok(DojoCaller {
            tenant_id,
            subject: claims.sub.clone(),
            display: claims.email.unwrap_or(claims.sub),
            access: dojo_role_to_access(&role),
            via: DojoAuthVia::Supabase,
        }),
        None => Err(DojoAuthError::Forbidden),
    }
}
