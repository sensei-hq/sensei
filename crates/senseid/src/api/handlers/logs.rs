use crate::api::state::AppState;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct LogBody {
    level: String,
    running_on: String,
    module: Option<String>,
    logged_at: String,
    message: Option<String>,
    context: Option<serde_json::Value>,
    data: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

/// POST /api/logs — ingest a structured log entry from CLI, MCP, or app.
pub(crate) async fn ingest_log(
    State(state): State<AppState>,
    Json(body): Json<LogBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .pg
        .insert_log(
            &body.level,
            &body.running_on,
            body.module.as_deref(),
            &body.logged_at,
            body.message.as_deref().unwrap_or(""),
            &body.context.unwrap_or(serde_json::json!({})),
            &body.data,
            &body.error,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"ok": true})))
}

/// Default row cap when the caller omits `limit`.
const DEFAULT_LOG_LIMIT: i64 = 200;
/// Hard ceiling on the row cap — a caller-supplied `limit` is clamped to this.
const MAX_LOG_LIMIT: i64 = 1000;

#[derive(Deserialize)]
pub(crate) struct LogsQuery {
    /// Exact `level` match (`trace|debug|info|warn|error`).
    #[serde(default)]
    level: Option<String>,
    /// Exact `running_on` match (daemon / cli / mcp / app).
    #[serde(default)]
    source: Option<String>,
    /// Exact `module` column match (scanner / watcher / analyzer / …).
    #[serde(default)]
    module: Option<String>,
    /// Lower bound on `logged_at`: a relative duration (`30s`, `15m`, `1h`,
    /// `24h`, `7d`), an RFC-3339 timestamp, or `all` for no lower bound.
    #[serde(default)]
    since: Option<String>,
    /// Row cap (defaults to `DEFAULT_LOG_LIMIT`, clamped to `MAX_LOG_LIMIT`).
    #[serde(default)]
    limit: Option<i64>,
}

/// GET /api/logs — read structured log rows for the Observatory · Logs screen.
///
/// All filters are optional and parameterized. Newest-first, capped. An
/// unparseable `since` is a `400` (never silently ignored); an empty result
/// is an empty JSON array, not an error.
pub(crate) async fn get_logs(
    State(state): State<AppState>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let since = match q.since.as_deref() {
        None => None,
        Some(s) => match parse_since(s) {
            Ok(cutoff) => cutoff,
            Err(()) => {
                tracing::warn!(since = %s, "get_logs: unparseable `since` filter");
                return Err(StatusCode::BAD_REQUEST);
            }
        },
    };

    let limit = q.limit.unwrap_or(DEFAULT_LOG_LIMIT).clamp(1, MAX_LOG_LIMIT);

    let rows = state
        .pg
        .query_logs(q.level.as_deref(), q.source.as_deref(), q.module.as_deref(), since, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "get_logs: query_logs failed");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!(rows)))
}

/// Resolve a `since` filter into an absolute UTC cutoff.
///
/// - `all` / empty → `Ok(None)` (no lower bound).
/// - relative duration (`30s`, `15m`, `1h`, `24h`, `7d`) → `Ok(Some(now - d))`.
/// - RFC-3339 timestamp → `Ok(Some(ts))`.
/// - anything else → `Err(())` so the caller can reject with `400`.
fn parse_since(s: &str) -> Result<Option<chrono::DateTime<chrono::Utc>>, ()> {
    let s = s.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("all") {
        return Ok(None);
    }
    if let Some(dur) = parse_relative_duration(s) {
        return Ok(Some(chrono::Utc::now() - dur));
    }
    match chrono::DateTime::parse_from_rfc3339(s) {
        Ok(ts) => Ok(Some(ts.with_timezone(&chrono::Utc))),
        Err(_) => Err(()),
    }
}

/// Parse a `<number><unit>` relative duration, unit in `s|m|h|d`. Returns
/// `None` for a bare number, empty unit, or unknown unit.
fn parse_relative_duration(s: &str) -> Option<chrono::Duration> {
    let split = s.find(|c: char| !c.is_ascii_digit())?;
    let (num, unit) = s.split_at(split);
    let n: i64 = num.parse().ok()?;
    match unit {
        "s" => Some(chrono::Duration::seconds(n)),
        "m" => Some(chrono::Duration::minutes(n)),
        "h" => Some(chrono::Duration::hours(n)),
        "d" => Some(chrono::Duration::days(n)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_since_all_and_empty_mean_no_bound() {
        assert_eq!(parse_since("all"), Ok(None));
        assert_eq!(parse_since("ALL"), Ok(None));
        assert_eq!(parse_since(""), Ok(None));
        assert_eq!(parse_since("  "), Ok(None));
    }

    #[test]
    fn parse_since_relative_durations() {
        let before = chrono::Utc::now();
        let cutoff = parse_since("1h").unwrap().unwrap();
        // ~1h in the past: comfortably between 61m and 59m ago.
        assert!(cutoff <= before - chrono::Duration::minutes(59));
        assert!(cutoff >= before - chrono::Duration::minutes(61));

        // Unit coverage.
        assert!(parse_since("30s").unwrap().is_some());
        assert!(parse_since("15m").unwrap().is_some());
        assert!(parse_since("24h").unwrap().is_some());
        assert!(parse_since("7d").unwrap().is_some());
    }

    #[test]
    fn parse_since_rfc3339_timestamp() {
        let got = parse_since("2026-07-01T12:00:00Z").unwrap().unwrap();
        assert_eq!(got.to_rfc3339(), "2026-07-01T12:00:00+00:00");
    }

    #[test]
    fn parse_since_rejects_garbage() {
        assert_eq!(parse_since("soon"), Err(()));
        assert_eq!(parse_since("10"), Err(())); // bare number, no unit
        assert_eq!(parse_since("5y"), Err(())); // unknown unit
        assert_eq!(parse_since("h1"), Err(())); // unit before number
    }

    #[test]
    fn parse_relative_duration_maps_units() {
        assert_eq!(parse_relative_duration("30s"), Some(chrono::Duration::seconds(30)));
        assert_eq!(parse_relative_duration("15m"), Some(chrono::Duration::minutes(15)));
        assert_eq!(parse_relative_duration("2h"), Some(chrono::Duration::hours(2)));
        assert_eq!(parse_relative_duration("7d"), Some(chrono::Duration::days(7)));
        assert_eq!(parse_relative_duration("100"), None);
        assert_eq!(parse_relative_duration("abc"), None);
    }
}
