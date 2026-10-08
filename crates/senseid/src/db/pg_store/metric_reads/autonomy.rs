//! `autonomy` group reads (#227) — how often the human has to step in, and how
//! often a run reaches the end on its own.
//!
//! Both reads bucket and window on the source row's TRUE occurrence time — the
//! event's client clock `assistant_events.ts` (epoch ms →
//! `to_timestamp(ts / 1000.0)`) for the interruption counts, and
//! `runs.started_at` for the run-completion counts — never an insert-time
//! `created_at`, which is `now` for a synthesized or back-dated row. `as_of =
//! None` keeps the rolling `now() - make_interval` window; `as_of = Some(D)`
//! selects the single day `D` (the backfill path), via the shared [`day_filter`]
//! / [`bind_day`] `$2` contract.

use super::{bind_day, day_filter};
use crate::db::pg_store::PgStore;

/// The `assistant_events.event_type` literals the interruption counts filter on,
/// bound as `$3`/`$4`. These are the raw hook names (`hook_event_name`) as
/// written by the capture path and used across `analyze` / `verdict_classifier`
/// / the transcript synthesizers.
const EVENT_STOP: &str = "Stop";
const EVENT_USER_PROMPT: &str = "UserPromptSubmit";

/// This group's occurrence-time anchors for the shared [`day_filter`] /
/// [`bind_day`] `$2` day-set contract. The interruption counts bucket/window on
/// the event's CLIENT clock `ts` (epoch ms → `to_timestamp(ts / 1000.0)`, the
/// same `ae.ts / 1000.0` convention `get_project_sessions_needing_enrichment`
/// uses), NOT the server insert `created_at` — a synthesized/back-dated event
/// carries its true occurrence time in `ts` while `created_at` is `now`. The
/// run-completion counts bucket on `runs.started_at`.
const ANCHOR_INTERRUPTION: &str = "to_timestamp(ae.ts / 1000.0)";
const ANCHOR_RUN_COMPLETION: &str = "r.started_at";

/// One day's interruption counts for a repository: `(day, repository_id,
/// stop_count, prompt_count)`. `prompt_count` is the `interruption_rate`
/// denominator; the row is keyed to the SESSION's resolved repository.
pub(crate) type InterruptionDay = (chrono::NaiveDate, uuid::Uuid, i64, i64);

/// One day's run counts for a project: `(day, done_count, started_count)`.
/// `started_count` (every run started that day) is the `run_completion`
/// denominator.
pub(crate) type RunCompletionDay = (chrono::NaiveDate, i64, i64);

impl PgStore {
    /// Daily `Stop` / `UserPromptSubmit` counts per REPOSITORY over the selected
    /// day-set (rolling window when `as_of = None`, the single day `D` when
    /// `Some(D)`).
    ///
    /// `activity.assistant_events` carries no project id — its `session_id` is
    /// the assistant's own session-id string. Events attribute to a session
    /// through `activity.sessions.client_session_id = assistant_events.
    /// session_id`, and to a REPOSITORY through that session's `repo_folder_id →
    /// sensei.folders.repository_id`. Counts are GROUP BY `(day, repository)`,
    /// so each day/repository yields its own row.
    ///
    /// Events whose session matches no session, a session in another project
    /// (the `sessions.project_id = $1` scope), or a session whose repository
    /// cannot be resolved (`repo_folder_id` NULL, or that folder's
    /// `repository_id` NULL) are EXCLUDED — the row is skipped, never attributed
    /// to a fabricated repository (I-E).
    ///
    /// Bucketed by the event's CLIENT `ts` (its true occurrence day), not the
    /// insert `created_at`.
    pub(crate) async fn autonomy_interruption_counts_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<InterruptionDay>, String> {
        let sql = format!(
            "SELECT date_trunc('day', to_timestamp(ae.ts / 1000.0))::date      AS day
                  , rf.repository_id                                            AS repository_id
                  , count(*) FILTER (WHERE ae.event_type = $3)::int8           AS stop_count
                  , count(*) FILTER (WHERE ae.event_type = $4)::int8           AS prompt_count
               FROM activity.assistant_events ae
               JOIN activity.sessions        s  ON s.client_session_id = ae.session_id
               JOIN sensei.folders           rf ON rf.id = s.repo_folder_id
              WHERE s.project_id      = $1
                AND rf.repository_id IS NOT NULL
                AND ae.event_type    IN ($3, $4)
                AND {}
              GROUP BY 1, 2
              ORDER BY 1, 2",
            day_filter(ANCHOR_INTERRUPTION, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, InterruptionDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of)
            .bind(EVENT_STOP)
            .bind(EVENT_USER_PROMPT)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())
    }

    /// Daily run-completion counts over the selected day-set (rolling window
    /// when `as_of = None`, the single day `D` when `Some(D)`), project-scoped
    /// via the direct `runs.project_id` FK.
    ///
    /// `done_count` is runs whose terminal `status = 'done'`; `started_count` is
    /// every run started that day (the denominator). Bucketed by `started_at` —
    /// a run counts on the day it started, regardless of when it finished.
    ///
    /// `activity.runs` carries NO repository, so these counts are project-wide
    /// and have no natural per-repository grain; the caller decides which
    /// repository to attribute them to.
    pub(crate) async fn autonomy_run_completion_by_day(
        &self,
        project_id: &uuid::Uuid,
        window_days: u32,
        as_of: Option<chrono::NaiveDate>,
    ) -> Result<Vec<RunCompletionDay>, String> {
        let sql = format!(
            "SELECT date_trunc('day', r.started_at)::date                          AS day
                  , count(*) FILTER (WHERE r.status = 'done'::sensei.run_status)::int8 AS done_count
                  , count(*)::int8                                                  AS started_count
               FROM activity.runs r
              WHERE r.project_id  = $1
                AND {}
              GROUP BY 1
              ORDER BY 1",
            day_filter(ANCHOR_RUN_COMPLETION, as_of),
        );
        let q = sqlx_core::query_as::query_as::<_, RunCompletionDay>(&sql).bind(project_id);
        bind_day(q, window_days, as_of).fetch_all(&self.pool).await.map_err(|e| e.to_string())
    }
}
