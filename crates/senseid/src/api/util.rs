//! Shared utilities for API handlers.

/// Validate a federation / Dōjō endpoint URL before a bearer token is ever sent
/// to it: parse it and require `https` unless the host is *exactly* a loopback
/// (so `http://127.0.0.1.evil.com` can't smuggle the token to an attacker host
/// in cleartext). `resource` names the field for the error message (e.g.
/// `"source url"`, `"registry url"`). Returns the parsed URL or a
/// human-readable reason. Shared by the federation and Dōjō registration paths.
pub(crate) fn require_secure_url(raw: &str, resource: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(raw).map_err(|_| format!("invalid {resource}"))?;
    let is_loopback = matches!(parsed.host_str(), Some("127.0.0.1") | Some("::1") | Some("localhost"));
    if parsed.scheme() != "https" && !is_loopback {
        return Err(format!("non-loopback {resource} must be https"));
    }
    Ok(parsed)
}

/// Validate that `value` is one of `allowed`, returning a field-named error
/// otherwise. The shared enum-membership check behind the Dōjō membership
/// (`dojo::memberships`) and collective-preferences (`collective::preferences`)
/// validators — keeps the `invalid {field}: … (allowed: …)` message identical
/// across every enum-typed API field.
pub(crate) fn require_member_of(value: &str, allowed: &[&str], field: &str) -> Result<(), String> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(format!("invalid {field}: {value:?} (allowed: {})", allowed.join(", ")))
    }
}

/// Extract a UUID from a JSON value's string field.
pub(crate) fn json_uuid(v: &serde_json::Value) -> Option<uuid::Uuid> {
    v.as_str().and_then(|s| uuid::Uuid::parse_str(s).ok())
}

/// Resolve `id` (from a path segment) as either a project UUID or a
/// project *name* → project UUID. Every `GET /api/projects/{id}/...`
/// endpoint should reach for this so an MCP caller passing `sensei`
/// (natural for assistants) works the same as passing the UUID.
///
/// Root cause of the "empty result" reports Jerry hit on 2026-07-07:
/// dozens of handlers only ran `Uuid::parse_str(id)` and 400'd on the
/// name shape, so `get_ftr_daily` / `get_quality_signals` /
/// `get_hotspots` for `project=sensei` came back empty even though the
/// data was there.
pub(crate) async fn resolve_project_uuid(
    state: &crate::api::state::AppState,
    id: &str,
) -> Option<uuid::Uuid> {
    if let Ok(uuid) = uuid::Uuid::parse_str(id) {
        return Some(uuid);
    }
    let row = state.pg.get_project_by_name(id).await.ok().flatten()?;
    json_uuid(&row["id"])
}
