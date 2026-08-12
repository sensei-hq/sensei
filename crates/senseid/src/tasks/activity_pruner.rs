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
/// Keep raw activity for 90 days by default (raw sessions/events/turns; the
/// distilled metric snapshots in `sensei.project_metrics` are the long-term store).
const DEFAULT_RETENTION_DAYS: i32 = 90;
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
    // Skip the immediate boot tick — do NOT prune the instant the daemon starts.
    // On boot the history synthesizer → analyzer → metric planner are re-capturing
    // backfilled history into durable snapshots; pruning immediately would reclaim
    // sessions older than the capture backstop BEFORE their session-anchored metrics
    // (ftr/throughput) are captured, since capture-before-reclaim can't protect a
    // session past the backstop. Waiting one interval lets capture win the race; the
    // backstop still bounds anything that is never captured (retention isn't urgent
    // at boot — one interval's delay retains at most one extra window of activity).
    ticker.tick().await;
    loop {
        ticker.tick().await; // subsequent ticks wait a full interval before pruning
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
                // Reclaim dead tuples + refresh planner stats once per daily tick,
                // after the prune ("after all processing"). Best-effort: a VACUUM
                // failure is logged, never fatal to the loop.
                if let Err(e) = pg.vacuum_activity().await {
                    tracing::warn!(error = %e, "activity_pruner: VACUUM (ANALYZE) failed");
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
        // Pin the documented default to the literal 90 so changing the const trips
        // this test instead of silently altering the retention window.
        assert_eq!(DEFAULT_RETENTION_DAYS, 90, "documented raw-activity retention default is 90 days");
        assert_eq!(parse_retention(None), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("x".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("0".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("-5".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("14".into())), 14);
        assert_eq!(parse_retention(Some("90".into())), 90);
    }

    #[test]
    fn capture_scope_is_session_derived_only_and_excludes_git_churn() {
        // The pruner scopes `prune_activity`'s capture-before-reclaim EXISTS to
        // `planner::day_keyed_task_names()`. That scope must stay SESSION-derived only
        // (`session_outcomes`, `autonomy`) and EXCLUDE git-derived `churn`: a churn row
        // for a day must NOT green-light reclaiming that day's sessions before their
        // session-anchored metrics (ftr/throughput) are captured — that would reopen
        // the earlier data-loss bug. Churn is still backfilled per-day by the planner;
        // it is only the capture authorization it must not carry (the #3 split).
        let scope = crate::tasks::handlers::metrics::planner::day_keyed_task_names();
        assert!(scope.contains(&"session_outcomes"), "session_outcomes authorizes capture");
        assert!(scope.contains(&"autonomy"), "autonomy authorizes capture");
        assert!(
            !scope.contains(&"churn"),
            "git-derived churn must NOT authorize capture-before-reclaim (pruner capture scope excludes churn)",
        );
    }

    #[test]
    fn parse_backstop_defaults_to_max_floor_or_twice_retention() {
        // Default at the 90-day retention → max(90, 180) = 180 (2× beats the floor).
        assert_eq!(parse_backstop(None, DEFAULT_RETENTION_DAYS), 180);
        // The floor still wins at short retentions.
        assert_eq!(parse_backstop(None, 30), BACKSTOP_FLOOR_DAYS);
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
