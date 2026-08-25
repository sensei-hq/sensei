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
//!   The full-refresh window is ALSO when `DetectCommunities` runs per indexed
//!   folder — community structure drifts slowly and the label-propagation pass
//!   is expensive, so a daily cadence keeps `inference.communities` current
//!   without dominating the queue on hot ticks.

use crate::db::pg_store::PgStore;
use crate::tasks::queue::TaskQueue;
use crate::tasks::{Task, TaskKind};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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

/// Enqueue the per-project analyzer tasks for one due project. Kept as a small
/// helper so the "what runs when a project is due" contract is unit-testable
/// against a real queue without driving the scheduler's infinite `run` loop.
///
/// Both tasks ride the same due signal (new activity, or the daily full
/// refresh): `AnalyzeProject` enriches sessions and derives signals, and
/// `ScanDocDrift` refreshes `inference.drift_items` so doc-drift populates on
/// its own instead of only via the manual `/drift/scan` endpoint. The drift
/// writer dedups + resolves in place, so re-running per due tick is safe and
/// bounded to active projects.
async fn enqueue_due_project(queue: &TaskQueue, pid: &uuid::Uuid) {
    queue.enqueue(Task::new(TaskKind::AnalyzeProject, "", &pid.to_string())).await;
    queue.enqueue(Task::new(TaskKind::ScanDocDrift, "", &pid.to_string())).await;
}

/// Enqueue the DAILY-only LLM process-quality pass for one project (spec
/// 2026-08-20). Separated from [`enqueue_due_project`] because it rides the daily
/// full-refresh window, NOT every hourly activity tick — it's up to
/// `batch_per_tick` local reasoning-chain calls, too heavy to run hourly. The
/// task itself is watermark-gated (only un-scored sessions) + batch-capped, so a
/// daily enqueue drains the backlog incrementally over successive days.
async fn enqueue_daily_process(queue: &TaskQueue, pid: &uuid::Uuid) {
    queue.enqueue(Task::new(TaskKind::AnalyzeSessionProcess, "", &pid.to_string())).await;
}

/// Enqueue the once-per-tick global aggregation passes, in dependency order.
/// Extracted as a small helper (like [`enqueue_due_project`]) so the ordering
/// contract is unit-testable without driving `run`'s infinite loop.
///
/// Order matters: `ClassifyPendingVerdicts` gap-fills verdicts for sessions
/// never opened in Replay and MUST run before `AggregateToolInsights`, which
/// reads `get_verdict_split_per_tool` — so the same tick's Health-tab snapshot
/// reflects the freshly-classified sessions, not just opened ones.
async fn enqueue_global_passes(queue: &TaskQueue) {
    queue.enqueue(Task::new(TaskKind::AggregateCorrections, "", "")).await;
    queue.enqueue(Task::new(TaskKind::MeasureVerdicts, "", "")).await;
    queue.enqueue(Task::new(TaskKind::ClassifyPendingVerdicts, "", "")).await;
    queue.enqueue(Task::new(TaskKind::AggregateToolInsights, "", "")).await;
    // Governance Tier-2: merge the global ruleset when it changed (source-hash
    // guarded, so an unchanged tick is a cheap no-op — no model call).
    queue.enqueue(Task::new(TaskKind::ConsolidateGovernance, "", "")).await;
    // Eager insight-copy: pre-generate mentor copy for pending recs so the
    // Insights/Today board reads cached copy on first view (idempotent — cached
    // recs skipped).
    queue.enqueue(Task::new(TaskKind::WarmInsightCopy, "", "")).await;
    // §9 learning loop: attribute confirmed playbook_run outcomes, aggregate FTR
    // stats, and apply the learn() plan (bounded reweight + learned proposals).
    // Idempotent — safe alongside the other global passes on every due tick.
    queue.enqueue(Task::new(TaskKind::LearnPlaybooks, "", "")).await;
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
    // Config-read failure isn't fatal — we fall back to the default
    // interval and log so an operator can see the DB is unhappy rather
    // than silently drifting off the configured cadence.
    let raw = match pg.get_config("analyzer.interval_secs").await {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "analyzer_scheduler: interval_secs config read failed — using default");
            None
        }
    };
    parse_interval(raw)
}

/// Spawn the scheduler for the daemon's lifetime.
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = interval_secs(&pg).await;
    // Same default-fall-back pattern as interval_secs — a bad config
    // read shouldn't crash the scheduler, but it also shouldn't be
    // invisible.
    let refresh_secs = match pg.get_config("analyzer.full_refresh_secs").await {
        Ok(raw) => raw
            .and_then(|v| v.trim().parse::<i64>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_FULL_REFRESH_SECS),
        Err(e) => {
            tracing::debug!(error = %e, "analyzer_scheduler: full_refresh_secs config read failed — using default");
            DEFAULT_FULL_REFRESH_SECS
        }
    };
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    // Restore the watermark + last-refresh from config so a restart resumes
    // incrementally instead of re-analyzing everything.
    let mut watermark = match pg.get_config(WATERMARK_KEY).await {
        Ok(Some(raw)) => deserialize_watermark(&raw),
        _ => HashMap::new(),
    };
    let mut last_refresh_ms: Option<i64> = match pg.get_config(LAST_REFRESH_KEY).await {
        Ok(raw) => raw.and_then(|v| v.trim().parse::<i64>().ok()),
        Err(e) => {
            tracing::warn!(error = %e, "analyzer_scheduler: last_refresh watermark read failed — treating as never-refreshed");
            None
        }
    };
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
        // insights stay fresh even when no new sessions arrived. Community
        // detection also runs here — see the module-level comment for why the
        // daily cadence is the right home for it.
        let now_ms = Utc::now().timestamp_millis();
        let is_full_refresh = due_for_full_refresh(now_ms, last_refresh_ms, refresh_secs);
        if is_full_refresh {
            for (pid, _) in &activity {
                if !due.contains(pid) {
                    due.push(*pid);
                }
            }
            last_refresh_ms = Some(now_ms);
            // Watermark persist matters at restart — without it we'd
            // redo the full refresh on every reboot. Log if the write
            // fails so an operator can act; in-memory watermark stays
            // advanced so the current process still behaves.
            if let Err(e) = pg.set_config(LAST_REFRESH_KEY, &now_ms.to_string()).await {
                tracing::warn!(error = %e, "analyzer_scheduler: persisting last_refresh watermark failed — restart may redo full refresh");
            }
            tracing::info!(projects = activity.len(), "analyzer_scheduler: daily full refresh");
            // Enqueue community detection per indexed folder for every active
            // project. Log-and-skip on lookup errors — a bad row shouldn't
            // block the rest of the daily pass.
            for (pid, _) in &activity {
                match pg.get_indexed_folder_paths_for_project(pid).await {
                    Ok(paths) => {
                        for abs_path in paths {
                            queue
                                .enqueue(Task::new(TaskKind::DetectCommunities, &abs_path, ""))
                                .await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!(project_id = %pid, error = %e, "analyzer_scheduler: could not list folders for community detection");
                    }
                }
            }
            // LLM process-quality pass, daily only (spec 2026-08-20): score each
            // active project's un-scored transcripts. Watermark-gated + batch-capped
            // in the handler, so this daily enqueue drains the backlog incrementally.
            for (pid, _) in &activity {
                enqueue_daily_process(&queue, pid).await;
            }
        }

        let any_due = !due.is_empty();
        for pid in &due {
            enqueue_due_project(&queue, pid).await;
        }
        // Global passes once per tick when work happened: corrections cluster
        // across projects; verdicts re-measure accepted recs' before/after FTR;
        // pending verdicts get classified so the Health-tab aggregate sees every
        // session (not just opened ones); tool insights snapshot the per-tool
        // signal cards so the observatory Insights tab reads from cache (T2 Slice D).
        if any_due {
            enqueue_global_passes(&queue).await;
            // Persist the advanced watermark so a restart doesn't redo this work.
            // Same restart-risk category as LAST_REFRESH_KEY — log on failure
            // so an operator can see we're at risk of re-analyzing after boot.
            if let Err(e) = pg.set_config(WATERMARK_KEY, &serialize_watermark(&watermark)).await {
                tracing::warn!(error = %e, "analyzer_scheduler: persisting activity watermark failed — restart may re-enqueue analyzed projects");
            }
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

    #[tokio::test]
    async fn due_project_enqueues_analyze_and_doc_drift() {
        let queue = TaskQueue::new();
        let pid = uuid::Uuid::new_v4();
        enqueue_due_project(&queue, &pid).await;

        // Exactly two tasks, both targeting this project: AnalyzeProject +
        // ScanDocDrift (doc-drift rides the same due signal).
        assert_eq!(queue.status().await.pending, 2);
        let mut kinds: Vec<TaskKind> = Vec::new();
        for _ in 0..2 {
            let t = queue.next_task().await;
            assert_eq!(t.path, pid.to_string(), "task carries the project id");
            kinds.push(t.kind);
        }
        kinds.sort_by_key(|k| k.to_string());
        assert_eq!(kinds, vec![TaskKind::AnalyzeProject, TaskKind::ScanDocDrift]);
    }

    #[tokio::test]
    async fn global_passes_classify_before_aggregate_tool_insights() {
        // Generous per-folder budget so draining the "" (global) folder doesn't
        // hit the concurrency cap while we read the FIFO enqueue order.
        let queue = TaskQueue::with_max_repos(16);
        enqueue_global_passes(&queue).await;

        let n = queue.status().await.pending;
        let mut kinds: Vec<TaskKind> = Vec::new();
        for _ in 0..n {
            kinds.push(queue.next_task().await.kind);
        }
        let ci = kinds
            .iter()
            .position(|k| *k == TaskKind::ClassifyPendingVerdicts)
            .expect("ClassifyPendingVerdicts enqueued");
        let ai = kinds
            .iter()
            .position(|k| *k == TaskKind::AggregateToolInsights)
            .expect("AggregateToolInsights enqueued");
        assert!(
            ci < ai,
            "ClassifyPendingVerdicts must be enqueued before AggregateToolInsights, got {kinds:?}",
        );
        assert!(
            kinds.contains(&TaskKind::ConsolidateGovernance),
            "governance Tier-2 consolidation must ride the global-passes tick, got {kinds:?}",
        );
        assert!(
            kinds.contains(&TaskKind::WarmInsightCopy),
            "eager insight-copy warming must ride the global-passes tick, got {kinds:?}",
        );
        assert!(
            kinds.contains(&TaskKind::LearnPlaybooks),
            "§9 learning-loop pass must ride the global-passes tick, got {kinds:?}",
        );
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
