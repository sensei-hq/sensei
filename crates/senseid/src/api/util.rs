//! Shared utilities for API handlers.

/// Extract a UUID from a JSON value's string field.
pub(crate) fn json_uuid(v: &serde_json::Value) -> Option<uuid::Uuid> {
    v.as_str().and_then(|s| uuid::Uuid::parse_str(s).ok())
}
