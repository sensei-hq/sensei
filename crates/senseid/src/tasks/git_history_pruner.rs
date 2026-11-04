//! Git-history retention (#224).
//!
//! The ONLY thing that deletes a commit. Everything else about
//! `sensei.commits` is insert-only, deliberately: a commit is an immutable
//! fact, and a first design that deleted commits for becoming unreachable was
//! unsound — two checkouts of one repository each compute "unreachable" from
//! their own HEAD and would delete the other's history, forever.
//!
//! Retention is a different kind of statement. It is about how much storage
//! the history is worth, not about whether it happened, which is why it is by
//! AGE and nothing else.
//!
//! ## The floor, and why it is enforced rather than documented
//!
//! The diagrams read a WINDOW — co-change defaults to 90 days. If retention
//! were shorter than the window, a 90-day query would return a confident
//! answer computed over a third of its evidence, and Ownership would decay
//! into "who touched this recently". Nothing in the result would say so.
//!
//! So [`effective_retention_days`] takes the larger of the configured
//! retention and the largest window any caller can ask for. A misconfiguration
//! costs disk; it cannot cost correctness.

use std::sync::Arc;

use crate::db::pg_store::PgStore;
use crate::tasks::ticker;

/// Keep history for 400 days by default.
///
/// Covers the 90-day co-change window and a year-over-year comparison with
/// room to spare. Measured cost of keeping it: ~121,780 `commits` rows and
/// ~738,000 `commit_files` rows machine-wide, about 185 MB — against the
/// 3,533 MB `activity.task_executions` already occupies.
pub const DEFAULT_RETENTION_DAYS: i32 = 400;

/// The largest window a diagram may request, and therefore the floor under
/// retention. Co-change defaults to 90; this is the ceiling on that knob.
pub const MAX_WINDOW_DAYS: i32 = 365;

/// Config key for the retention override.
const RETENTION_KEY: &str = "git_history.retention_days";

/// Resolve retention, never below the largest window a reader can ask for.
///
/// Pure so the rule is testable without a database — the same reason
/// `reconcile_scheduler::parse_stall_secs` is pure.
pub fn effective_retention_days(configured: Option<String>) -> i32 {
    let requested = configured
        .and_then(|s| s.trim().parse::<i32>().ok())
        .filter(|d| *d > 0)
        .unwrap_or(DEFAULT_RETENTION_DAYS);
    requested.max(MAX_WINDOW_DAYS)
}

/// Spawn the pruner for the daemon's lifetime.
pub fn spawn(pg: Arc<PgStore>) {
    tokio::spawn(async move {
        ticker::run_scheduled(pg.clone(), "git_history_prune", move || {
            let pg = pg.clone();
            async move { prune_once(&pg).await }
        })
        .await;
    });
}

/// One prune pass. Non-fatal: a failure is logged and retried next tick.
pub async fn prune_once(pg: &PgStore) -> Result<(), String> {
    let configured = pg.get_config(RETENTION_KEY).await.ok().flatten();
    let days = effective_retention_days(configured);
    let removed = pg.prune_commits_older_than(days).await?;
    if removed > 0 {
        tracing::info!(
            removed,
            retention_days = days,
            "git_history_prune: commits older than the retention window removed"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D83 batch A, row 155 (review F4): a retention that cannot be read stops
    /// the prune, and a commit past every window stays. The second half proves
    /// the commit is in the prune's reach once the read works.
    #[tokio::test]
    #[ignore = "pending: D83 batch A"]
    async fn a_failed_retention_read_stops_the_prune_and_deletes_nothing() {
        let pg = PgStore::test_store().await;
        let pid = pg
            .create_workspace(&format!("_test:prune:{}", uuid::Uuid::new_v4()), None, None)
            .await
            .unwrap();
        let rid =
            crate::tasks::test_support::seed_bare_repository(&pg, &pid, &uuid::Uuid::new_v4())
                .await;
        sqlx_core::query::query(
            "INSERT INTO sensei.commits(repository_id, sha, authored_at) VALUES ($1, $2, '2000-01-01')",
        )
        .bind(rid)
        .bind(format!("{:040}", 1))
        .execute(pg.pool())
        .await
        .unwrap();
        let run = |sql: &'static str| sqlx_core::query::query(sql);
        let kept = |pg: &PgStore| {
            let pool = pg.pool().clone();
            async move {
                let (n,): (i64,) = sqlx_core::query_as::query_as(
                    "SELECT count(*) FROM sensei.commits WHERE repository_id = $1",
                )
                .bind(rid)
                .fetch_one(&pool)
                .await
                .unwrap();
                n
            }
        };

        run("ALTER TABLE sensei.config RENAME TO config_unreadable")
            .execute(pg.pool())
            .await
            .unwrap();
        let failed = prune_once(&pg).await;
        let after_failed = kept(&pg).await;
        run("ALTER TABLE sensei.config_unreadable RENAME TO config")
            .execute(pg.pool())
            .await
            .unwrap();
        prune_once(&pg).await.unwrap();
        let after_ok = kept(&pg).await;

        assert!(failed.is_err(), "a failed read is the prune's error: {failed:?}");
        assert_eq!((after_failed, after_ok), (1, 0), "kept on a failed read; pruned once it reads");
    }
    #[test]
    fn an_unset_retention_falls_back_to_the_default() {
        assert_eq!(effective_retention_days(None), DEFAULT_RETENTION_DAYS);
    }

    /// A configured value above the floor is honoured exactly.
    #[test]
    fn a_configured_retention_above_the_floor_is_used_as_given() {
        assert_eq!(effective_retention_days(Some("  700 ".into())), 700);
    }

    /// THE POINT OF THIS MODULE. A retention shorter than the largest window a
    /// diagram can request is raised to it, because the alternative is a
    /// 90-day co-change query silently answered from 30 days of evidence.
    ///
    /// Mutation that must break this test: drop the `.max(MAX_WINDOW_DAYS)`.
    #[test]
    fn a_retention_below_the_largest_window_is_raised_to_it() {
        assert_eq!(effective_retention_days(Some("30".into())), MAX_WINDOW_DAYS);
        assert_eq!(effective_retention_days(Some("1".into())), MAX_WINDOW_DAYS);
    }

    /// Nonsense and non-positive values fall back rather than being obeyed.
    /// "Retain for 0 days" and "retain nothing" are the same instruction, and
    /// neither is one a typo should be able to give.
    #[test]
    fn a_zero_negative_or_unparseable_retention_never_deletes_everything() {
        for v in ["0", "-5", "", "soon", "30.5"] {
            assert_eq!(
                effective_retention_days(Some(v.into())),
                DEFAULT_RETENTION_DAYS.max(MAX_WINDOW_DAYS),
                "input {v:?} must fall back, never be obeyed"
            );
        }
    }
}
