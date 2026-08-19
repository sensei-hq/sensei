//! `autonomy` metric group computer (Phase 5.4).
//!
//! Follows the `session_outcomes` / `churn` / `knowledge` template (read the rolling
//! window, resolve `key → metric_id` via the active registry, write daily rows to
//! `sensei.project_metrics` via [`PgStore::upsert_project_metric_repo`]) for ONE
//! project. Every row is `grain = daily`, `scope = user`, `identity = NULL`,
//! `commit_sha = NULL`, `folder_id`/`session_id` NULL.
//!
//! ## Repo grain (I-A/I-B/I-C/I-D)
//! Under the repo-grain identity (`metric_id, repository_id, scope, identity,
//! commit_sha, computed_on, grain`) every production write carries a REAL
//! `repository_id`; `folder_id`/`session_id` are no longer part of the identity (a
//! `folder_id`-set or `grain = session` row would collide), and a null-repository row
//! would collide across projects. Both autonomy metrics therefore resolve a
//! repository and write `scope = user` ONLY (no whole-tree `repo` twin — autonomy is
//! not a churn/quality dual-derivation), `identity = NULL` (a single local user; NULL
//! keeps the row unique per `(metric, repo, user, day)`), and `commit_sha = NULL`
//! (day-bucketed cadence, not a commit-cadence measurement).
//!
//! v1 registry keys (all `task_name = "autonomy"`):
//! - `interruption_rate` (ratio, lower_better): `numerator` = # `Stop` events,
//!   `denominator` = # `UserPromptSubmit` events that day; `value` = num/den — the
//!   "how much the human keeps stepping in" babysitting signal. Attributed to the
//!   SESSION's repository (see below).
//! - `run_completion` (ratio, higher_better): `numerator` = # runs that reached
//!   `done`, `denominator` = # runs started that day; `value` = num/den. Attributed to
//!   the project's PRIMARY repository (see below).
//!
//! Every ratio row carries `props.low_n = true` when its `denominator < 10` (a
//! display gate for statistically-thin days) — the row is still written.
//!
//! ## `false_crash_rate` is DELIBERATELY NOT computed here (NEEDS_CONTEXT)
//! The catalog defines it as "runs killed at the recovery cap that were actually
//! just waiting ÷ non-done runs". The "killed at the recovery cap" half is cleanly
//! identifiable (`activity.runs.status = 'crashed'` reached via the watchdog's
//! bounded-recovery exhaustion, `recovery_attempts >= max_recovery`). The load-
//! bearing "that were actually just waiting" half is NOT: no column, `run_event`
//! detail, or status distinguishes a run that crashed because it was merely waiting
//! (a usage-limit reset, a blocking question) from one that genuinely died. The
//! watchdog crashes only `stalled` runs and never `paused`/`blocked` ones, and the
//! `Crashed` event detail carries only `{"note": "exhausted bounded recovery",
//! "attempts": N}` — no "was waiting" flag. `docs/analysis/2026-08-04-deep-dive/
//! 01-autonomy-babysitting-roadblocks.md` states these crashes are "indistinguish-
//! able from a real crash"; the classifying signal is a future P0 fix, not present
//! today. Rather than fabricate a proxy (e.g. treating every killed-at-cap run as a
//! false crash, which silently drops the "was waiting" qualifier), the metric is
//! SKIPPED — its registry key simply yields no rows until the signal exists. This
//! is honest-empty, not fabrication.
//!
//! ## Repository resolution (never leak another project's data, never fabricate one)
//! - `interruption_rate`: `activity.assistant_events` carries no project id — its
//!   `session_id` is the assistant's own session-id string. Events attribute to a
//!   session through `activity.sessions.client_session_id = assistant_events.
//!   session_id`, and to a REPOSITORY through that session's `repo_folder_id →
//!   sensei.folders.repository_id`. Counts are GROUP BY `(day, repository)` and each
//!   day/repository writes its own row. Events whose session matches no session, a
//!   session in another project (the `sessions.project_id = $1` scope), or a session
//!   whose repository cannot be resolved (`repo_folder_id` NULL, or that folder's
//!   `repository_id` NULL) are EXCLUDED — the row is skipped, never attributed to a
//!   fabricated repository (I-E).
//! - `run_completion`: `activity.runs` carries only a direct `project_id` FK and NO
//!   repository, so its per-day counts are project-wide. A project-wide value has no
//!   natural per-repository grain, so it is attributed to
//!   [`PgStore::primary_repository_for_project`] — the project's canonical
//!   (shallowest-checkout) repository. A project with NO repository-linked folder
//!   cannot be attributed to a repository, so `run_completion` writes NO row
//!   (honest-empty — never fabricate a repository), even when runs exist.
//!
//! Windowing/day-bucketing uses each source row's TRUE occurrence time — the
//! event's client clock `assistant_events.ts` (epoch ms →
//! `to_timestamp(ts / 1000.0)`) for `interruption_rate`, and `runs.started_at` for
//! `run_completion` — never an insert-time `created_at` (which is `now` for
//! synthesized/back-dated rows). `as_of=None` keeps the rolling `now() -
//! make_interval` window; `as_of=Some(D)` selects the single day `D` (backfill) via
//! the shared [`super::day_filter`] / [`super::bind_day`] `$2` contract.
//!
//! Never-fabricate: every DB call propagates `Err`; a metric/day with no data
//! writes NO row. A ratio with denominator 0 writes NO row (a 0/0 would be a
//! fabricated zero); a real denominator with 0 numerator writes a real `0.0`. A
//! repository that cannot be resolved skips the row (never a made-up repository).

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (autonomy writes daily rows only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — autonomy is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";
/// `sensei.metric_scope` text value — autonomy writes the local-user value only (the
/// default-project read); it has no whole-tree `repo` twin (that dual derivation is
/// churn/quality only).
const SCOPE_USER: &str = "user";

/// The registry `key`s this computer produces. `false_crash_rate` is intentionally
/// absent (NEEDS_CONTEXT — see the module doc).
const KEY_INTERRUPTION_RATE: &str = "interruption_rate";
const KEY_RUN_COMPLETION: &str = "run_completion";

/// The `assistant_events.event_type` literals `interruption_rate` counts. These are
/// the raw hook names (`hook_event_name`) as written by the capture path and used
/// across `analyze` / `verdict_classifier` / the transcript synthesizers.
const EVENT_STOP: &str = "Stop";
const EVENT_USER_PROMPT: &str = "UserPromptSubmit";

/// A ratio's denominator below which the day is statistically thin — flagged
/// `props.low_n = true` (the row is still written).
const LOW_N_THRESHOLD: i64 = 10;

/// One day's interruption counts for a repository: `(day, repository_id, stop_count,
/// prompt_count)`. `prompt_count` is the `interruption_rate` denominator; the row is
/// keyed to the SESSION's resolved repository.
type DayInterruption = (chrono::NaiveDate, uuid::Uuid, i64, i64);

/// One day's run counts for a project: `(day, done_count, started_count)`.
/// `started_count` (every run started that day) is the `run_completion` denominator.
type DayRunCompletion = (chrono::NaiveDate, i64, i64);

/// This group's occurrence-time anchors for the shared [`super::day_filter`] /
/// [`super::bind_day`] `$2` day-set contract. `interruption_rate` buckets/windows on
/// the event's CLIENT clock `ts` (epoch ms → `to_timestamp(ts / 1000.0)`, the same
/// `ae.ts / 1000.0` convention `get_project_sessions_needing_enrichment` uses), NOT
/// the server insert `created_at` — a synthesized/back-dated event carries its true
/// occurrence time in `ts` while `created_at` is `now`. `run_completion` buckets on
/// `runs.started_at`.
const ANCHOR_INTERRUPTION: &str = "to_timestamp(ae.ts / 1000.0)";
const ANCHOR_RUN_COMPLETION: &str = "r.started_at";

/// Daily `Stop` / `UserPromptSubmit` counts per REPOSITORY over the selected day-set
/// (rolling window when `as_of=None`, the single day `D` when `Some(D)`), attributed
/// to the project via `sessions.client_session_id = assistant_events.session_id` and
/// to a repository via `sessions.repo_folder_id → sensei.folders.repository_id`.
/// Events with no matching session, a session in another project, or a session whose
/// repository cannot resolve (`repo_folder_id` NULL / that folder's `repository_id`
/// NULL) are excluded. Bucketed by the event's CLIENT `ts` (its true occurrence day),
/// not the insert `created_at`, and GROUP BY `(day, repository)`.
async fn daily_interruption(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayInterruption>, String> {
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
        super::day_filter(ANCHOR_INTERRUPTION, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayInterruption>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .bind(EVENT_STOP)
        .bind(EVENT_USER_PROMPT)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Daily run-completion counts over the selected day-set (rolling window when
/// `as_of=None`, the single day `D` when `Some(D)`), project-scoped via the direct
/// `runs.project_id` FK. `done_count` is runs whose terminal `status = 'done'`;
/// `started_count` is every run started that day (the denominator). Bucketed by
/// `started_at` — a run counts on the day it started, regardless of when it finished.
/// Runs carry no repository, so the caller attributes these project-wide counts to
/// the project's primary repository.
async fn daily_run_completion(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> Result<Vec<DayRunCompletion>, String> {
    let sql = format!(
        "SELECT date_trunc('day', r.started_at)::date                          AS day
              , count(*) FILTER (WHERE r.status = 'done'::sensei.run_status)::int8 AS done_count
              , count(*)::int8                                                  AS started_count
           FROM activity.runs r
          WHERE r.project_id  = $1
            AND {}
          GROUP BY 1
          ORDER BY 1",
        super::day_filter(ANCHOR_RUN_COMPLETION, as_of),
    );
    let q = sqlx_core::query_as::query_as::<_, DayRunCompletion>(&sql).bind(project_id);
    super::bind_day(q, window_days, as_of)
        .fetch_all(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Build the ratio props for a row: exact `numerator` + `denominator` and the
/// `low_n` display flag (`denominator < 10`). `value` (computed by the caller) is
/// always `numerator / denominator`.
fn ratio_props(numerator: i64, denominator: i64) -> serde_json::Value {
    serde_json::json!({
        "numerator": numerator,
        "denominator": denominator,
        "low_n": denominator < LOW_N_THRESHOLD,
    })
}

/// Compute the `autonomy` group for one project.
///
/// `project_raw` is the project uuid carried in `task.folder_path`. `as_of` selects
/// the day-set (same contract as `session_outcomes`):
/// - `None` — the incremental run: every day in the rolling
///   [`metrics.window_days`] window (default 14), `computed_on` = the source day.
/// - `Some(D)` — the backfill/gap-fill run: ONLY the single day `D`, `computed_on` =
///   `D`. This is how a past day reaches the roll-up views (which bucket on
///   `computed_on` with no recent-window filter).
///
/// Both metrics bucket on their TRUE occurrence time — `interruption_rate` on the
/// event's client `ts`, `run_completion` on `runs.started_at` — never an insert-time
/// `created_at`, so a synthesized/back-dated row files on its historical day.
///
/// Repo grain (I-A/I-C/I-D): every row carries a real `repository_id`
/// (`interruption_rate` the session's repository, `run_completion` the project's
/// primary repository), `scope = user`, `identity = NULL`, `commit_sha = NULL`, and
/// no `folder_id`/`session_id`.
///
/// Returns the number of `project_metrics` rows written (`0` = honest-empty: no
/// events/runs on the selected day-set, no resolvable repository, or none of the
/// group's computed metrics active). Idempotent — re-running backfills in place via
/// the upsert identity. `false_crash_rate` is never written (see the module doc).
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("autonomy: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics. A key absent from the
    // map is inactive (retired / not-yet-effective / unseeded) → skipped.
    let ids = pg.active_metric_ids(MetricGroup::Autonomy.as_str()).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let interruption_id = ids.get(KEY_INTERRUPTION_RATE).copied();
    let run_completion_id = ids.get(KEY_RUN_COMPLETION).copied();
    // false_crash_rate: intentionally NOT resolved/computed (NEEDS_CONTEXT).

    let mut written = 0u32;

    // ── interruption_rate: # Stop / # UserPromptSubmit, per (day, session-repository) ──
    if let Some(mid) = interruption_id {
        for (day, repository_id, stop_count, prompt_count) in
            daily_interruption(pg, &project_id, window_days, as_of).await?
        {
            if prompt_count == 0 {
                // No UserPromptSubmit that day → no denominator → NO row (a 0/0, e.g.
                // a Stop-only day, would be a fabricated zero, not a measured ratio).
                continue;
            }
            let value = stop_count as f64 / prompt_count as f64;
            let props = ratio_props(stop_count, prompt_count);
            // repository_id = the session's resolved repo (I-A); scope=user (I-B),
            // identity=NULL (single local user, I-C), commit_sha=NULL (day cadence,
            // I-D), folder_id/session_id=NULL (not in the identity).
            pg.upsert_project_metric_repo(
                &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None, None, None,
                day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // ── run_completion: # runs done / # runs started, per day ──
    if let Some(mid) = run_completion_id {
        // Runs carry only project_id (no repository) → attribute the project-wide
        // per-day counts to the project's PRIMARY (canonical/root) repository. A
        // project with no repository-linked folder cannot be attributed → NO row
        // (honest-empty; never fabricate a repository), even when runs exist.
        if let Some(repository_id) = pg.primary_repository_for_project(&project_id).await? {
            for (day, done_count, started_count) in
                daily_run_completion(pg, &project_id, window_days, as_of).await?
            {
                if started_count == 0 {
                    // Defensive: GROUP BY only returns days with ≥1 run, so this never
                    // fires — but a real denominator of 0 must never write a fabricated 0/0.
                    continue;
                }
                let value = done_count as f64 / started_count as f64;
                let props = ratio_props(done_count, started_count);
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None, None, None,
                    day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::PgStore;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        purge_assistant_events, purge_runs, repository_for_folder, seed_assistant_event,
        seed_assistant_event_ex, seed_metrics_client_session, seed_metrics_project_folder,
        seed_run,
    };
    use sqlx_core::query_as::query_as;

    /// Anchor a client session to its repository checkout for the repo-grain
    /// resolution: `interruption_rate` groups by the session's `repo_folder_id →
    /// sensei.folders.repository_id`, so a seeded session must point `repo_folder_id`
    /// at the fixture's repository-linked folder (the fixture sets that folder's
    /// `repository_id`). [`seed_metrics_client_session`] sets only `folder_id`, so the
    /// tests set `repo_folder_id` here.
    async fn anchor_session_repo(
        pg: &PgStore,
        client_session_id: &str,
        repo_folder_id: &uuid::Uuid,
    ) {
        sqlx_core::query::query(
            "UPDATE activity.sessions SET repo_folder_id = $1 WHERE client_session_id = $2",
        )
        .bind(repo_folder_id)
        .bind(client_session_id)
        .execute(pg.pool())
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn autonomy_metrics_from_events_and_runs() {
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:autonomy:{uniq}");
        // Idempotent pre-clean: neither table cascades on the fixture delete.
        purge_assistant_events(pg, &[&csid]).await;
        purge_runs(pg, &[&pid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2); // fixed day
        seed_metrics_client_session(pg, &fid, &pid, &csid, ts).await;
        anchor_session_repo(pg, &csid, &fid).await; // repo-grain: attach the session's repository
        // 24 Stop / 25 UserPromptSubmit on one day → interruption_rate = 24/25 = 0.96,
        // low_n = false (25 >= 10).
        for _ in 0..24 {
            seed_assistant_event(pg, &csid, "Stop", ts).await;
        }
        for _ in 0..25 {
            seed_assistant_event(pg, &csid, "UserPromptSubmit", ts).await;
        }
        // 9 runs, 5 done (4 crashed) → run_completion = 5/9 ≈ 0.556, low_n = true (9 < 10).
        for _ in 0..5 {
            seed_run(pg, &pid, "done", ts).await;
        }
        for _ in 0..4 {
            seed_run(pg, &pid, "crashed", ts).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 2, "interruption_rate daily + run_completion daily (false_crash_rate skipped)");

        let daily = daily_rows(pg, &pid).await;

        let ir = daily.iter().find(|r| r.0 == "interruption_rate").expect("interruption_rate row present");
        assert!((ir.1 - 0.96).abs() < 1e-9, "interruption_rate value = 24/25 = 0.96");
        assert_eq!(ir.2["numerator"].as_i64(), Some(24), "interruption numerator = # Stop events");
        assert_eq!(ir.2["denominator"].as_i64(), Some(25), "interruption denominator = # UserPromptSubmit events");
        assert_eq!(ir.2["low_n"].as_bool(), Some(false), "denominator 25 >= 10 → not low_n");

        let rc = daily.iter().find(|r| r.0 == "run_completion").expect("run_completion row present");
        assert!((rc.1 - 5.0 / 9.0).abs() < 1e-9, "run_completion value = 5/9");
        assert_eq!(rc.2["numerator"].as_i64(), Some(5), "run_completion numerator = # runs reaching done");
        assert_eq!(rc.2["denominator"].as_i64(), Some(9), "run_completion denominator = # runs started");
        assert_eq!(rc.2["low_n"].as_bool(), Some(true), "denominator 9 < 10 → low_n");

        // Repo grain (I-A/I-C/I-D): BOTH rows are keyed to the fixture's repository
        // (interruption_rate via the session's repo_folder_id, run_completion via the
        // project's primary repository — the same repo for this single-folder project),
        // scope=user, identity/commit_sha/folder_id/session_id all NULL, grain daily.
        let repo = repository_for_folder(pg, &fid).await;
        let (repo_grain_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics \
              WHERE project_id = $1 AND repository_id = $2 AND scope = 'user'::sensei.metric_scope \
                AND identity IS NULL AND commit_sha IS NULL AND folder_id IS NULL AND session_id IS NULL \
                AND grain = 'daily'::sensei.metric_grain",
        )
        .bind(pid)
        .bind(repo)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(
            repo_grain_rows, 2,
            "both autonomy rows are repo-grain: repository set, scope=user, identity/commit_sha/folder/session NULL, grain daily",
        );

        // false_crash_rate is NEEDS_CONTEXT — no row is ever written for it.
        assert!(
            !daily.iter().any(|r| r.0 == "false_crash_rate"),
            "false_crash_rate is not computed (no clean 'was actually waiting' signal — never fabricated)",
        );

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(again, 2, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 2, "idempotent upsert — still 2 rows after a second run");

        purge_runs(pg, &[&pid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no events / runs writes NO rows.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:autonomy-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no events/runs in the window → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn autonomy_run_completion_no_repository_writes_no_row() {
        // Repo-grain honest-empty (I-A/I-E): a project with REAL runs but NO
        // repository-linked folder cannot attribute the project-wide run_completion
        // counts to any repository, so the row is SKIPPED — never a fabricated
        // repository. (interruption_rate needs a session-repository too and likewise
        // writes nothing here.)
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:autonomy-norepo:{uniq}"), None, None)
            .await
            .unwrap();
        purge_runs(pg, &[&pid]).await;

        // Runs exist, but the project has no repository-linked folder.
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_run(pg, &pid, "done", ts).await;
        seed_run(pg, &pid, "crashed", ts).await;
        assert!(
            pg.primary_repository_for_project(&pid).await.unwrap().is_none(),
            "fixture precondition: project has no repository to attribute to",
        );

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "runs present but no primary repository → run_completion cannot be attributed → honest-empty skip");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows when the project has no repository (never a fabricated repository)");

        purge_runs(pg, &[&pid]).await;
        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn autonomy_real_zero_interruption_is_written_low_n() {
        // A day with UserPromptSubmit events but ZERO Stop events → interruption_rate
        // numerator 0 over a REAL denominator → value 0.0 is WRITTEN (a real zero,
        // never skipped), and low_n = true (denominator 3 < 10).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:autonomy-zero:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_metrics_client_session(pg, &fid, &pid, &csid, ts).await;
        anchor_session_repo(pg, &csid, &fid).await;
        for _ in 0..3 {
            seed_assistant_event(pg, &csid, "UserPromptSubmit", ts).await;
        }
        // No Stop events and no runs.

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "interruption_rate real-zero row; run_completion has no runs → no row");

        let daily = daily_rows(pg, &pid).await;
        let ir = daily.iter().find(|r| r.0 == "interruption_rate").expect("real-zero interruption_rate row IS written");
        assert!(ir.1.abs() < 1e-9, "value is a real 0.0 (0 Stop over 3 UserPromptSubmit)");
        assert_eq!(ir.2["numerator"].as_i64(), Some(0), "numerator = 0 (no Stop events)");
        assert_eq!(ir.2["denominator"].as_i64(), Some(3), "denominator = 3 (real denominator → row written)");
        assert_eq!(ir.2["low_n"].as_bool(), Some(true), "denominator 3 < 10 → low_n");

        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_stop_only_day_writes_no_interruption_row() {
        // A day with Stop events but ZERO UserPromptSubmit → denominator 0 → NO row
        // (a 0/0 would be a fabricated zero, not a measured ratio).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:autonomy-stoponly:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_metrics_client_session(pg, &fid, &pid, &csid, ts).await;
        anchor_session_repo(pg, &csid, &fid).await;
        for _ in 0..3 {
            seed_assistant_event(pg, &csid, "Stop", ts).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "denominator 0 (no UserPromptSubmit) → no interruption_rate row (never a fabricated 0/0)");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows at all for a Stop-only day");

        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_low_n_threshold_boundary() {
        // Pin the low_n threshold at EXACTLY denominator == 10: a denominator of 10 is
        // NOT low_n (the rule is `denominator < 10`), while 9 IS. The 10-vs-9 pair
        // kills a `< 10` → `<= 10` mutation in `ratio_props` (which would flip the
        // den-10 row to low_n) that every other test's off-boundary denominator (25,
        // 9, 5, 3, 2) misses. Driven through interruption_rate (denominator = #
        // UserPromptSubmit), one exact-denominator project each.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let ts = chrono::Utc::now() - chrono::Duration::hours(2);

        // ── denominator == 10 → low_n = false (at the threshold, not below it) ──
        let uniq_10 = uuid::Uuid::new_v4();
        let (pid_10, fid_10) = seed_metrics_project_folder(pg, &uniq_10).await;
        let csid_10 = format!("_test:autonomy-lown10:{uniq_10}");
        purge_assistant_events(pg, &[&csid_10]).await;
        seed_metrics_client_session(pg, &fid_10, &pid_10, &csid_10, ts).await;
        anchor_session_repo(pg, &csid_10, &fid_10).await;
        for _ in 0..2 {
            seed_assistant_event(pg, &csid_10, "Stop", ts).await;
        }
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_10, "UserPromptSubmit", ts).await;
        }

        compute(&ctx, &pid_10.to_string(), None).await.unwrap();
        let daily_10 = daily_rows(pg, &pid_10).await;
        let ir_10 = daily_10.iter().find(|r| r.0 == "interruption_rate").expect("interruption_rate row (den 10)");
        assert_eq!(ir_10.2["denominator"].as_i64(), Some(10), "denominator is exactly 10");
        assert_eq!(ir_10.2["low_n"].as_bool(), Some(false), "denominator == 10 is NOT low_n (rule is `< 10`)");

        // ── denominator == 9 → low_n = true (one below the threshold) ──
        let uniq_9 = uuid::Uuid::new_v4();
        let (pid_9, fid_9) = seed_metrics_project_folder(pg, &uniq_9).await;
        let csid_9 = format!("_test:autonomy-lown9:{uniq_9}");
        purge_assistant_events(pg, &[&csid_9]).await;
        seed_metrics_client_session(pg, &fid_9, &pid_9, &csid_9, ts).await;
        anchor_session_repo(pg, &csid_9, &fid_9).await;
        for _ in 0..2 {
            seed_assistant_event(pg, &csid_9, "Stop", ts).await;
        }
        for _ in 0..9 {
            seed_assistant_event(pg, &csid_9, "UserPromptSubmit", ts).await;
        }

        compute(&ctx, &pid_9.to_string(), None).await.unwrap();
        let daily_9 = daily_rows(pg, &pid_9).await;
        let ir_9 = daily_9.iter().find(|r| r.0 == "interruption_rate").expect("interruption_rate row (den 9)");
        assert_eq!(ir_9.2["denominator"].as_i64(), Some(9), "denominator is exactly 9");
        assert_eq!(ir_9.2["low_n"].as_bool(), Some(true), "denominator == 9 IS low_n (one below the threshold)");

        purge_assistant_events(pg, &[&csid_10, &csid_9]).await;
        cleanup_metrics_fixture(pg, &pid_10, Some(&fid_10), &[]).await;
        cleanup_metrics_fixture(pg, &pid_9, Some(&fid_9), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_excludes_other_projects_events_and_runs() {
        // Cross-project isolation: a second real project's events + runs must NOT
        // leak into the project under test. Mutation-proof denominators: if B leaked,
        // interruption denominator would be 5+10=15 and run_completion 2+5=7.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_a = uuid::Uuid::new_v4();
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await; // a real SECOND project
        let csid_a = format!("_test:autonomy-a:{uniq_a}");
        let csid_b = format!("_test:autonomy-b:{uniq_b}");
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;
        purge_runs(pg, &[&pid_a, &pid_b]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // Project A: 4 Stop / 5 UserPromptSubmit; 2 runs, 1 done.
        seed_metrics_client_session(pg, &fid_a, &pid_a, &csid_a, ts).await;
        anchor_session_repo(pg, &csid_a, &fid_a).await;
        for _ in 0..4 {
            seed_assistant_event(pg, &csid_a, "Stop", ts).await;
        }
        for _ in 0..5 {
            seed_assistant_event(pg, &csid_a, "UserPromptSubmit", ts).await;
        }
        seed_run(pg, &pid_a, "done", ts).await;
        seed_run(pg, &pid_a, "crashed", ts).await;
        // Project B: 10 Stop / 10 UserPromptSubmit; 5 runs, all done — must NOT touch A.
        seed_metrics_client_session(pg, &fid_b, &pid_b, &csid_b, ts).await;
        anchor_session_repo(pg, &csid_b, &fid_b).await;
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_b, "Stop", ts).await;
        }
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_b, "UserPromptSubmit", ts).await;
        }
        for _ in 0..5 {
            seed_run(pg, &pid_b, "done", ts).await;
        }

        let written = compute(&ctx, &pid_a.to_string(), None).await.unwrap();
        assert_eq!(written, 2, "only A's own events/runs produce rows (interruption + run_completion)");

        let daily = daily_rows(pg, &pid_a).await;
        let ir = daily.iter().find(|r| r.0 == "interruption_rate").expect("A's interruption_rate row");
        assert_eq!(ir.2["numerator"].as_i64(), Some(4), "A's Stop count only (B's 10 excluded)");
        assert_eq!(ir.2["denominator"].as_i64(), Some(5), "A's UserPromptSubmit only (2 → 15 if B leaked)");
        assert!((ir.1 - 4.0 / 5.0).abs() < 1e-9, "A interruption_rate = 4/5 = 0.8");
        let rc = daily.iter().find(|r| r.0 == "run_completion").expect("A's run_completion row");
        assert_eq!(rc.2["numerator"].as_i64(), Some(1), "A's done runs only (B's 5 excluded)");
        assert_eq!(rc.2["denominator"].as_i64(), Some(2), "A's started runs only (2 → 7 if B leaked)");
        assert!((rc.1 - 0.5).abs() < 1e-9, "A run_completion = 1/2 = 0.5");

        // A's rows are attributed to A's OWN repository (not B's) — repo-grain
        // attribution stays project-local.
        let repo_a = repository_for_folder(pg, &fid_a).await;
        let (a_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1 AND repository_id = $2",
        )
        .bind(pid_a)
        .bind(repo_a)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(a_rows, 2, "both of A's rows keyed to A's own repository (never B's)");

        purge_runs(pg, &[&pid_a, &pid_b]).await;
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_backfills_a_historical_day_when_as_of_is_set() {
        // Phase 2: `as_of = Some(D)` computes the SINGLE historical day D and stamps
        // `computed_on = D` for BOTH autonomy metrics, so a past day reaches the
        // roll-up views (which bucket on `computed_on` with no recent-window filter).
        // The incremental (`None`) run omits the day because it is outside the rolling
        // window — that omission is exactly what the backfill path fixes.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:autonomy-asof:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;
        purge_runs(pg, &[&pid]).await;

        // Events + runs 60 days ago — well outside the default 14-day window.
        let ts = chrono::Utc::now() - chrono::Duration::days(60);
        let day = ts.date_naive();
        seed_metrics_client_session(pg, &fid, &pid, &csid, ts).await;
        anchor_session_repo(pg, &csid, &fid).await;
        for _ in 0..2 {
            seed_assistant_event(pg, &csid, "Stop", ts).await;
        }
        for _ in 0..5 {
            seed_assistant_event(pg, &csid, "UserPromptSubmit", ts).await;
        }
        seed_run(pg, &pid, "done", ts).await;
        seed_run(pg, &pid, "crashed", ts).await;

        // Incremental run (as_of=None): the 60-day-old day is out of window → NO rows.
        let incr = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old day is outside the rolling window → no incremental rows");

        // Backfill run (as_of=Some(day)): computes exactly that day's metrics.
        let written = compute(&ctx, &pid.to_string(), Some(day)).await.unwrap();
        assert_eq!(written, 2, "as_of=Some(D) computes the historical day (interruption_rate + run_completion)");

        // Both daily rows are stamped `computed_on = day` (the true occurrence day,
        // 60 days ago) — proof the past-dated rows reach the daily roll-up.
        let (days,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.computed_on = $2 \
                AND m.key IN ('interruption_rate', 'run_completion')",
        )
        .bind(pid)
        .bind(day)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(days, 2, "interruption_rate + run_completion daily rows stamped computed_on = the historical day");

        let daily = daily_rows(pg, &pid).await;
        let ir = daily.iter().find(|r| r.0 == "interruption_rate").expect("interruption_rate row present");
        assert!((ir.1 - 2.0 / 5.0).abs() < 1e-9, "interruption_rate value = 2/5 = 0.4");
        let rc = daily.iter().find(|r| r.0 == "run_completion").expect("run_completion row present");
        assert!((rc.1 - 0.5).abs() < 1e-9, "run_completion value = 1/2 = 0.5");

        // Idempotent: re-backfilling the same day upserts in place (no duplicate rows).
        let again = compute(&ctx, &pid.to_string(), Some(day)).await.unwrap();
        assert_eq!(again, 2, "re-running the same day backfills in place");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 2, "idempotent upsert — still 2 rows after a second backfill of the same day");

        purge_runs(pg, &[&pid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn autonomy_interruption_rate_anchors_on_client_ts_not_created_at() {
        // Anchor-fix mutation guard: `interruption_rate` must bucket on the event's
        // CLIENT `ts` (true occurrence time), NOT the server `created_at` (insert
        // time = now for synthesized/back-dated rows). Seed events whose `ts` is 45
        // days ago but whose `created_at` is NOW, then backfill that historical day.
        //
        // Old `created_at` path: `as_of=Some(historical_day)` filtering/bucketing on
        // `created_at` (= now) matches NOTHING on the historical day → 0 rows (and the
        // pre-fix as_of-ignored window path would file the row on TODAY instead). Only
        // a `ts`-anchored computer files the row on its true historical day.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let csid = format!("_test:autonomy-tsanchor:{uniq}");
        purge_assistant_events(pg, &[&csid]).await;

        let now = chrono::Utc::now();
        let occurred = now - chrono::Duration::days(45); // true occurrence time (client ts)
        let historical_day = occurred.date_naive();
        assert_ne!(historical_day, now.date_naive(), "the occurrence day differs from today");

        seed_metrics_client_session(pg, &fid, &pid, &csid, occurred).await;
        anchor_session_repo(pg, &csid, &fid).await;
        // ts = 45 days ago, created_at = now → a synthesized/back-dated event.
        for _ in 0..2 {
            seed_assistant_event_ex(pg, &csid, "Stop", occurred, now).await;
        }
        for _ in 0..5 {
            seed_assistant_event_ex(pg, &csid, "UserPromptSubmit", occurred, now).await;
        }

        let written = compute(&ctx, &pid.to_string(), Some(historical_day)).await.unwrap();
        assert_eq!(written, 1, "the back-dated events land on their historical day (interruption_rate only; no runs)");

        // The daily row is stamped `computed_on = the client-ts day`, NOT today.
        let (hist_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'interruption_rate' AND pm.grain = 'daily' \
                AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(historical_day)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(hist_rows, 1, "interruption_rate row filed on the client-ts day (created_at path would misfile/miss it)");

        // Nothing filed on today (the created_at day) — the anchor is the ts, period.
        let (today_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'interruption_rate' AND pm.grain = 'daily' \
                AND pm.computed_on = current_date",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(today_rows, 0, "no row on today's date — created_at (insert time) is NOT the anchor");

        let daily = daily_rows(pg, &pid).await;
        let ir = daily.iter().find(|r| r.0 == "interruption_rate").expect("interruption_rate row present");
        assert!((ir.1 - 2.0 / 5.0).abs() < 1e-9, "interruption_rate value = 2/5 = 0.4");

        purge_assistant_events(pg, &[&csid]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
