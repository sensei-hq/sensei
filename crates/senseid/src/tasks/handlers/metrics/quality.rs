//! `quality` metric group computer (Phase 8).
//!
//! Supersedes the former own-graph `duplication` computer: instead of a snapshot
//! over the current symbol embeddings, the code-quality family is now sourced from
//! **`qlty` scans of a git worktree checked out to a PAST commit**, so it backfills
//! over real history. It follows the `churn` template (git-sourced, per-day, honest-
//! empty on non-git/missing-tool, project-level rows) rather than the forward-only
//! snapshot template.
//!
//! Two registry keys (`task_name = "quality"`), both project-level (`folder_id`
//! NULL), stamped `computed_on = the sampled commit-day`:
//! - `duplication_ratio` (ratio, ▼): DISTINCT duplicated source lines ÷ total source
//!   lines. `numerator` = the union of physical line ranges qlty flags as duplicated
//!   (`qlty smells` — `identical-code`/`similar-code`), de-duplicated per file so the
//!   ratio stays in `[0, 1]`; `denominator` = total physical source lines
//!   (`qlty metrics` TOTAL row). Repoints the catalog's `duplication_ratio` from the
//!   embedding graph to qlty over real history.
//! - `module_quality` (ratio, ▼): MAINTAINABILITY — qlty's non-duplication smell
//!   findings (file/function complexity, deep nesting, long parameter lists, …) ÷
//!   total source lines. A per-line smell burden; lower is better.
//!
//! ## Coverage is deliberately OUT of history scope (never fabricated)
//! qlty maintainability + duplication are computable from source alone, but coverage
//! needs a real coverage artifact (lcov) that a historical worktree does NOT carry.
//! We do NOT run tests per-worktree and do NOT synthesize a coverage number: this
//! computer emits ONLY `duplication_ratio` + `module_quality`, leaving `coverage`
//! honest-empty (no row) for history.
//!
//! ## Sampled cadence (bounded scan cost, not a silent cap)
//! A qlty scan of a worktree is expensive, so history is sampled at [`sample_commit_days`]
//! — one commit-day per ISO week (that week's FIRST commit-day, a stable anchor as
//! history grows) — NOT every commit-day. The planner's [`super::planner`] data-day
//! set is this sampled set, and `compute` re-derives it so a trailing-window refresh
//! only scans the sampled anchors; the sampled count + window are `log()`ged so the
//! cadence is visible, never a silent cap.
//!
//! ## Worktree hygiene (bounded, always cleaned up)
//! Each sampled commit is scanned in a detached `git worktree` under a temp path via
//! [`scan_at_commit`]; a [`WorktreeGuard`] `git worktree remove --force`s it on Drop —
//! even on error/panic — so no dangling worktree is ever left behind. A non-git
//! project, an absent `qlty` CLI, a commit predating qlty's config, or any scan
//! failure produces NO row (honest-empty) + a warn log, never a fabricated score.
//!
//! ## Capture-scope split (governance, #3)
//! Quality is GIT/source-derived (like churn), so it must NOT authorize the activity
//! pruner's capture-before-reclaim: the planner backfills it per sampled commit-day
//! ([`super::planner::DayKeyedGroup`]), but its registry `capture_source` is `git`,
//! so the pruner's guard (which authorizes reclaim only on `capture_source =
//! 'session'`) excludes it.
//!
//! Never-fabricate: every DB call propagates `Err`; a git/qlty miss or a scan with no
//! parseable total (0 source lines) writes NO row. A real total with 0
//! duplicates/smells writes a real `0.0` (row written, never suppressed).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use chrono::{Datelike, NaiveDate};

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (quality writes daily rows only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — quality is measured (by qlty), not estimated.
const SOURCE_MEASURED: &str = "measured";

/// The registry `key`s this computer produces.
const KEY_DUPLICATION_RATIO: &str = "duplication_ratio";
const KEY_MODULE_QUALITY: &str = "module_quality";

/// Human-readable cadence label for the sampling `log()` — one qlty scan per ISO
/// week (the week's first commit-day). Kept as a const so the cadence is named, not
/// a magic rule buried in [`sample_commit_days`].
const SAMPLE_CADENCE: &str = "weekly (first commit-day per ISO week)";

/// One qlty scan's raw signals for a single commit: total physical source lines
/// (the denominator), the distinct duplicated line count, and the maintainability
/// smell count. Pure data so [`compute_with_scanner`] can be exercised with a fake
/// scanner (no `git worktree` / `qlty` subprocess) while the parsers below are unit-
/// tested on captured real qlty output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct QualityScan {
    pub total_lines: i64,
    pub duplicated_lines: i64,
    pub maintainability_smells: i64,
}

// ── qlty subprocess + parsers (pure where it counts) ────────────────────────

/// Run `qlty` with `args` in `dir` and return stdout, or `None` when `qlty` is not
/// on `PATH`, the directory has no `.qlty` config, or the command otherwise fails —
/// an honest "no qlty data" (the caller writes no row), never a fabricated value.
/// `qlty` is an OPTIONAL tool (kept SOFT — not a hard bootstrap prereq): absent → the
/// metric is honest-empty and the daemon runs fine. Mirrors the shell-out-and-
/// tolerate-absence discipline of [`super::churn::run_git`]. Note `qlty smells`/`qlty
/// metrics` exit `0` even when they find smells, so a non-success status is a genuine
/// failure (missing config / missing tool), not "found findings".
fn run_qlty(dir: &Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("qlty")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Strip ANSI/SGR escape sequences from `qlty metrics`' colored table so the TOTAL
/// row is parseable. `qlty` emits `\x1b[…m` resets around every cell regardless of
/// `NO_COLOR`, so a strip is required; each sequence runs from `ESC` to its final
/// letter byte. Pure.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // Consume the escape sequence up to and including its final letter byte
            // (e.g. the `m` of an SGR `\x1b[0m`).
            for d in chars.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Total physical source lines from `qlty metrics --all`' TOTAL row — the shared
/// denominator for both quality ratios. Locates the `lines` column by its header
/// name (robust to column reordering) after [`strip_ansi`], then reads that column
/// of the `TOTAL` row. `None` when there is no parseable TOTAL row (an empty scan) —
/// the caller then writes no row (honest-empty). Pure over captured qlty output.
fn parse_metrics_total_lines(stdout: &str) -> Option<i64> {
    let clean = strip_ansi(stdout);
    let mut lines_col: Option<usize> = None;
    for line in clean.lines() {
        if !line.contains('|') {
            continue; // skip the `---+---` separator + any non-table chrome
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // The first `|`-bearing row is the header — find the `lines` column index.
        let Some(idx) = lines_col else {
            if let Some(i) = cells.iter().position(|c| *c == "lines") {
                lines_col = Some(i);
            }
            continue;
        };
        if cells.first() == Some(&"TOTAL") {
            return cells.get(idx).and_then(|v| v.parse::<i64>().ok());
        }
    }
    None
}

/// The duplication + maintainability signals parsed from `qlty smells --all --sarif`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SmellCounts {
    /// Count of DISTINCT physical source lines flagged as duplicated (union of the
    /// line ranges across every `identical-code`/`similar-code` finding, de-duplicated
    /// per file so the ratio stays in `[0, 1]`).
    duplicated_lines: i64,
    /// Count of NON-duplication smell findings (complexity / nesting / parameters /
    /// …) — the maintainability burden.
    maintainability_smells: i64,
}

/// Whether a SARIF result is a duplication finding — by its `duplication` taxon (the
/// robust signal) with a `ruleId` substring fallback (`identical`/`similar`/`duplicat`).
fn is_duplication(result: &serde_json::Value) -> bool {
    let by_taxon = result["taxa"]
        .as_array()
        .is_some_and(|taxa| taxa.iter().any(|t| t["id"].as_str() == Some("duplication")));
    by_taxon
        || result["ruleId"]
            .as_str()
            .is_some_and(|r| r.contains("identical") || r.contains("similar") || r.contains("duplicat"))
}

/// Add every physical line covered by `result`'s primary + related locations to
/// `per_file` (keyed by artifact uri), so overlapping ranges across findings are
/// counted once per line.
fn collect_dup_lines(result: &serde_json::Value, per_file: &mut HashMap<String, HashSet<i64>>) {
    let mut add = |loc: &serde_json::Value| {
        let pl = &loc["physicalLocation"];
        let region = &pl["region"];
        let (Some(uri), Some(s), Some(e)) = (
            pl["artifactLocation"]["uri"].as_str(),
            region["startLine"].as_i64(),
            region["endLine"].as_i64(),
        ) else {
            return;
        };
        if s > 0 && e >= s {
            let set = per_file.entry(uri.to_string()).or_default();
            for ln in s..=e {
                set.insert(ln);
            }
        }
    };
    if let Some(locs) = result["locations"].as_array() {
        locs.iter().for_each(&mut add);
    }
    if let Some(locs) = result["relatedLocations"].as_array() {
        locs.iter().for_each(&mut add);
    }
}

/// Parse `qlty smells --all --sarif` into [`SmellCounts`]. A missing `results` array
/// is an honest zero (a clean scan), not an error; malformed JSON propagates `Err`
/// (a scan that ran but produced garbage is a genuine failure, not honest-empty).
/// Pure over captured qlty SARIF.
fn parse_smells(json: &str) -> Result<SmellCounts, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("quality: bad qlty SARIF: {e}"))?;
    let Some(results) = v["runs"][0]["results"].as_array() else {
        return Ok(SmellCounts::default()); // no run/results → clean scan (0/0)
    };
    let mut per_file: HashMap<String, HashSet<i64>> = HashMap::new();
    let mut maintainability_smells = 0i64;
    for result in results {
        if is_duplication(result) {
            collect_dup_lines(result, &mut per_file);
        } else {
            maintainability_smells += 1;
        }
    }
    let duplicated_lines = per_file.values().map(|s| s.len() as i64).sum();
    Ok(SmellCounts { duplicated_lines, maintainability_smells })
}

/// Run the two `qlty` scans in the worktree `wt` and assemble a [`QualityScan`].
/// `qlty` absent / no `.qlty` config / no parseable TOTAL → `Ok(None)` (honest-empty);
/// a scan that ran but produced unparseable SARIF → `Err` (genuine failure → retry).
fn run_qlty_scan(wt: &Path) -> Result<Option<QualityScan>, String> {
    let Some(metrics) = run_qlty(wt, &["metrics", "--all", "--quiet", "--no-upgrade-check"]) else {
        return Ok(None); // qlty missing or no config → honest-empty
    };
    let Some(total_lines) = parse_metrics_total_lines(&metrics) else {
        return Ok(None); // no TOTAL row (empty scan) → no denominator → honest-empty
    };
    let Some(sarif) = run_qlty(wt, &["smells", "--all", "--sarif", "--no-upgrade-check"]) else {
        return Ok(None);
    };
    let smells = parse_smells(&sarif)?;
    Ok(Some(QualityScan {
        total_lines,
        duplicated_lines: smells.duplicated_lines,
        maintainability_smells: smells.maintainability_smells,
    }))
}

// ── git worktree lifecycle ──────────────────────────────────────────────────

/// RAII guard that `git worktree remove --force`s the scanned worktree AND removes
/// its temp parent dir on Drop — so a worktree is ALWAYS torn down, even if the scan
/// errors or panics (worktree hygiene, #3). A remove failure is logged + best-effort
/// `worktree prune`d, never fatal (leaving the daemon otherwise healthy).
struct WorktreeGuard {
    root: String,
    path: String,
    base: std::path::PathBuf,
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        if super::churn::run_git(&self.root, &["worktree", "remove", "--force", &self.path])
            .is_none()
        {
            tracing::warn!(path = %self.path, "quality: git worktree remove failed — pruning");
            let _ = super::churn::run_git(&self.root, &["worktree", "prune"]);
        }
        let _ = std::fs::remove_dir_all(&self.base); // drop the temp parent dir too
    }
}

/// Check out `sha` into a detached temp worktree, run `scan` in it, and ALWAYS remove
/// the worktree afterwards (via [`WorktreeGuard`]). A failed `worktree add` yields
/// `Ok(None)` + warn (honest-empty, never fabricated). `scan` is injected so the
/// DB/as_of path can be tested with a fake and the real path uses [`run_qlty_scan`].
/// The worktree lives under a unique `sensei-qlty-<uuid>` temp dir (git does not
/// create missing parents, so the base is created first and torn down by the guard).
fn scan_at_commit<F>(root: &str, sha: &str, scan: F) -> Result<Option<QualityScan>, String>
where
    F: FnOnce(&Path) -> Result<Option<QualityScan>, String>,
{
    let base = std::env::temp_dir().join(format!("sensei-qlty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base).map_err(|e| format!("quality: worktree base dir: {e}"))?;
    let wt = base.join("wt");
    let wt_str = wt.to_string_lossy().into_owned();
    if super::churn::run_git(root, &["worktree", "add", "--detach", "-q", &wt_str, sha]).is_none() {
        tracing::warn!(root, sha, "quality: git worktree add failed — no quality row");
        let _ = std::fs::remove_dir_all(&base);
        return Ok(None);
    }
    let _guard = WorktreeGuard { root: root.to_string(), path: wt_str, base };
    scan(&wt)
    // `_guard` drops here → `worktree remove --force` + temp parent dir removed.
}

/// The last commit on or before end-of-`day` on the current branch (`git rev-list -1
/// --first-parent --before=<day 23:59:59> HEAD`). `None` when `day` predates the
/// repo's first commit, or git is unavailable — the caller then writes no row.
fn resolve_commit_as_of(root: &str, day: NaiveDate) -> Option<String> {
    let until = format!("{} 23:59:59", day.format("%Y-%m-%d"));
    let out = super::churn::run_git(
        root,
        &["rev-list", "-1", "--first-parent", "--before", &until, "HEAD"],
    )?;
    let sha = out.trim();
    (!sha.is_empty()).then(|| sha.to_string())
}

// ── sampling ────────────────────────────────────────────────────────────────

/// Sample `commit_days` to one anchor per ISO week — that week's FIRST (earliest)
/// commit-day. A stable anchor: adding a later commit in the same week never moves
/// it, so a settled week's snapshot is not re-scanned. `commit_days` need not be
/// sorted; the result is sorted ascending + de-duplicated. Pure — the cadence rule
/// lives in exactly one place, shared by the planner's data-day set and `compute`.
pub(super) fn sample_commit_days(commit_days: &[NaiveDate]) -> Vec<NaiveDate> {
    // ISO (year, week) → the earliest day seen in that week.
    let mut first_of_week: HashMap<(i32, u32), NaiveDate> = HashMap::new();
    for &d in commit_days {
        let iso = d.iso_week();
        let key = (iso.year(), iso.week());
        first_of_week
            .entry(key)
            .and_modify(|cur| {
                if d < *cur {
                    *cur = d;
                }
            })
            .or_insert(d);
    }
    let mut out: Vec<NaiveDate> = first_of_week.into_values().collect();
    out.sort_unstable();
    out
}

// ── compute ─────────────────────────────────────────────────────────────────

/// Compute the `quality` group for one project from `qlty` scans of git worktrees at
/// sampled past commits. `project_raw` is the project uuid carried in
/// `task.folder_path`. `as_of`:
/// - `Some(D)` — scan ONLY the sampled commit-day `D` (`computed_on = D`, the
///   backfill/gap-fill + trailing-window path). A `D` that is not a sampled anchor is
///   a cheap no-op (no worktree/scan) so the planner's trailing-window refresh does
///   not force a scan on every calendar day.
/// - `None` — the sampled anchors within the trailing [`metrics.window_days`] window.
///
/// Returns the number of `project_metrics` rows written (`0` = honest-empty: non-git
/// project, absent qlty, no commit as-of the day, a scan miss, or none of the group's
/// metrics active). Idempotent — re-running backfills in place via the upsert
/// identity. A past day already covered by every active quality metric is skipped (a
/// historical commit's code is immutable, so its scan can't change) — only `today` is
/// always re-scanned.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<NaiveDate>,
) -> Result<u32, String> {
    compute_with_scanner(ctx, project_raw, as_of, |root, sha| {
        scan_at_commit(root, sha, run_qlty_scan)
    })
    .await
}

/// [`compute`] with the scan injected — the real path passes the `git worktree` +
/// `qlty` scanner; tests pass a fake so the sampling/as_of/upsert logic is exercised
/// without a qlty subprocess.
async fn compute_with_scanner<S>(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<NaiveDate>,
    scan: S,
) -> Result<u32, String>
where
    S: Fn(&str, &str) -> Result<Option<QualityScan>, String>,
{
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("quality: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Resolve key → metric_id for this group's ACTIVE metrics. Absent key = inactive
    // (retired / not-yet-effective / unseeded) → skipped: never a value for an
    // inactive metric. No active quality metric → nothing to write.
    let ids = pg.active_metric_ids(MetricGroup::Quality.as_str()).await?;
    let dup_id = ids.get(KEY_DUPLICATION_RATIO).copied();
    let mq_id = ids.get(KEY_MODULE_QUALITY).copied();
    if dup_id.is_none() && mq_id.is_none() {
        return Ok(0);
    }
    let active_ids: Vec<uuid::Uuid> = [dup_id, mq_id].into_iter().flatten().collect();

    // The project's git-root working dir (shortest repo-root abs_path). No repo-root
    // folder / a non-git root → no git commit history → honest-empty (no rows).
    let Some(root) = pg.project_root_path(&project_id).await? else {
        return Ok(0);
    };
    let today = super::today(pg).await?;

    // The sampled anchor set (one commit-day per ISO week) — the SAME rule the planner
    // uses for the data-day set, so a planned day is a real anchor.
    let sampled = sample_commit_days(&super::churn::git_commit_days(&root));
    let sampled_set: HashSet<NaiveDate> = sampled.iter().copied().collect();

    // The target day-set: the single `as_of` day, else the sampled anchors inside the
    // trailing window (the today-incremental path).
    let target_days: Vec<NaiveDate> = match as_of {
        Some(d) => vec![d],
        None => {
            let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;
            let lo = today - chrono::Duration::days((window_days.max(1) - 1) as i64);
            sampled.iter().copied().filter(|d| *d >= lo && *d <= today).collect()
        }
    };

    tracing::info!(
        project = %project_raw,
        cadence = SAMPLE_CADENCE,
        sampled_anchors = sampled.len(),
        targets = target_days.len(),
        "quality: sampling git history for qlty scans",
    );

    let mut written = 0u32;
    for day in target_days {
        // Only sampled anchors are scanned; a non-anchor day (e.g. a trailing-window
        // calendar day with no anchor) is a cheap no-op — never a scan.
        if !sampled_set.contains(&day) {
            continue;
        }
        // A past day already covered by EVERY active quality metric is settled: a
        // historical commit's code is immutable, so re-scanning can't change it. Only
        // `today` (whose HEAD may advance) is always re-scanned.
        if day < today && day_fully_covered(pg, &active_ids, &project_id, day).await? {
            continue;
        }
        let Some(sha) = resolve_commit_as_of(&root, day) else {
            continue; // no commit on/before this day → honest-empty
        };
        let Some(qs) = scan(&root, &sha)? else {
            continue; // qlty absent / no config / scan miss → honest-empty
        };
        if qs.total_lines <= 0 {
            continue; // no source lines → no denominator → never a fabricated 0/0
        }

        // duplication_ratio: distinct duplicated lines ÷ total source lines.
        if let Some(mid) = dup_id {
            let value = qs.duplicated_lines as f64 / qs.total_lines as f64;
            let props = serde_json::json!({
                "numerator": qs.duplicated_lines,
                "denominator": qs.total_lines,
                "commit": sha,
            });
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }

        // module_quality (maintainability): non-duplication smells ÷ total source lines.
        if let Some(mid) = mq_id {
            let value = qs.maintainability_smells as f64 / qs.total_lines as f64;
            let props = serde_json::json!({
                "numerator": qs.maintainability_smells,
                "denominator": qs.total_lines,
                "commit": sha,
            });
            pg.upsert_project_metric(
                &mid, &project_id, None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    Ok(written)
}

/// Whether `day` already has a project-level daily row for EVERY id in `metric_ids`
/// — so a settled past day is skipped only when it is fully captured (a metric newly
/// activated after an earlier scan still backfills). Empty `metric_ids` → trivially
/// not covered.
async fn day_fully_covered(
    pg: &PgStore,
    metric_ids: &[uuid::Uuid],
    project_id: &uuid::Uuid,
    day: NaiveDate,
) -> Result<bool, String> {
    if metric_ids.is_empty() {
        return Ok(false);
    }
    let (present,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(DISTINCT metric_id)
           FROM sensei.project_metrics
          WHERE project_id = $1
            AND metric_id = ANY($2)
            AND folder_id IS NULL
            AND grain = 'daily'
            AND computed_on = $3",
    )
    .bind(project_id)
    .bind(metric_ids)
    .bind(day)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok(present as usize == metric_ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, git_commit_on_day,
        make_ctx, seed_git_project_folder, seed_metrics_project_folder,
    };
    use sqlx_core::query_as::query_as;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── Pure: qlty metrics TOTAL parser (ANSI-stripped) ──────────────────────

    #[test]
    fn parse_metrics_total_lines_reads_the_lines_column_of_the_total_row() {
        // A captured-shape `qlty metrics` table: SGR `\x1b[0m` resets around cells
        // (qlty emits them even with NO_COLOR), a `---+---` separator, one file row,
        // and the TOTAL row. The parser strips ANSI, finds the `lines` column by its
        // header name, and reads it off TOTAL — here 374 (NOT LOC=331).
        let e = "\u{1b}[0m";
        let sample = format!(
            "{e} name {e}|{e} classes | funcs | fields | cyclo | complex | LCOM |{e} lines {e}| LOC \n\
             ------+------+------+------+------+------+------+------+------\n\
             {e} lib.rs {e}| 0 | 7 | 0 | 2 | 1 | 0 | 81 | 72 \n\
             {e} TOTAL {e}| 5 | 26 | 15 | 29 | 14 | 1 |{e} 374 {e}| 331 \n"
        );
        assert_eq!(
            parse_metrics_total_lines(&sample),
            Some(374),
            "total physical `lines` (col 7), ANSI stripped, from the TOTAL row — not LOC",
        );
        // No TOTAL row → None (an empty scan writes no row, never a fabricated 0).
        assert_eq!(parse_metrics_total_lines("no table here"), None);
    }

    // ── Pure: qlty smells SARIF parser ───────────────────────────────────────

    #[test]
    fn parse_smells_counts_distinct_dup_lines_and_maintainability_findings() {
        // Captured-shape SARIF: ONE duplication finding spanning three 23-line blocks
        // in three DIFFERENT files (523..=545, 312..=334, 360..=382 → 3×23 = 69
        // distinct duplicated lines), plus TWO non-duplication (maintainability)
        // findings. duplicated_lines = 69 (deduped per file); maintainability = 2.
        let sarif = r#"{
          "runs": [{ "tool": {"driver": {"name": "qlty"}}, "results": [
            {
              "ruleId": "qlty:similar-code",
              "message": {"text": "Found 23 lines of similar code in 3 locations (mass = 124)"},
              "locations": [{"physicalLocation": {"artifactLocation": {"uri": "a.rs"}, "region": {"startLine": 523, "endLine": 545}}}],
              "relatedLocations": [
                {"physicalLocation": {"artifactLocation": {"uri": "b.rs"}, "region": {"startLine": 312, "endLine": 334}}},
                {"physicalLocation": {"artifactLocation": {"uri": "c.rs"}, "region": {"startLine": 360, "endLine": 382}}}
              ],
              "taxa": [{"id": "duplication", "name": "duplication"}]
            },
            {"ruleId": "qlty:function-complexity", "message": {"text": "High complexity (count = 33)"},
             "locations": [{"physicalLocation": {"artifactLocation": {"uri": "a.rs"}, "region": {"startLine": 70, "endLine": 95}}}]},
            {"ruleId": "qlty:nested-control-flow", "message": {"text": "Deeply nested control flow (level = 5)"},
             "locations": [{"physicalLocation": {"artifactLocation": {"uri": "a.rs"}, "region": {"startLine": 12, "endLine": 20}}}]}
          ]}]
        }"#;
        let counts = parse_smells(sarif).unwrap();
        assert_eq!(counts.duplicated_lines, 69, "3 distinct 23-line blocks across 3 files = 69");
        assert_eq!(counts.maintainability_smells, 2, "the 2 non-duplication findings");
    }

    #[test]
    fn parse_smells_dedupes_overlapping_ranges_within_a_file() {
        // Two duplication findings whose ranges OVERLAP within the same file
        // (1..=10 and 5..=14 → union 1..=14 = 14 distinct lines, NOT 10+10=20). Proves
        // the ratio numerator can't exceed the file's real duplicated-line surface.
        let sarif = r#"{"runs": [{"results": [
          {"ruleId": "qlty:identical-code", "taxa": [{"id": "duplication"}],
           "locations": [{"physicalLocation": {"artifactLocation": {"uri": "x.rs"}, "region": {"startLine": 1, "endLine": 10}}}]},
          {"ruleId": "qlty:identical-code", "taxa": [{"id": "duplication"}],
           "locations": [{"physicalLocation": {"artifactLocation": {"uri": "x.rs"}, "region": {"startLine": 5, "endLine": 14}}}]}
        ]}]}"#;
        let counts = parse_smells(sarif).unwrap();
        assert_eq!(counts.duplicated_lines, 14, "union of 1..=10 and 5..=14 = 14 distinct lines (deduped)");
        assert_eq!(counts.maintainability_smells, 0);
    }

    #[test]
    fn parse_smells_empty_results_is_a_clean_zero_and_bad_json_is_err() {
        // A clean scan (no results) is an honest 0/0 — never an error.
        let clean = parse_smells(r#"{"runs": [{"results": []}]}"#).unwrap();
        assert_eq!(clean, SmellCounts::default());
        // Malformed JSON propagates Err (a scan that ran but produced garbage is a
        // genuine failure → retry, never masked into a fabricated 0).
        assert!(parse_smells("not json").is_err());
    }

    // ── Pure: sampling cadence ───────────────────────────────────────────────

    #[test]
    fn sample_commit_days_keeps_one_first_anchor_per_iso_week() {
        // Mon 2025-06-02 .. Sun 2025-06-08 is ONE ISO week; 06-02 and 06-05 collapse to
        // the earliest (06-02). 06-09 starts the next week → its own anchor. 06-16
        // another week. Unsorted input; sorted, de-duplicated anchors out.
        let days = [d(2025, 6, 5), d(2025, 6, 2), d(2025, 6, 16), d(2025, 6, 9), d(2025, 6, 5)];
        assert_eq!(
            sample_commit_days(&days),
            vec![d(2025, 6, 2), d(2025, 6, 9), d(2025, 6, 16)],
            "one anchor per ISO week (the week's first commit-day), sorted + unique",
        );
        assert!(sample_commit_days(&[]).is_empty(), "no commits → no anchors (honest-empty)");
    }

    // ── DB-backed: single-day as_of over a seeded git repo (injected scanner) ──

    #[tokio::test]
    async fn quality_single_day_as_of_upserts_both_ratios_stamped_on_the_commit_day() {
        // THE history test: a commit on a historical day D (a sampled anchor) → a
        // qlty scan (injected: 40 duplicated / 20 maintainability smells over 200
        // source lines) → duplication_ratio 40/200=0.2 + module_quality 20/200=0.1,
        // both stamped computed_on = D. The scanner is faked so the sampling/as_of/
        // upsert path is tested without a real qlty subprocess.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;

        let dday = (chrono::Utc::now() - chrono::Duration::days(60)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);

        let fake = |_root: &str, _sha: &str| {
            Ok(Some(QualityScan { total_lines: 200, duplicated_lines: 40, maintainability_smells: 20 }))
        };

        // Incremental (as_of=None): D is 60 days ago, outside the rolling window → NO
        // rows (honest-empty for the recent window, never a fabricated backfill).
        let incr = compute_with_scanner(&ctx, &pid.to_string(), None, fake).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old anchor is outside the rolling window → no incremental rows");

        // Backfill (as_of=Some(D)): scans exactly that day's commit.
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), fake).await.unwrap();
        assert_eq!(written, 2, "duplication_ratio + module_quality for day D");

        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio row");
        assert!((dr.1 - 0.2).abs() < 1e-9, "duplication_ratio = 40/200 = 0.2");
        assert_eq!(dr.2["numerator"].as_i64(), Some(40), "numerator = distinct duplicated lines");
        assert_eq!(dr.2["denominator"].as_i64(), Some(200), "denominator = total source lines");
        let mq = daily.iter().find(|r| r.0 == "module_quality").expect("module_quality row");
        assert!((mq.1 - 0.1).abs() < 1e-9, "module_quality = 20/200 = 0.1");
        assert_eq!(mq.2["numerator"].as_i64(), Some(20), "numerator = maintainability smells");
        assert_eq!(mq.2["denominator"].as_i64(), Some(200), "denominator = total source lines");

        // computed_on stamped to the true commit day D.
        let (on_d,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND m.key = 'duplication_ratio' AND pm.computed_on = $2",
        )
        .bind(pid).bind(dday).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(on_d, 1, "the duplication_ratio row is stamped computed_on = the commit's day D");

        // Re-running the now-covered PAST day is a bounded no-op: the covered-skip
        // fires (a historical commit is immutable), so nothing is re-scanned or
        // re-written and the rows are unchanged — never duplicated.
        let again = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), fake).await.unwrap();
        assert_eq!(again, 0, "a covered past day is skipped on re-run (immutable history)");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total, 2, "still exactly 2 rows — the re-run wrote nothing, never a duplicate");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_real_zero_writes_rows() {
        // A real scan with 0 duplicates AND 0 smells over real source lines → BOTH
        // rows ARE written with value 0.0 (a real zero over a real denominator), never
        // suppressed.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(30)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n")]);

        let clean = |_r: &str, _s: &str| {
            Ok(Some(QualityScan { total_lines: 120, duplicated_lines: 0, maintainability_smells: 0 }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), clean).await.unwrap();
        assert_eq!(written, 2, "a real 0.0 over a real denominator is still written");
        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio row");
        assert!(dr.1.abs() < 1e-9, "value is a real 0.0, not a suppressed row");
        assert_eq!(dr.2["denominator"].as_i64(), Some(120), "real denominator → row written");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_zero_total_lines_writes_no_row() {
        // Never-fabricate: a scan reporting 0 total source lines has no denominator →
        // NO row (a 0/0 would be a fabricated zero).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(20)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n")]);

        let empty = |_r: &str, _s: &str| {
            Ok(Some(QualityScan { total_lines: 0, duplicated_lines: 0, maintainability_smells: 0 }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), empty).await.unwrap();
        assert_eq!(written, 0, "0 total lines → no denominator → no row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total, 0, "no rows for a zero-line scan (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_covered_past_day_is_skipped_but_scanner_untouched() {
        // Bounded work: a PAST day already covered by BOTH active quality metrics is
        // settled (a historical commit is immutable) → the scanner is NOT applied and
        // the stored value is untouched. A scanner that returns a DIFFERENT value
        // proves the skip: the pre-covered 0.5 must remain, not the scanner's 0.9.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(45)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);

        // Pre-cover D with BOTH metrics at value 0.5.
        let ids = pg.active_metric_ids("quality").await.unwrap();
        for key in ["duplication_ratio", "module_quality"] {
            let mid = *ids.get(key).expect("active quality metric");
            pg.upsert_project_metric(&mid, &pid, None, None, dday, "daily", 0.5,
                &serde_json::json!({"numerator": 1, "denominator": 2}), "measured").await.unwrap();
        }

        let different = |_r: &str, _s: &str| {
            Ok(Some(QualityScan { total_lines: 100, duplicated_lines: 90, maintainability_smells: 90 }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), different).await.unwrap();
        assert_eq!(written, 0, "the covered past day is skipped (immutable history) — scanner not applied");

        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("duplication_ratio row");
        assert!((dr.1 - 0.5).abs() < 1e-9, "the pre-covered 0.5 remains — the skip fired (not the scanner's 0.9)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── Honest-empty: non-git / real qlty path with no config ────────────────

    #[tokio::test]
    async fn quality_non_git_project_writes_no_rows() {
        // A project whose repo-root folder is NOT a git repo (the synthetic
        // `/_test/metrics-*` path) → no git commit history → NO rows (honest-empty).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid) = seed_metrics_project_folder(pg, &uniq).await;
        let panic_scan = |_r: &str, _s: &str| -> Result<Option<QualityScan>, String> {
            panic!("scanner must not run for a non-git project")
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), None, panic_scan).await.unwrap();
        assert_eq!(written, 0, "non-git root → no commit-days → no rows (scanner never called)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_real_scan_without_qlty_config_is_honest_empty() {
        // End-to-end REAL path (worktree + real `run_qlty_scan`): the seeded temp repo
        // has NO `.qlty` config committed (and qlty may be absent on CI), so the scan
        // misses → NO rows (honest-empty, never a fabricated score). Also exercises the
        // worktree add/remove lifecycle: no dangling worktree is left behind.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(10)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);

        let written = compute(&ctx, &pid.to_string(), Some(dday)).await.unwrap();
        assert_eq!(written, 0, "no .qlty config (or no qlty CLI) → scan miss → honest-empty");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total, 0, "no rows for a scan miss (never fabricated)");

        // Worktree hygiene: only the main worktree remains (the scan's temp worktree
        // was force-removed by the guard even though the scan missed).
        let list = super::super::churn::run_git(&repo.path().to_string_lossy(), &["worktree", "list"])
            .expect("git worktree list");
        assert_eq!(list.lines().count(), 1, "no dangling worktree left behind (guard removed it)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn scan_at_commit_removes_the_worktree_even_when_the_scan_errors() {
        // Worktree hygiene (#3): scan_at_commit ALWAYS tears the worktree down — even
        // when the injected scan returns Err — so nothing dangles. Uses a real repo +
        // a scan closure that fails.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = chrono::Utc::now().date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n")]);
        let root = repo.path().to_string_lossy().to_string();
        let sha = resolve_commit_as_of(&root, dday).expect("a commit exists today");

        let err_scan = |_wt: &Path| -> Result<Option<QualityScan>, String> { Err("boom".into()) };
        let res = scan_at_commit(&root, &sha, err_scan);
        assert!(res.is_err(), "the scan error propagates");

        let list = super::super::churn::run_git(&root, &["worktree", "list"]).expect("git worktree list");
        assert_eq!(list.lines().count(), 1, "the failing scan's worktree was still removed (no dangle)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
