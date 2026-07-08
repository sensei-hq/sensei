//! Activity-data pruner (#74).
//!
//! A long-lived tokio task (mirroring [`crate::tasks::log_pruner`]) that
//! periodically deletes captured activity older than a retention window,
//! after the analyzer has extracted its value. Both the interval and the
//! retention are config-driven (`activity.prune_interval_secs`,
//! `activity.retention_days`) with sensible defaults.
//!
//! What gets pruned:
//!   - `activity.sessions` where `analyzed_at IS NOT NULL AND started_at < cutoff`
//!   - `activity.turns` — cascades on session delete
//!   - `activity.transcript_turns` — by client_session_id (text, no FK)
//!   - `activity.assistant_events` — by client_session_id AND session-less
//!     orphans by ts
//!
//! What is NOT pruned:
//!   - `inference.detected_patterns` / `recommendations` / `reasoning_traces`
//!   - `sensei.memories`
//!
//! The analyzer already distilled those; they survive the raw-event window.

use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;

/// Prune daily by default.
const DEFAULT_INTERVAL_SECS: u64 = 86_400;
/// Keep raw activity for 30 days by default.
const DEFAULT_RETENTION_DAYS: i32 = 30;

fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

fn parse_retention(cfg: Option<String>) -> i32 {
    cfg.and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}

/// Spawn the activity pruner for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(run(pg));
}

async fn run(pg: Arc<PgStore>) {
    let secs = parse_interval(pg.get_config("activity.prune_interval_secs").await.ok().flatten());
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    loop {
        ticker.tick().await; // first tick fires immediately → prune on startup
        // Re-read retention each tick so config changes take effect without a restart.
        let days = parse_retention(pg.get_config("activity.retention_days").await.ok().flatten());
        match pg.prune_activity(days).await {
            Ok(c) => {
                let total = c.sessions + c.turns + c.transcript_turns + c.assistant_events;
                if total > 0 {
                    tracing::info!(
                        sessions = c.sessions,
                        turns = c.turns,
                        transcript_turns = c.transcript_turns,
                        assistant_events = c.assistant_events,
                        "activity_pruner: pruned {total} rows older than {days}d",
                    );
                }
            }
            Err(e) => tracing::warn!(error = %e, days, "activity_pruner: prune failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_interval_falls_back_on_missing_invalid_or_zero() {
        assert_eq!(parse_interval(None), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("nope".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("0".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("3600".into())), 3600);
        assert_eq!(parse_interval(Some(" 7200 ".into())), 7200);
    }

    #[test]
    fn parse_retention_falls_back_on_missing_invalid_or_nonpositive() {
        assert_eq!(parse_retention(None), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("x".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("0".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("-5".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("14".into())), 14);
        assert_eq!(parse_retention(Some("90".into())), 90);
    }
}
