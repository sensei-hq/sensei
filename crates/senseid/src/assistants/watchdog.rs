//! Daemon-side orchestration: merges the DB-backed capture-freshness check into
//! each adapter's pure config_health, runs the hourly watchdog with a circuit
//! breaker, and fires notifications. The pure parts (config keys, tick policy)
//! are unit-tested; the loop is thin glue.

use crate::assistants::health::{capture_freshness, AdapterCheck, AdapterHealth, CheckStatus};
use crate::db::pg_store::PgStore;

pub const DEFAULT_WINDOW_HOURS: f64 = 24.0;
pub const DEFAULT_EXCLUDE_WEEKENDS: bool = true;

/// Assistant family that captures hook events into the DB (and thus gets the
/// freshness check). Matches `ClaudeCodeAssistant::family()` and the
/// `assistant_family` enum value written by `ingest_hook_event`.
const CLAUDE_FAMILY: &str = "claude";

#[derive(Debug, Clone, Copy)]
pub struct CaptureWindow { pub hours: f64, pub exclude_weekends: bool }

impl Default for CaptureWindow {
    fn default() -> Self { Self { hours: DEFAULT_WINDOW_HOURS, exclude_weekends: DEFAULT_EXCLUDE_WEEKENDS } }
}

/// Parse the two config strings into a CaptureWindow, falling back to defaults
/// on missing/garbage values. Pure — unit-tested without a DB.
pub fn parse_window(hours: Option<&str>, exclude_weekends: Option<&str>) -> CaptureWindow {
    CaptureWindow {
        // Reject NaN AND infinities — `"inf"` parses to f64::INFINITY and would
        // silently make the staleness check a permanent no-op (everything is
        // `<= INFINITY`). Only a finite, positive window is meaningful.
        hours: hours.and_then(|s| s.trim().parse::<f64>().ok())
            .filter(|h| h.is_finite() && *h > 0.0)
            .unwrap_or(DEFAULT_WINDOW_HOURS),
        exclude_weekends: exclude_weekends.and_then(|s| s.trim().parse::<bool>().ok()).unwrap_or(DEFAULT_EXCLUDE_WEEKENDS),
    }
}

/// Load the capture window from sensei.config (keys
/// `capture.max_inactivity_hours`, `capture.exclude_weekends`). A DB read error
/// is logged (not swallowed silently — a missing key and an unreachable DB must
/// be distinguishable in the log) and falls back to defaults.
pub async fn load_window(pg: &PgStore) -> CaptureWindow {
    let hours = pg.get_config("capture.max_inactivity_hours").await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "capture watchdog: read capture.max_inactivity_hours failed"); None });
    let weekends = pg.get_config("capture.exclude_weekends").await
        .unwrap_or_else(|e| { tracing::warn!(error = %e, "capture watchdog: read capture.exclude_weekends failed"); None });
    parse_window(hours.as_deref(), weekends.as_deref())
}

/// Compute health for every configured adapter, appending the DB-backed
/// `events` freshness check to the `claude` family.
pub async fn health_report(pg: &PgStore, now_ms: i64) -> Vec<AdapterHealth> {
    let window = load_window(pg).await;
    let mut out = Vec::new();
    for status in crate::assistants::detect() {
        if !status.configured { continue; }
        let mut checks = config_health_for(&status.id);
        if status.family == CLAUDE_FAMILY {
            let last = pg.latest_hook_event_ts(CLAUDE_FAMILY).await
                .unwrap_or_else(|e| { tracing::warn!(error = %e, "capture watchdog: latest_hook_event_ts failed"); None });
            checks.push(capture_freshness(last, now_ms, window.hours, window.exclude_weekends));
        }
        out.push(AdapterHealth::new(&status.id, &status.family, checks, true));
    }
    out
}

/// config_health for an adapter id, via the registry. Returns a single Unknown
/// check if the id is not in the registry (defensive).
fn config_health_for(adapter_id: &str) -> Vec<AdapterCheck> {
    crate::assistants::config_health_for_id(adapter_id)
        .unwrap_or_else(|| vec![AdapterCheck::new("configured", "configured", CheckStatus::Unknown,
            Some(format!("unknown adapter {adapter_id}")))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_window_defaults_on_garbage() {
        let w = parse_window(None, None);
        assert_eq!(w.hours, 24.0);
        assert!(w.exclude_weekends);
        let w2 = parse_window(Some("abc"), Some("nope"));
        assert_eq!(w2.hours, 24.0);
        assert!(w2.exclude_weekends);
    }
    #[test]
    fn parse_window_reads_values() {
        let w = parse_window(Some("6"), Some("false"));
        assert_eq!(w.hours, 6.0);
        assert!(!w.exclude_weekends);
    }
    #[test]
    fn parse_window_rejects_invalid_hours() {
        assert_eq!(parse_window(Some("0"), None).hours, 24.0);
        assert_eq!(parse_window(Some("-3"), None).hours, 24.0);
        // Infinities parse as f64 but would disable staleness detection — reject.
        assert_eq!(parse_window(Some("inf"), None).hours, 24.0);
        assert_eq!(parse_window(Some("infinity"), None).hours, 24.0);
        assert_eq!(parse_window(Some("NaN"), None).hours, 24.0);
    }
}
