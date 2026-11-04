//! Log pruner (#74) — log TTL/retention.
//!
//! A long-lived tokio task (mirroring [`crate::tasks::analyzer_scheduler`]) that
//! periodically deletes `public.logs` rows older than a retention window, so the
//! structured-log table doesn't grow without bound. Both the interval and the
//! Cadence lives in `sensei.schedules` (name `log_prune`), not here. Retention is
//! still config-driven (`logs.retention_days`) and re-read each run.
//!
//! (the retired `logs.prune_interval_secs` key is superseded by that row;
//! `logs.retention_days`) with sensible defaults.
//!
//! Scope: this prunes the flat structured logs ingested via `POST /api/logs`.
//! Activity-data retention (sessions / turns / hook_events) is the separate
//! analyzer-GC concern tracked under #74's sibling work.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::ticker;

/// Keep logs for 30 days by default.
const DEFAULT_RETENTION_DAYS: i32 = 30;

/// Resolve the retention window (days) from config. Must be positive; missing /
/// unparseable / non-positive values fall back to the default.
fn parse_retention(cfg: Option<String>) -> i32 {
    sensei_bootstrap::config::positive_or(cfg, DEFAULT_RETENTION_DAYS)
}

/// Spawn the pruner for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(run(pg));
}

async fn run(pg: Arc<PgStore>) {
    // Schedule-driven: when to prune is `sensei.schedules`, not a constant here.
    // The tick itself is unchanged — what to do stays code.
    let store = pg.clone();
    ticker::run_scheduled(pg, "log_prune", move || {
        let pg = store.clone();
        async move { prune_once(&pg).await }
    })
    .await;
}

/// One prune pass. Re-reads retention each run so config changes take effect
/// without a restart. A failure is returned rather than only logged: the
/// schedule row records it, so "when did this last work?" has an answer.
async fn prune_once(pg: &PgStore) -> Result<(), String> {
    let days = crate::tasks::retention::setting(
        pg.get_config("logs.retention_days").await,
        parse_retention,
    )?;
    let n = pg.prune_logs(days).await?;
    if n > 0 {
        tracing::info!("log_pruner: deleted {n} log rows older than {days}d");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D83 batch A, audit row 182: a retention that cannot be read stops the
    /// prune, and a log row past every window stays. The second half proves the
    /// row is in the prune's reach once the read works.
    #[tokio::test]
    async fn a_failed_retention_read_stops_the_prune_and_deletes_nothing() {
        let pg = PgStore::test_store().await;
        let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
            "INSERT INTO public.logs (level, running_on, logged_at) VALUES ('info', 'test', '2000-01-01') RETURNING id",
        )
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let run = |sql: &'static str| sqlx_core::query::query(sql);
        let kept = |pg: &PgStore| {
            let pool = pg.pool().clone();
            async move {
                let (n,): (i64,) =
                    sqlx_core::query_as::query_as("SELECT count(*) FROM public.logs WHERE id = $1")
                        .bind(id)
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                n
            }
        };

        run("ALTER TABLE sensei.config RENAME TO config_unreadable")
            .execute(pg.pool())
            .await
            .unwrap();
        let failed = prune_once(&pg).await;
        let after_failed = kept(&pg).await;
        run("ALTER TABLE sensei.config_unreadable RENAME TO config")
            .execute(pg.pool())
            .await
            .unwrap();
        prune_once(&pg).await.unwrap();
        let after_ok = kept(&pg).await;

        assert!(failed.is_err(), "a failed read is the prune's error: {failed:?}");
        assert_eq!((after_failed, after_ok), (1, 0), "kept on a failed read; pruned once it reads");
    }

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
