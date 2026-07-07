//! Shared utilities for API handlers.

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
