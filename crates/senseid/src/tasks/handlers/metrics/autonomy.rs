//! `autonomy` metric group computer (Phase 5.4).
//!
//! Follows the `session_outcomes` / `churn` template (read the rolling window,
//! resolve `key → metric_id` via the active registry, write daily project-scope
//! rows to `sensei.project_metrics` via [`PgStore::upsert_project_metric`]) for ONE
//! project. All rows are `grain = daily`, `folder_id` NULL (project scope).
//!
//! v1 registry keys (all `task_name = "autonomy"`):
//! - `interruption_rate` (ratio, lower_better): `numerator` = # `Stop` events,
//!   `denominator` = # `UserPromptSubmit` events that day; `value` = num/den — the
//!   "how much the human keeps stepping in" babysitting signal.
//! - `run_completion` (ratio, higher_better): `numerator` = # runs that reached
//!   `done`, `denominator` = # runs started that day; `value` = num/den.
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
//! ## Attribution (never leak another project's data)
//! - `interruption_rate`: `activity.assistant_events` carries no project id — its
//!   `session_id` is the assistant's own session-id string. Events attribute to a
//!   project through `activity.sessions.client_session_id = assistant_events.
//!   session_id` then `sessions.project_id` (the same linkage `get_project_prompts`
//!   uses). Events whose `session_id` matches no session, or a session in another
//!   project, are simply not counted for this project.
//! - `run_completion`: `activity.runs.project_id` is a direct FK — scope by it.
//!
//! Windowing/day-bucketing uses the server-side timestamp of each source row
//! (`assistant_events.created_at`, `runs.started_at`), matching the `now() -
//! make_interval` window the churn/session_outcomes computers use.
//!
//! Never-fabricate: every DB call propagates `Err`; a metric/day with no data
//! writes NO row. A ratio with denominator 0 writes NO row (a 0/0 would be a
//! fabricated zero); a real denominator with 0 numerator writes a real `0.0`.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (autonomy writes daily project rows only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — autonomy is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

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

/// One day's interruption counts for a project: `(day, stop_count, prompt_count)`.
/// `prompt_count` is the `interruption_rate` denominator.
type DayInterruption = (chrono::NaiveDate, i64, i64);

/// One day's run counts for a project: `(day, done_count, started_count)`.
/// `started_count` (every run started that day) is the `run_completion` denominator.
type DayRunCompletion = (chrono::NaiveDate, i64, i64);

/// Daily `Stop` / `UserPromptSubmit` counts over the window, attributed to the
/// project via `sessions.client_session_id = assistant_events.session_id` (events
/// with no matching session, or a session in another project, are excluded).
/// Bucketed by the server-side `created_at`.
async fn daily_interruption(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<DayInterruption>, String> {
    sqlx_core::query_as::query_as(
        "SELECT date_trunc('day', ae.created_at)::date                     AS day
              , count(*) FILTER (WHERE ae.event_type = $2)::int8           AS stop_count
              , count(*) FILTER (WHERE ae.event_type = $3)::int8           AS prompt_count
           FROM activity.assistant_events ae
           JOIN activity.sessions        s ON s.client_session_id = ae.session_id
          WHERE s.project_id   = $1
            AND ae.event_type IN ($2, $3)
            AND ae.created_at >= now() - make_interval(days => $4::int)
          GROUP BY 1
          ORDER BY 1",
    )
    .bind(project_id)
    .bind(EVENT_STOP)
    .bind(EVENT_USER_PROMPT)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// Daily run-completion counts over the window, project-scoped via the direct
/// `runs.project_id` FK. `done_count` is runs whose terminal `status = 'done'`;
/// `started_count` is every run started that day (the denominator). Bucketed by
/// `started_at` — a run counts on the day it started, regardless of when it finished.
async fn daily_run_completion(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<Vec<DayRunCompletion>, String> {
    sqlx_core::query_as::query_as(
        "SELECT date_trunc('day', r.started_at)::date                          AS day
              , count(*) FILTER (WHERE r.status = 'done'::sensei.run_status)::int8 AS done_count
              , count(*)::int8                                                  AS started_count
           FROM activity.runs r
          WHERE r.project_id  = $1
            AND r.started_at >= now() - make_interval(days => $2::int)
          GROUP BY 1
          ORDER BY 1",
    )
    .bind(project_id)
    .bind(window_days as i32)
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

/// Compute the `autonomy` group for one project over the configured window.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no events/runs, or
/// none of the group's computed metrics active). Idempotent — re-running backfills
/// in place via the upsert identity. `false_crash_rate` is never written (see the
/// module doc).
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
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

    // ── interruption_rate: # Stop / # UserPromptSubmit, per day ──
    if let Some(mid) = interruption_id {
        for (day, stop_count, prompt_count) in
            daily_interruption(pg, &project_id, window_days).await?
        {
            if prompt_count == 0 {
                // No UserPromptSubmit that day → no denominator → NO row (a 0/0, e.g.
                // a Stop-only day, would be a fabricated zero, not a measured ratio).
                continue;
            }
            let value = stop_count as f64 / prompt_count as f64;
            let props = ratio_props(stop_count, prompt_count);
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    // ── run_completion: # runs done / # runs started, per day ──
    if let Some(mid) = run_completion_id {
        for (day, done_count, started_count) in
            daily_run_completion(pg, &project_id, window_days).await?
        {
            if started_count == 0 {
                // Defensive: GROUP BY only returns days with ≥1 run, so this never
                // fires — but a real denominator of 0 must never write a fabricated 0/0.
                continue;
            }
            let value = done_count as f64 / started_count as f64;
            let props = ratio_props(done_count, started_count);
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
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
        cleanup_metrics_fixture, make_ctx, purge_assistant_events, purge_runs,
        seed_assistant_event, seed_metrics_client_session, seed_metrics_project_folder, seed_run,
    };
    use sqlx_core::query_as::query_as;

    /// The daily project-scope rows (`folder_id IS NULL`) keyed by metric `key`.
    async fn daily_rows(pg: &PgStore, pid: &uuid::Uuid) -> Vec<(String, f64, serde_json::Value)> {
        query_as(
            "SELECT m.key, pm.value::float8, pm.props \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.folder_id IS NULL \
              ORDER BY m.key",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap()
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

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
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

        // false_crash_rate is NEEDS_CONTEXT — no row is ever written for it.
        assert!(
            !daily.iter().any(|r| r.0 == "false_crash_rate"),
            "false_crash_rate is not computed (no clean 'was actually waiting' signal — never fabricated)",
        );

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
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

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
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
        for _ in 0..3 {
            seed_assistant_event(pg, &csid, "UserPromptSubmit", ts).await;
        }
        // No Stop events and no runs.

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
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
        for _ in 0..3 {
            seed_assistant_event(pg, &csid, "Stop", ts).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
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
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_b, "Stop", ts).await;
        }
        for _ in 0..10 {
            seed_assistant_event(pg, &csid_b, "UserPromptSubmit", ts).await;
        }
        for _ in 0..5 {
            seed_run(pg, &pid_b, "done", ts).await;
        }

        let written = compute(&ctx, &pid_a.to_string()).await.unwrap();
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

        purge_runs(pg, &[&pid_a, &pid_b]).await;
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }
}
