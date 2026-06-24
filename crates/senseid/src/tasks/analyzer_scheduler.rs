//! Analyzer scheduler (#67).
//!
//! A long-lived tokio task (mirroring `progress_emitter::spawn`) that wakes on
//! an interval and enqueues `TaskKind::AnalyzeProject` for every project whose
//! sessions have changed since it was last analyzed — so session enrichment
//! (#66, L0) and the downstream analyzer layers run on their own as sessions
//! accrue. The on-demand path is the API enqueueing the same task directly.
//!
//! The watermark is in-memory: on a daemon restart it re-analyzes once, which
//! is harmless because enrichment is idempotent. A persisted watermark is a
//! future refinement if restart churn ever matters.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

const DEFAULT_INTERVAL_SECS: u64 = 3600;

/// Projects whose latest session activity is newer than the watermark value we
/// last acted on. Pure: it also advances the watermark for the projects it
/// returns, so a subsequent call with the same activity returns nothing.
pub fn projects_due(
    activity: &[(uuid::Uuid, DateTime<Utc>)],
    watermark: &mut HashMap<uuid::Uuid, DateTime<Utc>>,
) -> Vec<uuid::Uuid> {
    let mut due = Vec::new();
    for (pid, latest) in activity {
        let is_new = match watermark.get(pid) {
            Some(prev) => latest > prev,
            None => true,
        };
        if is_new {
            watermark.insert(*pid, *latest);
            due.push(*pid);
        }
    }
    due
}

/// Resolve the tick interval from a config value, falling back to the default
/// for missing / unparseable / zero values.
fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_INTERVAL_SECS)
}

async fn interval_secs(pg: &PgStore) -> u64 {
    parse_interval(pg.get_config("analyzer.interval_secs").await.ok().flatten())
}

/// Spawn the scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = interval_secs(&pg).await;
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    // tokio's first tick fires immediately → analyze on startup.
    let mut watermark: HashMap<uuid::Uuid, DateTime<Utc>> = HashMap::new();
    loop {
        ticker.tick().await;
        match pg.get_projects_with_session_activity().await {
            Ok(activity) => {
                let due = projects_due(&activity, &mut watermark);
                let any_due = !due.is_empty();
                for pid in due {
                    queue
                        .enqueue(Task::new(TaskKind::AnalyzeProject, "", &pid.to_string()))
                        .await;
                }
                // Corrections cluster globally across projects, so derive once per
                // tick after the per-project analyses — only when something changed.
                if any_due {
                    queue
                        .enqueue(Task::new(TaskKind::AggregateCorrections, "", ""))
                        .await;
                }
            }
            Err(e) => tracing::warn!(error = %e, "analyzer_scheduler: session-activity query failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn projects_due_picks_new_then_only_advanced() {
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let mut wm = HashMap::new();

        // First sight of both → both due.
        let mut due = projects_due(&[(p1, ts(100)), (p2, ts(100))], &mut wm);
        due.sort();
        let mut expect = vec![p1, p2];
        expect.sort();
        assert_eq!(due, expect);

        // Unchanged activity → nothing due.
        assert!(projects_due(&[(p1, ts(100)), (p2, ts(100))], &mut wm).is_empty());

        // Only p1's activity advanced → only p1 due.
        assert_eq!(projects_due(&[(p1, ts(200)), (p2, ts(100))], &mut wm), vec![p1]);
    }

    #[test]
    fn parse_interval_falls_back_on_missing_invalid_or_zero() {
        assert_eq!(parse_interval(None), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("abc".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("0".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("1800".into())), 1800);
        assert_eq!(parse_interval(Some("  900 ".into())), 900);
    }
}
