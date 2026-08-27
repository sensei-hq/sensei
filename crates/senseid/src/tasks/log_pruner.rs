//! Log pruner (#74) — log TTL/retention.
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler`]) that
//! periodically deletes `public.logs` rows older than a retention window, so the
//! structured-log table doesn't grow without bound. Both the interval and the
//! retention are config-driven (`logs.prune_interval_secs`,
//! `logs.retention_days`) with sensible defaults.
//!
//! Scope: this prunes the flat structured logs ingested via `POST /api/logs`.
//! Activity-data retention (sessions / turns / hook_events) is the separate
//! analyzer-GC concern tracked under #74's sibling work.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::ticker::{self, FirstTick};

/// Prune daily by default.
const DEFAULT_INTERVAL_SECS: u64 = 86_400;
/// Keep logs for 30 days by default.
const DEFAULT_RETENTION_DAYS: i32 = 30;

/// Resolve the retention window (days) from config. Must be positive; missing /
/// unparseable / non-positive values fall back to the default.
fn parse_retention(cfg: Option<String>) -> i32 {
    cfg.and_then(|v| v.trim().parse::<i32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_RETENTION_DAYS)
}

/// Spawn the pruner for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(run(pg));
}

async fn run(pg: Arc<PgStore>) {
    let mut ticker = ticker::from_config(
        &pg,
        "logs.prune_interval_secs",
        DEFAULT_INTERVAL_SECS,
        FirstTick::Immediate,
    )
    .await;
    loop {
        ticker.tick().await; // first tick fires immediately → prune on startup
        // Re-read retention each tick so config changes take effect without a restart.
        let days = parse_retention(pg.get_config("logs.retention_days").await.ok().flatten());
        match pg.prune_logs(days).await {
            Ok(n) if n > 0 => tracing::info!("log_pruner: deleted {n} log rows older than {days}d"),
            Ok(_) => {}
            Err(e) => tracing::warn!(error = %e, "log_pruner: prune failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_retention_falls_back_on_missing_invalid_or_nonpositive() {
        assert_eq!(parse_retention(None), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("x".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("0".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("-5".into())), DEFAULT_RETENTION_DAYS);
        assert_eq!(parse_retention(Some("7".into())), 7);
        assert_eq!(parse_retention(Some("90".into())), 90);
    }
}
