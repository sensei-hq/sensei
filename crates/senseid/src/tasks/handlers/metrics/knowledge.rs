//! `knowledge` metric group computer (Phase 5.5).
//!
//! Follows the `session_outcomes` / `churn` / `duplication` / `autonomy` template
//! (resolve `key → metric_id` via the active registry, write a daily row to
//! `sensei.project_metrics` via [`PgStore::upsert_project_metric_repo`]) for ONE
//! project. The single row is `grain = daily`, `scope = user`, `identity = NULL`,
//! `folder_id`/`session_id` NULL, `computed_on = today` — a SNAPSHOT (see below),
//! matching the other snapshot computers (`churn`'s `rework_density`,
//! `duplication`'s `duplication_ratio`).
//!
//! ## Repo grain: attributed to the project's PRIMARY repository
//! Under the repo-grain identity (`metric_id, repository_id, scope, identity,
//! commit_sha, computed_on, grain`) every production write carries a real
//! `repository_id`; `folder_id`/`session_id` are no longer part of the identity and
//! a null-repository row would collide across projects. `knowledge` is
//! project-scoped and has NO natural per-repository grain (it counts memories /
//! patterns / corrections that belong to the whole project), so its single daily row
//! is attributed to [`PgStore::primary_repository_for_project`] — the project's
//! canonical (shallowest-checkout) repository. A project with NO repository-linked
//! folder cannot be attributed to a repository, so it writes NO row (honest-empty —
//! never fabricate a repository). `scope = user` (the default-project read), and
//! `identity = NULL` (a single local user; NULL keeps the row unique per
//! `(metric, repo, user, day)`). `commit_sha = NULL` — this is a day-bucketed
//! snapshot, not a commit-cadence measurement.
//!
//! v1 registry key (`task_name = "knowledge"`):
//! - `memory_promotion` (ratio, higher_better): does the measure→distill→govern
//!   loop close? `numerator` = # memories CREATED in the rolling window;
//!   `denominator` = # currently-ELIGIBLE patterns/corrections (those that have
//!   recurred `>= 3` times); `value` = num/den.
//!
//! ## A real 0 is THE signal here (not honest-empty)
//! Unlike a windowed ratio where a 0/0 is meaningless, `memory_promotion`'s 0 is
//! the point: a project with eligible items (`denominator >= 1`) but ZERO memories
//! created writes a REAL `0.0` (distillation has stalled — the catalog's "≈0 is the
//! signal" loop-health alert). Only a `denominator` of 0 (NO eligible items at all)
//! writes NO row — there is genuinely nothing to distill, so there is nothing to
//! measure (a 0/0 would be a fabricated zero, not a measured one). (A missing primary
//! repository ALSO writes NO row — see above — for the separate honest-empty reason
//! that a value cannot be attributed to any repository.)
//!
//! ## Numerator is windowed; denominator is a snapshot
//! The numerator counts memories CREATED in the last [`window_days`] (default 14),
//! bucketed on `sensei.memories.created_at` — the stable "learned" timestamp, NOT
//! `modified_at` (which is bumped on every reinforcement and so cannot date a
//! creation). The denominator is the CURRENT count of eligible items (a snapshot of
//! "how much is waiting to be distilled" right now), so the ratio reads as "of the
//! backlog that should be distilled, how much did we distill lately". The whole row
//! is therefore a point-in-time snapshot stored on [`super::today`].
//!
//! ## Eligibility spans BOTH patterns and corrections (summed)
//! The catalog formula is `count(memories created) / count(eligible
//! patterns+corrections)`, so the denominator sums two sources:
//! - `inference.detected_patterns` — column `instance_count`; project-scoped by the
//!   NOT-NULL `project_id` FK. Eligible = `instance_count >= 3`.
//! - `inference.corrections` — the recurrence tally column is `count` (there is no
//!   `instance_count` here); a correction is a GLOBAL cluster attributed to
//!   projects through its `project_ids` UUID ARRAY (it can span several). Eligible
//!   for THIS project = `count >= 3` AND `project_id = ANY(project_ids)`.
//!
//! Both are cleanly project-attributable, so both are counted (the display props
//! carry the split for transparency).
//!
//! Never-fabricate: every DB call propagates `Err`; `denominator = 0` writes NO row;
//! a project with no primary repository writes NO row; a real `denominator >= 1` with
//! 0 memories writes a real `0.0`. Strict project scoping (patterns via `project_id`,
//! corrections via `project_ids` membership, memories via `project_id`) — another
//! project's items never leak in.

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (knowledge writes a daily snapshot row only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — knowledge is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";
/// `sensei.metric_scope` text value — knowledge writes the local-user value only
/// (the default-project read); it has no whole-tree `repo` twin.
const SCOPE_USER: &str = "user";

/// The registry `key` this computer produces.
const KEY_MEMORY_PROMOTION: &str = "memory_promotion";

/// Recurrence threshold for an "eligible" pattern/correction — an item that has
/// recurred at least this many times is a distillation candidate. Applies to
/// `detected_patterns.instance_count` AND `corrections.count`.
const ELIGIBILITY_MIN_INSTANCES: i32 = 3;

/// # memories created for this project within the rolling window (the numerator).
/// Bucketed on `created_at` (the stable creation timestamp), project-scoped by the
/// `project_id` FK. A memory later archived still counts — it WAS distilled.
async fn memories_created(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    window_days: u32,
) -> Result<i64, String> {
    let (n,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*)::int8
           FROM sensei.memories
          WHERE project_id  = $1
            AND created_at >= now() - make_interval(days => $2::int)",
    )
    .bind(project_id)
    .bind(window_days as i32)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok(n)
}

/// `(eligible_patterns, eligible_corrections)` — the two halves of the denominator,
/// each a current snapshot of items that have recurred `>= min_instances` times.
/// Patterns are project-scoped by `project_id`; corrections by membership in the
/// `project_ids` array (`$1 = ANY(project_ids)`). Returned split so the caller can
/// sum them AND expose the breakdown in the row's display props.
async fn eligible_counts(
    pg: &PgStore,
    project_id: &uuid::Uuid,
    min_instances: i32,
) -> Result<(i64, i64), String> {
    let (patterns, corrections): (i64, i64) = sqlx_core::query_as::query_as(
        "SELECT (SELECT count(*)::int8
                   FROM inference.detected_patterns
                  WHERE project_id     = $1
                    AND instance_count >= $2)                       AS eligible_patterns
              , (SELECT count(*)::int8
                   FROM inference.corrections
                  WHERE $1 = ANY(project_ids)
                    AND count >= $2)                                AS eligible_corrections",
    )
    .bind(project_id)
    .bind(min_instances)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok((patterns, corrections))
}

/// Compute the `knowledge` group for one project as a snapshot as of today.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no eligible
/// patterns/corrections, no primary repository to attribute to, or the metric is
/// inactive). Idempotent — re-running backfills in place via the upsert identity.
/// This is a FORWARD-ONLY snapshot (Phase 3): the eligible denominator is a snapshot
/// of the current backlog and cannot be reconstructed for a past day, so a historical
/// `as_of` (`Some(D)`, `D != today`) writes NO row (see [`super::is_historical`]);
/// `None`/today keep the current snapshot-on-today behavior.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("knowledge: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Forward-only: the eligible-item denominator is a current snapshot and cannot be
    // reconstructed for a past day, so a historical `as_of` writes NO row.
    if super::is_historical(pg, as_of).await? {
        return Ok(0);
    }

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics. An absent key is
    // inactive (retired / not-yet-effective / unseeded) → skipped: the computer
    // never writes a value for an inactive metric.
    let ids = pg.active_metric_ids(MetricGroup::Knowledge.as_str()).await?;
    let Some(mid) = ids.get(KEY_MEMORY_PROMOTION).copied() else {
        return Ok(0);
    };

    let numerator = memories_created(pg, &project_id, window_days).await?;
    let (eligible_patterns, eligible_corrections) =
        eligible_counts(pg, &project_id, ELIGIBILITY_MIN_INSTANCES).await?;
    let denominator = eligible_patterns + eligible_corrections;

    if denominator == 0 {
        // No eligible items → no denominator → NO row (a 0/0 would be a fabricated
        // zero). A real denominator with 0 memories is handled below as a real 0.0
        // (the distillation-stalled SIGNAL), which is the whole point of the metric.
        return Ok(0);
    }

    // Repo grain: knowledge has no natural per-repo grain, so its single daily row is
    // attributed to the project's PRIMARY (shallowest-checkout) repository. A project
    // with no repository-linked folder cannot be attributed → NO row (honest-empty;
    // never fabricate a repository).
    let Some(repository_id) = pg.primary_repository_for_project(&project_id).await? else {
        return Ok(0);
    };

    // denominator >= 1 here: 0 memories over real eligible items writes a REAL 0.0
    // (never suppressed) — that flatline-at-0 is the loop-health signal.
    let value = numerator as f64 / denominator as f64;
    let props = serde_json::json!({
        "numerator": numerator,
        "denominator": denominator,
        "eligible_patterns": eligible_patterns,
        "eligible_corrections": eligible_corrections,
    });
    let day = super::today(pg).await?;
    // scope=user, identity=NULL (single local user), commit_sha=NULL (day-bucketed
    // snapshot, not commit cadence), folder_id/session_id=NULL (not in the identity).
    pg.upsert_project_metric_repo(
        &mid,
        &project_id,
        Some(&repository_id),
        SCOPE_USER,
        None,
        None,
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
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, make_ctx,
        purge_corrections, seed_correction, seed_memory, seed_metrics_project_folder,
        seed_pattern_with_instances,
    };
    use sqlx_core::query_as::query_as;

    #[tokio::test]
    #[allow(clippy::type_complexity)] // the repo-grain read-back is an inherent 6-column row
    async fn memory_promotion_real_zero_is_the_signal() {
        // The defining case: eligible items exist (real denominator) but ZERO memories
        // were created → value 0.0 is WRITTEN (a real zero, NOT a suppressed row). This
        // flatline-at-0 is the "distill loop is silent" signal the catalog calls out.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;

        // 3 eligible patterns (instance_count == 3), NO memories.
        for i in 0..3 {
            seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("p{i}-{uniq}"), true, 3).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "a real-zero memory_promotion row IS written (not suppressed)");

        let daily = daily_rows(pg, &pid).await;
        let mp = daily.iter().find(|r| r.0 == "memory_promotion").expect("memory_promotion row present");
        assert!(mp.1.abs() < 1e-9, "value is a real 0.0 (0 memories over 3 eligible) — the distill-stalled signal");
        assert_eq!(mp.2["numerator"].as_i64(), Some(0), "numerator = 0 (no memories created)");
        assert_eq!(mp.2["denominator"].as_i64(), Some(3), "denominator = 3 eligible (real denominator → row written)");
        assert_eq!(mp.2["eligible_patterns"].as_i64(), Some(3), "denominator breakdown: 3 eligible patterns");
        assert_eq!(mp.2["eligible_corrections"].as_i64(), Some(0), "denominator breakdown: 0 eligible corrections");

        // Repo grain (I-A/I-C): the row is attributed to the project's PRIMARY
        // repository with scope=user, identity NULL, commit_sha NULL, and no
        // folder_id/session_id.
        let expected_repo = pg
            .primary_repository_for_project(&pid)
            .await
            .unwrap()
            .expect("fixture project has a repository-linked folder");
        let (repo_id, scope, identity, commit_sha, folder_id, session_id): (
            Option<uuid::Uuid>,
            String,
            Option<String>,
            Option<String>,
            Option<uuid::Uuid>,
            Option<uuid::Uuid>,
        ) = query_as(
            "SELECT pm.repository_id, pm.scope::text, pm.identity, pm.commit_sha, pm.folder_id, pm.session_id \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'memory_promotion'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(repo_id, Some(expected_repo), "attributed to the project's primary repository (I-A)");
        assert_eq!(scope, "user", "knowledge writes scope=user only (I-B)");
        assert_eq!(identity, None, "identity NULL — single local user (I-C)");
        assert_eq!(commit_sha, None, "commit_sha NULL — day-bucketed snapshot, not commit cadence (I-D)");
        assert_eq!(folder_id, None, "folder_id NULL — not in the repo-grain identity (I-A)");
        assert_eq!(session_id, None, "session_id NULL — not in the repo-grain identity (I-A)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_positive_sums_patterns_and_corrections() {
        // A positive rate that PINS both eligibility sources AND both filters:
        // numerator = 2 memories created in-window (one out-of-window memory excluded);
        // denominator = 2 eligible patterns + 2 eligible corrections = 4 (an
        // instance_count-2 pattern and a count-2 correction are BELOW the >=3 threshold
        // and excluded) → value 2/4 = 0.5, props 2/4.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let sig_a = format!("_test:corr-a:{uniq}");
        let sig_b = format!("_test:corr-b:{uniq}");
        let sig_lo = format!("_test:corr-lo:{uniq}");
        purge_corrections(pg, &[&sig_a, &sig_b, &sig_lo]).await; // idempotent pre-clean

        // Numerator: 2 in-window memories + 1 memory created 30 days ago (excluded).
        let now = chrono::Utc::now();
        seed_memory(pg, &pid, now - chrono::Duration::hours(2)).await;
        seed_memory(pg, &pid, now - chrono::Duration::hours(3)).await;
        seed_memory(pg, &pid, now - chrono::Duration::days(30)).await; // outside the window

        // Denominator: 2 eligible patterns (instance_count 3 & 4) + 1 ineligible (2).
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("pe1-{uniq}"), true, 3).await;
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("pe2-{uniq}"), true, 4).await;
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("pi-{uniq}"), true, 2).await; // ineligible
        // + 2 eligible corrections (count 3 & 5) + 1 ineligible (count 2), all naming pid.
        seed_correction(pg, &sig_a, 3, &[pid]).await;
        seed_correction(pg, &sig_b, 5, &[pid]).await;
        seed_correction(pg, &sig_lo, 2, &[pid]).await; // ineligible

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "one memory_promotion project row");

        let daily = daily_rows(pg, &pid).await;
        let mp = daily.iter().find(|r| r.0 == "memory_promotion").expect("memory_promotion row present");
        assert!((mp.1 - 0.5).abs() < 1e-9, "value = 2 memories / 4 eligible = 0.5");
        assert_eq!(mp.2["numerator"].as_i64(), Some(2), "numerator = 2 (in-window memories; the 30-day-old one excluded)");
        assert_eq!(mp.2["denominator"].as_i64(), Some(4), "denominator = 4 (2 patterns + 2 corrections; the count-2 ones excluded)");
        assert_eq!(mp.2["eligible_patterns"].as_i64(), Some(2), "eligible patterns = 2 (instance_count-2 pattern excluded)");
        assert_eq!(mp.2["eligible_corrections"].as_i64(), Some(2), "eligible corrections = 2 (count-2 correction excluded)");

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(again, 1, "re-run recomputes the same row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 1, "idempotent upsert — still 1 row after a second run");

        purge_corrections(pg, &[&sig_a, &sig_b, &sig_lo]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_no_eligible_writes_no_row() {
        // 0 eligible items → denominator 0 → NO row, EVEN THOUGH memories exist (a 0/0
        // would be a fabricated zero, not a measured signal). Seeds an ineligible
        // pattern (instance_count 2) + an ineligible correction (count 2) so it's the
        // >=3 threshold — not "no items at all" — that yields zero.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let sig = format!("_test:corr-noeli:{uniq}");
        purge_corrections(pg, &[&sig]).await;

        seed_memory(pg, &pid, chrono::Utc::now() - chrono::Duration::hours(2)).await;
        seed_memory(pg, &pid, chrono::Utc::now() - chrono::Duration::hours(3)).await;
        seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("pi-{uniq}"), true, 2).await; // below >=3
        seed_correction(pg, &sig, 2, &[pid]).await; // below >=3

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "0 eligible items → denominator 0 → NO row (never a fabricated 0/0)");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows at all when nothing is eligible (memories present but no denominator)");

        purge_corrections(pg, &[&sig]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_no_repository_writes_no_row() {
        // Repo-grain honest-empty (I-A/I-E): a project with a REAL denominator (3
        // eligible patterns) but NO repository-linked folder cannot be attributed to a
        // repository, so its single daily row is SKIPPED — never a fabricated
        // repository. The SAME denominator with a repository writes a real-zero row
        // (see `memory_promotion_real_zero_is_the_signal`), so a 0 here is the
        // missing-repository guard, not empty/ineligible data.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:knowledge-norepo:{uniq}"), None, None)
            .await
            .unwrap();

        // 3 eligible patterns with NO folder (folder_id NULL) → project has no
        // repository-linked folder, so `primary_repository_for_project` is None.
        for i in 0..3 {
            seed_pattern_with_instances(pg, &pid, None, &format!("p{i}-{uniq}"), true, 3).await;
        }
        assert!(
            pg.primary_repository_for_project(&pid).await.unwrap().is_none(),
            "fixture precondition: project has no repository to attribute to"
        );

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no primary repository → single row cannot be attributed → honest-empty skip");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows when the project has no repository (never a fabricated repository)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no memories / patterns / corrections writes
        // NO rows (not a defaulted 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:knowledge-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no knowledge data in the window → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_excludes_other_projects() {
        // Cross-project isolation (mutation-proof): a SECOND real project's memories,
        // patterns, and a correction that names ONLY B must NOT leak into A. A
        // correction naming A specifically IS counted for A — proving the
        // `project_id = ANY(project_ids)` membership scope, not a global count.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_a = uuid::Uuid::new_v4();
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await; // a real SECOND project
        let sig_a = format!("_test:corr-onlyA:{uniq_a}");
        let sig_b = format!("_test:corr-onlyB:{uniq_b}");
        purge_corrections(pg, &[&sig_a, &sig_b]).await;

        // Project A: 1 memory, 2 eligible patterns, 1 eligible correction naming A only.
        seed_memory(pg, &pid_a, chrono::Utc::now() - chrono::Duration::hours(2)).await;
        seed_pattern_with_instances(pg, &pid_a, Some(&fid_a), &format!("pa1-{uniq_a}"), true, 3).await;
        seed_pattern_with_instances(pg, &pid_a, Some(&fid_a), &format!("pa2-{uniq_a}"), true, 3).await;
        seed_correction(pg, &sig_a, 4, &[pid_a]).await;
        // Project B: 5 memories, 5 eligible patterns, 1 eligible correction naming B only
        // — none of which must touch A.
        for _ in 0..5 {
            seed_memory(pg, &pid_b, chrono::Utc::now() - chrono::Duration::hours(2)).await;
        }
        for i in 0..5 {
            seed_pattern_with_instances(pg, &pid_b, Some(&fid_b), &format!("pb{i}-{uniq_b}"), true, 5).await;
        }
        seed_correction(pg, &sig_b, 9, &[pid_b]).await;

        let written = compute(&ctx, &pid_a.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "only A's own knowledge produces a row");

        let daily = daily_rows(pg, &pid_a).await;
        let mp = daily.iter().find(|r| r.0 == "memory_promotion").expect("A's memory_promotion row");
        assert_eq!(mp.2["numerator"].as_i64(), Some(1), "A's numerator = 1 (B's 5 memories excluded)");
        assert_eq!(mp.2["denominator"].as_i64(), Some(3), "A's denominator = 2 patterns + 1 correction = 3 (B's excluded, not 8)");
        assert_eq!(mp.2["eligible_patterns"].as_i64(), Some(2), "A's eligible patterns = 2 (B's 5 excluded)");
        assert_eq!(mp.2["eligible_corrections"].as_i64(), Some(1), "A's eligible corrections = 1 (B-only correction excluded — project_ids membership)");
        assert!((mp.1 - 1.0 / 3.0).abs() < 1e-9, "A value = 1/3 (B's data excluded)");

        // A's row is attributed to A's OWN primary repository (not B's) — repo-grain
        // attribution stays project-local.
        let expected_repo_a = pg
            .primary_repository_for_project(&pid_a)
            .await
            .unwrap()
            .expect("A has a repository-linked folder");
        let (repo_id_a,): (Option<uuid::Uuid>,) = query_as(
            "SELECT pm.repository_id \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'memory_promotion'",
        )
        .bind(pid_a)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(repo_id_a, Some(expected_repo_a), "A's row attributed to A's own primary repository");

        purge_corrections(pg, &[&sig_a, &sig_b]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }

    #[tokio::test]
    async fn memory_promotion_historical_as_of_skips() {
        // Forward-only guard (Phase 3): `memory_promotion`'s eligible denominator is a
        // snapshot of the CURRENT patterns/corrections backlog and cannot be
        // reconstructed for a past day, so a historical `as_of` (Some(D), D != today)
        // writes ZERO rows — never a fabricated historical snapshot. The SAME fixture
        // writes a real-zero row with `as_of = None` (see
        // `memory_promotion_real_zero_is_the_signal`), so a 0 here is the guard, not
        // empty data.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        for i in 0..3 {
            seed_pattern_with_instances(pg, &pid, Some(&fid), &format!("p{i}-{uniq}"), true, 3).await;
        }

        let past = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let written = compute(&ctx, &pid.to_string(), Some(past)).await.unwrap();
        assert_eq!(written, 0, "historical as_of → forward-only skip → zero rows");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for a historical as_of (never a fabricated snapshot)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
