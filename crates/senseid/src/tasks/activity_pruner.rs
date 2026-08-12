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
/// Hard backstop floor: never let a session linger longer than this even when
/// its day never got captured into `sensei.project_metrics`.
const BACKSTOP_FLOOR_DAYS: i32 = 90;

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

/// The capture-before-reclaim backstop: a session older than this is reclaimed
/// even when its day was never captured, so nothing lingers forever. Explicit
/// `activity.capture_backstop_days` wins; otherwise the default is
/// `max(BACKSTOP_FLOOR_DAYS, 2 × retention)` — always comfortably beyond the
/// retention window so the capture path (not the backstop) is the normal gate.
fn parse_backstop(cfg: Option<String>, retention_days: i32) -> i32 {
    cfg.and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| BACKSTOP_FLOOR_DAYS.max(retention_days.saturating_mul(2)))
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
        let backstop = parse_backstop(
            pg.get_config("activity.capture_backstop_days").await.ok().flatten(),
            days,
        );
        // Scope the capture-before-reclaim guard to the DAY-KEYED (delivery) groups
        // — the single source is the planner, so the pruner can't drift from the
        // groups actually backfilled per-day (a forward-only snapshot row must never
        // mark a session's day "captured").
        let day_keyed = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        match pg.prune_activity(days, backstop, &day_keyed).await {
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

    #[test]
    fn parse_backstop_defaults_to_max_floor_or_twice_retention() {
        // Default at the 30-day retention → max(90, 60) = 90 (the floor wins).
        assert_eq!(parse_backstop(None, DEFAULT_RETENTION_DAYS), BACKSTOP_FLOOR_DAYS);
        // With a long retention 2× beats the floor.
        assert_eq!(parse_backstop(None, 60), 120);
        assert_eq!(parse_backstop(None, 45), 90);
        // Invalid / non-positive config falls back to the derived default.
        assert_eq!(parse_backstop(Some("nope".into()), 30), 90);
        assert_eq!(parse_backstop(Some("0".into()), 30), 90);
        assert_eq!(parse_backstop(Some("-5".into()), 30), 90);
        // A valid explicit override wins outright.
        assert_eq!(parse_backstop(Some("200".into()), 30), 200);
        assert_eq!(parse_backstop(Some(" 45 ".into()), 30), 45);
    }
}
