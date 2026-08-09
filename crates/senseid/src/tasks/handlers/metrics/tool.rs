//! `tool` metric group computer (Phase 5.6).
//!
//! Follows the `session_outcomes` / `churn` / `duplication` / `autonomy` /
//! `knowledge` template (resolve `key → metric_id` via the active registry, write a
//! daily project-scope row to `sensei.project_metrics` via
//! [`PgStore::upsert_project_metric`]) for ONE project. The single row is `grain =
//! daily`, `folder_id` NULL (project scope), `computed_on = today` — a SNAPSHOT of
//! dead tool surface over the rolling window.
//!
//! v1 registry key (`task_name = "tool"`):
//! - `unused_tools` (count, lower_better): the number of REGISTERED tools with ZERO
//!   outcome-positive calls attributable to this project in the window — the "dead
//!   surface area" signal. Reported as a SINGLE aggregate count (never one row per
//!   tool). `count` type ⇒ the `value` IS the count; no numerator/denominator. The
//!   display prop `total_tools` carries the in-scope registry size for context.
//!
//! ## Project scoping of a GLOBAL registry (the design decision)
//! `sensei.assistant_tools` is the tool REGISTRY and is GLOBAL per
//! `assistant_family` — it has NO `project_id` (a Claude install's tool inventory is
//! the same regardless of which project is open). The compute task, however, runs
//! per `(project, task_name)`, so this computes tools unused BY THIS PROJECT:
//! - **Registered-in-scope** = registry rows whose `assistant_family` is one the
//!   project actually uses. A project's families = `DISTINCT activity.sessions.acp_id`
//!   for the project (the hook session-recorder stamps the harness family string —
//!   `record_session_event` — into `acp_id`, and it aligns with
//!   `assistant_tools.assistant_family`, e.g. `claude`). This keeps a project from
//!   being dinged for another harness's tools it never had, and keeps the metric
//!   per-project. If a family alignment ever drifts (`acp_id` vs `assistant_family`
//!   string), the scope narrows — it never fabricates or mis-attributes.
//! - **Positive outcome** = a `sensei.tool_call_verdicts` row with `verdict =
//!   'used'` (the classifier's "the assistant actually consumed the response";
//!   `partial`/`ignored` are NOT positive). Attributed to the project via
//!   `tool_call_verdicts.session_id = activity.sessions.client_session_id` →
//!   `sessions.project_id` (the same linkage `autonomy` uses for `assistant_events`).
//!   Joined to the registry on `assistant_tools.invoked_name =
//!   tool_call_verdicts.tool_name` (both the harness-qualified name).
//!
//! ## Windowing on CALL time, not classify time (a correctness requirement)
//! A verdict is in-window when its underlying CALL happened in the last
//! [`window_days`] — the `activity.assistant_events.created_at` reached via
//! `tool_call_verdicts.event_id`. It is DELIBERATELY NOT windowed on
//! `tool_call_verdicts.classified_at`: the classifier is idempotent and rebuildable,
//! and `upsert_verdicts_batch` sets `classified_at = now()` on EVERY re-run — so a
//! reclassification would sweep every old call into the window and make all tools
//! look freshly used. The call's `created_at` is the honest "when was this tool
//! used" timestamp (the same field `autonomy` and the tools-health grid window on).
//!
//! Never-fabricate: every DB call propagates `Err`. 0 registered-in-scope tools ⇒
//! NO row (nothing to measure — a 0 would be a fabricated "0 dead tools"). When
//! tools ARE registered and every one has an in-window positive call, `value = 0` is
//! a REAL written zero (0 dead tools). Strict project scoping on both the family set
//! and the verdicts — another project's usage never marks this project's tools used.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (tool writes a daily snapshot row only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — tool is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key` this computer produces.
const KEY_UNUSED_TOOLS: &str = "unused_tools";

/// The `tool_call_verdicts.verdict` that counts as an outcome-POSITIVE call — the
/// assistant actually consumed the tool's response. `partial`/`ignored` do not.
const VERDICT_USED: &str = "used";

/// `(total_tools, unused)` for a project over the window:
/// - `total_tools` = # registered tools in the project's family scope (the display
///   denominator; `0` ⇒ the caller writes NO row).
/// - `unused` = of those, how many have NO in-window `used` verdict attributable to
///   the project (the metric `value`).
async fn unused_tool_counts(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<(i64, i64), String> {
    let (total_tools, unused): (i64, i64) = sqlx_core::query_as::query_as(
        "WITH proj_fam AS (
             -- assistant families this project uses (aligns with assistant_family)
             SELECT DISTINCT s.acp_id AS family
               FROM activity.sessions s
              WHERE s.project_id = $1
                AND s.acp_id IS NOT NULL
         ),
         registered AS (
             -- registered tools in scope: those in a family the project uses
             SELECT at.id, at.invoked_name
               FROM sensei.assistant_tools at
               JOIN proj_fam pf ON pf.family = at.assistant_family
         ),
         used AS (
             -- invoked_names with an in-window 'used' verdict attributable to $1,
             -- windowed on the CALL time (assistant_events.created_at via event_id)
             SELECT DISTINCT v.tool_name AS invoked_name
               FROM sensei.tool_call_verdicts v
               JOIN activity.sessions        s  ON s.client_session_id = v.session_id
               JOIN activity.assistant_events ae ON ae.id = v.event_id
              WHERE s.project_id  = $1
                AND v.verdict     = $2
                AND ae.created_at >= now() - make_interval(days => $3::int)
         )
         SELECT count(*)::int8                                        AS total_tools
              , count(*) FILTER (WHERE u.invoked_name IS NULL)::int8  AS unused
           FROM registered r
           LEFT JOIN used u ON u.invoked_name = r.invoked_name",
    )
    .bind(project_id)
    .bind(VERDICT_USED)
    .bind(window_days as i32)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok((total_tools, unused))
}

/// Compute the `tool` group for one project as a snapshot as of today.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no registered tools
/// in the project's scope, or the metric is inactive). Idempotent — re-running
/// backfills in place via the upsert identity.
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("tool: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics. An absent key is
    // inactive (retired / not-yet-effective / unseeded) → skipped: the computer
    // never writes a value for an inactive metric.
    let ids = pg.active_metric_ids(MetricGroup::Tool.as_str()).await?;
    let Some(mid) = ids.get(KEY_UNUSED_TOOLS).copied() else {
        return Ok(0);
    };

    let (total_tools, unused) = unused_tool_counts(pg, &project_id, window_days).await?;

    if total_tools == 0 {
        // 0 registered tools in the project's scope → nothing to measure → NO row (a
        // written 0 would fabricate a "0 dead tools" reading where there is no
        // registry to measure against).
        return Ok(0);
    }

    // A real registry with every tool used gives `unused = 0` — a REAL written zero
    // (0 dead tools), never suppressed. `count` type: value IS the count, no
    // numerator/denominator; `total_tools` rides along as a display prop.
    let props = serde_json::json!({ "total_tools": total_tools });
    let day = super::today(pg).await?;
    pg.upsert_project_metric(
        &mid, &project_id, None, None, day, GRAIN_DAILY, unused as f64, &props, SOURCE_MEASURED,
    )
    .await?;

    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, make_ctx, purge_assistant_events, purge_assistant_tools,
        purge_tool_verdicts, seed_assistant_tool, seed_metrics_project_folder, seed_tool_session,
        seed_tool_verdict,
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
    async fn unused_tools_counts_tools_without_positive_verdicts() {
        // 4 registered tools pin verdict PRECISION — ONLY `verdict='used'` is a
        // positive outcome: t1 'used' (positive → used), t2 'ignored'-only (NOT
        // positive → unused), t3 'partial'-only (NOT positive → unused), t4 no verdict
        // (→ unused) → 3 unused. value = 3, total_tools = 4. The `partial`-only t3 is
        // load-bearing: broadening the filter to `verdict IN ('used','partial')` would
        // wrongly count t3 as used (value → 2) and this assertion goes red — a
        // regression the `ignored`-only case alone can't catch.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let fam = format!("_test-fam-{uniq}");
        let csid = format!("_test:tool:{uniq}");
        purge_assistant_tools(pg, &[&fam]).await; // idempotent pre-clean
        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2); // in-window call time
        seed_tool_session(pg, &fid, &pid, &csid, &fam, ts).await;
        for t in ["t1", "t2", "t3", "t4"] {
            seed_assistant_tool(pg, &fam, "builtin", "builtin", t, t).await;
        }
        seed_tool_verdict(pg, &csid, "t1", "used", ts).await; // positive → used
        seed_tool_verdict(pg, &csid, "t2", "ignored", ts).await; // NOT positive → unused
        seed_tool_verdict(pg, &csid, "t3", "partial", ts).await; // NOT positive → unused
        // t4: no verdict at all → unused.

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "one unused_tools project row");

        let daily = daily_rows(pg, &pid).await;
        let ut = daily.iter().find(|r| r.0 == "unused_tools").expect("unused_tools row present");
        assert!((ut.1 - 3.0).abs() < 1e-9, "value = 3 unused (t2 ignored-only, t3 partial-only, t4 no-verdict; only t1 'used' excluded)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(4), "total_tools = 4 registered in scope");

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(again, 1, "re-run recomputes the same row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 1, "idempotent upsert — still 1 row after a second run");

        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        purge_assistant_tools(pg, &[&fam]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_all_used_writes_real_zero() {
        // Every registered tool has an in-window 'used' verdict → 0 unused → value 0.0
        // is WRITTEN (a real zero: "0 dead tools"), never suppressed.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let fam = format!("_test-fam-{uniq}");
        let csid = format!("_test:tool:{uniq}");
        purge_assistant_tools(pg, &[&fam]).await;
        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_tool_session(pg, &fid, &pid, &csid, &fam, ts).await;
        for t in ["t1", "t2"] {
            seed_assistant_tool(pg, &fam, "builtin", "builtin", t, t).await;
            seed_tool_verdict(pg, &csid, t, "used", ts).await;
        }

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "a real-zero unused_tools row IS written (not suppressed)");

        let daily = daily_rows(pg, &pid).await;
        let ut = daily.iter().find(|r| r.0 == "unused_tools").expect("unused_tools row present");
        assert!(ut.1.abs() < 1e-9, "value is a real 0.0 (every registered tool used in-window)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(2), "total_tools = 2 registered in scope");

        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        purge_assistant_tools(pg, &[&fam]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_no_registered_tools_writes_no_row() {
        // 0 registered tools in scope → NO row (nothing to measure), EVEN THOUGH the
        // project is active (has a session in family `fam`) — no assistant_tools were
        // registered for `fam`, so the registry scope is empty. It's the empty
        // registry, not "no sessions", that yields zero here.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let fam = format!("_test-fam-{uniq}");
        let csid = format!("_test:tool:{uniq}");
        purge_assistant_tools(pg, &[&fam]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        seed_tool_session(pg, &fid, &pid, &csid, &fam, ts).await; // active project, but…
        // …no seed_assistant_tool for `fam` → 0 registered in scope.

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "0 registered tools in scope → NO row (never a fabricated 0)");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows at all when the in-scope registry is empty");

        purge_assistant_tools(pg, &[&fam]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no sessions (hence no families → no in-scope
        // registry) writes NO rows.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:tool-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "no sessions/tools in scope → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_positive_verdict_outside_window_counts_as_unused() {
        // Window boundary: a tool whose ONLY 'used' verdict fired OUTSIDE the window
        // counts as UNUSED. 1 registered tool t1 with a single 'used' verdict whose
        // call time is 30 days ago → value 1 (t1 dead in-window). A missing/broken
        // window filter would give value 0 (t1 seen as used).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let fam = format!("_test-fam-{uniq}");
        let csid = format!("_test:tool:{uniq}");
        purge_assistant_tools(pg, &[&fam]).await;
        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;

        let now = chrono::Utc::now();
        seed_tool_session(pg, &fid, &pid, &csid, &fam, now - chrono::Duration::hours(2)).await;
        seed_assistant_tool(pg, &fam, "builtin", "builtin", "t1", "t1").await;
        // The only positive verdict is 30 days old — outside the 14-day window.
        seed_tool_verdict(pg, &csid, "t1", "used", now - chrono::Duration::days(30)).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "one unused_tools row (t1 is registered in scope)");

        let daily = daily_rows(pg, &pid).await;
        let ut = daily.iter().find(|r| r.0 == "unused_tools").expect("unused_tools row present");
        assert!((ut.1 - 1.0).abs() < 1e-9, "value = 1 (t1's only 'used' verdict is out of window → unused)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(1), "total_tools = 1 registered in scope");

        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        purge_assistant_tools(pg, &[&fam]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_excludes_other_projects() {
        // Cross-project isolation (mutation-proof): project B uses a DIFFERENT family
        // with its own 3 registered tools, all 'used'. A's scope must see only A's 3
        // family-A tools (total_tools = 3, not 6), and B's 'used' verdicts must not
        // mark A's tools used (A unused = 2, not fewer).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_a = uuid::Uuid::new_v4();
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await; // a real SECOND project
        let fam_a = format!("_test-fam-a-{uniq_a}");
        let fam_b = format!("_test-fam-b-{uniq_b}");
        let csid_a = format!("_test:tool-a:{uniq_a}");
        let csid_b = format!("_test:tool-b:{uniq_b}");
        purge_assistant_tools(pg, &[&fam_a, &fam_b]).await;
        purge_tool_verdicts(pg, &[&csid_a, &csid_b]).await;
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;

        let ts = chrono::Utc::now() - chrono::Duration::hours(2);
        // Project A: 3 family-A tools; only a1 used → A unused = 2.
        seed_tool_session(pg, &fid_a, &pid_a, &csid_a, &fam_a, ts).await;
        for t in ["a1", "a2", "a3"] {
            seed_assistant_tool(pg, &fam_a, "builtin", "builtin", t, t).await;
        }
        seed_tool_verdict(pg, &csid_a, "a1", "used", ts).await;
        // Project B: 3 family-B tools, ALL used — must NOT touch A's counts.
        seed_tool_session(pg, &fid_b, &pid_b, &csid_b, &fam_b, ts).await;
        for t in ["b1", "b2", "b3"] {
            seed_assistant_tool(pg, &fam_b, "builtin", "builtin", t, t).await;
            seed_tool_verdict(pg, &csid_b, t, "used", ts).await;
        }

        let written = compute(&ctx, &pid_a.to_string()).await.unwrap();
        assert_eq!(written, 1, "only A's own family scope produces a row");

        let daily = daily_rows(pg, &pid_a).await;
        let ut = daily.iter().find(|r| r.0 == "unused_tools").expect("A's unused_tools row");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(3), "A's total_tools = 3 (family A only, not 6)");
        assert!((ut.1 - 2.0).abs() < 1e-9, "A unused = 2 (a2, a3; a1 used — B's usage excluded)");

        purge_tool_verdicts(pg, &[&csid_a, &csid_b]).await;
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;
        purge_assistant_tools(pg, &[&fam_a, &fam_b]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }
}
