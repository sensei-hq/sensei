//! `churn` metric group computer.
//!
//! Follows the `session_outcomes` template (resolve `key → metric_id` via the
//! active registry, write daily rows to `sensei.project_metrics` via
//! [`PgStore::upsert_project_metric`]) for ONE project, but its two churn metrics
//! are sourced from **git**, not the file-indexing feed:
//!
//! - `churn_rate` (count, project daily): the number of DISTINCT source files
//!   touched by that day's commits — the real files-changed-over-time signal
//!   (GitClear's churn *definition*, measured first-party from `git log`). One
//!   project-level row per commit-day.
//! - `churn_concentration` (pct, project daily): the share of the day's line-churn
//!   absorbed by the busiest 20% of files (Pareto) — `numerator` = line-churn of
//!   the top-20% files, `denominator` = total line-churn. A day with commits but
//!   zero line-churn (only binary/mode changes) has no denominator ⇒ NO
//!   concentration row (never a fabricated 0/0), though `churn_rate` still counts
//!   the touched files.
//! - `rework_density` (ratio, per-module + project): UNCHANGED — a forward-only
//!   snapshot of `inference.detected_patterns` (`name LIKE 'rework: %'`) over
//!   `sensei.nodes` project files. Out of scope for the git re-sourcing; it keeps
//!   its DB source and its historical-`as_of`-skips behavior.
//!
//! ## Git sourcing (why + how)
//! The two churn metrics were previously counted from `activity.task_executions`
//! (the file-INDEXING feed), so a re-index spiked `churn_rate` and only recent
//! history existed. Real churn is files-changed-over-time from GIT, so they now
//! read `git log` for the project's repo root — resolved via
//! [`PgStore::project_root_path`] (the shortest repo-root `folders.abs_path`). A
//! project with no repo-root folder, a root that is not a git repo, git being
//! absent, or a repo with no commits on the selected day all produce NO row: an
//! honest "no git churn data", never a fabricated value. This mirrors the git
//! discipline already used by `indexer/cross_repo` and `tasks/handlers/scan`
//! (shell out to git, tolerate its absence).
//!
//! Commits bucket on the COMMITTER date (`%cd --date=short`); both the planner's
//! per-day discovery ([`git_commit_days`]) and this computer read that same field,
//! so a planned commit-day maps to the day the compute buckets it under. Merges
//! are excluded (`--no-merges`).
//!
//! ## `as_of` (per-day, mirrors `session_outcomes`/`autonomy`)
//! `churn` is a PER-DAY planned group: the planner enqueues one
//! `ComputeMetrics{as_of=Some(D)}` per git-commit-day plus the trailing window, so
//! churn backfills over real git history.
//! - `Some(D)` — compute ONLY day `D` from that day's commits, `computed_on = D`.
//! - `None` — the rolling window: every commit-day in the trailing
//!   [`metrics.window_days`] window through today.
//!
//! ## Capture-scope split (governance, #3)
//! Churn is GIT-derived, so its day-set must NOT authorize the activity pruner's
//! capture-before-reclaim: a git-churn row for a day must never green-light
//! reclaiming that day's sessions before their session-anchored metrics (ftr /
//! throughput) are captured. The planner backfills churn per-day, but
//! `planner::day_keyed_task_names()` (the pruner's capture scope) stays
//! SESSION-derived only (`session_outcomes`, `autonomy`) and EXCLUDES churn.
//!
//! Never-fabricate: every DB call propagates `Err`; a git failure/miss produces NO
//! row (honest-empty). A ratio/pct with denominator 0 writes NO row.

use std::collections::HashMap;

use chrono::NaiveDate;

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

/// Pareto cut for `churn_concentration`: the busiest 20% of files. `ceil`-rounded
/// so any day with ≥1 file has ≥1 file in the top set.
const CONCENTRATION_TOP_FRACTION: f64 = 0.20;

/// The date format `git log --date=short` emits (`%cd`) and both this computer and
/// the planner parse — the single place the committer-day format lives.
const GIT_DATE_FMT: &str = "%Y-%m-%d";
/// Sentinel byte prefixed to each commit's date line in `git log` output so header
/// lines are unambiguously separable from `--numstat` body lines (SOH never
/// appears in a file path or an ISO date). See [`parse_numstat_log`].
const COMMIT_MARK: char = '\u{1}';

/// Run `git log` in `root` with `args` and return stdout, or `None` when git is
/// unavailable, `root` is not a git repo / does not exist, the repo has no commits,
/// or the command otherwise fails. A `None` is an honest "no git churn data" (the
/// caller writes no row), never a fabricated value — matching the git discipline in
/// `indexer/cross_repo` and `tasks/handlers/scan` (shell out, tolerate absence).
fn run_git(root: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Parse `git log --numstat --date=short --pretty=format:'\x01%cd'` output into
/// per-`(day, file)` line-churn. Each commit contributes a header line
/// `\x01YYYY-MM-DD` followed by `--numstat` rows `added\tdeleted\tpath`
/// (`added`/`deleted` are `-` for binary files). A file touched with only binary or
/// zero-line diffs still appears (weight 0) so it counts toward `churn_rate`'s
/// distinct-file total. Pure over the captured text so it is unit-testable without a
/// git subprocess. A day appears ONLY when ≥1 numstat body line was seen (so an
/// empty commit contributes no day → no fabricated churn).
fn parse_numstat_log(stdout: &str) -> HashMap<NaiveDate, HashMap<String, i64>> {
    let mut by_day: HashMap<NaiveDate, HashMap<String, i64>> = HashMap::new();
    let mut current: Option<NaiveDate> = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix(COMMIT_MARK) {
            current = NaiveDate::parse_from_str(rest.trim(), GIT_DATE_FMT).ok();
            continue;
        }
        let Some(day) = current else { continue };
        // numstat body: added \t deleted \t path (path may itself contain tabs).
        let mut parts = line.splitn(3, '\t');
        let (Some(add), Some(del), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if path.is_empty() {
            continue;
        }
        // Binary files report '-' for both counts → 0 line-churn, still a touched file.
        let weight = add.parse::<i64>().unwrap_or(0) + del.parse::<i64>().unwrap_or(0);
        *by_day.entry(day).or_default().entry(path.to_string()).or_insert(0) += weight;
    }
    by_day
}

/// Distinct committer-days (`%cd --date=short`) across `root`'s whole history — the
/// churn DATA-day set the planner backfills over. Empty on a non-git root, an absent
/// git, or an empty repo (honest "no git churn", never a fabricated day). Shared with
/// the planner (`DayKeyedGroup::Churn::data_days`) so its per-day discovery and this
/// module's per-day compute read the SAME committer-day field and can't drift.
pub(super) fn git_commit_days(root: &str) -> Vec<NaiveDate> {
    let Some(out) = run_git(
        root,
        &["log", "--no-merges", "--date=short", "--pretty=format:%cd"],
    ) else {
        return Vec::new();
    };
    let mut days: std::collections::BTreeSet<NaiveDate> = std::collections::BTreeSet::new();
    for line in out.lines() {
        if let Ok(d) = NaiveDate::parse_from_str(line.trim(), GIT_DATE_FMT) {
            days.insert(d);
        }
    }
    days.into_iter().collect()
}

/// Per-`(day, file)` line-churn for the compute's day-set, read from `git log`.
/// `as_of = Some(D)` → the single committer-day `D`; `None` → the trailing
/// `window_days` window through `today`. The scan is bounded with `--since`/`--until`
/// (a ±2-day buffer that comfortably absorbs a commit's timezone offset, which is
/// < 1 day) and then filtered to the EXACT `[lo, hi]` committer-day range on the
/// parsed `%cd` date — so the bucketing matches [`git_commit_days`] regardless of
/// git's date-boundary interpretation. Empty on a non-git root / absent git / no
/// commits on the range (honest-empty).
fn git_day_file_churn(
    root: &str,
    window_days: u32,
    today: NaiveDate,
    as_of: Option<NaiveDate>,
) -> HashMap<NaiveDate, HashMap<String, i64>> {
    let (lo, hi) = match as_of {
        Some(d) => (d, d),
        None => (
            today - chrono::Duration::days((window_days.max(1) - 1) as i64),
            today,
        ),
    };
    let since = (lo - chrono::Duration::days(2)).format(GIT_DATE_FMT).to_string();
    let until = (hi + chrono::Duration::days(2)).format(GIT_DATE_FMT).to_string();
    let pretty = format!("--pretty=format:{COMMIT_MARK}%cd");
    let Some(out) = run_git(
        root,
        &[
            "log", "--no-merges", "--numstat", "--date=short", &pretty, "--since", &since,
            "--until", &until,
        ],
    ) else {
        return HashMap::new();
    };
    let mut by_day = parse_numstat_log(&out);
    by_day.retain(|d, _| *d >= lo && *d <= hi);
    by_day
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

/// Compute the `churn` group for one project. `project_raw` is the project uuid
/// carried in `task.folder_path`. `as_of` is the target `computed_on` day:
/// - `churn_rate` / `churn_concentration` (git-sourced): `Some(D)` computes ONLY
///   day `D`'s commits (`computed_on = D`, the backfill/gap-fill path); `None`
///   computes every commit-day in the trailing window.
/// - `rework_density` (forward-only snapshot): a historical `as_of` (`Some(D)`,
///   `D != today`) skips it (see [`super::is_historical`]).
///
/// Returns the number of `project_metrics` rows written (`0` = honest-empty: no git
/// churn on the day-set, no rework/file data, or none of the group's metrics
/// active). Idempotent — re-running backfills in place via the upsert identity.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<NaiveDate>,
) -> Result<u32, String> {
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

    // ── churn_rate + churn_concentration (git-sourced, per-day) ──────────────
    if churn_rate_id.is_some() || concentration_id.is_some() {
        // The project's git-root working dir (shortest repo-root abs_path). No
        // repo-root folder → no git churn source → honest-empty (no rows).
        if let Some(root) = pg.project_root_path(&project_id).await? {
            let today = super::today(pg).await?;
            for (day, files) in git_day_file_churn(&root, window_days, today, as_of) {
                // churn_rate (count): # distinct files touched that day. A returned
                // day always carries ≥1 file (the parser records a day only for a
                // real numstat line), so this is ≥ 1 — never a fabricated 0.
                if let Some(mid) = churn_rate_id {
                    let no_props = serde_json::json!({});
                    pg.upsert_project_metric(
                        &mid, &project_id, None, None, day, GRAIN_DAILY, files.len() as f64,
                        &no_props, SOURCE_MEASURED,
                    )
                    .await?;
                    written += 1;
                }
                // churn_concentration (pct): Σ top-20% files' line-churn / Σ all.
                if let Some(mid) = concentration_id {
                    let total: i64 = files.values().sum();
                    if total == 0 {
                        // Commits touched files but with zero line-churn (all
                        // binary/mode) → no denominator → NO row (never a 0/0).
                        continue;
                    }
                    // Rank files by churn desc, path asc (deterministic ties), take
                    // the busiest ceil(20%) — ≥1 whenever there is ≥1 file.
                    let mut ranked: Vec<(&String, i64)> =
                        files.iter().map(|(p, &c)| (p, c)).collect();
                    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
                    let top_k = ((ranked.len() as f64) * CONCENTRATION_TOP_FRACTION).ceil() as usize;
                    let top_k = top_k.max(1);
                    let numerator: i64 = ranked.iter().take(top_k).map(|(_, c)| *c).sum();
                    let value = numerator as f64 / total as f64;
                    let props =
                        serde_json::json!({ "numerator": numerator, "denominator": total });
                    pg.upsert_project_metric(
                        &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props,
                        SOURCE_MEASURED,
                    )
                    .await?;
                    written += 1;
                }
            }
        }
    }

    // ── rework_density (ratio): per-module + project snapshot as of today ──
    // Forward-only: a rework snapshot reflects the CURRENT signal vs current project
    // files and cannot be reconstructed for a past day, so a historical `as_of`
    // (Some(D), D != today) skips it entirely — no fabricated historical snapshot.
    // This is the LAST block, so a `return` here is equivalent to skipping it.
    if let Some(mid) = rework_id {
        if super::is_historical(pg, as_of).await? {
            return Ok(written);
        }
        let (project_files, folder_files) = project_file_counts(pg, &project_id).await?;
        let (rework_total, folder_rework) = rework_counts(pg, &project_id).await?;
        let day = super::today(pg).await?;

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
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, git_commit_on_day,
        make_ctx, module_metric_rows as module_rows, seed_detected_pattern, seed_file_node,
        seed_git_project_folder, seed_metrics_project_folder,
    };
    use sqlx_core::query_as::query_as;

    // ── Pure: numstat parser ─────────────────────────────────────────────────

    #[test]
    fn parse_numstat_log_buckets_line_churn_by_day_and_file() {
        // Two commits on 2020-01-01 (a.rs +3/-1, b.rs +0/-2) and one on 2020-01-02
        // (a.rs +5/-0, bin -/- binary). Per-(day,file) weight = added + deleted; a
        // binary file (`-`/`-`) still appears with weight 0 (a touched file).
        let m = '\u{1}';
        let log = format!(
            "{m}2020-01-01\n3\t1\ta.rs\n0\t2\tb.rs\n{m}2020-01-01\n2\t0\ta.rs\n{m}2020-01-02\n5\t0\ta.rs\n-\t-\tbin.png\n"
        );
        let by_day = parse_numstat_log(&log);
        let d1 = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2020, 1, 2).unwrap();
        assert_eq!(by_day[&d1]["a.rs"], 6, "a.rs day-1 churn = (3+1)+(2+0) = 6");
        assert_eq!(by_day[&d1]["b.rs"], 2, "b.rs day-1 churn = 0+2 = 2");
        assert_eq!(by_day[&d1].len(), 2, "two distinct files touched on day 1");
        assert_eq!(by_day[&d2]["a.rs"], 5, "a.rs day-2 churn = 5+0 = 5");
        assert_eq!(by_day[&d2]["bin.png"], 0, "binary file counts as a touched file (weight 0)");
        assert_eq!(by_day[&d2].len(), 2, "a.rs + bin.png distinct on day 2");
    }

    // ── Git-sourced churn (temp repo fixtures) ───────────────────────────────

    #[tokio::test]
    async fn churn_git_single_day_as_of_over_seeded_commits() {
        // THE re-sourcing test: churn_rate + churn_concentration come from `git log`
        // for the project's repo root, computed for a SINGLE historical day D via
        // as_of=Some(D), stamped computed_on=D (mirrors session_outcomes/autonomy).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;

        // A commit on D (60 days ago, well outside the default 14-day window):
        // a.rs +4 lines, b.rs +2 lines. Line-churn a=4, b=2 (total 6); 2 files.
        let d = (chrono::Utc::now() - chrono::Duration::days(60)).date_naive();
        let day = d.format("%Y-%m-%d").to_string();
        git_commit_on_day(repo.path(), &day, &[("a.rs", "1\n2\n3\n4\n"), ("b.rs", "1\n2\n")]);

        // Incremental (as_of=None): D is outside the rolling window → NO churn rows
        // (honest-empty for the recent window, never a fabricated backfill).
        let incr = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old commit-day is outside the rolling window → no incremental churn rows");

        // Backfill (as_of=Some(D)): computes exactly that day's churn.
        let written = compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();
        assert_eq!(written, 2, "churn_rate + churn_concentration for day D (no file nodes/rework seeded)");

        let daily = daily_rows(pg, &pid).await;
        let rate = daily.iter().find(|r| r.0 == "churn_rate").expect("churn_rate project row");
        assert!((rate.1 - 2.0).abs() < 1e-9, "churn_rate = 2 distinct files changed (a.rs, b.rs)");
        let conc = daily.iter().find(|r| r.0 == "churn_concentration").expect("churn_concentration row");
        // top ceil(20% of 2) = 1 busiest file = a.rs (4) / total line-churn 6.
        assert!((conc.1 - 4.0 / 6.0).abs() < 1e-9, "concentration = top-file line-churn 4 / total 6");
        assert_eq!(conc.2["numerator"].as_i64(), Some(4), "concentration numerator = busiest-20% line-churn");
        assert_eq!(conc.2["denominator"].as_i64(), Some(6), "concentration denominator = total line-churn");

        // computed_on is stamped to the true commit day D (60 days ago).
        let (rate_on_d,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND m.key = 'churn_rate' AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(d)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rate_on_d, 1, "the churn_rate row is stamped computed_on = the commit's day D");

        // Idempotent: re-backfilling the same day upserts in place.
        let again = compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();
        assert_eq!(again, 2, "re-running the same day backfills in place");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 2, "idempotent upsert — still 2 rows after a second backfill");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_concentration_top_20_percent_rounds_up_over_git() {
        // Pin the ceil(20%) Pareto rule with N=6 files, where ceil(6×0.2)=2 diverges
        // from a floor-then-max(1) mutation (=1). One commit on day D with per-file
        // line-churn 4,3,1,1,1,1 (total 11); the top TWO busiest files (4+3=7) are
        // the numerator, NOT the top-1 (4).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;

        let d = (chrono::Utc::now() - chrono::Duration::days(30)).date_naive();
        let day = d.format("%Y-%m-%d").to_string();
        let lines = |n: usize| (0..n).map(|i| format!("{i}")).collect::<Vec<_>>().join("\n") + "\n";
        git_commit_on_day(
            repo.path(),
            &day,
            &[
                ("f1.rs", &lines(4)),
                ("f2.rs", &lines(3)),
                ("f3.rs", &lines(1)),
                ("f4.rs", &lines(1)),
                ("f5.rs", &lines(1)),
                ("f6.rs", &lines(1)),
            ],
        );

        compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();

        let daily = daily_rows(pg, &pid).await;
        let rate = daily.iter().find(|r| r.0 == "churn_rate").expect("churn_rate row");
        assert!((rate.1 - 6.0).abs() < 1e-9, "churn_rate = 6 distinct files changed");
        let conc = daily.iter().find(|r| r.0 == "churn_concentration").expect("concentration row");
        assert_eq!(conc.2["denominator"].as_i64(), Some(11), "denominator = total line-churn (11)");
        assert_eq!(conc.2["numerator"].as_i64(), Some(7), "numerator = top ceil(6×0.2)=2 files (4+3=7), NOT top-1 (4)");
        assert!((conc.1 - 7.0 / 11.0).abs() < 1e-9, "value = 7/11");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_non_git_project_writes_no_churn_rows_but_rework_still_computes() {
        // A project whose repo-root folder is NOT a git repo (the synthetic
        // `/_test/metrics-*` path) → git churn source misses → NO churn_rate /
        // churn_concentration rows (honest-empty, never a fabricated 0). rework_density
        // is DB-sourced and unaffected — it still computes its real 0.0 over real files.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await; // synthetic, non-git path
        for f in ["a.rs", "b.rs", "c.rs"] {
            seed_file_node(pg, &fid, &format!("/_test/metrics-{uniq}/{f}")).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "only the rework_density project row (0.0 over 3 files); no git churn rows");

        let daily = daily_rows(pg, &pid).await;
        assert!(!daily.iter().any(|r| r.0 == "churn_rate"), "no churn_rate row for a non-git root (honest-empty)");
        assert!(!daily.iter().any(|r| r.0 == "churn_concentration"), "no churn_concentration row for a non-git root");
        let rd = daily.iter().find(|r| r.0 == "rework_density").expect("rework_density row IS written (DB-sourced)");
        assert!(rd.1.abs() < 1e-9, "rework_density = 0 rework over 3 real files = 0.0");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── rework_density (unchanged — DB-sourced, out of scope for the git re-source) ──

    #[tokio::test]
    async fn churn_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no repo-root folder, no patterns, and no
        // files writes NO rows (not a defaulted 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:churn-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no git churn / rework / files → zero rows written");

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

        let written_a = compute(&ctx, &pid_a.to_string(), None).await.unwrap();
        assert_eq!(written_a, 2, "rework_density project row + one per-module row (non-git root → no churn rows)");

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

        let written_b = compute(&ctx, &pid_b.to_string(), None).await.unwrap();
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

    #[tokio::test]
    async fn churn_rework_density_writes_real_zero() {
        // A project with REAL project files but ZERO rework patterns → a rework_density
        // row IS written with value 0.0 / numerator 0 (a real zero over a real
        // denominator), never a suppressed row.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        for f in ["a.rs", "b.rs", "c.rs"] {
            seed_file_node(pg, &fid, &format!("/_test/metrics-{uniq}/{f}")).await;
        }

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "one rework_density project row (real 0.0); no per-module rows (no folder has a rework signal)");

        let daily = daily_rows(pg, &pid).await;
        let rd = daily.iter().find(|r| r.0 == "rework_density").expect("rework_density row IS written for a REAL zero");
        assert!(rd.1.abs() < 1e-9, "value is a real 0.0 (0 rework over 3 real files), not a suppressed row");
        assert_eq!(rd.2["numerator"].as_i64(), Some(0), "numerator = 0 (no rework-flagged files)");
        assert_eq!(rd.2["denominator"].as_i64(), Some(3), "denominator = 3 project files (real denominator → row written)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_rework_density_historical_as_of_skips() {
        // Forward-only guard: `rework_density` is a snapshot of the CURRENT rework
        // signal vs current project files and cannot be reconstructed for a past day,
        // so a historical `as_of` (Some(D), D != today) writes NO rework_density row —
        // never a fabricated historical snapshot. Seeds ONLY rework data (a non-git
        // root, so no git churn) so the row count isolates rework_density.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        for f in ["a.rs", "b.rs", "c.rs", "d.rs"] {
            seed_file_node(pg, &fid, &format!("/_test/metrics-{uniq}/{f}")).await;
        }
        seed_detected_pattern(pg, &pid, Some(&fid), "rework: a.rs", true).await;
        seed_detected_pattern(pg, &pid, Some(&fid), "rework: b.rs", true).await;

        let past = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let written = compute(&ctx, &pid.to_string(), Some(past)).await.unwrap();
        assert_eq!(written, 0, "historical as_of → rework_density (forward-only) skipped, non-git root → no churn → zero rows");

        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no project_metrics rows for a historical as_of (never a fabricated snapshot)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
