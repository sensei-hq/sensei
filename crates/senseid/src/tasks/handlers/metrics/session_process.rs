//! `session_process` metric group computer (spec 2026-08-20) — the day-grain
//! aggregation of the LLM process-quality judgments the
//! [`crate::tasks::handlers::session_process`] analyzer wrote into
//! `activity.sessions.props.process`.
//!
//! Repo-grain, `scope = 'user'`, daily rows into `sensei.project_metrics` (via
//! [`PgStore::upsert_project_metric_repo`]), exactly like `session_outcomes` — a
//! project value is the pooling view over these per-repository rows. Four metrics:
//! - `spec_depth` (score): mean 0-5 over sessions whose `props.process.spec_depth.score`
//!   is non-null (a stated plan was judged). N/A sessions don't dilute it.
//! - `spec_deviation_rate` (pct): `present=true` / sessions where spec_deviation is a
//!   real boolean (`present` not null — i.e. a plan existed).
//! - `refuted_finding_rate` (pct): `present=true` / process-scored measurable sessions.
//! - `incomplete_analysis_llm_rate` (pct): `present=true` / process-scored measurable sessions.
//!
//! Never-fabricate: only sessions with `props ? 'process'` (analyzer-scored) count,
//! and a (day, repo) with a zero denominator writes NO row (a 0/0 would be a
//! fabricated zero). Every read propagates `Err`. Mirrors `session_outcomes`'
//! repo resolution + measurable base + day-filter/window contract.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

const GRAIN_DAILY: &str = "daily";
const SCOPE_USER: &str = "user";
const SOURCE_MEASURED: &str = "measured";

const KEY_SPEC_DEPTH: &str = "spec_depth";
const KEY_SPEC_DEVIATION: &str = "spec_deviation_rate";
const KEY_REFUTED: &str = "refuted_finding_rate";
const KEY_INCOMPLETE: &str = "incomplete_analysis_llm_rate";

/// Sessions bucket/window on `started_at` (same anchor as `session_outcomes`).
const DAY_ANCHOR: &str = "s.started_at";

/// One (day × repository) roll-up of the process judgments:
/// `(day, repository_id, depth_sum, depth_n, dev_present, dev_applicable,
///  refuted_present, incomplete_present, scored_n)`.
/// - `depth_sum`/`depth_n` → mean spec_depth over sessions with a non-null score.
/// - `dev_present`/`dev_applicable` → spec_deviation_rate (applicable = plan existed).
/// - `refuted_present`/`incomplete_present` over `scored_n` (process-scored sessions).
type DayProc = (chrono::NaiveDate, uuid::Uuid, f64, i64, i64, i64, i64, i64, i64);

/// Per-(day × repository) process-judgment aggregates over the selected day-set,
/// project-scoped, for sessions the analyzer has scored (`props ? 'process'`).
/// The measurable base matches `session_outcomes` (`outcome is not null <> empty`);
/// a session whose repo anchor can't resolve is EXCLUDED (never fabricated).
async fn daily_process_aggregates(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayProc>, String> {
    // Booleans/scores read out of props.process. `?` presence gates the base.
    let sql = format!(
        "SELECT date_trunc('day', s.started_at)::date                                        AS day
              , rf.repository_id                                                              AS repository_id
              , coalesce(sum((s.props->'process'->'spec_depth'->>'score')::float8), 0)::float8 AS depth_sum
              , count(*) FILTER (WHERE (s.props->'process'->'spec_depth'->>'score') IS NOT NULL)::int8 AS depth_n
              , count(*) FILTER (WHERE (s.props->'process'->'spec_deviation'->>'present')::bool)::int8 AS dev_present
              , count(*) FILTER (WHERE jsonb_typeof(s.props->'process'->'spec_deviation'->'present') = 'boolean')::int8 AS dev_applicable
              , count(*) FILTER (WHERE (s.props->'process'->'refuted_findings'->>'present')::bool)::int8 AS refuted_present
              , count(*) FILTER (WHERE (s.props->'process'->'incomplete_analysis_llm'->>'present')::bool)::int8 AS incomplete_present
              , count(*)::int8                                                                AS scored_n
           FROM activity.sessions s
           JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
          WHERE s.project_id  = $1
            AND rf.repository_id IS NOT NULL
            AND s.props ? 'process'
            AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome
            AND {}
          GROUP BY 1, 2
          ORDER BY 1, 2",
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayProc>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of).fetch_all(pg.pool()).await.map_err(|e| e.to_string())
}

/// Compute the `session_process` group for one project. Mirrors
/// [`super::session_outcomes::compute`]'s contract (repo grain, `as_of` day-set,
/// idempotent upsert, honest-empty). Returns the number of rows written.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("session_process: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    let ids = pg.active_metric_ids(MetricGroup::SessionProcess.as_str()).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let depth_id = ids.get(KEY_SPEC_DEPTH).copied();
    let dev_id = ids.get(KEY_SPEC_DEVIATION).copied();
    let refuted_id = ids.get(KEY_REFUTED).copied();
    let incomplete_id = ids.get(KEY_INCOMPLETE).copied();

    let mut written = 0u32;
    for (
        day,
        repository_id,
        depth_sum,
        depth_n,
        dev_present,
        dev_applicable,
        refuted_present,
        incomplete_present,
        scored_n,
    ) in daily_process_aggregates(pg, &project_id, window_days, as_of).await?
    {
        // spec_depth (score): mean over sessions with a real plan-depth score.
        if let Some(mid) = depth_id
            && depth_n > 0
        {
            let value = depth_sum / depth_n as f64;
            let props = serde_json::json!({ "n": depth_n });
            pg.upsert_project_metric_repo(
                &mid,
                &repository_id,
                SCOPE_USER,
                None,
                None,
                day,
                GRAIN_DAILY,
                value,
                &props,
                SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
        // spec_deviation_rate (pct): denominator = sessions with a plan (applicable).
        if let Some(mid) = dev_id
            && dev_applicable > 0
        {
            let value = dev_present as f64 / dev_applicable as f64;
            let props =
                serde_json::json!({ "numerator": dev_present, "denominator": dev_applicable });
            pg.upsert_project_metric_repo(
                &mid,
                &repository_id,
                SCOPE_USER,
                None,
                None,
                day,
                GRAIN_DAILY,
                value,
                &props,
                SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
        // refuted_finding_rate + incomplete_analysis_llm_rate (pct): denominator =
        // process-scored measurable sessions that day.
        if scored_n > 0 {
            for (mid_opt, present) in
                [(refuted_id, refuted_present), (incomplete_id, incomplete_present)]
            {
                if let Some(mid) = mid_opt {
                    let value = present as f64 / scored_n as f64;
                    let props =
                        serde_json::json!({ "numerator": present, "denominator": scored_n });
                    pg.upsert_project_metric_repo(
                        &mid,
                        &repository_id,
                        SCOPE_USER,
                        None,
                        None,
                        day,
                        GRAIN_DAILY,
                        value,
                        &props,
                        SOURCE_MEASURED,
                    )
                    .await?;
                    written += 1;
                }
            }
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        seed_metrics_project_folder, seed_metrics_session,
    };

    /// Set a session's props.process to the given judgment object.
    async fn set_process(pg: &PgStore, sid: &uuid::Uuid, process: serde_json::Value) {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET props = coalesce(props,'{}'::jsonb) || jsonb_build_object('process', $2::jsonb) WHERE id=$1",
        )
        .bind(sid)
        .bind(process)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn computes_depth_mean_and_occurrence_rates() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);

        // s1: plan depth 4, deviated, no refute, no incomplete.
        let s1 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        set_process(pg, &s1, serde_json::json!({
            "spec_depth": {"score": 4}, "spec_deviation": {"present": true},
            "refuted_findings": {"present": false}, "incomplete_analysis_llm": {"present": false}
        })).await;
        // s2: plan depth 2, did NOT deviate, refuted a finding, incomplete analysis.
        let s2 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        set_process(
            pg,
            &s2,
            serde_json::json!({
                "spec_depth": {"score": 2}, "spec_deviation": {"present": false},
                "refuted_findings": {"present": true}, "incomplete_analysis_llm": {"present": true}
            }),
        )
        .await;
        // s3: NO plan (depth null, deviation null) — excluded from depth mean +
        // deviation denominator, but still counts for refuted/incomplete base.
        let s3 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        set_process(pg, &s3, serde_json::json!({
            "spec_depth": {"score": null}, "spec_deviation": {"present": null},
            "refuted_findings": {"present": false}, "incomplete_analysis_llm": {"present": false}
        })).await;
        // s4: measurable but NOT process-scored (no props.process) — excluded entirely.
        let _s4 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 4, "4 metric rows (depth + deviation + refuted + incomplete)");
        let daily = daily_rows(pg, &pid).await;

        let depth = daily.iter().find(|r| r.0 == "spec_depth").expect("spec_depth row");
        assert!(
            (depth.1 - 3.0).abs() < 1e-9,
            "mean(4,2) = 3 over the two planned sessions (s3 null excluded)"
        );
        assert_eq!(depth.2["n"].as_i64(), Some(2), "depth n = sessions with a plan");

        let dev = daily.iter().find(|r| r.0 == "spec_deviation_rate").expect("deviation row");
        assert!(
            (dev.1 - 0.5).abs() < 1e-9,
            "1 deviated / 2 planned = 0.5 (s3 has no plan → excluded)"
        );
        assert_eq!(
            dev.2["denominator"].as_i64(),
            Some(2),
            "deviation denominator = planned sessions"
        );

        let refuted = daily.iter().find(|r| r.0 == "refuted_finding_rate").expect("refuted row");
        assert!(
            (refuted.1 - 1.0 / 3.0).abs() < 1e-9,
            "1 refuted / 3 process-scored = 1/3 (s4 unscored excluded)"
        );
        assert_eq!(
            refuted.2["denominator"].as_i64(),
            Some(3),
            "refuted denominator = process-scored sessions"
        );

        let incomplete =
            daily.iter().find(|r| r.0 == "incomplete_analysis_llm_rate").expect("incomplete row");
        assert!((incomplete.1 - 1.0 / 3.0).abs() < 1e-9, "1 incomplete / 3 = 1/3");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn no_planned_session_writes_no_depth_or_deviation_row() {
        // A day whose scored sessions all lack a plan → depth denominator 0 +
        // deviation applicable 0 → NO depth/deviation rows (never a fabricated 0/0);
        // refuted/incomplete still compute over the scored base.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        let s = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        set_process(pg, &s, serde_json::json!({
            "spec_depth": {"score": null}, "spec_deviation": {"present": null},
            "refuted_findings": {"present": false}, "incomplete_analysis_llm": {"present": false}
        })).await;

        compute(&ctx, &pid.to_string(), None).await.unwrap();
        let daily = daily_rows(pg, &pid).await;
        assert!(
            daily.iter().all(|r| r.0 != "spec_depth"),
            "no spec_depth row when no session had a plan"
        );
        assert!(
            daily.iter().all(|r| r.0 != "spec_deviation_rate"),
            "no deviation row when nothing was applicable"
        );
        assert!(
            daily.iter().any(|r| r.0 == "refuted_finding_rate"),
            "refuted rate still computes over the scored base"
        );

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unscored_project_writes_zero_rows() {
        // A project whose sessions are not process-scored (no props.process) writes
        // NO rows (honest-empty), even though the sessions are measurable.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no process-scored sessions → no rows (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
