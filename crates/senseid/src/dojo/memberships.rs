//! Daemon-side Dōjō connection model + CRUD orchestration.
//!
//! A "connection" is the daemon's local record of one Dōjō it is a member of.
//! Fork 1: the authoritative `dojo.memberships` row lives in the Dōjō service's
//! own Postgres; this module manages the local mirror
//! (`sensei.dojo_memberships`) and the Keychain credential, mirroring the
//! federation `knowledge_sources` discipline — the device token is stored in the
//! OS Keychain via [`crate::gateway_keys`], NEVER in Postgres.
//!
//! Raw SQL lives in [`crate::db::pg_store`] (next to `knowledge_sources`); this
//! module owns the domain types, input validation, and the create/list/bind/
//! set-sync-status orchestration the API handlers call.

use crate::api::util::require_member_of;
use crate::db::pg_store::{DojoMembership, NewDojoMembership, PgStore};
pub use dojo_protocol::AttributionMode;

/// The relationship a membership represents — the daemon-side analogue of the
/// `dojo.membership_kind` enum. The `dojo` schema (and its enum types) does NOT
/// exist in the daemon's `sensei` DB (Fork 1 — the `default` scope excludes it),
/// so the local mirror stores `kind` as `text` and this enum is the typed view
/// used by routing. `kind` drives client-precedence routing (see
/// [`crate::dojo::routing`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipKind {
    Employer,
    Client,
    Community,
    Personal,
}

impl MembershipKind {
    pub const ALL: [MembershipKind; 4] = [
        MembershipKind::Employer,
        MembershipKind::Client,
        MembershipKind::Community,
        MembershipKind::Personal,
    ];

    /// The `dojo.membership_kind` enum string. MUST match `membership_kind.ddl`.
    pub fn as_db_str(&self) -> &'static str {
        match self {
            MembershipKind::Employer => "employer",
            MembershipKind::Client => "client",
            MembershipKind::Community => "community",
            MembershipKind::Personal => "personal",
        }
    }

    /// Parse a `dojo.membership_kind` enum string.
    pub fn from_db_str(s: &str) -> Option<Self> {
        MembershipKind::ALL.into_iter().find(|k| k.as_db_str() == s)
    }
}

/// Allowed `role` values — mirrors `dojo.member_role` (member_role.ddl).
pub const MEMBER_ROLES: [&str; 4] = ["contributor", "maintainer", "lead", "admin"];
/// Allowed `authenticated_via` values — mirrors `dojo.auth_method` (auth_method.ddl).
pub const AUTH_METHODS: [&str; 3] = ["sso", "github_oauth", "device_code"];
/// Allowed `sync_status` values — mirrors `dojo.sync_status` (sync_status.ddl).
pub const SYNC_STATUSES: [&str; 4] = ["healthy", "stale", "error", "authenticating"];

/// Normalise a set of git-remote owner slugs for storage/matching: lowercase,
/// trim, drop empties, and dedupe while preserving first-seen order. Pure — the
/// single source of truth for org-slug canonicalisation (both the connect body
/// and the org-tagging edit route through this, and `infer_binding` matches
/// against lowercased owners, so the two sides always agree). See
/// [`crate::dojo::routing::infer_binding`].
pub fn normalize_org_slugs<I, S>(slugs: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out: Vec<String> = Vec::new();
    for s in slugs {
        let norm = s.as_ref().trim().to_ascii_lowercase();
        if !norm.is_empty() && !out.contains(&norm) {
            out.push(norm);
        }
    }
    out
}

/// Derive the full membership URL from the registry base + tenant discovery
/// path, e.g. `("http://localhost:7755", "github/sensei-hq")` →
/// `"http://localhost:7755/github/sensei-hq"`. Pure.
pub fn derive_dojo_url(registry_url: &str, tenant_key: &str) -> String {
    format!("{}/{}", registry_url.trim_end_matches('/'), tenant_key.trim_start_matches('/'),)
}

/// The auth-gated Dōjō run-detail URL for a run — `<registry_url>/you/runs/<id>`.
/// The `/you/*` web routes are guarded by the Dōjō's sign-in (kavach), so the link
/// is a clean handoff: open it, sign in, watch the run. Pure — same origin the run
/// federates to (mirrors [`derive_dojo_url`]).
pub fn dojo_run_url(registry_url: &str, run_id: &uuid::Uuid) -> String {
    format!("{}/you/runs/{}", registry_url.trim_end_matches('/'), run_id)
}

/// A validated new connection — everything but the Keychain credential, which
/// [`register`] mints. All enum-typed fields are already checked.
#[derive(Debug)]
pub struct NewConnection {
    /// The service membership id (`dojo.memberships.id`) — becomes the local PK
    /// and the value `sensei.projects.dojo_id` points at.
    pub membership_id: uuid::Uuid,
    pub registry_url: String,
    pub tenant_key: String,
    pub dojo_url: String,
    pub kind: MembershipKind,
    /// Git-remote owner slugs this membership covers (normalised lowercase),
    /// e.g. `["sensei-hq", "acme"]`. Feeds infer-at-detect auto-bind.
    pub org_slugs: Vec<String>,
    pub role: String,
    pub authenticated_via: String,
    pub attribution_default: AttributionMode,
    pub sync_status: String,
}

impl NewConnection {
    /// Validate raw string fields (from an API body) into a typed connection.
    /// `registry_url`/`tenant_key`/`dojo_url` are trusted trimmed strings; the
    /// caller enforces URL-scheme security (see `api::util::require_secure_url`).
    /// `org_slugs` is normalised via [`normalize_org_slugs`].
    #[allow(clippy::too_many_arguments)]
    pub fn validated(
        membership_id: uuid::Uuid,
        registry_url: String,
        tenant_key: String,
        dojo_url: String,
        kind: &str,
        org_slugs: &[String],
        role: &str,
        authenticated_via: &str,
        attribution_default: &str,
        sync_status: &str,
    ) -> Result<Self, String> {
        if tenant_key.trim().is_empty() {
            return Err("tenant_key must not be empty".to_string());
        }
        let kind = MembershipKind::from_db_str(kind).ok_or_else(|| {
            format!("invalid kind: {kind:?} (allowed: employer, client, community, personal)")
        })?;
        require_member_of(role, &MEMBER_ROLES, "role")?;
        require_member_of(authenticated_via, &AUTH_METHODS, "authenticated_via")?;
        require_member_of(sync_status, &SYNC_STATUSES, "sync_status")?;
        let attribution_default = AttributionMode::from_db_str(attribution_default)
            .ok_or_else(|| format!("invalid attribution_default: {attribution_default:?} (allowed: named, anonymous)"))?;
        Ok(Self {
            membership_id,
            registry_url,
            tenant_key,
            dojo_url,
            kind,
            org_slugs: normalize_org_slugs(org_slugs),
            role: role.to_string(),
            authenticated_via: authenticated_via.to_string(),
            attribution_default,
            sync_status: sync_status.to_string(),
        })
    }
}

/// Register a Dōjō connection: mint a Keychain credential_ref, store the device
/// `token` in the OS Keychain (never in Postgres), then insert the local mirror
/// row. If the row insert fails the Keychain entry is rolled back so no orphan
/// secret is left behind. Returns the membership id (the local PK).
pub async fn register(
    pg: &PgStore,
    conn: NewConnection,
    token: &str,
) -> Result<uuid::Uuid, String> {
    if token.trim().is_empty() {
        return Err("credential (device token) must not be empty".to_string());
    }
    let credential_ref = format!("dojo-{}", uuid::Uuid::new_v4());

    // Keychain write is blocking (shells out to /usr/bin/security).
    let cref = credential_ref.clone();
    let tok = token.to_string();
    tokio::task::spawn_blocking(move || crate::gateway_keys::set_key(&cref, &tok))
        .await
        .map_err(|e| format!("keychain task join failed: {e}"))?
        .map_err(|e| format!("keychain store failed: {e}"))?;

    let row = NewDojoMembership {
        id: conn.membership_id,
        registry_url: conn.registry_url,
        tenant_key: conn.tenant_key,
        dojo_url: conn.dojo_url,
        kind: conn.kind.as_db_str().to_string(),
        org_slugs: conn.org_slugs,
        role: conn.role,
        authenticated_via: conn.authenticated_via,
        attribution_default: conn.attribution_default.as_db_str().to_string(),
        credential_ref: credential_ref.clone(),
        sync_status: conn.sync_status,
    };
    if let Err(e) = pg.create_dojo_membership(&row).await {
        // Roll back the Keychain entry so a failed insert leaves no orphan token.
        let cref = credential_ref.clone();
        if let Err(del) =
            tokio::task::spawn_blocking(move || crate::gateway_keys::delete_key(&cref)).await
        {
            tracing::warn!(error = %del, "dojo: keychain rollback task join failed after insert error");
        }
        return Err(e);
    }
    Ok(conn.membership_id)
}

/// Bind a local project to a Dōjō membership (sets `sensei.projects.dojo_id`).
/// A project binds to exactly one membership. Returns `false` if the project is
/// unknown. Client-membership bindings take precedence at routing time (see
/// [`crate::dojo::routing`]).
pub async fn bind_project(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    membership_id: &uuid::Uuid,
) -> Result<bool, String> {
    pg.bind_project_to_dojo(project_id, Some(membership_id)).await
}

/// Replace the git-remote owner slugs a membership covers (the org-tagging
/// edit). Slugs are normalised via [`normalize_org_slugs`] before storage so
/// they match `infer_binding`'s lowercased owners. Returns `false` if unknown.
pub async fn set_orgs(
    pg: &PgStore,
    membership_id: &uuid::Uuid,
    org_slugs: &[String],
) -> Result<bool, String> {
    pg.set_dojo_membership_orgs(membership_id, &normalize_org_slugs(org_slugs)).await
}

/// Update a connection's sync status (validated). Returns `false` if unknown.
///
/// Forward seam: its first non-test caller is the C5 heartbeat/sync-status
/// derivation — allowed until then (the underlying `set_dojo_sync_status` is
/// exercised by the pg_store CRUD roundtrip test).
#[allow(dead_code)]
pub async fn set_sync_status(
    pg: &PgStore,
    membership_id: &uuid::Uuid,
    status: &str,
) -> Result<bool, String> {
    require_member_of(status, &SYNC_STATUSES, "sync_status")?;
    pg.set_dojo_sync_status(membership_id, status).await
}

/// A SUGGESTED project→Dōjō binding for the About-panel confirm chip (the
/// enriched form of [`crate::dojo::routing::InferredBinding`] with the display
/// fields the chip needs). Confirm-inferred: surfaced for the user to accept;
/// binding is only written when they confirm (`POST …/dojo-binding`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct BindingSuggestion {
    pub membership_id: uuid::Uuid,
    /// employer | client | community | personal.
    pub kind: String,
    /// The project's git-remote owner slug that matched the membership.
    pub matched_slug: String,
    pub tenant_key: String,
    pub dojo_url: String,
}

/// Infer a project→Dōjō binding from the project's git-remote owner(s), for the
/// About-panel confirm chip. Returns `None` when the project is already bound
/// (no suggestion needed), has no git owner, or no connected membership covers
/// its owner. Pure inference lives in [`crate::dojo::routing::infer_binding`];
/// this only gathers the inputs and enriches the result for display — it never
/// writes the binding (that is the user-confirmed [`bind_project`]).
pub async fn suggest_binding(
    pg: &PgStore,
    project_id: &uuid::Uuid,
) -> Result<Option<BindingSuggestion>, String> {
    use crate::dojo::routing::{BindCandidate, infer_binding};

    // Already bound → the chip doesn't apply.
    if pg.project_bound_membership(project_id).await?.is_some() {
        return Ok(None);
    }
    let owners = pg.project_org_owners(project_id).await?;
    if owners.is_empty() {
        return Ok(None);
    }
    let rows = pg.list_dojo_memberships().await?;
    let candidates: Vec<BindCandidate> = rows
        .iter()
        .filter_map(|r| {
            MembershipKind::from_db_str(&r.kind).map(|kind| BindCandidate {
                membership_id: r.id,
                kind,
                org_slugs: r.org_slugs.clone(),
            })
        })
        .collect();

    Ok(infer_binding(&owners, &candidates).map(|b| {
        // Enrich with display fields from the matched membership row.
        let row = rows.iter().find(|r| r.id == b.membership_id);
        BindingSuggestion {
            membership_id: b.membership_id,
            kind: b.kind.as_db_str().to_string(),
            matched_slug: b.matched_slug,
            tenant_key: row.map(|r| r.tenant_key.clone()).unwrap_or_default(),
            dojo_url: row.map(|r| r.dojo_url.clone()).unwrap_or_default(),
        }
    }))
}

/// A local project bound to a membership — for the connections pane's
/// "bound projects" strip.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BoundProject {
    pub id: uuid::Uuid,
    pub name: String,
}

/// One connection as surfaced by `GET /api/dojo/memberships`. Deliberately omits
/// `credential_ref` — the Keychain reference is never exposed over the API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionView {
    pub id: uuid::Uuid,
    pub registry_url: String,
    pub tenant_key: String,
    pub dojo_url: String,
    pub kind: String,
    /// Git-remote owner slugs this membership covers — surfaced so the
    /// connections pane can display and edit the org-tagging.
    pub org_slugs: Vec<String>,
    pub role: String,
    pub authenticated_via: String,
    pub attribution_default: String,
    pub sync_status: String,
    pub last_seq: i64,
    pub last_heartbeat_at: Option<String>,
    pub enabled: bool,
    pub bound_projects: Vec<BoundProject>,
}

impl ConnectionView {
    fn from_row(row: DojoMembership, bound_projects: Vec<BoundProject>) -> Self {
        Self {
            id: row.id,
            registry_url: row.registry_url,
            tenant_key: row.tenant_key,
            dojo_url: row.dojo_url,
            kind: row.kind,
            org_slugs: row.org_slugs,
            role: row.role,
            authenticated_via: row.authenticated_via,
            attribution_default: row.attribution_default,
            sync_status: row.sync_status,
            last_seq: row.last_seq,
            last_heartbeat_at: row.last_heartbeat_at,
            enabled: row.enabled,
            bound_projects,
        }
    }
}

/// List every Dōjō connection with its bound projects — the
/// `GET /api/dojo/memberships` payload.
pub async fn list(pg: &PgStore) -> Result<Vec<ConnectionView>, String> {
    let rows = pg.list_dojo_memberships().await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let bound = pg
            .projects_bound_to_dojo(&row.id)
            .await?
            .into_iter()
            .map(|(id, name)| BoundProject { id, name })
            .collect();
        out.push(ConnectionView::from_row(row, bound));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn membership_kind_db_strings_match_ddl_and_round_trip() {
        // MUST match enum/dojo/membership_kind.ddl exactly.
        let expected = ["employer", "client", "community", "personal"];
        let got: Vec<&str> = MembershipKind::ALL.iter().map(|k| k.as_db_str()).collect();
        assert_eq!(got, expected);
        for k in MembershipKind::ALL {
            assert_eq!(MembershipKind::from_db_str(k.as_db_str()), Some(k));
        }
        assert_eq!(MembershipKind::from_db_str("nope"), None);
    }

    #[test]
    fn derive_dojo_url_joins_registry_and_tenant_without_double_slash() {
        assert_eq!(
            derive_dojo_url("http://localhost:7755", "github/sensei-hq"),
            "http://localhost:7755/github/sensei-hq"
        );
        assert_eq!(
            derive_dojo_url("http://localhost:7755/", "/github/sensei-hq"),
            "http://localhost:7755/github/sensei-hq"
        );
    }

    #[test]
    fn dojo_run_url_builds_the_auth_gated_run_detail_link() {
        let id = uuid::Uuid::nil();
        assert_eq!(
            dojo_run_url("http://localhost:5173", &id),
            format!("http://localhost:5173/you/runs/{id}")
        );
        // trailing slash trimmed (no double slash before /you).
        assert_eq!(
            dojo_run_url("https://dojo.sensei-hq.com/", &id),
            format!("https://dojo.sensei-hq.com/you/runs/{id}")
        );
    }

    #[test]
    fn normalize_org_slugs_lowercases_trims_dedupes_and_drops_empties() {
        assert_eq!(
            normalize_org_slugs(["Sensei-HQ", " acme ", "acme", "", "  ", "ACME"]),
            vec!["sensei-hq".to_string(), "acme".to_string()],
        );
        assert!(normalize_org_slugs(Vec::<String>::new()).is_empty());
    }

    #[test]
    fn new_connection_validates_every_enum_field() {
        let id = uuid::Uuid::new_v4();
        let ok = NewConnection::validated(
            id,
            "http://localhost:7755".into(),
            "github/acme".into(),
            "http://localhost:7755/github/acme".into(),
            "client",
            &["Acme".into(), "acme".into()],
            "contributor",
            "device_code",
            "anonymous",
            "authenticating",
        );
        assert!(ok.is_ok());
        let c = ok.unwrap();
        assert_eq!(c.kind, MembershipKind::Client);
        assert_eq!(c.attribution_default, AttributionMode::Anonymous);
        assert_eq!(
            c.org_slugs,
            vec!["acme".to_string()],
            "org_slugs normalised (lowercased + deduped)"
        );

        // Each bad enum field is rejected with a field-named error.
        let bad_kind = NewConnection::validated(
            id,
            "u".into(),
            "t".into(),
            "d".into(),
            "boss",
            &[],
            "contributor",
            "device_code",
            "named",
            "healthy",
        );
        assert!(bad_kind.unwrap_err().contains("kind"));
        let bad_role = NewConnection::validated(
            id,
            "u".into(),
            "t".into(),
            "d".into(),
            "client",
            &[],
            "overlord",
            "device_code",
            "named",
            "healthy",
        );
        assert!(bad_role.unwrap_err().contains("role"));
        let bad_auth = NewConnection::validated(
            id,
            "u".into(),
            "t".into(),
            "d".into(),
            "client",
            &[],
            "contributor",
            "carrier_pigeon",
            "named",
            "healthy",
        );
        assert!(bad_auth.unwrap_err().contains("authenticated_via"));
        let bad_attr = NewConnection::validated(
            id,
            "u".into(),
            "t".into(),
            "d".into(),
            "client",
            &[],
            "contributor",
            "device_code",
            "public",
            "healthy",
        );
        assert!(bad_attr.unwrap_err().contains("attribution_default"));
        let bad_sync = NewConnection::validated(
            id,
            "u".into(),
            "t".into(),
            "d".into(),
            "client",
            &[],
            "contributor",
            "device_code",
            "named",
            "vibing",
        );
        assert!(bad_sync.unwrap_err().contains("sync_status"));
        let empty_tenant = NewConnection::validated(
            id,
            "u".into(),
            "  ".into(),
            "d".into(),
            "client",
            &[],
            "contributor",
            "device_code",
            "named",
            "healthy",
        );
        assert!(empty_tenant.unwrap_err().contains("tenant_key"));
    }
}
