//! `session_outcomes` group reads (#227) — what the day's sessions did, per
//! repository.
//!
//! Repo grain: every aggregate GROUPs BY the session's repository — the
//! `sensei.folders.repository_id` of the session's durable repo anchor
//! (`activity.sessions.repo_folder_id`). Sessions whose repo can't be resolved
//! (`repo_folder_id` NULL, or that folder's `repository_id` NULL) are EXCLUDED —
//! never fabricated into a made-up repository.

use super::{bind_day, day_filter};
use crate::db::pg_store::PgStore;

/// This group's occurrence-time anchor for the shared [`day_filter`] /
/// [`bind_day`] `$2` day-set contract: sessions bucket/window on `s.started_at`.
const DAY_ANCHOR: &str = "s.started_at";

/// One (day × repository) session-level aggregate for a project: `(day,
/// repository_id, session_count, ftr_count, correction_count)`. Only
/// (day, repository) pairs WITH ≥1 measurable session appear, so `session_count`
/// (the `ftr` denominator) is always ≥ 1.
pub(crate) type SessionAggDay = (chrono::NaiveDate, uuid::Uuid, i64, i64, i64);

/// One (day × repository) turn-level aggregate for `rework_ratio`: `(day,
/// repository_id, corrected_tool_calls, total_tool_calls)` summed from
/// `activity.turns.tool_calls`.
pub(crate) type ReworkDay = (chrono::NaiveDate, uuid::Uuid, i64, i64);

/// One (day × repository) `time_to_useful_result`: `(day, repository_id,
/// median_seconds, n)`. `n` = the number of sessions that contributed a
/// first-useful latency that day for that repository.
pub(crate) type TturDay = (chrono::NaiveDate, uuid::Uuid, f64, i64);

impl PgStore {
    /// Per-(day × repository) session-level aggregates over the selected day-set
    /// (rolling window when `as_of=None`, the single day `D` when `Some(D)`),
    /// project-scoped via `activity.sessions.project_id`. The session's repository is
    /// its repo anchor's `repository_id` (`sensei.folders.repository_id` WHERE
    /// `folders.id = s.repo_folder_id`); a session whose anchor can't be resolved to a
    /// repository is EXCLUDED (never fabricated into a made-up repository).
    /// `outcome is not null` restricts to measurable (analyzed) sessions — in-flight
    /// sessions whose `ftr`/`outcome` are still `NULL` are excluded from the FTR base.
    pub(crate) async fn session_outcomes_sessions_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<SessionAggDay>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date              AS day
                  , rf.repository_id                                    AS repository_id
                  , count(*)::int8                                     AS session_count
                  , count(*) FILTER (WHERE s.ftr)::int8                AS ftr_count
                  , coalesce(sum(s.corrections), 0)::int8              AS correction_count
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, SessionAggDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) tool-call sums for `rework_ratio`: `corrected_tool_calls`
    /// (numerator) over sessions with `outcome = 'corrected'`, and `total_tool_calls`
    /// (denominator) over all measurable sessions that day for that repository.
    /// Tool-calls come from `activity.turns` (per-turn), never from `activity.sessions`;
    /// a session with no turns contributes 0 either way. Same repo-resolution +
    /// measurable base as [`PgStore::session_outcomes_sessions_by_day`].
    pub(crate) async fn session_outcomes_rework_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<ReworkDay>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date                                         AS day
                  , rf.repository_id                                                               AS repository_id
                  , coalesce(sum(t.tool_calls) FILTER (WHERE s.outcome = 'corrected'::sensei.session_outcome), 0)::int8 AS corrected_tool_calls
                  , coalesce(sum(t.tool_calls), 0)::int8                                           AS total_tool_calls
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
               JOIN activity.turns    t  ON t.session_id = s.id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, ReworkDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) median `time_to_useful_result` (seconds). For each
    /// measurable session, the latency is `started_at → ended_at of the FIRST
    /// non-correction turn` (the first usable output). `percentile_cont(0.5)` medians
    /// those per-session latencies within each (day, repository). Sessions whose only
    /// turns are corrections — or that have no turns — produce no usable output and are
    /// dropped by the inner `LIMIT 1` join (never a fabricated 0). `n` is the
    /// contributing session count that day for that repository. Same repo-resolution +
    /// measurable base as [`PgStore::session_outcomes_sessions_by_day`].
    pub(crate) async fn session_outcomes_time_to_useful_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<TturDay>, String> {
        let sql = format!(
            "WITH first_useful AS ( \
                 SELECT date_trunc('day', s.started_at)::date                        AS day \
                      , rf.repository_id                                             AS repository_id \
                      , EXTRACT(EPOCH FROM (fu.ended_at - s.started_at))::float8      AS secs \
                   FROM activity.sessions s \
                   JOIN sensei.folders    rf ON rf.id = s.repo_folder_id \
                   JOIN LATERAL ( \
                          SELECT t.ended_at \
                            FROM activity.turns t \
                           WHERE t.session_id     = s.id \
                             AND t.is_correction  = false \
                           ORDER BY t.turn_number \
                           LIMIT 1 \
                        ) fu ON true \
                  WHERE s.project_id  = $1 \
                    AND rf.repository_id IS NOT NULL \
                    AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome \
                    AND {} \
             ) \
             SELECT day \
                  , repository_id \
                  , percentile_cont(0.5) WITHIN GROUP (ORDER BY secs)::float8         AS median_secs \
                  , count(*)::int8                                                     AS n \
               FROM first_useful \
              WHERE secs >= 0 \
              GROUP BY day, repository_id \
              ORDER BY day, repository_id",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, TturDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) context-pressure counts: `(day, repository_id,
    /// pressured, total)` — sessions carrying a context-pressure trouble signal
    /// (Phase D `props.trouble.hint` ∈ {context-pressure, suggested-restart}) over the
    /// measurable base. The rate is `pressured / total`; a (day, repository) with a
    /// real denominator writes a row (even a 0). Same repo-resolution + measurable base
    /// as [`PgStore::session_outcomes_sessions_by_day`].
    pub(crate) async fn session_outcomes_context_pressure_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<(chrono::NaiveDate, uuid::Uuid, i64, i64)>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date                                          AS day
                  , rf.repository_id                                                                AS repository_id
                  , count(*) FILTER (WHERE s.props->'trouble'->>'hint' IN ('context-pressure','suggested-restart'))::int8 AS pressured
                  , count(*)::int8                                                                  AS total
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
            .bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) token-volume sums: `(day, repository_id, sum_in,
    /// sum_out, n)`. Base = sessions that carry token usage (`tokens_in IS NOT NULL`)
    /// — token volume is independent of outcome analysis, so this base is NOT the
    /// measurable-outcome base the rate metrics use; a session with no captured tokens
    /// contributes nothing (never a fabricated 0). `n` = sessions with tokens that day.
    pub(crate) async fn session_outcomes_token_volume_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<(chrono::NaiveDate, uuid::Uuid, i64, i64, i64)>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date        AS day
                  , rf.repository_id                              AS repository_id
                  , coalesce(sum(s.tokens_in), 0)::int8           AS sum_in
                  , coalesce(sum(s.tokens_out), 0)::int8          AS sum_out
                  , count(*)::int8                                AS n
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.tokens_in IS NOT NULL
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64, i64)>(
            &sql,
        )
        .bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) mean active session duration in SECONDS: `(day,
    /// repository_id, avg_secs, n)`. Base = sessions with a recorded `duration`
    /// interval (gap-aware active work time); a session with no duration contributes
    /// nothing (honest-empty, never a fabricated 0). `n` = contributing sessions.
    pub(crate) async fn session_outcomes_session_duration_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<(chrono::NaiveDate, uuid::Uuid, f64, i64)>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date                       AS day
                  , rf.repository_id                                            AS repository_id
                  , avg(EXTRACT(EPOCH FROM s.duration))::float8                 AS avg_secs
                  , count(*)::int8                                              AS n
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.duration   IS NOT NULL
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, f64, i64)>(&sql)
            .bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) `tokens_per_result`: `(day, repository_id,
    /// sum_out, completed)` — Σ output tokens over COMPLETED sessions (`outcome =
    /// 'completed'`) that carry token usage / count of those sessions. Output-token
    /// based so it isn't inflated by input cache. A (day, repo) with no completed
    /// token-bearing session writes NO row (honest-empty).
    pub(crate) async fn session_outcomes_tokens_per_result_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<(chrono::NaiveDate, uuid::Uuid, i64, i64)>, String> {
        let sql = format!(
            "SELECT date_trunc('day', s.started_at)::date        AS day
                  , rf.repository_id                              AS repository_id
                  , coalesce(sum(s.tokens_out), 0)::int8          AS sum_out
                  , count(*)::int8                                AS completed
               FROM activity.sessions s
               JOIN sensei.folders    rf ON rf.id = s.repo_folder_id
              WHERE s.project_id  = $1
                AND rf.repository_id IS NOT NULL
                AND s.outcome    = 'completed'::sensei.session_outcome
                AND s.tokens_out IS NOT NULL
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
            .bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }

    /// Per-(day × repository) edit-before-read counts for `incomplete_analysis_rate`:
    /// `(day, repository_id, flagged, measurable)`. Over the measurable base, for each
    /// session it compares the first EDIT-like tool event to the first READ/SEARCH-like
    /// one (from `activity.assistant_events`, joined on `client_session_id`); a session
    /// is FLAGGED when it edits before it reads (or edits with no read at all).
    /// `measurable` = sessions with ≥1 edit-like event that day for the repository —
    /// sessions with no edits are not measurable for this signal (excluded, never a
    /// fabricated 0). Tool-name classification is a cross-adapter heuristic (regex on
    /// the normalized `tool_name`), not intent.
    pub(crate) async fn session_outcomes_incomplete_analysis_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<(chrono::NaiveDate, uuid::Uuid, i64, i64)>, String> {
        // Heuristic tool-name classes, normalized across adapters (Claude Edit/Read,
        // Zed edit_file/read_file, OpenCode edit/read, etc.). EDIT-like is restricted to
        // modifications of existing files (edit/multiedit/str_replace) — NOT `write`,
        // which is usually new-file creation and has nothing to read first.
        const EDIT_RE: &str = "^(edit|multiedit|str_replace)";
        const READ_RE: &str = "^(read|grep|glob|find|ls|list|search|cat)";
        let sql = format!(
            "WITH per_session AS ( \
                 SELECT s.id                                              AS sid \
                      , date_trunc('day', s.started_at)::date             AS day \
                      , rf.repository_id                                  AS repository_id \
                      , min(e.ts) FILTER (WHERE e.tool_name ~* '{edit}')  AS edit_min \
                      , min(e.ts) FILTER (WHERE e.tool_name ~* '{read}')  AS read_min \
                   FROM activity.sessions s \
                   JOIN sensei.folders          rf ON rf.id = s.repo_folder_id \
                   JOIN activity.assistant_events e ON e.session_id = s.client_session_id \
                  WHERE s.project_id  = $1 \
                    AND rf.repository_id IS NOT NULL \
                    AND s.outcome    IS NOT NULL AND s.outcome <> 'empty'::sensei.session_outcome \
                    AND {day} \
                  GROUP BY s.id, 2, 3 \
             ) \
             SELECT day \
                  , repository_id \
                  , count(*) FILTER (WHERE edit_min IS NOT NULL AND (read_min IS NULL OR edit_min < read_min))::int8 AS flagged \
                  , count(*) FILTER (WHERE edit_min IS NOT NULL)::int8                                               AS measurable \
               FROM per_session \
              GROUP BY day, repository_id \
              ORDER BY day, repository_id",
            edit = EDIT_RE,
            read = READ_RE,
            day = day_filter(DAY_ANCHOR, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
            .bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }
}
