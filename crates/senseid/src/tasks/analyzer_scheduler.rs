//! Analyzer scheduler (#67).
//!
//! A long-lived tokio task (mirroring `progress_emitter::spawn`) that wakes on
//! an interval and enqueues `TaskKind::AnalyzeProject` for every project whose
//! sessions have changed since it was last analyzed — so session enrichment
//! (#66, L0) and the downstream analyzer layers run on their own as sessions
//! accrue. The on-demand path is the API enqueueing the same task directly.
//!
//! Incrementality (the steady state):
//! - **Persisted watermark** — the per-project "last activity I acted on" map is
//!   saved to `sensei.config` (`analyzer.watermark`), so a daemon restart does
//!   NOT re-analyze every project; only genuinely-new activity is due.
//! - **Global passes** — when any project is due, `AggregateCorrections` and
//!   `MeasureVerdicts` run once (corrections cluster globally; verdicts re-measure
//!   accepted recs' before/after FTR).
//! - **Daily full refresh** — once per `full_refresh_secs`, ALL active projects
//!   are re-analyzed even without new sessions, so time-based/decay insights
//!   (maturity, pattern effectiveness, model effectiveness, ranking) stay fresh.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};

const DEFAULT_INTERVAL_SECS: u64 = 3600;
/// Re-analyze every active project at least this often, regardless of new
/// activity — keeps decay/staleness/effectiveness insights current.
const DEFAULT_FULL_REFRESH_SECS: i64 = 86_400;
/// Config keys for the persisted scheduler state.
const WATERMARK_KEY: &str = "analyzer.watermark";
const LAST_REFRESH_KEY: &str = "analyzer.last_full_refresh";

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

/// True when a daily full refresh is due: never refreshed, or the interval has
/// elapsed. Pure (clock injected) so it's testable.
fn due_for_full_refresh(now_ms: i64, last_refresh_ms: Option<i64>, interval_secs: i64) -> bool {
    match last_refresh_ms {
        None => true,
        Some(prev) => now_ms - prev >= interval_secs * 1000,
    }
}

/// Serialize the watermark to JSON (`{project_uuid: rfc3339}`) for persistence.
fn serialize_watermark(watermark: &HashMap<uuid::Uuid, DateTime<Utc>>) -> String {
    let map: serde_json::Map<String, serde_json::Value> = watermark
        .iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_rfc3339())))
        .collect();
    serde_json::Value::Object(map).to_string()
}

/// Parse a persisted watermark back into the map. Malformed entries are skipped
/// (never panics — a bad config value just means "re-analyze", which is safe).
fn deserialize_watermark(raw: &str) -> HashMap<uuid::Uuid, DateTime<Utc>> {
    let mut out = HashMap::new();
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(raw) else {
        return out;
    };
    for (k, v) in map {
        if let (Ok(id), Some(ts)) = (uuid::Uuid::parse_str(&k), v.as_str())
            && let Ok(dt) = DateTime::parse_from_rfc3339(ts)
        {
            out.insert(id, dt.with_timezone(&Utc));
        }
    }
    out
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
    let refresh_secs = pg
        .get_config("analyzer.full_refresh_secs")
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_FULL_REFRESH_SECS);
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    // Restore the watermark + last-refresh from config so a restart resumes
    // incrementally instead of re-analyzing everything.
    let mut watermark = match pg.get_config(WATERMARK_KEY).await {
        Ok(Some(raw)) => deserialize_watermark(&raw),
        _ => HashMap::new(),
    };
    let mut last_refresh_ms: Option<i64> = pg
        .get_config(LAST_REFRESH_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|v| v.trim().parse::<i64>().ok());
    loop {
        ticker.tick().await;
        let activity = match pg.get_projects_with_session_activity().await {
            Ok(a) => a,
            Err(e) => {
                tracing::warn!(error = %e, "analyzer_scheduler: session-activity query failed");
                continue;
            }
        };
        // Incremental: projects whose activity advanced past the watermark.
        let mut due = projects_due(&activity, &mut watermark);

        // Daily full refresh: re-analyze ALL active projects so decay/effectiveness
        // insights stay fresh even when no new sessions arrived.
        let now_ms = Utc::now().timestamp_millis();
        if due_for_full_refresh(now_ms, last_refresh_ms, refresh_secs) {
            for (pid, _) in &activity {
                if !due.contains(pid) {
                    due.push(*pid);
                }
            }
            last_refresh_ms = Some(now_ms);
            let _ = pg.set_config(LAST_REFRESH_KEY, &now_ms.to_string()).await;
            tracing::info!(projects = activity.len(), "analyzer_scheduler: daily full refresh");
        }

        let any_due = !due.is_empty();
        for pid in &due {
            queue.enqueue(Task::new(TaskKind::AnalyzeProject, "", &pid.to_string())).await;
        }
        // Global passes once per tick when work happened: corrections cluster
        // across projects; verdicts re-measure accepted recs' before/after FTR.
        if any_due {
            queue.enqueue(Task::new(TaskKind::AggregateCorrections, "", "")).await;
            queue.enqueue(Task::new(TaskKind::MeasureVerdicts, "", "")).await;
            // Persist the advanced watermark so a restart doesn't redo this work.
            let _ = pg.set_config(WATERMARK_KEY, &serialize_watermark(&watermark)).await;
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

    #[test]
    fn full_refresh_due_when_never_run_or_interval_elapsed() {
        let day = 86_400i64;
        assert!(due_for_full_refresh(1_000_000, None, day), "never refreshed → due");
        // exactly one day later → due
        assert!(due_for_full_refresh(day * 1000, Some(0), day));
        // half a day later → not due
        assert!(!due_for_full_refresh(day * 500, Some(0), day));
    }

    #[test]
    fn watermark_survives_serialize_roundtrip() {
        let p1 = uuid::Uuid::new_v4();
        let p2 = uuid::Uuid::new_v4();
        let mut wm = HashMap::new();
        wm.insert(p1, ts(1000));
        wm.insert(p2, ts(2000));
        let restored = deserialize_watermark(&serialize_watermark(&wm));
        assert_eq!(restored, wm, "watermark round-trips through config persistence");
    }

    #[test]
    fn deserialize_watermark_tolerates_garbage() {
        assert!(deserialize_watermark("not json").is_empty());
        assert!(deserialize_watermark("{}").is_empty());
        // a bad value is skipped, a good one kept
        let p = uuid::Uuid::new_v4();
        let raw = format!("{{\"{p}\":\"2026-06-25T10:00:00+00:00\",\"bad-uuid\":\"x\"}}");
        let m = deserialize_watermark(&raw);
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&p));
    }
}
