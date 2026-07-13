//! `sensei-dojo provision` — bootstrap a Dōjō tenant + membership + API key.
//!
//! The multi-tenant artifact routes (`/v1/t/{tenant_key}/…`) authenticate a
//! daemon through the API-key plane in [`crate::auth`]. Standing up a new tenant
//! Dōjō therefore needs three things wired together: the tenant row (the
//! isolation boundary), a membership (the user's enrolment record the
//! Supabase-JWT plane resolves against), and an API key whose member role maps to
//! the desired dojo authority. This is the bootstrap the admin console will
//! eventually own; until then the CLI mints them directly against the embedded
//! PG.
//!
//! Everything here REUSES the shipped store + keygen helpers (DRY): tenant
//! resolve/create ([`DojoStore::resolve_tenant`] / [`DojoStore::create_tenant`]),
//! membership create ([`DojoStore::create_membership`]), and
//! [`crate::keygen::generate_key`] for the key — so the token hashes and
//! verifies through the exact same sha256 path the dojo API-key plane
//! ([`crate::auth::authenticate_dojo`]) checks.

use crate::store::DojoStore;
use uuid::Uuid;

/// A provisioned tenant + membership + API key. The `token` plaintext is shown
/// once (only its sha256 hash is stored) — mirrors [`crate::store::IssuedKey`].
#[derive(Debug, Clone)]
pub struct Provisioned {
    /// Canonical discovery path `"<origin>/<org>[/<dojo>]"` (the tenant key).
    pub tenant_key: String,
    pub tenant_id: Uuid,
    pub membership_id: Uuid,
    /// The API-key member role the token was minted with (maps to the requested
    /// dojo role's [`crate::auth::DojoAccess`]).
    pub member_role: &'static str,
    /// Plaintext bearer token — shown once; the daemon presents it as
    /// `Authorization: Bearer <token>` on the tenant's dojo routes.
    pub token: String,
}

/// Map a requested dojo role onto the API-key member role whose
/// [`crate::auth::DojoAccess`] matches — so a key minted here carries the right
/// authority on the tenant's dojo routes (see `auth::member_role_to_access`):
///
/// * `maintainer` / `admin` → member `admin` → `DojoAccess::Maintainer` (triage).
/// * `contributor`          → member `publisher` → `DojoAccess::Contributor`.
///
/// Returns `(dojo_role, member_role)` on success, or an error naming the accepted
/// roles. The `dojo_role` is echoed back (validated) for the membership row.
pub fn roles_for(dojo_role: &str) -> Result<(&'static str, &'static str), String> {
    match dojo_role {
        "contributor" => Ok(("contributor", "publisher")),
        "maintainer" => Ok(("maintainer", "admin")),
        "admin" => Ok(("admin", "admin")),
        other => Err(format!(
            "invalid role '{other}' (contributor|maintainer|admin)"
        )),
    }
}

/// Build the canonical tenant discovery key `"<origin>/<org>[/<dojo>]"`.
pub fn tenant_key(origin: &str, org: &str, dojo: Option<&str>) -> String {
    match dojo {
        Some(d) => format!("{origin}/{org}/{d}"),
        None => format!("{origin}/{org}"),
    }
}

/// Default membership `kind` for a provisioned member, derived from the tenant
/// scope: the public `global` collective is joined as `community`; a normal
/// private org Dōjō enrols members as `employer`.
pub fn default_kind(scope: &str) -> &'static str {
    match scope {
        "global" => "community",
        _ => "employer",
    }
}

/// Provision a tenant + membership + API key against the service's embedded PG.
///
/// Idempotent on the tenant — an existing `tenant_key` is reused (never
/// duplicated), and the requested `scope` is (re)applied. Always creates a
/// fresh membership + API key. Returns the tenant key/id, the membership id, and
/// the plaintext token (shown once).
pub async fn provision(
    store: &DojoStore,
    origin: &str,
    org: &str,
    dojo: Option<&str>,
    user: &str,
    role: &str,
    scope: &str,
) -> Result<Provisioned, String> {
    if !matches!(origin, "github" | "org") {
        return Err(format!("invalid origin '{origin}' (github|org)"));
    }
    if !matches!(scope, "private" | "global") {
        return Err(format!("invalid scope '{scope}' (private|global)"));
    }
    let (dojo_role, member_role) = roles_for(role)?;

    let key = tenant_key(origin, org, dojo);
    let name = dojo.unwrap_or(org).to_string();
    let dojo_url = format!("dojo.sensei-hq.org/{key}");

    // Tenant: idempotent — reuse an existing tenant, else create it (reusing the
    // shipped resolve/create store methods rather than a second insert path).
    let tenant_id = match store.resolve_tenant(&key).await? {
        Some(id) => id,
        None => {
            store
                .create_tenant(&key, origin, org, dojo, &name, &dojo_url)
                .await?
        }
    };
    // `create_tenant` lands at the DDL-default scope (`private`); apply the
    // requested scope explicitly so `--scope global` takes effect (and a
    // re-provision keeps an existing tenant's scope in sync).
    store.set_tenant_scope(&tenant_id, scope).await?;

    // Membership: the user's enrolment record (the Supabase-JWT plane resolves
    // against it). `user_id` is a fresh subject placeholder — the daemon
    // authenticates with the API key below, not this membership; the CLI is a
    // device-code pairing, so `authenticated_via = device_code`.
    let user_id = Uuid::new_v4();
    let kind = default_kind(scope);
    let membership_id = store
        .create_membership(&tenant_id, &user_id, kind, "device_code", dojo_role)
        .await?;

    // API key: minted through the SAME keygen path the daemon's API-key plane
    // verifies (sha256 hash stored; plaintext shown once). The member role gives
    // it the mapped DojoAccess on the tenant's dojo routes.
    let label = format!("provision:{key}:{user}");
    let token = crate::keygen::generate_key(store, user, member_role, Some(&label)).await?;

    Ok(Provisioned {
        tenant_key: key,
        tenant_id,
        membership_id,
        member_role,
        token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_for_maps_dojo_role_to_member_role() {
        assert_eq!(roles_for("contributor").unwrap(), ("contributor", "publisher"));
        assert_eq!(roles_for("maintainer").unwrap(), ("maintainer", "admin"));
        assert_eq!(roles_for("admin").unwrap(), ("admin", "admin"));
    }

    #[test]
    fn roles_for_rejects_unknown_role() {
        let err = roles_for("wizard").unwrap_err();
        assert!(err.contains("wizard"), "err names the bad role: {err}");
        assert!(
            err.contains("contributor|maintainer|admin"),
            "err lists the accepted roles: {err}"
        );
    }

    #[test]
    fn tenant_key_formats_with_and_without_dojo() {
        assert_eq!(tenant_key("github", "sensei-hq", None), "github/sensei-hq");
        assert_eq!(
            tenant_key("github", "sensei-hq", Some("client-a")),
            "github/sensei-hq/client-a"
        );
    }

    #[test]
    fn default_kind_follows_scope() {
        assert_eq!(default_kind("global"), "community");
        assert_eq!(default_kind("private"), "employer");
    }
}
