//! `cost` metric group — what a delivered result actually cost.
//!
//! The only metric here that money can honestly answer. Everything token-based
//! moved to the `usage` family, because under a flat subscription the marginal
//! cost of a token is zero: a token count read as money is ~8x high (≈98% of the
//! input total is cache reads, which bill far cheaper) and moves the WRONG WAY —
//! it rises as caching improves. See [`crate::cost`].
//!
//! ## Why a trailing window rather than a daily point
//!
//! Delivery is lumpy. On real data only 2 of the last 14 days carried any result
//! at all, so a per-day figure would be null ~86% of the time and violent on the
//! days it existed. A trailing [`COST_WINDOW_DAYS`] window also matches how the
//! fee is incurred: you are billed for a period, not a day. So this is a SNAPSHOT
//! group — computed today-only, forward-only, no watermark (like `knowledge`).
//!
//! ## What counts as a result
//!
//! Merged runs + accepted recommendations. Both are things the user shipped or
//! adopted — a completed *session* is activity, not delivery, and counting it
//! would make the metric fall simply because someone worked more.
//!
//! Honest-empty throughout: no configured subscription → no row (never a
//! fabricated price); a window that delivered nothing → no row (dividing by zero
//! is infinite, and charging the whole fee to "nothing" would call an idle
//! stretch infinitely expensive rather than idle).

use crate::cost::{COST_WINDOW_DAYS, SUBSCRIPTION_CONFIG_KEY, Subscription};
use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

const GRAIN_DAILY: &str = "daily";
const SOURCE_MEASURED: &str = "measured";
const SCOPE_USER: &str = "user";
const KEY_COST_PER_RESULT: &str = "cost_per_result";

/// Results delivered for `project_id` within the trailing window: merged runs +
/// accepted recommendations. Counted separately so the props can show which
/// contributed — a cost that moved because recommendations dried up is a
/// different story from one that moved because runs stopped merging.
async fn results_in_window(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<(i64, i64), String> {
    let sql = format!(
        "SELECT (SELECT count(*) FROM activity.runs \
                  WHERE project_id = $1 AND status = 'done' \
                    AND completed_at > now() - interval '{window_days} days')::int8, \
                (SELECT count(*) FROM inference.recommendations \
                  WHERE project_id = $1 AND status = 'accepted' \
                    AND acted_at > now() - interval '{window_days} days')::int8"
    );
    sqlx_core::query_as::query_as::<_, (i64, i64)>(&sql)
        .bind(project_id)
        .fetch_one(pg.pool())
        .await
        .map_err(|e| e.to_string())
}

pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("cost: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Forward-only: the window is anchored to NOW and the subscription is current
    // state, so a past `as_of` cannot be honestly reconstructed → no row.
    if super::is_historical(pg, as_of).await? {
        return Ok(0);
    }

    // No plan configured → no cost. Not 0.00, which a caller could not tell from a
    // genuinely free plan.
    let raw = pg.get_config(SUBSCRIPTION_CONFIG_KEY).await?;
    let Some(sub) = Subscription::parse(raw.as_deref()) else {
        return Ok(0);
    };

    let ids = pg.active_metric_ids(MetricGroup::Cost.as_str()).await?;
    let Some(mid) = ids.get(KEY_COST_PER_RESULT).copied() else {
        return Ok(0);
    };

    let (merged_runs, accepted_recs) = results_in_window(pg, &project_id, COST_WINDOW_DAYS).await?;
    let results = merged_runs + accepted_recs;
    let Ok(results_u32) = u32::try_from(results) else {
        return Ok(0);
    };
    let Some(value) = sub.cost_per_result(results_u32, COST_WINDOW_DAYS) else {
        return Ok(0); // nothing delivered in the window
    };

    // Same attribution rule as the other snapshot groups: cost has no natural
    // per-repo grain, so the single daily row rides the project's primary
    // repository. No repository-linked folder → no row (never fabricate one).
    let Some(repository_id) = pg.primary_repository_for_project(&project_id).await? else {
        return Ok(0);
    };

    let props = serde_json::json!({
        "numerator": sub.window_cost(COST_WINDOW_DAYS),
        "denominator": results,
        "merged_runs": merged_runs,
        "accepted_recommendations": accepted_recs,
        "window_days": COST_WINDOW_DAYS,
        "currency": sub.currency,
        "plan": sub.plan,
    });
    let day = super::today(pg).await?;
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

    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `cost.subscription` is ONE row in a database every test shares (#183), so
    /// the two tests that set it would otherwise race: one clears the key while
    /// the other has just written it, and whichever reads second sees the wrong
    /// plan. Observed as a flake in a full-suite run before this existed.
    ///
    /// A tokio `Mutex` rather than a `std` one because both holders `.await`
    /// across the critical section.
    static SUBSCRIPTION_KEY: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test]
    async fn no_configured_plan_writes_no_row() {
        // The honest-empty case that matters most: an unconfigured user must not
        // see a cost of 0.00, which reads as "free" rather than "unknown".
        let _guard = SUBSCRIPTION_KEY.lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return };
        let before = pg.get_config(SUBSCRIPTION_CONFIG_KEY).await.unwrap();
        pg.set_config(SUBSCRIPTION_CONFIG_KEY, "").await.unwrap();

        let ctx = crate::tasks::test_support::make_ctx().await;
        let pid = uuid::Uuid::new_v4();
        let n = compute(&ctx, &pid.to_string(), None).await.unwrap();

        if let Some(v) = before {
            pg.set_config(SUBSCRIPTION_CONFIG_KEY, &v).await.unwrap();
        }
        assert_eq!(n, 0, "no plan configured ⇒ no cost row");
    }

    #[tokio::test]
    async fn a_bad_project_id_is_an_error_not_a_silent_zero() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        assert!(compute(&ctx, "not-a-uuid", None).await.is_err());
    }

    /// `results_in_window` actually RUNS, and its answer reaches the row (#227).
    ///
    /// The two tests above both return before it: one at "no plan configured",
    /// the other at the uuid parse. So the only SQL in this module — four
    /// schema objects across two schemas, assembled with `format!` and
    /// therefore invisible to `check-sql-against-schema.py` — was executed by
    /// nothing, and a column renamed under it would have broken production with
    /// every test still green. Measured: cross-joining a table that does not
    /// exist into that query left this module GREEN before this test existed.
    ///
    /// Mutation that must break this test: break the `activity.runs` half of
    /// `results_in_window` — the assertion is on the count it returns, not
    /// merely on the call succeeding.
    #[tokio::test]
    async fn a_configured_plan_counts_delivered_results_through_the_window_query() {
        let _guard = SUBSCRIPTION_KEY.lock().await;
        let Ok(pg) = PgStore::connect_test().await else { return };
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = crate::tasks::test_support::seed_metrics_project_folder(&pg, &uniq).await;
        // Two merged runs inside the window and one outside it, so the row's
        // `merged_runs` can only be right if the window predicate ran too.
        //
        // `completed_at` is set explicitly because the query keys on IT, not on
        // `started_at` — a run that began inside the window and never finished
        // delivered nothing. `seed_run` leaves it NULL, which is the state of a
        // run still going, so a fixture that only sets `started_at` counts zero
        // results and this test would pass for the wrong reason.
        let now = chrono::Utc::now();
        let old = now - chrono::Duration::days(i64::from(COST_WINDOW_DAYS) + 5);
        for at in [now, now, old] {
            let rid = crate::tasks::test_support::seed_run(&pg, &pid, "done", at).await;
            sqlx_core::query::query("UPDATE activity.runs SET completed_at = $2 WHERE id = $1")
                .bind(rid)
                .bind(at)
                .execute(pg.pool())
                .await
                .unwrap();
        }

        let before = pg.get_config(SUBSCRIPTION_CONFIG_KEY).await.unwrap();
        pg.set_config(
            SUBSCRIPTION_CONFIG_KEY,
            r#"{"amount":100.0,"period":"monthly","currency":"USD","plan":"_test"}"#,
        )
        .await
        .unwrap();

        let ctx = crate::tasks::test_support::make_ctx().await;
        let wrote = compute(&ctx, &pid.to_string(), None).await.unwrap();

        pg.set_config(SUBSCRIPTION_CONFIG_KEY, before.as_deref().unwrap_or("")).await.unwrap();

        assert_eq!(wrote, 1, "a configured plan with results writes one row");
        let rows = crate::tasks::test_support::daily_project_metric_rows(&pg, &pid).await;
        let (_, value, props) = rows
            .iter()
            .find(|(k, _, _)| k == KEY_COST_PER_RESULT)
            .unwrap_or_else(|| panic!("no {KEY_COST_PER_RESULT} row in {rows:?}"))
            .clone();
        assert_eq!(props["merged_runs"], 2, "the run outside the window is not counted");
        assert_eq!(props["accepted_recommendations"], 0);
        assert_eq!(props["denominator"], 2);
        assert!(value > 0.0, "a positive fee over two results is a positive cost");

        crate::tasks::test_support::cleanup_metrics_fixture(&pg, &pid, Some(&fid), &[]).await;
    }
}
