//! Compute-time per-datapoint explainer enrichment.
//!
//! After a metric GROUP writes its project-scope DAILY rows for a day, this
//! generates (or refreshes) each datapoint's one-line explainer and MERGES it
//! into that row's `props.explainer`. It runs INLINE in the metrics
//! [`compute()`](super::compute) dispatcher — the explainer is produced WITH the
//! value, never on the read path.
//!
//! Cache-guarded: [`metric_day_explainer::explain`] hashes facts that INCLUDE the
//! value (and its prior day + delta), so an unchanged value re-hits the
//! `insight_copy` cache and makes NO model call, while a changed value misses and
//! regenerates. The value upsert's `ON CONFLICT` overwrites `props` wholesale, so
//! this MERGE runs after it and is the last writer (numerator/denominator survive).
//!
//! Best-effort by construction: a read/generate/merge failure for one row is
//! logged and skipped — the value is already persisted, so a failed explainer
//! never fails the compute task (never wedges the queue) and never fabricates.
//! Rides the planner's per-day `Some(D)` tasks (backfill + trailing-window refresh
//! + today); the decision record is `docs/analysis/metric-explainability-generation.md`.

use crate::analysis::metric_day_explainer::{explain, MetricDayFacts};
use crate::tasks::executor::TaskContext;

/// Enrich every project-scope DAILY datapoint a `task_name` group holds for `day`
/// with its one-line explainer (merged into `props.explainer`). Returns the number
/// of rows enriched (`0` = honest no-op: the group wrote no project-scope daily row
/// that day). Never errors — per-row failures are logged and skipped.
pub(super) async fn enrich_day(
    ctx: &TaskContext,
    project_id: &uuid::Uuid,
    task_name: &str,
    day: chrono::NaiveDate,
) -> u32 {
    let pg = ctx.pg();
    let rows = match pg.get_group_daily_metrics_for_day(project_id, task_name, day).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, task_name, %day, "metric explainer: group-rows read failed — skipped");
            return 0;
        }
    };
    if rows.is_empty() {
        return 0;
    }
    // The day's session-outcome context is identical for every metric in the group
    // on this day — read it ONCE, not per row.
    let (sessions_total, sessions_completed, first_try) =
        match pg.get_day_session_outcome_counts(project_id, day).await {
            Ok(counts) => counts,
            Err(e) => {
                tracing::warn!(error = %e, %day, "metric explainer: day-context read failed — skipped");
                return 0;
            }
        };
    // Deref-coerce Arc<Gateway> → &Gateway for the producer.
    let gateway: &gateway::Gateway = &ctx.app_state.gateway;

    let mut enriched = 0u32;
    for (row_id, key, value) in rows {
        // What the metric measures (registry `how_to_read`); empty when the key
        // names no registered metric — the producer then leans on key + numbers.
        let meaning = pg
            .get_metric_meaning(&key)
            .await
            .ok()
            .flatten()
            .map(|m| m.how_to_read)
            .unwrap_or_default();
        let prev_value = match pg.get_prev_daily_metric_value(project_id, &key, day).await {
            Ok(prev) => prev,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, %day, "metric explainer: prev-value read failed — row skipped");
                continue;
            }
        };
        let facts = MetricDayFacts {
            metric: key.clone(),
            meaning,
            value,
            prev_value,
            delta: prev_value.map(|p| value - p),
            day: day.to_string(),
            sessions_completed,
            sessions_total,
            first_try,
        };
        // Cache HIT (unchanged value) → reuse, no model call; MISS → generate;
        // model None → deterministic honest fallback. Never blocks, never fabricates.
        let line = explain(pg, gateway, &facts).await;
        match pg.merge_metric_explainer(&row_id, &line).await {
            Ok(()) => enriched += 1,
            Err(e) => {
                tracing::warn!(error = %e, key = %key, %day, "metric explainer: props merge failed — row skipped")
            }
        }
    }
    enriched
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::insight_copy::{facts_hash, InsightKind};
    use crate::analysis::metric_day_explainer::MetricDayFacts;
    use crate::tasks::test_support::{make_ctx, seed_metrics_project_folder};
    use crate::tasks::{Task, TaskKind};
    use serde_json::json;
    use sqlx_core::query_as::query_as;

    /// The `props->>'explainer'` (and a preserved `numerator`) for the project's
    /// single seeded daily row.
    async fn read_props(
        pg: &crate::db::pg_store::PgStore,
        pid: &uuid::Uuid,
    ) -> (Option<String>, Option<String>) {
        query_as(
            "SELECT props->>'explainer', props->>'numerator'
               FROM sensei.project_metrics
              WHERE project_id = $1 AND grain = 'daily'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap()
    }

    /// Build the SAME facts the enrichment builds (by calling the same store
    /// helpers), so a seeded `insight_copy` row lands on the identical `facts_hash`
    /// — a deterministic cache HIT independent of any live model.
    async fn seed_cached_explainer(
        pg: &crate::db::pg_store::PgStore,
        pid: &uuid::Uuid,
        key: &str,
        value: f64,
        day: chrono::NaiveDate,
        detail: &str,
    ) {
        let meaning = pg
            .get_metric_meaning(key)
            .await
            .unwrap()
            .map(|m| m.how_to_read)
            .unwrap_or_default();
        let (total, completed, first_try) =
            pg.get_day_session_outcome_counts(pid, day).await.unwrap();
        let prev = pg.get_prev_daily_metric_value(pid, key, day).await.unwrap();
        let facts = MetricDayFacts {
            metric: key.to_string(),
            meaning,
            value,
            prev_value: prev,
            delta: prev.map(|p| value - p),
            day: day.to_string(),
            sessions_completed: completed,
            sessions_total: total,
            first_try,
        };
        let fh = facts_hash(InsightKind::MetricDayExplainer, &facts.to_facts_json());
        pg.upsert_insight_copy("metric_day_explainer", &fh, "a quiet climb", detail, None, None)
            .await;
    }

    #[tokio::test]
    async fn enrich_day_merges_cached_explainer_preserving_other_props() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let day = (chrono::Utc::now() - chrono::Duration::days(4)).date_naive();

        // A session_outcomes daily ftr datapoint at 0.75 with real parts in props.
        let mid = *pg.active_metric_ids("session_outcomes").await.unwrap().get("ftr").expect("ftr metric");
        let rid = crate::tasks::test_support::repository_for_folder(pg, &fid).await;
        pg.upsert_project_metric_repo(&mid, &rid, "user", None, None, day, "daily", 0.75, &json!({"numerator": 3, "denominator": 4}), "measured")
            .await
            .unwrap();
        seed_cached_explainer(pg, &pid, "ftr", 0.75, day, "three of four landed first-try").await;

        let n = enrich_day(&ctx, &pid, "session_outcomes", day).await;
        assert_eq!(n, 1, "the single daily datapoint was enriched");

        let (explainer, numerator) = read_props(pg, &pid).await;
        assert_eq!(explainer.as_deref(), Some("three of four landed first-try"), "the cached copy is merged in (cache HIT, no model call)");
        assert_eq!(numerator.as_deref(), Some("3"), "the value's other props survive the merge");

        crate::tasks::test_support::cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn compute_dispatcher_enriches_the_groups_datapoints() {
        // Wiring: the top-level compute() runs the group computer THEN the explainer
        // enrichment. session_outcomes finds no seeded sessions (writes no new row),
        // so the pre-seeded ftr row stands in for a just-computed datapoint; a cached
        // copy proves compute() invoked the enrichment.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let day = (chrono::Utc::now() - chrono::Duration::days(6)).date_naive();

        let mid = *pg.active_metric_ids("session_outcomes").await.unwrap().get("ftr").expect("ftr metric");
        let rid = crate::tasks::test_support::repository_for_folder(pg, &fid).await;
        pg.upsert_project_metric_repo(&mid, &rid, "user", None, None, day, "daily", 0.6, &json!({"numerator": 3, "denominator": 5}), "measured")
            .await
            .unwrap();
        seed_cached_explainer(pg, &pid, "ftr", 0.6, day, "three of five first-try").await;

        let task = Task::new(TaskKind::ComputeGroupMetrics, &pid.to_string(), "session_outcomes").with_as_of(day);
        super::super::compute_group(&ctx, &task).await.unwrap();

        let (explainer, _) = read_props(pg, &pid).await;
        assert_eq!(explainer.as_deref(), Some("three of five first-try"), "compute_group() ran the explainer enrichment on the group's daily datapoint");

        crate::tasks::test_support::cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn enrich_day_is_honest_noop_when_group_has_no_daily_row() {
        // No project_metrics row for the day → nothing to explain → 0 (honest-empty),
        // never a fabricated datapoint.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let day = (chrono::Utc::now() - chrono::Duration::days(9)).date_naive();

        let n = enrich_day(&ctx, &pid, "session_outcomes", day).await;
        assert_eq!(n, 0, "no daily datapoint → no enrichment");

        crate::tasks::test_support::cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
