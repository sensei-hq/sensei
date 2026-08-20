//! `session_outcomes` metric group computer (Phase 5.1; repo-grain cutover).
//!
//! The FIRST real base-metric computer and the TEMPLATE the other five groups
//! follow. It reads the configured rolling window ([`metrics.window_days`],
//! default 14) and writes REPOSITORY-grain, `scope = 'user'`, daily rows to
//! `sensei.project_metrics` (via [`PgStore::upsert_project_metric_repo`]) for ONE
//! project.
//!
//! Repo grain: every aggregate GROUPs BY the session's repository — the
//! `sensei.folders.repository_id` of the session's durable repo anchor
//! (`activity.sessions.repo_folder_id`) — so ONE project-day yields ONE row per
//! repository the day's sessions touched (a project is a GROUP of repositories;
//! the project value is the pooling view over them, not a row this computer
//! writes). Sessions whose repo can't be resolved (`repo_folder_id` NULL, or that
//! folder's `repository_id` NULL) are EXCLUDED — never fabricated into a made-up
//! repository.
//!
//! v1 registry keys (all `task_name = "session_outcomes"`), all DAILY grain:
//! - `ftr` (pct): first-try-right rate per repository per day (`numerator` =
//!   #`ftr` sessions, `denominator` = # sessions).
//! - `rework_ratio` (ratio): Σ tool-calls in `corrected` sessions / Σ tool-calls
//!   across all sessions — per repository per day.
//! - `throughput` (count): sessions per repository per day.
//! - `time_to_useful_result` (duration): daily median first-useful latency, per
//!   repository.
//! - `context_pressure_rate` (pct): pressured / measurable sessions, per repository.
//!
//! Identity/scope contract (repo-grain identity `_v2`): these rows are
//! `scope = 'user'` (the local user's default-project value), `identity = NULL`
//! (single local user — NULL keeps the identity unique per
//! (metric, repository, day)), `commit_sha = NULL` (day cadence, not commit),
//! `folder_id`/`session_id = NULL`, and `grain = 'daily'` ALWAYS. The former
//! per-session `grain = 'session'` FTR rows are RETIRED: the repo-grain identity
//! carries no `session_id`, so per-session rows would collide — and the daily
//! roll-up views were already the single FTR source of truth.
//!
//! FTR definition: the daily `ftr` is `ftr_count / session_count` over the
//! measurable session base (`outcome is not null`, project scope via
//! `activity.sessions.project_id`, `outcome <> 'empty'`) for that repository — and
//! stores the parts + counts the roll-up views re-aggregate from. These `ftr`
//! rows are the single FTR source of truth (Phase 8 retired the legacy
//! `sensei.ftr_daily` / `sensei.project_ftr_metrics` views).
//!
//! Never-fabricate: every DB call propagates `Err`; a repository-day/metric with
//! no data writes NO row (a `0` value is written only when a real denominator
//! exists). `tool_calls` live on `activity.turns` (per-turn), never on `sessions`.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value — every row this group writes is daily grain
/// (the per-session grain is retired under the repo-grain identity).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_scope` text value — the local user's default-project value.
const SCOPE_USER: &str = "user";
/// `sensei.metric_source` text value — these are measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key`s this computer produces.
const KEY_FTR: &str = "ftr";
const KEY_REWORK: &str = "rework_ratio";
const KEY_THROUGHPUT: &str = "throughput";
const KEY_TTUR: &str = "time_to_useful_result";
const KEY_CONTEXT_PRESSURE: &str = "context_pressure_rate";
/// Token-volume + duration + efficiency + process keys (2026-08-20 addition).
const KEY_TOKENS_PER_DAY: &str = "tokens_per_day";
const KEY_TOKENS_IN: &str = "tokens_in_per_day";
const KEY_TOKENS_OUT: &str = "tokens_out_per_day";
const KEY_SESSION_DURATION: &str = "session_duration";
const KEY_TOKENS_PER_RESULT: &str = "tokens_per_result";
const KEY_INCOMPLETE_ANALYSIS: &str = "incomplete_analysis_rate";

/// One (day × repository) session-level aggregate for a project: `(day,
/// repository_id, session_count, ftr_count, correction_count)`. Only
/// (day, repository) pairs WITH ≥1 measurable session appear, so `session_count`
/// (the `ftr` denominator) is always ≥ 1.
type DayAgg = (chrono::NaiveDate, uuid::Uuid, i64, i64, i64);

/// One (day × repository) turn-level aggregate for `rework_ratio`: `(day,
/// repository_id, corrected_tool_calls, total_tool_calls)` summed from
/// `activity.turns.tool_calls`.
type DayRework = (chrono::NaiveDate, uuid::Uuid, i64, i64);

/// One (day × repository) `time_to_useful_result`: `(day, repository_id,
/// median_seconds, n)`. `n` = the number of sessions that contributed a
/// first-useful latency that day for that repository.
type DayTtur = (chrono::NaiveDate, uuid::Uuid, f64, i64);

/// This group's occurrence-time anchor for the shared [`super::day_filter`] /
/// [`super::bind_day`] `$2` day-set contract: sessions bucket/window on
/// `s.started_at`.
const DAY_ANCHOR: &str = "s.started_at";

/// Per-(day × repository) session-level aggregates over the selected day-set
/// (rolling window when `as_of=None`, the single day `D` when `Some(D)`),
/// project-scoped via `activity.sessions.project_id`. The session's repository is
/// its repo anchor's `repository_id` (`sensei.folders.repository_id` WHERE
/// `folders.id = s.repo_folder_id`); a session whose anchor can't be resolved to a
/// repository is EXCLUDED (never fabricated into a made-up repository).
/// `outcome is not null` restricts to measurable (analyzed) sessions — in-flight
/// sessions whose `ftr`/`outcome` are still `NULL` are excluded from the FTR base.
async fn daily_session_aggregates(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayAgg>, String> {
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayAgg>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) tool-call sums for `rework_ratio`: `corrected_tool_calls`
/// (numerator) over sessions with `outcome = 'corrected'`, and `total_tool_calls`
/// (denominator) over all measurable sessions that day for that repository.
/// Tool-calls come from `activity.turns`; a session with no turns contributes 0
/// either way. Same repo-resolution + measurable base as
/// [`daily_session_aggregates`].
async fn daily_rework(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayRework>, String> {
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayRework>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) median `time_to_useful_result` (seconds). For each
/// measurable session, the latency is `started_at → ended_at of the FIRST
/// non-correction turn` (the first usable output). `percentile_cont(0.5)` medians
/// those per-session latencies within each (day, repository). Sessions whose only
/// turns are corrections — or that have no turns — produce no usable output and are
/// dropped by the inner `LIMIT 1` join (never a fabricated 0). `n` is the
/// contributing session count that day for that repository. Same repo-resolution +
/// measurable base as [`daily_session_aggregates`].
async fn daily_time_to_useful(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayTtur>, String> {
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayTtur>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) context-pressure counts: `(day, repository_id,
/// pressured, total)` — sessions carrying a context-pressure trouble signal
/// (Phase D `props.trouble.hint` ∈ {context-pressure, suggested-restart}) over the
/// measurable base. The rate is `pressured / total`; a (day, repository) with a
/// real denominator writes a row (even a 0). Same repo-resolution + measurable base
/// as [`daily_session_aggregates`].
async fn daily_context_pressure(
    pg: &PgStore,
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
        .bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) token-volume sums: `(day, repository_id, sum_in,
/// sum_out, n)`. Base = sessions that carry token usage (`tokens_in IS NOT NULL`)
/// — token volume is independent of outcome analysis, so this base is NOT the
/// measurable-outcome base the rate metrics use; a session with no captured tokens
/// contributes nothing (never a fabricated 0). `n` = sessions with tokens that day.
async fn daily_token_volume(
    pg: &PgStore,
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64, i64)>(&sql)
        .bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) mean active session duration in SECONDS: `(day,
/// repository_id, avg_secs, n)`. Base = sessions with a recorded `duration`
/// interval (gap-aware active work time); a session with no duration contributes
/// nothing (honest-empty, never a fabricated 0). `n` = contributing sessions.
async fn daily_session_duration(
    pg: &PgStore,
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, f64, i64)>(&sql)
        .bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Per-(day × repository) `tokens_per_result`: `(day, repository_id,
/// sum_out, completed)` — Σ output tokens over COMPLETED sessions (`outcome =
/// 'completed'`) that carry token usage / count of those sessions. Output-token
/// based so it isn't inflated by input cache. A (day, repo) with no completed
/// token-bearing session writes NO row (honest-empty).
async fn daily_tokens_per_result(
    pg: &PgStore,
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
        super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
        .bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
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
async fn daily_incomplete_analysis(
    pg: &PgStore,
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
        day = super::day_filter(DAY_ANCHOR, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, (chrono::NaiveDate, uuid::Uuid, i64, i64)>(&sql)
        .bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Compute the `session_outcomes` group for one project.
///
/// `project_raw` is the project uuid carried in `task.folder_path`. `as_of`
/// selects the day-set:
/// - `None` — the incremental run: every measurable day in the rolling
///   [`metrics.window_days`] window (default 14), `computed_on` = the session's day.
/// - `Some(D)` — the backfill/gap-fill run: ONLY sessions whose day is exactly `D`
///   (`date_trunc('day', started_at)::date = D`), `computed_on` = `D`. This is how
///   a past day reaches the roll-up views (which bucket on `computed_on` with no
///   recent-window filter).
///
/// Every row is written at REPOSITORY grain (`repository_id = Some(..)`,
/// `scope = 'user'`, `identity = NULL`, `commit_sha = NULL`, `folder_id`/
/// `session_id = NULL`, `grain = 'daily'`), one per repository the day's sessions
/// touched. Returns the number of `project_metrics` rows written (`0` =
/// honest-empty: no measurable sessions with a resolvable repository on the
/// selected day-set, or none of the group's metrics active). Idempotent —
/// re-running backfills in place via the upsert identity.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("session_outcomes: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    // Unused on the `as_of=Some` single-day path; the day filter replaces it.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics via the shared store
    // helper. A key absent from the map is inactive (retired / not-yet-effective /
    // unseeded) → skipped: the computer never writes a value for an inactive metric.
    let ids = pg.active_metric_ids(MetricGroup::SessionOutcomes.as_str()).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let ftr_id = ids.get(KEY_FTR).copied();
    let rework_id = ids.get(KEY_REWORK).copied();
    let throughput_id = ids.get(KEY_THROUGHPUT).copied();
    let ttur_id = ids.get(KEY_TTUR).copied();
    let context_id = ids.get(KEY_CONTEXT_PRESSURE).copied();
    let tokens_day_id = ids.get(KEY_TOKENS_PER_DAY).copied();
    let tokens_in_id = ids.get(KEY_TOKENS_IN).copied();
    let tokens_out_id = ids.get(KEY_TOKENS_OUT).copied();
    let duration_id = ids.get(KEY_SESSION_DURATION).copied();
    let tokens_result_id = ids.get(KEY_TOKENS_PER_RESULT).copied();
    let incomplete_id = ids.get(KEY_INCOMPLETE_ANALYSIS).copied();

    let mut written = 0u32;

    // Per-repository daily session-level metrics: ftr (pct) + throughput (count).
    if ftr_id.is_some() || throughput_id.is_some() {
        for (day, repository_id, session_count, ftr_count, correction_count) in
            daily_session_aggregates(pg, &project_id, window_days, as_of).await?
        {
            if let Some(mid) = ftr_id {
                // denominator (session_count) is ≥ 1 for any returned (day, repo).
                let value = ftr_count as f64 / session_count as f64;
                let props = serde_json::json!({
                    "numerator": ftr_count,
                    "denominator": session_count,
                    "correction_count": correction_count,
                });
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
            if let Some(mid) = throughput_id {
                // count-type: value IS the count; no numerator/denominator needed.
                let props = serde_json::json!({});
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, session_count as f64, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    // Per-repository daily rework_ratio (ratio) — only if active.
    if let Some(mid) = rework_id {
        for (day, repository_id, corrected_tool_calls, total_tool_calls) in
            daily_rework(pg, &project_id, window_days, as_of).await?
        {
            if total_tool_calls == 0 {
                // No tool-call data that day → no denominator → NO row (a 0/0 would
                // be a fabricated zero, not a measured one).
                continue;
            }
            let value = corrected_tool_calls as f64 / total_tool_calls as f64;
            let props = serde_json::json!({
                "numerator": corrected_tool_calls,
                "denominator": total_tool_calls,
            });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-repository daily time_to_useful_result (duration, median seconds) — only
    // if active. A (day, repo) with no session that produced a usable turn writes
    // NO row (honest-empty).
    if let Some(mid) = ttur_id {
        for (day, repository_id, median_secs, n) in
            daily_time_to_useful(pg, &project_id, window_days, as_of).await?
        {
            let props = serde_json::json!({ "n": n });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, median_secs, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-repository daily context_pressure_rate (pct) — only if active. A
    // (day, repo) with a real denominator writes a row (even a 0); one with no
    // measurable session is skipped (honest-empty, never a fabricated 0/0).
    if let Some(mid) = context_id {
        for (day, repository_id, pressured, total) in
            daily_context_pressure(pg, &project_id, window_days, as_of).await?
        {
            if total == 0 {
                continue;
            }
            let value = pressured as f64 / total as f64;
            let props = serde_json::json!({ "numerator": pressured, "denominator": total });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-repository daily token volume: tokens_per_day (in+out), tokens_in_per_day,
    // tokens_out_per_day — count-type, so value IS the daily sum (the pooling view
    // sums across repositories). Base = sessions carrying token usage.
    if tokens_day_id.is_some() || tokens_in_id.is_some() || tokens_out_id.is_some() {
        for (day, repository_id, sum_in, sum_out, n) in
            daily_token_volume(pg, &project_id, window_days, as_of).await?
        {
            let props = serde_json::json!({ "sessions": n, "tokens_in": sum_in, "tokens_out": sum_out });
            if let Some(mid) = tokens_day_id {
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, (sum_in + sum_out) as f64, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
            if let Some(mid) = tokens_in_id {
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, sum_in as f64, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
            if let Some(mid) = tokens_out_id {
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, sum_out as f64, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    // Per-repository daily session_duration (duration, mean active seconds) — the
    // pooling view averages across repositories. Base = sessions with a duration.
    if let Some(mid) = duration_id {
        for (day, repository_id, avg_secs, n) in
            daily_session_duration(pg, &project_id, window_days, as_of).await?
        {
            let props = serde_json::json!({ "n": n });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, avg_secs, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-repository daily tokens_per_result (ratio: Σ output tokens / completed
    // sessions) — pools Σnum/Σden. A (day, repo) with no completed token-bearing
    // session writes NO row (honest-empty, never a fabricated 0/0).
    if let Some(mid) = tokens_result_id {
        for (day, repository_id, sum_out, completed) in
            daily_tokens_per_result(pg, &project_id, window_days, as_of).await?
        {
            if completed == 0 {
                continue;
            }
            let value = sum_out as f64 / completed as f64;
            let props = serde_json::json!({ "numerator": sum_out, "denominator": completed });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // Per-repository daily incomplete_analysis_rate (pct: edit-before-read sessions
    // / sessions-with-edits) — a (day, repo) with no edit-bearing session has no
    // denominator → NO row (honest-empty, never a fabricated 0/0).
    if let Some(mid) = incomplete_id {
        for (day, repository_id, flagged, measurable) in
            daily_incomplete_analysis(pg, &project_id, window_days, as_of).await?
        {
            if measurable == 0 {
                continue;
            }
            let value = flagged as f64 / measurable as f64;
            let props = serde_json::json!({ "numerator": flagged, "denominator": measurable });
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        repository_for_folder, seed_assistant_event_tool, seed_metrics_client_session,
        seed_metrics_project_folder, seed_metrics_session, seed_metrics_turn, seed_metrics_turn_ex,
        seed_second_repository,
    };
    use sqlx_core::query_as::query_as;

    #[tokio::test]
    async fn session_outcomes_writes_ftr_rework_throughput() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // Sessions on ONE day (fixed instant so inserts can't straddle midnight): 3
        // first-try (completed, 0 corrections, 2 tool-calls each) + 1 corrected
        // (ftr=false, 2 corrections, 6 tool-calls). Σ measurable tool-calls = 12.
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..3 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 2, ts).await;
        }
        let corrected = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 2, ts).await;
        seed_metrics_turn(pg, &corrected, 6, ts).await;
        // An in-flight session (outcome NULL, ftr NULL) on the SAME day, WITH
        // tool-calls — it is not yet measurable and MUST be excluded from every
        // session_outcomes metric (the `outcome is not null` FTR base). If the
        // filter regressed, session_count→5, ftr→3/5, throughput→5, rework→6/17.
        let inflight = seed_metrics_session(pg, &fid, &pid, None, None, 0, ts).await;
        seed_metrics_turn(pg, &inflight, 5, ts).await;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 5, "5 repo-grain daily rows (ftr, rework, throughput, time_to_useful, context_pressure); per-session grain retired");

        // ── Repo-grain proof: the daily rows are scope=user, keyed on the session's
        //    repository, folder_id/session_id NULL, grain=daily ──────────────
        let rid = repository_for_folder(pg, &fid).await;
        let (row_repo, row_scope, row_grain, row_folder, row_session): (
            Option<uuid::Uuid>, String, String, Option<uuid::Uuid>, Option<uuid::Uuid>,
        ) = query_as(
            "SELECT pm.repository_id, pm.scope::text, pm.grain::text, pm.folder_id, pm.session_id \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'ftr'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(row_repo, Some(rid), "the ftr daily row is keyed on the session's repository");
        assert_eq!(row_scope, "user", "the local-user default-project value is scope=user");
        assert_eq!(row_grain, "daily", "the row is daily grain (per-session grain retired)");
        assert_eq!(row_folder, None, "no folder_id under the repo-grain identity (I-A)");
        assert_eq!(row_session, None, "no session_id under the repo-grain identity (I-A)");

        // ── Daily rows ────────────────────────────────────────────────────
        let daily = daily_rows(pg, &pid).await;

        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present");
        assert!((ftr.1 - 0.75).abs() < 1e-9, "ftr value = 3/4 = 0.75 (avg(case when ftr) over the measurable base)");
        assert_eq!(ftr.2["numerator"].as_i64(), Some(3), "ftr numerator = # first-try sessions (in-flight excluded)");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(4), "ftr denominator = session_count (in-flight excluded)");
        assert_eq!(ftr.2["correction_count"].as_i64(), Some(2), "correction_count = Σ corrections (display)");

        let rework = daily.iter().find(|r| r.0 == "rework_ratio").expect("rework_ratio daily row present");
        assert!((rework.1 - 0.5).abs() < 1e-9, "rework value = 6/12 = 0.5 (in-flight's 5 tool-calls excluded)");
        assert_eq!(rework.2["numerator"].as_i64(), Some(6), "rework numerator = corrected-session tool-calls");
        assert_eq!(rework.2["denominator"].as_i64(), Some(12), "rework denominator = all measurable tool-calls");

        let throughput = daily.iter().find(|r| r.0 == "throughput").expect("throughput daily row present");
        assert!((throughput.1 - 4.0).abs() < 1e-9, "throughput value = 4 measurable sessions (in-flight excluded)");

        // ── FTR parity (Phase-8 consolidation check) ──────────────────────
        // Asserted against the ARITHMETIC the retired `sensei.ftr_daily` view
        // computed — `avg(case when s.ftr then 1 else 0)` and `count(*)` over the
        // measurable base — read straight from the base tables, NOT the view
        // object, so this survives the view being dropped in Phase 8.5.
        let (fd_rate, fd_count): (f64, i64) = query_as(
            "SELECT avg(CASE WHEN s.ftr THEN 1.0 ELSE 0.0 END)::float8, count(*)::int8 \
               FROM activity.sessions s \
              WHERE s.project_id = $1 AND s.outcome IS NOT NULL",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!((fd_rate - ftr.1).abs() < 1e-9, "computed daily ftr value == avg(case when ftr) over the measurable base (old ftr_daily arithmetic)");
        assert_eq!(
            Some(fd_count), ftr.2["denominator"].as_i64(),
            "ftr denominator == count(*) over the measurable base (old ftr_daily.session_count)",
        );

        // ── Per-session grain is RETIRED — no grain='session' rows exist ──
        let (session_grain_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1 AND grain = 'session'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(session_grain_rows, 0, "no per-session grain rows (retired under the repo-grain identity)");

        // ── Idempotency: re-run backfills in place, never duplicates ──────
        let again = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(again, 5, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 5, "idempotent upsert — still 5 rows after a second run");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_writes_one_row_per_repository() {
        // Repo grain: two checkouts (two distinct repositories) in ONE project each
        // get their OWN daily rows keyed on repository_id — a project-day is never
        // merged into a single row here (the pooling to a project value is the
        // view's job, over these per-repository rows).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let (fid2, rid2) = seed_second_repository(pg, &pid, &uniq).await;
        let rid1 = repository_for_folder(pg, &fid).await;
        assert_ne!(rid1, rid2, "the two checkouts resolve to distinct repositories");

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // repo 1: 2 first-try + 1 corrected → ftr 2/3 over 3 sessions.
        for _ in 0..2 {
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        }
        seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
        // repo 2: 1 first-try + 1 corrected → ftr 1/2 over 2 sessions.
        seed_metrics_session(pg, &fid2, &pid, Some("completed"), Some(true), 0, ts).await;
        seed_metrics_session(pg, &fid2, &pid, Some("corrected"), Some(false), 1, ts).await;

        compute(&ctx, &pid.to_string(), None).await.unwrap();

        // Two ftr rows, one per repository, each carrying ITS OWN value + denominator.
        let rows: Vec<(Option<uuid::Uuid>, f64, i64)> = query_as(
            "SELECT pm.repository_id, pm.value::float8, (pm.props->>'denominator')::int8 \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'ftr' AND pm.grain = 'daily' AND pm.scope = 'user' \
              ORDER BY pm.repository_id",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 2, "one ftr row per repository — repo grain never merges the two");
        let r1 = rows.iter().find(|r| r.0 == Some(rid1)).expect("repo 1 ftr row present");
        let r2 = rows.iter().find(|r| r.0 == Some(rid2)).expect("repo 2 ftr row present");
        assert!((r1.1 - 2.0 / 3.0).abs() < 1e-9, "repo 1 ftr = 2/3 (independent of repo 2)");
        assert_eq!(r1.2, 3, "repo 1 denominator = its own 3 sessions");
        assert!((r2.1 - 0.5).abs() < 1e-9, "repo 2 ftr = 1/2 (independent of repo 1)");
        assert_eq!(r2.2, 2, "repo 2 denominator = its own 2 sessions");

        // Clean up the second checkout folder (the fixed-signature cleanup only
        // removes `fid`); its sessions detach via ON DELETE SET NULL, and the
        // fixture's repositories rows are cleared by cleanup_metrics_fixture.
        sqlx_core::query::query("DELETE FROM sensei.folders WHERE id = $1")
            .bind(fid2)
            .execute(pg.pool())
            .await
            .unwrap();
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    /// Phase 8.4 parity proof, written as formula-equivalence so it survives the
    /// `sensei.ftr_daily` / `sensei.project_ftr_metrics` drop: the store-derived
    /// FTR (from `project_metrics`, read back through the getters) equals the
    /// ARITHMETIC the retired views computed, expressed directly over the seeded
    /// sessions — NOT queried from any view object. Single repository, so the
    /// pooling view is a pass-through of the one per-repository row.
    #[tokio::test]
    async fn ftr_parity_store_vs_views() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // Seed a measurable set on ONE day (all `outcome is not null`, so the old
        // per-session-average view formula and the store's Σnum/Σden coincide):
        // 3 first-try + 1 corrected. Direct arithmetic over these seeds:
        //   ftr_daily      : avg(case when ftr) = (1+1+1+0)/4 = 0.75; session_count = 4
        //   project_ftr    : Σnumerator/Σdenominator = 3/4 = 0.75; sessions_7d = 4
        // Anchor to the START of TODAY's UTC day — NOT `now() - 2h`, which crosses the
        // UTC day boundary in the ~2h after midnight and lands the session on YESTERDAY
        // (then the `today` row assertion below misses). Start-of-day is always on
        // today's UTC date and ≤ now, so the seed is deterministically "today".
        let ts = chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        for _ in 0..3 {
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        }
        seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert!(written > 0, "compute wrote store rows for the seeded sessions");

        // Store-derived daily FTR (get_ftr_daily reads project_metric_daily) ==
        // the old ftr_daily arithmetic over the seed.
        let daily = pg.get_ftr_daily(Some(&pid), 14).await.unwrap();
        let today = (chrono::Utc::now()).date_naive().to_string();
        let row = daily.iter().find(|r| r["day"].as_str() == Some(today.as_str()))
            .expect("today's daily ftr row present");
        assert!((row["ftr_rate"].as_f64().unwrap() - 0.75).abs() < 1e-9,
            "store daily ftr_rate == avg(case when ftr) over seeds = 0.75");
        assert_eq!(row["session_count"].as_i64(), Some(4),
            "store session_count == count(*) over the measurable base = 4");

        // Store-derived 14d headline (get_project_ftr reads project_metric_daily)
        // == the old project_ftr_metrics formula (Σnum/Σden over 14d) over the seed.
        let ftr = pg.get_project_ftr(&pid).await.unwrap();
        assert!((ftr["ftr14d"].as_f64().unwrap() - 0.75).abs() < 1e-9,
            "store ftr14d == Σnumerator/Σdenominator over 14d = 3/4 = 0.75");
        assert_eq!(ftr["sessions7d"].as_i64(), Some(4),
            "store sessions7d == Σdenominator over 7d = 4");

        // And the shared rate helper the legacy surfaces call agrees.
        let rate = pg.get_project_ftr_rate(&pid).await.unwrap();
        assert!((rate.expect("rate present for a project with ftr rows") - 0.75).abs() < 1e-9,
            "get_project_ftr_rate == the same 14d headline number");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    /// FIX 2: `get_project_ftr`'s inline trend must use the same analyzed base as
    /// the headline (`outcome is not null`), so the sparkline's last point agrees
    /// with `ftr14d` even when the day has in-flight sessions. Before the fix the
    /// unfiltered trend scored the in-flight session as 0 and dragged the point
    /// below the headline (last=0.5 vs ftr14d=2/3). Mutation guard: dropping the
    /// `outcome is not null` filter from the trend query fails the last assert.
    #[tokio::test]
    async fn trend_last_point_matches_headline_with_inflight_sessions() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // 2 first-try + 1 corrected (measurable) + 1 in-flight (outcome NULL) today.
        for _ in 0..2 {
            seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        }
        seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
        seed_metrics_session(pg, &fid, &pid, None, None, 0, ts).await; // in-flight, excluded

        compute(&ctx, &pid.to_string(), None).await.unwrap();

        let ftr = pg.get_project_ftr(&pid).await.unwrap();
        let headline = ftr["ftr14d"].as_f64().expect("ftr14d present");
        assert!((headline - 2.0 / 3.0).abs() < 1e-9,
            "headline = 2/3 over the 3 measurable sessions (in-flight excluded)");
        let last = ftr["ftrTrend"].as_array().and_then(|a| a.last()).and_then(|v| v.as_f64())
            .expect("trend has a last point");
        assert!((last - headline).abs() < 1e-9,
            "trend's last point agrees with the headline (same `outcome is not null` base)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn empty_sessions_excluded_from_ftr_and_throughput() {
        // Phase A (transcript-ground-truth): an `empty` session (0 turns, nothing
        // attempted) is NOT measurable — it must not count toward ftr or throughput.
        // Seed 1 completed(ftr) + 1 corrected + 1 EMPTY(ftr=true) on one day: ftr must
        // be 1/2 (not 2/3) and throughput 2 (not 3). Mutation guard: if the
        // `outcome <> 'empty'` filter regresses, the empty ftr=true session drags
        // ftr→2/3 and throughput→3.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        let a = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        seed_metrics_turn(pg, &a, 2, ts).await;
        let b = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
        seed_metrics_turn(pg, &b, 2, ts).await;
        let _empty = seed_metrics_session(pg, &fid, &pid, Some("empty"), Some(true), 0, ts).await;

        compute(&ctx, &pid.to_string(), None).await.unwrap();
        let daily = daily_rows(pg, &pid).await;
        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present");
        assert!((ftr.1 - 0.5).abs() < 1e-9, "ftr = 1/2 — the empty session is excluded (not 2/3)");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(2), "ftr denominator excludes the empty session");
        let throughput = daily.iter().find(|r| r.0 == "throughput").expect("throughput daily row present");
        assert!((throughput.1 - 2.0).abs() < 1e-9, "throughput = 2 measurable sessions (empty excluded)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_no_data_writes_zero_rows() {
        // Never-fabricate: a project with zero sessions in the window writes NO rows
        // (not a defaulted 0). Honest-empty is the correct output for no data.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:so-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no sessions in the window → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_session_without_resolvable_repository_is_excluded() {
        // Never-fabricate (I-E): a measurable session whose repo anchor can't be
        // resolved to a repository (repo_folder_id NULL) is EXCLUDED — it must not be
        // invented into a made-up repository, and it must not silently poison the
        // resolvable repository's counts. Seed one resolvable first-try session and
        // one unresolvable corrected session; ftr must be 1/1 over the resolvable
        // repository, not 1/2.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        // An unresolvable session: outcome measurable, but repo_folder_id NULL.
        let (orphan,): (uuid::Uuid,) = query_as(
            "INSERT INTO activity.sessions (project_id, outcome, ftr, corrections, started_at) \
             VALUES ($1, 'corrected'::sensei.session_outcome, false, 1, $2) RETURNING id",
        )
        .bind(pid)
        .bind(ts)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        let _ = orphan;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();

        let rows: Vec<(f64, i64)> = query_as(
            "SELECT pm.value::float8, (pm.props->>'denominator')::int8 \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'ftr' AND pm.grain = 'daily' AND pm.scope = 'user'",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();
        assert_eq!(rows.len(), 1, "exactly one repository's ftr row — the orphan session is not fabricated into a repository");
        assert!((rows[0].0 - 1.0).abs() < 1e-9, "ftr = 1/1 over the resolvable repository (orphan excluded, not 1/2)");
        assert_eq!(rows[0].1, 1, "denominator counts only the resolvable session");
        assert!(written > 0, "the resolvable repository still wrote its rows");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_zero_tool_calls_writes_no_rework_row() {
        // A day with measurable sessions whose turns carry ZERO tool-calls has a real
        // rework denominator of 0 → NO rework_ratio row (a 0/0 would be a fabricated
        // zero). ftr + throughput still compute (they don't depend on tool-calls).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // 2 completed sessions, each with a turn but ZERO tool-calls → the day appears
        // in the rework aggregate with total_tool_calls = 0 (exercises the skip path).
        for _ in 0..2 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
            seed_metrics_turn(pg, &sid, 0, ts).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();

        let (rework_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'rework_ratio'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rework_rows, 0, "no rework_ratio row when total tool-calls is 0 (never a fabricated 0/0)");
        assert_eq!(written, 4, "ftr + throughput + time_to_useful + context_pressure repo-grain daily; rework skipped");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_all_corrected_writes_zero_ftr() {
        // A day where EVERY session is corrected → ftr numerator 0 but a REAL
        // denominator → value 0.0 is WRITTEN (a real zero, never skipped).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..2 {
            let sid = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, ts).await;
            seed_metrics_turn(pg, &sid, 3, ts).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 5, "ftr (0.0) + rework + throughput + time_to_useful + context_pressure repo-grain daily");

        let daily = daily_rows(pg, &pid).await;
        let ftr = daily.iter().find(|r| r.0 == "ftr").expect("ftr daily row present (a real zero is still written)");
        assert!(ftr.1.abs() < 1e-9, "ftr value is a real 0.0 (0 numerator over a real denominator)");
        assert_eq!(ftr.2["numerator"].as_i64(), Some(0), "ftr numerator = 0 (no first-try sessions)");
        assert_eq!(ftr.2["denominator"].as_i64(), Some(2), "ftr denominator = 2 (real denominator → row written, not skipped)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn time_to_useful_result_is_median_of_first_non_correction_turn_latency() {
        // Definition (B): latency = session.started_at → ended_at of the FIRST turn
        // that is not a correction. Median over the day's measurable sessions.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        let t = chrono::Utc::now() - chrono::Duration::hours(2);
        let at = |n: i64| t + chrono::Duration::seconds(n);

        // A: only turn is useful at +10s → 10s.
        let a = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &a, 1, t, at(10), false, 1).await;
        // B: turn 1 is a correction (+3s), turn 2 is the first useful one (+30s) → 30s.
        let b = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &b, 1, t, at(3), true, 1).await;
        seed_metrics_turn_ex(pg, &b, 2, at(3), at(30), false, 1).await;
        // C: session-outcome 'corrected' but its first TURN is useful at +20s → 20s
        // (session-level correction != turn-level correction).
        let c = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, t).await;
        seed_metrics_turn_ex(pg, &c, 1, t, at(20), false, 1).await;
        // D: ONLY a correction turn → produced no usable output → excluded.
        let d = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, t).await;
        seed_metrics_turn_ex(pg, &d, 1, t, at(5), true, 1).await;
        // In-flight (outcome NULL) with a useful turn → not measurable → excluded.
        let inflight = seed_metrics_session(pg, &fid, &pid, None, None, 0, t).await;
        seed_metrics_turn_ex(pg, &inflight, 1, t, at(1), false, 1).await;

        compute(&ctx, &pid.to_string(), None).await.unwrap();

        let daily = daily_rows(pg, &pid).await;
        let ttur = daily.iter().find(|r| r.0 == "time_to_useful_result")
            .expect("time_to_useful_result daily row present");
        // median([10, 20, 30]) = 20; D (correction-only) + in-flight excluded.
        assert!((ttur.1 - 20.0).abs() < 1e-6,
            "median first-useful latency = median(10,20,30) = 20s, got {}", ttur.1);
        assert_eq!(ttur.2["n"].as_i64(), Some(3),
            "n = 3 sessions with a usable turn (correction-only + in-flight excluded)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn time_to_useful_result_no_usable_turn_writes_no_row() {
        // Never-fabricate: if every measurable session's only turn is a correction,
        // no session produced a usable output → NO time_to_useful_result row (not a 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let t = chrono::Utc::now() - chrono::Duration::hours(2);
        for _ in 0..2 {
            let s = seed_metrics_session(pg, &fid, &pid, Some("corrected"), Some(false), 1, t).await;
            seed_metrics_turn_ex(pg, &s, 1, t, t + chrono::Duration::seconds(4), true, 1).await;
        }

        compute(&ctx, &pid.to_string(), None).await.unwrap();

        let (rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'time_to_useful_result'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rows, 0, "no usable turn in any session → no time_to_useful_result row (never a fabricated 0)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_writes_token_volume_duration_efficiency() {
        // Token volume (count → summed), session duration (duration → averaged), and
        // tokens_per_result (ratio → Σout/completed) over sessions carrying tokens.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // Two completed sessions: (in=100,out=20,60s) + (in=300,out=80,120s).
        let s1 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        let s2 = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        for (sid, tin, tout, secs) in [(s1, 100i32, 20i32, 60.0f64), (s2, 300, 80, 120.0)] {
            sqlx_core::query::query(
                "UPDATE activity.sessions SET tokens_in=$2, tokens_out=$3, duration=make_interval(secs => $4) WHERE id=$1",
            )
            .bind(sid).bind(tin).bind(tout).bind(secs)
            .execute(pg.pool()).await.unwrap();
        }

        compute(&ctx, &pid.to_string(), None).await.unwrap();
        let daily = daily_rows(pg, &pid).await;

        let tpd = daily.iter().find(|r| r.0 == "tokens_per_day").expect("tokens_per_day row");
        assert!((tpd.1 - 500.0).abs() < 1e-9, "tokens_per_day = (100+20)+(300+80) = 500 (summed across sessions)");
        let tin = daily.iter().find(|r| r.0 == "tokens_in_per_day").expect("tokens_in_per_day row");
        assert!((tin.1 - 400.0).abs() < 1e-9, "tokens_in_per_day = 100+300");
        let tout = daily.iter().find(|r| r.0 == "tokens_out_per_day").expect("tokens_out_per_day row");
        assert!((tout.1 - 100.0).abs() < 1e-9, "tokens_out_per_day = 20+80");
        let dur = daily.iter().find(|r| r.0 == "session_duration").expect("session_duration row");
        assert!((dur.1 - 90.0).abs() < 1e-6, "session_duration = avg(60,120) = 90s");
        let tpr = daily.iter().find(|r| r.0 == "tokens_per_result").expect("tokens_per_result row");
        assert!((tpr.1 - 50.0).abs() < 1e-9, "tokens_per_result = 100 out / 2 completed = 50");
        assert_eq!(tpr.2["numerator"].as_i64(), Some(100), "tpr numerator = Σ output tokens over completed");
        assert_eq!(tpr.2["denominator"].as_i64(), Some(2), "tpr denominator = # completed token-bearing sessions");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn incomplete_analysis_flags_edit_before_read() {
        // Edit-before-read: a session that edits before it reads (or edits with no
        // read) is flagged; one that reads before editing is not; one with no edit is
        // not even measurable. Rate = flagged / sessions-with-edits.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let t = chrono::Utc::now() - chrono::Duration::hours(2);
        let at = |n: i64| t + chrono::Duration::seconds(n);

        // Seed a completed client-session (needs client_session_id to join events +
        // outcome to be measurable) and return its id string.
        async fn completed(pg: &PgStore, fid: &uuid::Uuid, pid: &uuid::Uuid, csid: &str, t: chrono::DateTime<chrono::Utc>) {
            let sid = seed_metrics_client_session(pg, fid, pid, csid, t).await;
            sqlx_core::query::query("UPDATE activity.sessions SET outcome='completed'::sensei.session_outcome, ftr=true WHERE id=$1")
                .bind(sid).execute(pg.pool()).await.unwrap();
        }
        let a = format!("cs-a-{uniq}"); // read @0 then edit @10 → NOT flagged
        let b = format!("cs-b-{uniq}"); // edit @0 then read @10 → flagged
        let c = format!("cs-c-{uniq}"); // edit only → flagged
        let d = format!("cs-d-{uniq}"); // read only → NOT measurable (no edit)
        for cs in [&a, &b, &c, &d] {
            completed(pg, &fid, &pid, cs, t).await;
        }
        seed_assistant_event_tool(pg, &a, "Read", at(0)).await;
        seed_assistant_event_tool(pg, &a, "Edit", at(10)).await;
        seed_assistant_event_tool(pg, &b, "Edit", at(0)).await;
        seed_assistant_event_tool(pg, &b, "Read", at(10)).await;
        seed_assistant_event_tool(pg, &c, "Edit", at(0)).await;
        seed_assistant_event_tool(pg, &d, "Read", at(0)).await;

        compute(&ctx, &pid.to_string(), None).await.unwrap();
        let daily = daily_rows(pg, &pid).await;
        let ia = daily.iter().find(|r| r.0 == "incomplete_analysis_rate").expect("incomplete_analysis_rate row");
        assert!((ia.1 - 2.0 / 3.0).abs() < 1e-9, "flagged(b,c)=2 / measurable(a,b,c)=3 = 2/3; d (no edit) excluded");
        assert_eq!(ia.2["numerator"].as_i64(), Some(2), "flagged = edit-before-read sessions (b + c)");
        assert_eq!(ia.2["denominator"].as_i64(), Some(3), "measurable = sessions with ≥1 edit (a,b,c); d excluded");

        // clean up the events (keyed on the unique client_session_ids)
        for cs in [&a, &b, &c, &d] {
            sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id=$1")
                .bind(cs).execute(pg.pool()).await.ok();
        }
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn session_outcomes_backfills_a_historical_day_when_as_of_is_set() {
        // Phase 1: `as_of = Some(D)` computes the SINGLE historical day D and stamps
        // `computed_on = D`, so a past day reaches the roll-up views (which bucket on
        // `computed_on` with no recent-window filter). The incremental (`None`) run
        // omits the day because it is outside the rolling window — that omission is
        // exactly what the backfill path fixes.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // One measurable session 60 days ago — well outside the default 14-day
        // window. `date_naive()` is its true occurrence day.
        let ts = chrono::Utc::now() - chrono::Duration::days(60);
        let day = ts.date_naive();
        let sid = seed_metrics_session(pg, &fid, &pid, Some("completed"), Some(true), 0, ts).await;
        seed_metrics_turn(pg, &sid, 2, ts).await;

        // Incremental run (as_of=None): the 60-day-old day is out of window → NO rows
        // (honest-empty for the recent window, never a fabricated backfill).
        let incr = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old day is outside the rolling window → no incremental rows");

        // Backfill run (as_of=Some(day)): computes exactly that day's metrics.
        let written = compute(&ctx, &pid.to_string(), Some(day)).await.unwrap();
        assert!(written > 0, "as_of=Some(D) computes the historical day D (window-only behavior would still write 0)");

        // The daily `ftr` row is stamped `computed_on = day` (the true occurrence
        // day, 60 days ago) at repo grain — proof the past-dated row reaches the
        // daily roll-up.
        let (ftr_days,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.scope = 'user' AND m.key = 'ftr' AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(day)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(ftr_days, 1, "a daily ftr row is stamped computed_on = the session's true day (60 days ago)");

        // Idempotent: re-backfilling the same day upserts in place (no duplicate row).
        let again = compute(&ctx, &pid.to_string(), Some(day)).await.unwrap();
        assert_eq!(again, written, "re-running the same day backfills in place");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
