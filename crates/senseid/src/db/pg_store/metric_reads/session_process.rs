//! `session_process` group reads (#227) — the per-day roll-up of the LLM
//! process-quality judgments the `session_process` analyzer wrote into
//! `activity.sessions.props.process`.

use super::{bind_day, day_filter};
use crate::db::pg_store::PgStore;

/// Sessions bucket/window on `started_at` (same anchor as `session_outcomes`).
const DAY_ANCHOR: &str = "s.started_at";

/// One (day × repository) roll-up of the process judgments:
/// `(day, repository_id, depth_sum, depth_n, dev_present, dev_applicable,
///  refuted_present, incomplete_present, scored_n)`.
/// - `depth_sum`/`depth_n` → mean spec_depth over sessions with a non-null score.
/// - `dev_present`/`dev_applicable` → spec_deviation_rate (applicable = plan existed).
/// - `refuted_present`/`incomplete_present` over `scored_n` (process-scored sessions).
pub(crate) type ProcessJudgmentDay =
    (chrono::NaiveDate, uuid::Uuid, f64, i64, i64, i64, i64, i64, i64);

impl PgStore {
    /// Per-(day × repository) process-judgment aggregates over the selected day-set,
    /// project-scoped, for sessions the analyzer has scored (`props ? 'process'`).
    /// The measurable base matches `session_outcomes` (`outcome is not null <> empty`);
    /// a session whose repo anchor can't resolve is EXCLUDED (never fabricated).
    pub(crate) async fn session_process_judgments_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<ProcessJudgmentDay>, String> {
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
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, ProcessJudgmentDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }
}
