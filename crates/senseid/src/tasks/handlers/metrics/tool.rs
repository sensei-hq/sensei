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
//! - `unused_tools` (count, lower_better): the number of RELEVANT tools with ZERO
//!   outcome-positive calls attributable to this project in the window — the "dead
//!   surface area" signal, scoped to tools this repo has actually engaged. Reported
//!   as a SINGLE aggregate count (never one row per tool). `count` type ⇒ the `value`
//!   IS the count; no numerator/denominator. Display props carry the framing:
//!   `relevant_tools` (the honest denominator M), `used_tools` (N, the in-window
//!   positive-outcome count → the UI reads "N of M relevant tools used"), and
//!   `total_tools` (T, the full in-scope registry size, for context on how much of
//!   the family inventory is even relevant to this repo).
//!
//! ## Relevance (why the denominator is NOT the whole registry)
//! A family's tool inventory is GLOBAL (e.g. a Claude install exposes ~100 tools);
//! most are irrelevant to any one repo. Counting all of them as "dead surface"
//! punishes a project for tools it never had reason to touch. So relevance is
//! evidence-based: a registered-in-scope tool is RELEVANT to this project iff the
//! project has actually INVOKED it — i.e. there is a `tool_call_verdicts` row (ANY
//! verdict, all-time) attributable to the project for that tool. Relevance is a
//! STANDING property (all-time invocation), usage is a ROLLING-window positive
//! outcome. A registered tool the repo has never invoked is neither relevant nor
//! counted — it drops out of both M and the `value`, never fabricated as "dead".
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

/// `(total_tools, relevant_tools, used_tools)` for a project over the window:
/// - `total_tools` (T) = # registered tools in the project's family scope (context
///   only; the full inventory, most of which may be irrelevant to this repo).
/// - `relevant_tools` (M) = of those, how many the project has ever INVOKED (a
///   `tool_call_verdicts` row of any verdict, all-time) — the honest denominator.
///   `0` ⇒ no relevance evidence ⇒ the caller writes NO row.
/// - `used_tools` (N) = of the M relevant, how many have an in-window `used` verdict
///   attributable to the project. The metric `value` (dead relevant surface) is
///   `M - N`; `N ≤ M` always (a `used` verdict is itself an invocation).
async fn tool_usage_counts(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<(i64, i64, i64), String> {
    let (total_tools, relevant_tools, used_tools): (i64, i64, i64) = sqlx_core::query_as::query_as(
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
         invoked AS (
             -- RELEVANCE evidence: tools this project has ever invoked (any verdict,
             -- ALL-TIME — relevance is a standing property, not a rolling window).
             SELECT DISTINCT v.tool_name AS invoked_name
               FROM sensei.tool_call_verdicts v
               JOIN activity.sessions s ON s.client_session_id = v.session_id
              WHERE s.project_id = $1
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
         ),
         relevant AS (
             -- registered ∩ invoked: the honest denominator (M). Registered tools the
             -- repo never invoked are excluded — never counted as dead surface.
             SELECT r.invoked_name
               FROM registered r
               JOIN invoked i ON i.invoked_name = r.invoked_name
         )
         SELECT (SELECT count(*) FROM registered)::int8 AS total_tools
              , (SELECT count(*) FROM relevant)::int8   AS relevant_tools
              , (SELECT count(*)
                   FROM relevant rel
                   JOIN used u ON u.invoked_name = rel.invoked_name)::int8 AS used_tools",
    )
    .bind(project_id)
    .bind(VERDICT_USED)
    .bind(window_days as i32)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok((total_tools, relevant_tools, used_tools))
}

/// Compute the `tool` group for one project as a snapshot as of today.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no RELEVANT tools —
/// the project has invoked none of its registered-in-scope tools — or the metric is
/// inactive). Idempotent — re-running backfills in place via the upsert identity.
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

    let (total_tools, relevant_tools, used_tools) =
        tool_usage_counts(pg, &project_id, window_days).await?;

    if relevant_tools == 0 {
        // No relevant tools — the project has never invoked any registered-in-scope
        // tool, so there is no honest denominator → NO row. A written value here
        // would fabricate a "0 dead tools" reading where there is no relevance
        // evidence to measure against (covers the empty-registry case too, since an
        // empty registry can have no invocations).
        return Ok(0);
    }

    // `value` = dead RELEVANT surface = M - N. Every relevant tool used in-window
    // gives `unused = 0` — a REAL written zero (0 dead relevant tools), never
    // suppressed. `count` type: value IS the count, no numerator/denominator; the
    // framing (`relevant_tools` M, `used_tools` N, `total_tools` T) rides along as
    // display props → the UI shows "N of M relevant tools used".
    let unused = relevant_tools - used_tools;
    let props = serde_json::json!({
        "total_tools": total_tools,
        "relevant_tools": relevant_tools,
        "used_tools": used_tools,
    });
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
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        purge_assistant_events, purge_assistant_tools, purge_tool_verdicts, seed_assistant_tool,
        seed_metrics_project_folder, seed_tool_session, seed_tool_verdict,
    };
    use sqlx_core::query_as::query_as;

    #[tokio::test]
    async fn unused_tools_counts_tools_without_positive_verdicts() {
        // 4 registered tools pin verdict PRECISION *and* relevance. ONLY
        // `verdict='used'` is a positive outcome: t1 'used' (invoked+used), t2
        // 'ignored'-only (invoked → relevant, NOT used → dead), t3 'partial'-only
        // (invoked → relevant, NOT used → dead), t4 NO verdict (never invoked → NOT
        // relevant, drops out of the denominator entirely). So total_tools=4 (registry
        // scope), relevant_tools=3 (t1,t2,t3 invoked), used_tools=1 (t1), value = M-N =
        // 2 dead relevant tools. The `partial`-only t3 is load-bearing: broadening the
        // used filter to `verdict IN ('used','partial')` would count t3 as used
        // (value → 1) and this goes red. t4 is load-bearing for relevance: counting
        // never-invoked tools as dead would push value → 3 and relevant → 4.
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
        seed_tool_verdict(pg, &csid, "t1", "used", ts).await; // invoked + used → not dead
        seed_tool_verdict(pg, &csid, "t2", "ignored", ts).await; // invoked, NOT used → dead
        seed_tool_verdict(pg, &csid, "t3", "partial", ts).await; // invoked, NOT used → dead
        // t4: no verdict at all → never invoked → NOT relevant (drops out of M).

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "one unused_tools project row");

        let daily = daily_rows(pg, &pid).await;
        let ut = daily.iter().find(|r| r.0 == "unused_tools").expect("unused_tools row present");
        assert!((ut.1 - 2.0).abs() < 1e-9, "value = M-N = 2 dead relevant tools (t2 ignored, t3 partial; t1 used, t4 irrelevant)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(4), "total_tools = 4 registered in scope");
        assert_eq!(ut.2["relevant_tools"].as_i64(), Some(3), "relevant_tools = 3 (t1,t2,t3 invoked; t4 never invoked → excluded)");
        assert_eq!(ut.2["used_tools"].as_i64(), Some(1), "used_tools = 1 (only t1 has an in-window 'used' verdict)");

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
        // Every relevant tool (both invoked + used in-window) → 0 dead → value 0.0 is
        // WRITTEN (a real zero: "0 dead relevant tools"), never suppressed. M=N=2.
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
        assert!(ut.1.abs() < 1e-9, "value is a real 0.0 (every relevant tool used in-window)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(2), "total_tools = 2 registered in scope");
        assert_eq!(ut.2["relevant_tools"].as_i64(), Some(2), "relevant_tools = 2 (both invoked)");
        assert_eq!(ut.2["used_tools"].as_i64(), Some(2), "used_tools = 2 (both used in-window)");

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
        // Window boundary + relevance interplay: a tool whose ONLY 'used' verdict fired
        // OUTSIDE the usage window is still RELEVANT (relevance is all-time invocation,
        // and a 'used' verdict is an invocation) but NOT used in-window → dead relevant
        // surface. 1 registered t1, single 'used' verdict 30 days ago → relevant M=1,
        // used N=0, value = 1 (t1 dead in-window). A missing/broken usage-window filter
        // would give N=1 → value 0 (t1 wrongly seen as used).
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
        assert!((ut.1 - 1.0).abs() < 1e-9, "value = 1 (t1 relevant but its only 'used' verdict is out of window)");
        assert_eq!(ut.2["total_tools"].as_i64(), Some(1), "total_tools = 1 registered in scope");
        assert_eq!(ut.2["relevant_tools"].as_i64(), Some(1), "relevant_tools = 1 (t1 invoked all-time → relevant)");
        assert_eq!(ut.2["used_tools"].as_i64(), Some(0), "used_tools = 0 (no in-window 'used' verdict)");

        purge_tool_verdicts(pg, &[&csid]).await;
        purge_assistant_events(pg, &[&csid]).await;
        purge_assistant_tools(pg, &[&fam]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn unused_tools_excludes_other_projects() {
        // Cross-project isolation (mutation-proof) + relevance: project B uses a
        // DIFFERENT family with its own 3 registered tools, all 'used'. A registers 3
        // family-A tools: a1 'used' (invoked+used), a2 'ignored' (invoked → relevant,
        // not used), a3 no verdict (never invoked → irrelevant). A's scope must see
        // total_tools = 3 (family A only, not 6), relevant = 2 (a1,a2; a3 excluded),
        // used = 1 (a1 only — B's 'used' verdicts must not mark A's tools), value = 1.
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
        // Project A: 3 family-A tools; a1 used, a2 invoked-not-used, a3 never invoked.
        seed_tool_session(pg, &fid_a, &pid_a, &csid_a, &fam_a, ts).await;
        for t in ["a1", "a2", "a3"] {
            seed_assistant_tool(pg, &fam_a, "builtin", "builtin", t, t).await;
        }
        seed_tool_verdict(pg, &csid_a, "a1", "used", ts).await; // invoked + used
        seed_tool_verdict(pg, &csid_a, "a2", "ignored", ts).await; // invoked, not used → relevant
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
        assert_eq!(ut.2["relevant_tools"].as_i64(), Some(2), "A relevant = 2 (a1,a2 invoked; a3 never invoked → excluded)");
        assert_eq!(ut.2["used_tools"].as_i64(), Some(1), "A used = 1 (a1; B's usage never marks A's tools)");
        assert!((ut.1 - 1.0).abs() < 1e-9, "A unused = M-N = 1 (a2 invoked-not-used; a3 irrelevant, B excluded)");

        purge_tool_verdicts(pg, &[&csid_a, &csid_b]).await;
        purge_assistant_events(pg, &[&csid_a, &csid_b]).await;
        purge_assistant_tools(pg, &[&fam_a, &fam_b]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }
}
