//! `project_health` derived-score computer (Phase 6).
//!
//! The `ComputeHealth` barrier's real body: it rolls the project's base metrics up
//! into ONE 0–100 composite score and writes a single project-scope daily row for
//! the `project_health` registry metric (`type = score`, `family = composite`,
//! `higher_better`). It runs AFTER the base `ComputeMetrics` tasks land for a
//! project (the scheduler wires it `blocked_by` those ids), so the latest daily
//! component values are already in `sensei.project_metrics` when it reads them.
//!
//! ## The score
//! `value = round(100 × weighted mean of each active component metric's latest
//! daily value NORMALIZED to [0,1])`, weighted by `sensei.metrics.weight`.
//!
//! For each ACTIVE component metric, take its LATEST daily project-scope value
//! (grain `daily`, `folder_id` NULL — via [`PgStore::get_project_metrics`]) and
//! normalize to [0,1] by its `direction`/`type` (see [`normalize`]):
//! - **ratio/pct, higher_better**: `norm = clamp(value, 0, 1)`.
//! - **ratio/pct, lower_better**:  `norm = clamp(1 - value, 0, 1)`.
//! - **count/duration/currency, higher_better** (needs `target`):
//!   `norm = clamp(value / target, 0, 1)`.
//! - **count/duration/currency, lower_better** (needs `target`):
//!   `norm = 1.0` when `value <= target` (including value 0), else
//!   `clamp(target / value, 0, 1)`.
//!
//! A component is EXCLUDED from the mean (does NOT contribute) when:
//! - its `direction` is `neutral` (read only with its companion, never scored);
//! - it is a `count`/`duration`/`currency` metric with `target IS NULL` (no bound
//!   to normalize against);
//! - it is the `project_health` metric itself (`type = score` / `family =
//!   composite`) — never recurse into the score;
//! - it has NO latest daily value (no data — honest-absent, NOT a zero).
//!
//! Then `score = round(100 × Σ(weightᵢ·normᵢ) / Σ(weightᵢ))` over the INCLUDED
//! components, stored alongside `props.components` (the included keys → their norms)
//! for display.
//!
//! ## Never-fabricate (the load-bearing property)
//! - No included components ⇒ write NO health row (never a fabricated 100 or 0). An
//!   empty project, or one where every metric is excluded/absent, gets no row.
//! - Every DB read propagates its `Err` (`.await?`); no `.unwrap_or_default()` /
//!   `.ok()` masking, no fabricated ids. Strict project scoping — a component value
//!   is only ever read for THIS project (`get_project_metrics` is project-scoped).

use crate::tasks::executor::TaskContext;

/// `sensei.metric_grain` text value (health writes one daily project row).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — the score is derived from measured values.
const SOURCE_MEASURED: &str = "measured";
/// The composite metric's registry `key` (its `task_name` is
/// [`super::HEALTH_TASK_NAME`]). Resolved to an id via the active registry.
const KEY_PROJECT_HEALTH: &str = "project_health";

/// Normalize one component metric's latest daily `value` to `[0, 1]` by its
/// `metric_type` and `direction`, returning `None` when the component is EXCLUDED
/// from the score (no defined normalization — never fabricated into the mean):
/// a `neutral` direction, a `count`/`duration`/`currency` with no `target`, or a
/// `score`/`value` type (the composite itself). Pure — unit-tested without a DB.
fn normalize(metric_type: &str, direction: &str, value: f64, target: Option<f64>) -> Option<f64> {
    match (metric_type, direction) {
        // Ratios/percentages are already in [0,1] units — clamp, flipping for
        // lower_better so "less is better" scores higher.
        ("ratio" | "pct", "higher_better") => Some(value.clamp(0.0, 1.0)),
        ("ratio" | "pct", "lower_better") => Some((1.0 - value).clamp(0.0, 1.0)),
        // Magnitudes need a target to normalize against; target-null ⇒ excluded.
        ("count" | "duration" | "currency", "higher_better") => {
            target.map(|t| (value / t).clamp(0.0, 1.0))
        }
        ("count" | "duration" | "currency", "lower_better") => {
            target.map(|t| if value <= t { 1.0 } else { (t / value).clamp(0.0, 1.0) })
        }
        // neutral direction, or a score/value type (incl. project_health itself):
        // no defined normalization ⇒ excluded from the score.
        _ => None,
    }
}

/// Compute the derived `project_health` score for one project and write its single
/// project-scope daily row. `project_raw` is the project uuid carried in
/// `task.folder_path`. Returns the number of rows written: `1` when at least one
/// component is included, else `0` (never-fabricate: no components ⇒ NO row, not a
/// phantom 100/0). Idempotent — re-running backfills the row in place.
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("health: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Resolve the composite metric's id via the active registry (task_name "health").
    // Inactive/unseeded ⇒ nothing to write (honest-absent, never fabricated).
    let health_ids = pg.active_metric_ids(super::HEALTH_TASK_NAME).await?;
    let Some(&health_id) = health_ids.get(KEY_PROJECT_HEALTH) else {
        return Ok(0);
    };

    // Every active component metric carries the weight/target/direction/type the
    // roll-up needs; the project's latest daily project-scope value per metric key
    // supplies the value. A metric with no latest value is simply absent from the map
    // (honest-absent) and skipped below.
    let active = pg.active_metrics().await?;
    let latest: std::collections::HashMap<String, f64> = pg
        .get_project_metrics(&project_id)
        .await?
        .into_iter()
        .map(|r| (r.metric, r.value))
        .collect();

    let mut sum_weighted = 0.0f64;
    let mut sum_weight = 0.0f64;
    let mut components = serde_json::Map::new();
    for m in &active {
        // Never recurse: the composite score never includes itself.
        if m.id == health_id {
            continue;
        }
        // Honest-absent: a component with no latest daily value contributes nothing
        // (NOT a fabricated 0 that would drag the score down).
        let Some(&value) = latest.get(&m.key) else {
            continue;
        };
        // Excluded (neutral direction, target-null magnitude, score/value type) ⇒
        // None ⇒ not counted toward the weighted mean.
        let Some(norm) = normalize(&m.metric_type, &m.direction, value, m.target) else {
            continue;
        };
        sum_weighted += m.weight * norm;
        sum_weight += m.weight;
        components.insert(m.key.clone(), serde_json::json!(norm));
    }

    // Never-fabricate: no included components ⇒ write NO row (not a phantom 100/0).
    // `sum_weight <= 0` also guards the division when every included weight is 0.
    if sum_weight <= 0.0 {
        return Ok(0);
    }

    let score = (100.0 * (sum_weighted / sum_weight)).round();
    let props = serde_json::json!({ "components": serde_json::Value::Object(components) });
    let day = super::today(pg).await?;
    pg.upsert_project_metric(
        &health_id, &project_id, None, None, day, GRAIN_DAILY, score, &props, SOURCE_MEASURED,
    )
    .await?;
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::PgStore;
    use crate::tasks::test_support::{cleanup_metrics_fixture, make_ctx, seed_metrics_project_folder};
    use sqlx_core::query_as::query_as;

    /// Seed a dedicated `sensei.metrics` component row (unique `key` ⇒ parallel-safe:
    /// its per-project VALUES are what a health computation reads, and no other
    /// project has a value for this key). Active from `current_date`, never retired.
    /// The real registry's weights are all `1`, so tests seed their own to exercise
    /// weighting/normalization. Returns the metric id. Cleared via [`purge_metrics`].
    async fn seed_health_metric(
        pg: &PgStore,
        key: &str,
        metric_type: &str,
        direction: &str,
        weight: f64,
        target: Option<f64>,
    ) -> uuid::Uuid {
        let (id,): (uuid::Uuid,) = query_as(
            "INSERT INTO sensei.metrics
                (key, name, description, family, type, direction, purpose, how_to_read, formula,
                 task_name, weight, target, effective_from)
             VALUES ($1, $1, 'test component', 'quality'::sensei.metric_family, $2::sensei.metric_type,
                     $3::sensei.metric_direction, 'p', 'h', 'f', $1,
                     $4::float8::numeric, $5::float8::numeric, current_date)
             RETURNING id",
        )
        .bind(key)
        .bind(metric_type)
        .bind(direction)
        .bind(weight)
        .bind(target)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        id
    }

    /// Upsert one latest daily project-scope value for a component metric.
    async fn seed_value(pg: &PgStore, mid: &uuid::Uuid, pid: &uuid::Uuid, value: f64) {
        let day = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        pg.upsert_project_metric(mid, pid, None, None, day, GRAIN_DAILY, value, &serde_json::json!({}), SOURCE_MEASURED)
            .await
            .unwrap();
    }

    /// The stored `project_health` daily score for a project (project scope), or
    /// `None` when no health row was written.
    async fn health_row(pg: &PgStore, pid: &uuid::Uuid) -> Option<(f64, serde_json::Value)> {
        query_as(
            "SELECT pm.value::float8, pm.props \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'project_health' \
                AND pm.grain = 'daily' AND pm.folder_id IS NULL",
        )
        .bind(pid)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
    }

    /// Delete dedicated component metrics (cascades their `project_metrics`).
    async fn purge_metrics(pg: &PgStore, keys: &[&str]) {
        let keys: Vec<String> = keys.iter().map(|s| s.to_string()).collect();
        sqlx_core::query::query("DELETE FROM sensei.metrics WHERE key = ANY($1)")
            .bind(&keys)
            .execute(pg.pool())
            .await
            .unwrap();
    }

    #[test]
    fn normalize_matrix() {
        let approx = |got: Option<f64>, want: f64| {
            let g = got.unwrap_or_else(|| panic!("expected Some({want})"));
            assert!((g - want).abs() < 1e-9, "normalize => {g}, want {want}");
        };
        // ratio/pct higher_better: clamp(value).
        approx(normalize("ratio", "higher_better", 0.8, None), 0.8);
        approx(normalize("pct", "higher_better", 1.5, None), 1.0); // clamped above 1
        approx(normalize("pct", "higher_better", -0.2, None), 0.0); // clamped below 0
        // ratio/pct lower_better: clamp(1 - value).
        approx(normalize("ratio", "lower_better", 0.3, None), 0.7);
        approx(normalize("ratio", "lower_better", 1.4, None), 0.0); // 1-1.4 clamped to 0
        // count/duration/currency higher_better: clamp(value/target).
        approx(normalize("count", "higher_better", 10.0, Some(5.0)), 1.0); // 10/5 clamped
        approx(normalize("count", "higher_better", 2.0, Some(5.0)), 0.4);
        approx(normalize("duration", "higher_better", 4.0, Some(8.0)), 0.5);
        // count/duration lower_better: 1.0 at/under target (incl 0), else target/value.
        approx(normalize("count", "lower_better", 6.0, Some(3.0)), 0.5);
        approx(normalize("count", "lower_better", 0.0, Some(3.0)), 1.0); // value 0 under target
        approx(normalize("count", "lower_better", 3.0, Some(3.0)), 1.0); // value == target
        approx(normalize("currency", "lower_better", 4.0, Some(2.0)), 0.5);
        // Exclusions ⇒ None.
        assert_eq!(normalize("count", "higher_better", 10.0, None), None, "count needs a target");
        assert_eq!(normalize("count", "lower_better", 10.0, None), None, "count needs a target");
        assert_eq!(normalize("pct", "neutral", 0.5, None), None, "neutral excluded");
        assert_eq!(normalize("ratio", "neutral", 0.5, None), None, "neutral excluded");
        assert_eq!(normalize("score", "higher_better", 50.0, None), None, "score type excluded (never recurse)");
        assert_eq!(normalize("value", "higher_better", 1.0, Some(2.0)), None, "value type excluded");
    }

    #[tokio::test]
    async fn project_health_normalizes_by_direction_and_weight() {
        // ftr-like: pct, higher_better, weight 2, value 0.8 → norm 0.8.
        // rework-like: ratio, lower_better, weight 1, value 0.3 → norm 1-0.3 = 0.7.
        // score = round(100 × (2·0.8 + 1·0.7)/3) = round(76.67) = 77.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let ftr_key = format!("_test:health:{uniq}:ftr");
        let rework_key = format!("_test:health:{uniq}:rework");
        let m_ftr = seed_health_metric(pg, &ftr_key, "pct", "higher_better", 2.0, None).await;
        let m_rework = seed_health_metric(pg, &rework_key, "ratio", "lower_better", 1.0, None).await;
        seed_value(pg, &m_ftr, &pid, 0.8).await;
        seed_value(pg, &m_rework, &pid, 0.3).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "one project_health row written");

        let (value, props) = health_row(pg, &pid).await.expect("project_health row present");
        assert!((value - 77.0).abs() < 1e-9, "weighted, direction-normalized score = 77, got {value}");
        // props.components carries the included keys → their norms.
        assert!((props["components"][&ftr_key].as_f64().unwrap() - 0.8).abs() < 1e-9, "ftr norm 0.8");
        assert!((props["components"][&rework_key].as_f64().unwrap() - 0.7).abs() < 1e-9, "rework norm 0.7");

        purge_metrics(pg, &[&ftr_key, &rework_key]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn project_health_absent_when_no_components() {
        // Never-fabricate: a project with no component daily values gets NO health
        // row — never a phantom 100 or 0.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:health-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "no components → no health row written");
        assert!(health_row(pg, &pid).await.is_none(), "no project_health row exists (never fabricated)");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows at all for an empty project");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn project_health_count_normalization_and_target_null_excluded() {
        // throughput-like: count higher_better, target 5, value 10 → 10/5 clamps to 1.0.
        // churn-like:      count lower_better,  target 3, value 6  → 3/6 = 0.5.
        // target-null count: value 99 → EXCLUDED (no bound to normalize against).
        // score = round(100 × (1·1.0 + 1·0.5)/2) = 75 — only holds if the null one is out.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let tp_key = format!("_test:health:{uniq}:tp");
        let cr_key = format!("_test:health:{uniq}:cr");
        let null_key = format!("_test:health:{uniq}:null");
        let m_tp = seed_health_metric(pg, &tp_key, "count", "higher_better", 1.0, Some(5.0)).await;
        let m_cr = seed_health_metric(pg, &cr_key, "count", "lower_better", 1.0, Some(3.0)).await;
        let m_null = seed_health_metric(pg, &null_key, "count", "higher_better", 1.0, None).await;
        seed_value(pg, &m_tp, &pid, 10.0).await;
        seed_value(pg, &m_cr, &pid, 6.0).await;
        seed_value(pg, &m_null, &pid, 99.0).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 1, "one project_health row written");

        let (value, props) = health_row(pg, &pid).await.expect("project_health row present");
        assert!((value - 75.0).abs() < 1e-9, "count-normalized score = 75 (target-null excluded), got {value}");
        assert!((props["components"][&tp_key].as_f64().unwrap() - 1.0).abs() < 1e-9, "throughput norm 1.0 (clamped)");
        assert!((props["components"][&cr_key].as_f64().unwrap() - 0.5).abs() < 1e-9, "churn norm 0.5");
        assert!(props["components"].get(&null_key).is_none(), "target-null count is not a component");

        purge_metrics(pg, &[&tp_key, &cr_key, &null_key]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn project_health_neutral_direction_excluded() {
        // A neutral-direction metric present with a value must NOT affect the score.
        // contributor: pct higher_better value 0.6 → norm 0.6 → score 60.
        // If neutral leaked in (mis-scored as higher_better), (0.6+0.9)/2 → 75.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let a_key = format!("_test:health:{uniq}:a");
        let neutral_key = format!("_test:health:{uniq}:neutral");
        let m_a = seed_health_metric(pg, &a_key, "pct", "higher_better", 1.0, None).await;
        let m_neutral = seed_health_metric(pg, &neutral_key, "pct", "neutral", 1.0, None).await;
        seed_value(pg, &m_a, &pid, 0.6).await;
        seed_value(pg, &m_neutral, &pid, 0.9).await;

        compute(&ctx, &pid.to_string()).await.unwrap();

        let (value, props) = health_row(pg, &pid).await.expect("project_health row present");
        assert!((value - 60.0).abs() < 1e-9, "neutral excluded → score reflects only the contributor (60), got {value}");
        assert!(props["components"].get(&neutral_key).is_none(), "neutral metric is not a component");

        purge_metrics(pg, &[&a_key, &neutral_key]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn project_health_absent_component_excluded() {
        // A component that is ACTIVE but has NO daily value is skipped — it must not
        // drag the score toward 0. contributor value 0.5 → score 50; if the absent
        // one were counted as 0, (0.5+0)/2 → 25.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let a_key = format!("_test:health:{uniq}:a");
        let absent_key = format!("_test:health:{uniq}:absent");
        let m_a = seed_health_metric(pg, &a_key, "pct", "higher_better", 1.0, None).await;
        let _m_absent = seed_health_metric(pg, &absent_key, "pct", "higher_better", 1.0, None).await;
        seed_value(pg, &m_a, &pid, 0.5).await; // NB: no value seeded for the absent metric

        compute(&ctx, &pid.to_string()).await.unwrap();

        let (value, props) = health_row(pg, &pid).await.expect("project_health row present");
        assert!((value - 50.0).abs() < 1e-9, "absent component skipped → score 50 (not dragged to 25), got {value}");
        assert!(props["components"].get(&absent_key).is_none(), "the value-less metric is not a component");

        purge_metrics(pg, &[&a_key, &absent_key]).await;
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn project_health_isolates_across_projects() {
        // Cross-project isolation: the same metric key holds value 0.9 for A and 0.1
        // for B. A's score reflects ONLY A's value (90); if B's value leaked into A's
        // roll-up, the mean would collapse to 0.5 → 50.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq_a = uuid::Uuid::new_v4();
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await;
        let shared_key = format!("_test:health:{uniq_a}:shared");
        let m = seed_health_metric(pg, &shared_key, "pct", "higher_better", 1.0, None).await;
        seed_value(pg, &m, &pid_a, 0.9).await;
        seed_value(pg, &m, &pid_b, 0.1).await;

        compute(&ctx, &pid_a.to_string()).await.unwrap();
        compute(&ctx, &pid_b.to_string()).await.unwrap();

        let (a_val, _) = health_row(pg, &pid_a).await.expect("A project_health row present");
        let (b_val, _) = health_row(pg, &pid_b).await.expect("B project_health row present");
        assert!((a_val - 90.0).abs() < 1e-9, "A reflects only A's value (90, not 50 if B leaked), got {a_val}");
        assert!((b_val - 10.0).abs() < 1e-9, "B reflects only B's value (10), got {b_val}");

        purge_metrics(pg, &[&shared_key]).await;
        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }
}
