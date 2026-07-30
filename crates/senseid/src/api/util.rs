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

/// Read one optional string-enum field from a request body: absent/null takes
/// `default`; a present value must be a string and a member of `allowed`
/// (validated via [`require_member_of`], so the message is identical everywhere).
/// Shared by the collective-preferences and stance validators — one parser for
/// "optional enum field, default when absent".
pub(crate) fn parse_enum_field(
    body: &serde_json::Value,
    field: &str,
    allowed: &[&str],
    default: &str,
) -> Result<String, String> {
    match body.get(field) {
        None | Some(serde_json::Value::Null) => Ok(default.to_string()),
        Some(v) => {
            let s = v.as_str().ok_or_else(|| format!("{field} must be a string"))?;
            require_member_of(s, allowed, field)?;
            Ok(s.to_string())
        }
    }
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
///
/// Returns `Ok(Some(uuid))` when resolved, `Ok(None)` on a genuine miss (no such
/// project — caller 404s/400s), and `Err(500)` when the name lookup itself fails.
/// Failing closed on the DB error is deliberate: swallowing it (the old
/// `.ok().flatten()`) turned a transient failure into an indistinguishable
/// "unknown project", masking outages as 404s. See the #109 fabrication audit.
pub(crate) async fn resolve_project_uuid(
    state: &crate::api::state::AppState,
    id: &str,
) -> Result<Option<uuid::Uuid>, axum::http::StatusCode> {
    if let Ok(uuid) = uuid::Uuid::parse_str(id) {
        return Ok(Some(uuid));
    }
    let row = state
        .pg
        .get_project_by_name(id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(row.and_then(|r| json_uuid(&r["id"])))
}

#[cfg(test)]
mod tests {
    /// #100 guard: a `{id}` on an `/api/projects/{id}/*` route is name-or-uuid
    /// (AI callers pass a project name), so it MUST resolve via
    /// [`resolve_project_uuid`] — never `Uuid::parse_str` directly, which 400s
    /// on a name and returns a silent empty. This scans handler source for the
    /// `let project_id = …Uuid::parse_str` smell so the 2026-07-07 fix can't
    /// regress. (Non-project uuids — chain/memory/session ids — are unaffected;
    /// the pattern keys on the `project_id` binding specifically.)
    #[test]
    fn no_handler_parses_a_project_id_raw() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/api/handlers");
        let mut offenders = Vec::new();
        for entry in std::fs::read_dir(dir).expect("handlers dir readable") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for (i, line) in src.lines().enumerate() {
                let l = line.trim_start();
                if l.starts_with("let project_id")
                    && l.contains("Uuid::parse_str")
                {
                    offenders.push(format!(
                        "{}:{}: {}",
                        path.file_name().unwrap().to_string_lossy(),
                        i + 1,
                        l.trim()
                    ));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "project ids on /api/projects/{{id}}/* must resolve via \
             resolve_project_uuid (name-or-uuid), not raw Uuid::parse_str:\n{}",
            offenders.join("\n")
        );
    }
}
