//! `churn` metric group computer (Phase 5.2).
//!
//! The SECOND real base-metric computer; it follows the `session_outcomes`
//! template (read the rolling window, resolve `key → metric_id` via the active
//! registry, write daily + per-module rows to `sensei.project_metrics` via
//! [`PgStore::upsert_project_metric`]) for ONE project.
//!
//! v1 registry keys (all `task_name = "churn"`):
//! - `churn_rate` (count): # `process_file` executions attributed to a folder that
//!   day — per-module rows (grain `daily`, `folder_id` set) AND a project-level
//!   daily row (`folder_id` NULL) = Σ over folders.
//! - `churn_concentration` (pct, project daily): share of churn from the busiest
//!   20% of files — `numerator` = churn of the top-20% files, `denominator` = total
//!   churn (per day). No churn that day ⇒ NO row.
//! - `rework_density` (ratio, per-module + project): `numerator` = # rework-flagged
//!   files (`inference.detected_patterns` where `name LIKE 'rework: %'`),
//!   `denominator` = # project files (`kind = 'file'` `sensei.nodes`). 0 project
//!   files ⇒ NO row.
//!
//! ## Governance property (non-negotiable — this is why churn is reviewed hard)
//! `activity.task_executions` carries only a `folder_path` (no project/folder id).
//! Every execution is attributed via [`PgStore::resolve_folder_by_path`]
//! (alias-aware). When it returns `None` — the `folder_path` can't be resolved to a
//! project/module — the execution is `warn!`-logged and SKIPPED: it counts toward
//! NEITHER the module NOR the project aggregate, and produces no row on its
//! account. An execution that resolves to a DIFFERENT project is simply not ours
//! (that project's own churn task counts it). NEVER mis-attribute an unresolvable
//! execution to a fallback/default project (see the #109 fail-closed audit).
//!
//! Never-fabricate: every DB call propagates `Err`; a metric/day/module with no
//! data writes NO row. A ratio/pct with denominator 0 writes NO row (a 0/0 would be
//! a fabricated zero); a real denominator with 0 numerator writes a real `0.0`.
//!
//! Caveat (a known data-quality issue, NOT a bug to fix here): `churn_rate` /
//! `churn_concentration` counts are inflated by the version-rescan bug until a
//! debounce lands — the first live numbers carry that caveat (as the catalog's
//! `how_to_read` says: de-noise before trending).

use std::collections::HashMap;

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (churn writes daily rows only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — churn is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key`s this computer produces.
const KEY_CHURN_RATE: &str = "churn_rate";
const KEY_CHURN_CONCENTRATION: &str = "churn_concentration";
const KEY_REWORK_DENSITY: &str = "rework_density";

/// The `activity.task_executions.task_kind` that IS churn — a per-file rewrite pass.
const PROCESS_FILE_KIND: &str = "process_file";

/// Pareto cut for `churn_concentration`: the busiest 20% of files. `ceil`-rounded
/// so any day with ≥1 file has ≥1 file in the top set.
const CONCENTRATION_TOP_FRACTION: f64 = 0.20;

/// One `(folder_path, day, file_path, count)` churn tuple from `task_executions` —
/// resolution (`folder_path → project/folder`) and project-filtering happen in
/// Rust so the load-bearing alias-aware attribution goes through the shared
/// [`PgStore::resolve_folder_by_path`], not a hand-rolled join.
type ExecRow = (String, chrono::NaiveDate, String, i64);

/// Raw per-`(folder_path, day, file)` counts of `process_file` executions over the
/// window — EVERY folder_path (attribution to a project happens after resolution).
/// `path` is the absolute file the execution processed (globally unique, so it is
/// a safe `churn_concentration` file key).
async fn process_file_executions(
    pg: &PgStore,
    window_days: u32,
) -> Result<Vec<ExecRow>, String> {
    sqlx_core::query_as::query_as(
        "SELECT folder_path                                   AS folder_path
              , date_trunc('day', started_at)::date           AS day
              , path                                           AS file_path
              , count(*)::int8                                 AS n
           FROM activity.task_executions
          WHERE task_kind    = $1
            AND started_at  >= now() - make_interval(days => $2::int)
          GROUP BY folder_path, day, file_path",
    )
    .bind(PROCESS_FILE_KIND)
    .bind(window_days as i32)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())
}

/// `(project_file_count, per-folder file counts)` — "project files" are
/// `kind = 'file'` nodes across the project's folders (the `rework_density`
/// denominator). Per-folder counts feed the per-module denominators.
async fn project_file_counts(
    pg: &PgStore,
    project_id: &uuid::Uuid,
) -> Result<(i64, HashMap<uuid::Uuid, i64>), String> {
    let rows: Vec<(uuid::Uuid, i64)> = sqlx_core::query_as::query_as(
        "SELECT n.folder_id                                   AS folder_id
              , count(*)::int8                                 AS file_count
           FROM sensei.nodes   n
           JOIN sensei.folders f ON f.id = n.folder_id
          WHERE f.project_id  = $1
            AND n.kind        = 'file'::sensei.node_kind
          GROUP BY n.folder_id",
    )
    .bind(project_id)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    let total = rows.iter().map(|(_, c)| c).sum();
    Ok((total, rows.into_iter().collect()))
}

/// `(project rework count, per-folder rework counts)` — rework-flagged files are
/// `inference.detected_patterns` rows the analyzer writes as `name = "rework:
/// <file>"` (`is_anti_pattern`). One row per file (the table's uniqueness is
/// `(project_id, name, is_anti_pattern)`), so a row count IS a distinct-file count.
/// Per-folder counts (via the `folder_id` locus) feed the per-module numerators.
async fn rework_counts(
    pg: &PgStore,
    project_id: &uuid::Uuid,
) -> Result<(i64, HashMap<uuid::Uuid, i64>), String> {
    let (total,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*)::int8
           FROM inference.detected_patterns
          WHERE project_id      = $1
            AND is_anti_pattern
            AND name LIKE 'rework: %'",
    )
    .bind(project_id)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    let rows: Vec<(uuid::Uuid, i64)> = sqlx_core::query_as::query_as(
        "SELECT folder_id                                     AS folder_id
              , count(*)::int8                                 AS rework_count
           FROM inference.detected_patterns
          WHERE project_id      = $1
            AND is_anti_pattern
            AND name LIKE 'rework: %'
            AND folder_id IS NOT NULL
          GROUP BY folder_id",
    )
    .bind(project_id)
    .fetch_all(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok((total, rows.into_iter().collect()))
}

/// Today's date (DB `current_date`) — the `computed_on` for `rework_density`, which
/// is a point-in-time snapshot (flagged files ÷ project files now), not a windowed
/// per-day series. Read from the DB so its day boundary matches the
/// `date_trunc('day', started_at)::date` the churn window uses (same session TZ).
async fn today(pg: &PgStore) -> Result<chrono::NaiveDate, String> {
    let (d,): (chrono::NaiveDate,) = sqlx_core::query_as::query_as("SELECT current_date")
        .fetch_one(pg.pool())
        .await
        .map_err(|e| e.to_string())?;
    Ok(d)
}

/// Compute the `churn` group for one project over the configured window.
/// `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no churn/rework
/// data, or none of the group's metrics active). Idempotent — re-running backfills
/// in place via the upsert identity.
pub(super) async fn compute(ctx: &TaskContext, project_raw: &str) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("churn: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Reuse the scheduler's window reader (config key + parser + default) — DRY.
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;

    // Resolve key → metric_id for this group's ACTIVE metrics. A key absent from the
    // map is inactive (retired / not-yet-effective / unseeded) → skipped.
    let ids = pg.active_metric_ids(MetricGroup::Churn.as_str()).await?;
    if ids.is_empty() {
        return Ok(0);
    }
    let churn_rate_id = ids.get(KEY_CHURN_RATE).copied();
    let concentration_id = ids.get(KEY_CHURN_CONCENTRATION).copied();
    let rework_id = ids.get(KEY_REWORK_DENSITY).copied();

    let mut written = 0u32;

    // ── churn_rate + churn_concentration (both read process_file executions) ──
    if churn_rate_id.is_some() || concentration_id.is_some() {
        // Per-(folder_id, day) churn for the per-module rows; per-day totals for the
        // project rows; per-(day, file) churn for concentration's Pareto cut.
        let mut by_folder_day: HashMap<(uuid::Uuid, chrono::NaiveDate), i64> = HashMap::new();
        let mut by_day_total: HashMap<chrono::NaiveDate, i64> = HashMap::new();
        let mut by_day_file: HashMap<chrono::NaiveDate, HashMap<String, i64>> = HashMap::new();
        // folder_path → resolution, cached so each distinct path resolves once and
        // an unresolvable path is warned about exactly once.
        let mut resolved: HashMap<String, Option<(uuid::Uuid, uuid::Uuid)>> = HashMap::new();

        for (folder_path, day, file_path, n) in process_file_executions(pg, window_days).await? {
            if !resolved.contains_key(&folder_path) {
                let r = pg.resolve_folder_by_path(&folder_path).await?;
                if r.is_none() {
                    // GOVERNANCE: unresolvable folder_path → skip entirely (counts
                    // toward NEITHER module nor project). Never fall back to a
                    // default project id.
                    tracing::warn!(
                        folder_path = %folder_path,
                        group = "churn",
                        "compute_churn: folder_path does not resolve to a project — \
                         skipping execution (attributed to no module and no project)",
                    );
                }
                resolved.insert(folder_path.clone(), r);
            }
            // Only executions resolving to THIS project count. `None` (unresolvable)
            // and other projects both skip; only `None` was warned above.
            let Some((folder_id, resolved_project)) = resolved[&folder_path] else {
                continue;
            };
            if resolved_project != project_id {
                continue;
            }

            *by_folder_day.entry((folder_id, day)).or_insert(0) += n;
            *by_day_total.entry(day).or_insert(0) += n;
            *by_day_file.entry(day).or_default().entry(file_path).or_insert(0) += n;
        }

        // churn_rate (count): per-module rows (folder_id set) + project daily rows.
        if let Some(mid) = churn_rate_id {
            let no_props = serde_json::json!({});
            for ((folder_id, day), count) in &by_folder_day {
                pg.upsert_project_metric(
                    &mid, &project_id, Some(folder_id), None, *day, GRAIN_DAILY,
                    *count as f64, &no_props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
            for (day, count) in &by_day_total {
                pg.upsert_project_metric(
                    &mid, &project_id, None, None, *day, GRAIN_DAILY,
                    *count as f64, &no_props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }

        // churn_concentration (pct, project daily): Σ top-20% files / Σ all files.
        if let Some(mid) = concentration_id {
            for (day, files) in &by_day_file {
                let total: i64 = files.values().sum();
                if total == 0 {
                    // No churn that day → no denominator → NO row (never a 0/0).
                    continue;
                }
                // Rank files by churn desc, path asc (deterministic ties), take the
                // busiest ceil(20%) — ≥1 whenever there is ≥1 file.
                let mut ranked: Vec<(&String, i64)> =
                    files.iter().map(|(p, &c)| (p, c)).collect();
                ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
                let top_k = ((ranked.len() as f64) * CONCENTRATION_TOP_FRACTION).ceil() as usize;
                let top_k = top_k.max(1);
                let numerator: i64 = ranked.iter().take(top_k).map(|(_, c)| *c).sum();
                let value = numerator as f64 / total as f64;
                let props = serde_json::json!({ "numerator": numerator, "denominator": total });
                pg.upsert_project_metric(
                    &mid, &project_id, None, None, *day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    // ── rework_density (ratio): per-module + project snapshot as of today ──
    if let Some(mid) = rework_id {
        let (project_files, folder_files) = project_file_counts(pg, &project_id).await?;
        let (rework_total, folder_rework) = rework_counts(pg, &project_id).await?;
        let day = today(pg).await?;

        // Project row: rework files ÷ project files. 0 project files → NO row (a real
        // denominator of 0 would be a fabricated 0/0). 0 rework over real files → a
        // real 0.0 (row written).
        if project_files > 0 {
            let value = rework_total as f64 / project_files as f64;
            let props = serde_json::json!({ "numerator": rework_total, "denominator": project_files });
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }

        // Per-module rows for folders that carry a rework signal. Denominator is the
        // folder's own file count; a folder with 0 files → NO row (no denominator).
        for (folder_id, rework_count) in &folder_rework {
            let files_here = folder_files.get(folder_id).copied().unwrap_or(0);
            if files_here == 0 {
                continue;
            }
            let value = *rework_count as f64 / files_here as f64;
            let props = serde_json::json!({ "numerator": rework_count, "denominator": files_here });
            pg.upsert_project_metric(
                &mid, &project_id, Some(folder_id), None, day, GRAIN_DAILY, value, &props,
                SOURCE_MEASURED,
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
        cleanup_metrics_fixture, make_ctx, seed_detected_pattern, seed_file_node,
        seed_metrics_project_folder, seed_task_execution,
    };
    use sqlx_core::query_as::query_as;

    /// Daily project-scope rows (`folder_id IS NULL`) keyed by metric `key`.
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

    /// Per-module rows (`folder_id` set) for one metric `key`: `(folder_id, value, props)`.
    async fn module_rows(
        pg: &PgStore,
        pid: &uuid::Uuid,
        key: &str,
    ) -> Vec<(uuid::Uuid, f64, serde_json::Value)> {
        query_as(
            "SELECT pm.folder_id, pm.value::float8, pm.props \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.folder_id IS NOT NULL \
                AND m.key = $2 ORDER BY pm.folder_id",
        )
        .bind(pid)
        .bind(key)
        .fetch_all(pg.pool())
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn churn_attributes_to_project_via_folder_path() {
        // THE load-bearing governance test: churn is attributed via
        // resolve_folder_by_path — a live path resolves, an ALIASED old path resolves
        // to the same folder, and an UNRESOLVABLE path is skipped (no row, warned) and
        // mis-attributes to NOTHING.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let abs = format!("/_test/metrics-{uniq}"); // the fixture folder's abs_path (resolves)
        let alias = format!("/_test/old-{uniq}"); // an old path aliased to the SAME folder
        let unknown = format!("/_test/unknown-{uniq}"); // resolves to no folder/project
        pg.add_folder_path_alias(&alias, &fid, "rename").await.unwrap();

        let ts = chrono::Utc::now() - chrono::Duration::hours(2); // fixed day
        // Resolvable churn (all → folder `fid`): a.rs ×2, b.rs ×1 under the live path;
        // c.rs ×1 under the ALIAS path (must resolve to `fid` too). Σ = 4.
        seed_task_execution(pg, "process_file", &abs, &format!("{abs}/a.rs"), ts).await;
        seed_task_execution(pg, "process_file", &abs, &format!("{abs}/a.rs"), ts).await;
        seed_task_execution(pg, "process_file", &abs, &format!("{abs}/b.rs"), ts).await;
        seed_task_execution(pg, "process_file", &alias, &format!("{abs}/c.rs"), ts).await;
        // UNRESOLVABLE folder_path → SKIPPED (must not appear in any aggregate).
        seed_task_execution(pg, "process_file", &unknown, &format!("{unknown}/e.rs"), ts).await;
        // A non-churn task_kind under the live path → must NOT be counted.
        seed_task_execution(pg, "process_folder", &abs, &abs, ts).await;

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        // 1 churn_rate module row + 1 churn_rate project row + 1 concentration row.
        // (No file nodes / rework patterns seeded → rework_density writes nothing.)
        assert_eq!(written, 3, "churn_rate module + project + concentration; rework_density empty");

        // ── churn_rate per-module: one folder (fid), count 4 (a×2 + b×1 + c×1) ──
        let modules = module_rows(pg, &pid, "churn_rate").await;
        assert_eq!(modules.len(), 1, "exactly one module row (the resolved fixture folder)");
        assert_eq!(modules[0].0, fid, "the module row is attributed to the resolved folder");
        assert!((modules[0].1 - 4.0).abs() < 1e-9, "module churn = 4 (aliased c.rs counted; unresolvable e.rs and process_folder excluded)");

        // ── churn_rate project daily = Σ folders = 4 (this is the mutation-proof
        // assertion: if the unresolvable exec were mis-attributed to this project it
        // would be 5) ──
        let daily = daily_rows(pg, &pid).await;
        let rate = daily.iter().find(|r| r.0 == "churn_rate").expect("churn_rate project row");
        assert!((rate.1 - 4.0).abs() < 1e-9, "project churn = 4 (unresolvable execution skipped, not mis-attributed)");

        // ── churn_concentration: files a=2,b=1,c=1 (total 4); top ceil(20% of 3)=1
        // busiest file is a (2) → 2/4 = 0.5 ──
        let conc = daily.iter().find(|r| r.0 == "churn_concentration").expect("churn_concentration row");
        assert!((conc.1 - 0.5).abs() < 1e-9, "concentration = top-file churn 2 / total 4 = 0.5");
        assert_eq!(conc.2["numerator"].as_i64(), Some(2), "concentration numerator = busiest-20% churn");
        assert_eq!(conc.2["denominator"].as_i64(), Some(4), "concentration denominator = total churn");

        // ── Idempotency: re-run backfills in place, never duplicates ──
        let again = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(again, 3, "re-run recomputes the same rows");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 3, "idempotent upsert — still 3 rows after a second run");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[&abs, &alias, &unknown]).await;
    }

    #[tokio::test]
    async fn churn_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no executions / patterns / files writes NO
        // rows (not a defaulted 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:churn-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string()).await.unwrap();
        assert_eq!(written, 0, "no churn/rework data in the window → zero rows written");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for an empty project (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, None, &[]).await;
    }

    #[tokio::test]
    async fn churn_rework_density_ratio_and_zero_project_files() {
        // rework_density = # rework-flagged files ÷ # project files, with exact
        // props.numerator/denominator; and the 0-project-files case writes NO row.
        let ctx = make_ctx().await;
        let pg = ctx.pg();

        // ── Project A: 2 rework files over 4 project files → 2/4 = 0.5 ──
        let uniq_a = uuid::Uuid::new_v4();
        let (pid_a, fid_a) = seed_metrics_project_folder(pg, &uniq_a).await;
        for f in ["a.rs", "b.rs", "c.rs", "d.rs"] {
            seed_file_node(pg, &fid_a, &format!("/_test/metrics-{uniq_a}/{f}")).await;
        }
        seed_detected_pattern(pg, &pid_a, Some(&fid_a), "rework: a.rs", true).await;
        seed_detected_pattern(pg, &pid_a, Some(&fid_a), "rework: b.rs", true).await;
        // A non-rework anti-pattern must NOT be counted.
        seed_detected_pattern(pg, &pid_a, Some(&fid_a), "correction-prone", true).await;

        let written_a = compute(&ctx, &pid_a.to_string()).await.unwrap();
        assert_eq!(written_a, 2, "rework_density project row + one per-module row (no executions → no churn rows)");

        let daily_a = daily_rows(pg, &pid_a).await;
        let rd = daily_a.iter().find(|r| r.0 == "rework_density").expect("rework_density project row");
        assert!((rd.1 - 0.5).abs() < 1e-9, "rework_density = 2 rework files / 4 project files = 0.5");
        assert_eq!(rd.2["numerator"].as_i64(), Some(2), "numerator = # rework-flagged files (correction-prone excluded)");
        assert_eq!(rd.2["denominator"].as_i64(), Some(4), "denominator = # project files");

        let modules_a = module_rows(pg, &pid_a, "rework_density").await;
        assert_eq!(modules_a.len(), 1, "one per-module rework_density row (the folder with the signal)");
        assert_eq!(modules_a[0].0, fid_a, "module row attributed to the rework folder");
        assert!((modules_a[0].1 - 0.5).abs() < 1e-9, "module rework_density = 2/4 = 0.5");
        assert_eq!(modules_a[0].2["numerator"].as_i64(), Some(2), "module numerator = folder rework files");
        assert_eq!(modules_a[0].2["denominator"].as_i64(), Some(4), "module denominator = folder file count");

        // ── Project B: rework flagged but ZERO project files → NO row ──
        let uniq_b = uuid::Uuid::new_v4();
        let (pid_b, fid_b) = seed_metrics_project_folder(pg, &uniq_b).await;
        seed_detected_pattern(pg, &pid_b, Some(&fid_b), "rework: x.rs", true).await;

        let written_b = compute(&ctx, &pid_b.to_string()).await.unwrap();
        assert_eq!(written_b, 0, "0 project files → no denominator → NO rework_density row (never a fabricated 0/0)");
        let (total_b,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid_b)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total_b, 0, "no rows at all for the zero-project-files project");

        cleanup_metrics_fixture(pg, &pid_a, Some(&fid_a), &[]).await;
        cleanup_metrics_fixture(pg, &pid_b, Some(&fid_b), &[]).await;
    }
}
