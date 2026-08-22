//! `coverage` metric group computer.
//!
//! Test coverage, sourced by INGESTING an lcov report the project's own test run /
//! CI already produced — the daemon NEVER runs the project's tests (that decision is
//! Jerry's, 2026-08-19). A whole-suite property (never per-author), so — following the
//! `knowledge` convention for a non-author metric — it writes `scope = user` with
//! `identity = NULL`: `scope = user` is the value the DEFAULT project read surfaces
//! (the pooled `project_metric_daily` view filters `scope = 'user'`), and a NULL
//! identity keeps it a single value, not an author split. One row PER repository the
//! project spans, so the pooled project view sums `Σ hit / Σ found` across them.
//!
//! ## Forward-only snapshot (the default, cheap path)
//! Coverage is a SNAPSHOT of the CURRENT checkout: for each repository the project
//! spans ([`PgStore::repository_roots_for_project`]) it reads an lcov report at a
//! known/config path under that checkout, sums `LH` (lines hit) / `LF` (lines found)
//! across all records, and writes ONE `coverage` row per repository
//! (`ratio = LH/LF`, `computed_on = today`, `commit_sha`/`identity`/`folder_id`/
//! `session_id` NULL). No report present, or 0 instrumented lines → NO row (honest-
//! empty — never a fabricated 0). Because it reflects CURRENT state, it is a
//! forward-only snapshot: a historical `as_of` writes NO row (see
//! [`super::is_historical`]) — historical coverage is the OPT-IN backfill (below),
//! not this path.
//!
//! ## Backfill is opt-in (configured or explicitly requested)
//! You CAN reconstruct historical coverage — checkout a past commit, run the repo's
//! coverage command, ingest the produced lcov — but that RUNS the test suite, so it is
//! gated (config `metrics.coverage_command` + an explicit request), never automatic.
//! That path lives in [`backfill`] and reuses the `quality` worktree machinery; the
//! default scheduler wave only runs the forward snapshot here.
//!
//! ## lcov source resolution
//! The lcov path is one of [`DEFAULT_LCOV_CANDIDATES`] (common locations across
//! ecosystems) under the checkout root — the FIRST that exists wins — OR an explicit
//! `metrics.coverage_lcov` config override (comma-separated relative-or-absolute
//! paths, tried in order). A repo whose test run has not produced a report yet simply
//! has no coverage row until it does (honest-empty, never fabricated).
//!
//! Never-fabricate: every DB call propagates `Err`; a missing/empty/unparseable lcov
//! (0 lines found) writes NO row; a real report with 0 lines hit writes a real `0.0`
//! (a measured zero — the suite is instrumented but nothing is covered — never
//! suppressed).

use std::path::{Path, PathBuf};

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (coverage writes a daily snapshot row only).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — coverage is measured (from a real report).
const SOURCE_MEASURED: &str = "measured";
/// `sensei.metric_scope` text value — coverage is a whole-suite property with NO
/// author dimension, so (like `knowledge`) it writes only the `scope = user` value
/// (identity NULL) that the DEFAULT pooled project read surfaces; there is no separate
/// author-filtered twin.
const SCOPE_USER: &str = "user";

/// The registry `key` this computer produces.
const KEY_COVERAGE: &str = "coverage";

/// Config key: a comma-separated list of lcov paths (relative to the checkout root, or
/// absolute) to try in order. Empty/unset → [`DEFAULT_LCOV_CANDIDATES`].
const LCOV_PATH_KEY: &str = "metrics.coverage_lcov";

/// Common lcov report locations across ecosystems, tried in order under each checkout
/// root. The FIRST that exists wins (a single merged report, not a glob-merge). A
/// project whose tool writes elsewhere points at it via [`LCOV_PATH_KEY`].
const DEFAULT_LCOV_CANDIDATES: &[&str] = &[
    "lcov.info",
    "coverage/lcov.info",
    "coverage/lcov-report/lcov.info",
    "target/coverage/lcov.info",
    "target/llvm-cov/lcov.info",
    "target/nextest/coverage/lcov.info",
    ".coverage/lcov.info",
];

/// The ordered lcov candidate paths: the [`LCOV_PATH_KEY`] override (comma-separated,
/// trimmed, non-empty entries) if set, else [`DEFAULT_LCOV_CANDIDATES`]. Read-error →
/// the defaults (logged), never a hard failure (coverage is a soft, ingest-only metric).
pub(super) async fn lcov_candidates(pg: &PgStore) -> Vec<String> {
    let raw = match pg.get_config(LCOV_PATH_KEY).await {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!(error = %e, "coverage: lcov-path config read failed — using defaults");
            None
        }
    };
    let overrides: Vec<String> = raw
        .into_iter()
        .flat_map(|s| s.split(',').map(|p| p.trim().to_string()).collect::<Vec<_>>())
        .filter(|p| !p.is_empty())
        .collect();
    if overrides.is_empty() {
        DEFAULT_LCOV_CANDIDATES.iter().map(|s| s.to_string()).collect()
    } else {
        overrides
    }
}

/// Sum `(lines_hit, lines_found)` across every record in an lcov report. Prefers the
/// per-record `LH:` / `LF:` summary lines (what genhtml/lcov/most tools emit); if a
/// report carries no `LF` at all (a minimal writer), falls back to counting `DA:` line
/// records (`DA:<line>,<hits>` — instrumented = every `DA`, hit = `DA` with `hits > 0`).
/// Pure so it is unit-tested on captured real lcov text.
pub(super) fn parse_lcov(report: &str) -> (i64, i64) {
    let (mut lh, mut lf) = (0i64, 0i64);
    let (mut da_total, mut da_hit) = (0i64, 0i64);
    for line in report.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("LH:") {
            lh += rest.trim().parse::<i64>().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("LF:") {
            lf += rest.trim().parse::<i64>().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("DA:") {
            // `DA:<line_number>,<execution_count>[,<checksum>]`
            da_total += 1;
            if let Some(count) = rest.split(',').nth(1)
                && count.trim().parse::<i64>().unwrap_or(0) > 0
            {
                da_hit += 1;
            }
        }
    }
    if lf > 0 {
        (lh, lf)
    } else {
        // No LF summary lines → derive from the DA records (0,0 if neither present).
        (da_hit, da_total)
    }
}

/// The first existing lcov report under `root` among `candidates` (relative paths are
/// joined to `root`; absolute paths are used as-is). `None` when no candidate exists —
/// the repo's test run has not produced a report, so coverage is honest-empty for it.
fn find_lcov(root: &Path, candidates: &[String]) -> Option<PathBuf> {
    candidates.iter().find_map(|c| {
        let p = Path::new(c);
        let full = if p.is_absolute() { p.to_path_buf() } else { root.join(p) };
        full.is_file().then_some(full)
    })
}

/// `(lines_hit, lines_found)` for the checkout at `root`, or `None` when no lcov report
/// exists / it cannot be read. A found report that cannot be read is an honest miss
/// (logged), never a fabricated value.
fn read_coverage(root: &str, candidates: &[String]) -> Option<(i64, i64)> {
    let path = find_lcov(Path::new(root), candidates)?;
    match std::fs::read_to_string(&path) {
        Ok(text) => Some(parse_lcov(&text)),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "coverage: lcov report unreadable — honest-empty");
            None
        }
    }
}

/// Compute the `coverage` group for one project as a snapshot as of today: ingest each
/// repository's current lcov report and write one `scope = repo` row per repository that
/// has one. `project_raw` is the project uuid carried in `task.folder_path`. Returns the
/// number of `project_metrics` rows written (`0` = honest-empty: no repository has a
/// readable report with instrumented lines, or the metric is inactive). Idempotent —
/// re-running upserts each row in place. FORWARD-ONLY (Phase 3): a historical `as_of`
/// (`Some(D)`, `D != today`) writes NO row here — historical coverage is the opt-in
/// [`backfill`], not this snapshot path.
pub(super) async fn compute(
    ctx: &TaskContext,
    project_raw: &str,
    as_of: Option<chrono::NaiveDate>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("coverage: bad project id {project_raw:?}: {e}"))?;
    let pg = ctx.pg();

    // Forward-only: an old checkout's report is not the CURRENT tree's coverage, so a
    // historical target day writes no row (that is the opt-in backfill's job).
    if super::is_historical(pg, as_of).await? {
        return Ok(0);
    }

    // Resolve key → metric_id; an inactive (retired / not-yet-effective / unseeded)
    // coverage metric writes nothing.
    let ids = pg.active_metric_ids(MetricGroup::Coverage.as_str()).await?;
    let Some(mid) = ids.get(KEY_COVERAGE).copied() else {
        return Ok(0);
    };

    let candidates = lcov_candidates(pg).await;
    let day = super::today(pg).await?;
    let mut written = 0u32;

    // One row per repository the project spans that has a readable report with real
    // instrumented lines. scope=user (identity NULL) so it pools into the default read.
    for (repository_id, abs_path) in pg.repository_roots_for_project(&project_id).await? {
        let Some((hit, found)) = read_coverage(&abs_path, &candidates) else {
            continue; // no report for this checkout → honest-empty
        };
        if found <= 0 {
            continue; // 0 instrumented lines → no denominator → no row (never a 0/0)
        }
        // found >= 1 here: 0 lines hit writes a REAL 0.0 (the suite is instrumented but
        // nothing is covered), never suppressed.
        let value = hit as f64 / found as f64;
        let props = serde_json::json!({ "numerator": hit, "denominator": found });
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
        written += 1;
    }

    Ok(written)
}

/// Config key: the shell command that produces an lcov report when run in a checkout
/// (e.g. `cargo llvm-cov --lcov --output-path lcov.info`, `vitest run --coverage`).
/// Empty/unset → backfill is DISABLED — the daemon never runs your tests unless you
/// explicitly configure this.
const COVERAGE_COMMAND_KEY: &str = "metrics.coverage_command";

/// Run the configured coverage command in the worktree `wt` (`sh -c <command>`, cwd
/// = `wt`). Ok on a zero exit; Err otherwise (the caller writes no row for that
/// commit). Runs an ARBITRARY user-configured command — only ever reached through the
/// opt-in [`backfill`], never the default snapshot wave.
fn run_coverage_command(wt: &Path, command: &str) -> Result<(), String> {
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(wt)
        .status()
        .map_err(|e| format!("coverage: spawn coverage command: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("coverage: command exited non-zero ({status}) in {}", wt.display()))
    }
}

/// OPT-IN historical backfill (configured or explicitly requested — NEVER the default
/// wave): reconstruct past coverage by checking out sampled commits, running the
/// configured `metrics.coverage_command` in each worktree (reusing the `quality`
/// worktree machinery), and ingesting the lcov it produces. Writes one row per
/// `(repository, sampled commit)` stamped `computed_on = commit-day`, `commit_sha =
/// sha` (a commit-cadence historical point distinct from the forward snapshot's
/// `commit_sha = NULL` row). DISABLED (`Ok(0)`, logged) unless the command is
/// configured — because it RUNS the project's test suite. `weeks` bounds how many of
/// the most-recent sampled ISO-week anchors to walk (`None` = all). Returns the number
/// of rows written. Never fabricates: a commit that predates the repo, a failed
/// command, or an unproduced/empty lcov writes NO row for that commit.
pub(crate) async fn backfill(
    pg: &PgStore,
    project_raw: &str,
    weeks: Option<u32>,
) -> Result<u32, String> {
    let project_id = uuid::Uuid::parse_str(project_raw)
        .map_err(|e| format!("coverage: bad project id {project_raw:?}: {e}"))?;

    let command = pg.get_config(COVERAGE_COMMAND_KEY).await?.unwrap_or_default().trim().to_string();
    if command.is_empty() {
        tracing::info!(
            "coverage: backfill requested but {COVERAGE_COMMAND_KEY} is unset — skipped (opt-in: the daemon does not run your tests unless configured)",
        );
        return Ok(0);
    }

    let ids = pg.active_metric_ids(MetricGroup::Coverage.as_str()).await?;
    let Some(mid) = ids.get(KEY_COVERAGE).copied() else {
        return Ok(0);
    };
    let candidates = lcov_candidates(pg).await;
    let mut written = 0u32;

    for (repository_id, abs_path) in pg.repository_roots_for_project(&project_id).await? {
        // The SAME weekly anchors quality samples, bounded to the most recent `weeks`.
        let mut sampled =
            super::quality::sample_commit_days(&super::churn::git_commit_days(&abs_path));
        if let Some(w) = weeks {
            let w = w as usize;
            if sampled.len() > w {
                sampled = sampled.split_off(sampled.len() - w);
            }
        }
        for day in sampled {
            let Some(sha) = super::quality::resolve_commit_as_of(&abs_path, day) else {
                continue; // day predates the repo / git miss → no row
            };
            // Checkout the commit, run the coverage command in it, ingest the lcov it
            // produced. The worktree is always torn down (quality's WorktreeGuard).
            let cmd = command.clone();
            let cands = candidates.clone();
            let scan = super::quality::scan_at_commit(&abs_path, &sha, move |wt| {
                run_coverage_command(wt, &cmd)?;
                Ok(read_coverage(&wt.to_string_lossy(), &cands))
            })?;
            let Some((hit, found)) = scan else {
                continue; // command failed / no lcov produced → honest-empty for this commit
            };
            if found <= 0 {
                continue;
            }
            let value = hit as f64 / found as f64;
            let props = serde_json::json!({ "numerator": hit, "denominator": found });
            pg.upsert_project_metric_repo(
                &mid,
                &project_id,
                Some(&repository_id),
                SCOPE_USER,
                None,
                Some(&sha), // commit_sha set → a historical, commit-cadence coverage point
                None,
                None,
                day,
                GRAIN_DAILY,
                value,
                &props,
                SOURCE_MEASURED,
            )
            .await?;
            written += 1;
        }
    }

    tracing::info!(project = %project_raw, rows = written, "coverage: backfill complete");
    Ok(written)
}

#[cfg(test)]
// Test gates are blocking `std::sync::Mutex` held across awaits ON PURPOSE —
// see `crate::tasks::test_support::TestGate` for why an async mutex loses
// wakeups across per-test runtimes. One allow per test module, not per site.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, git_commit_on_day,
        make_ctx, repository_for_folder, seed_git_project_folder, TestGate,
    };

    /// Coverage resolves its lcov path through the GLOBAL `metrics.coverage_lcov`
    /// config, and `coverage_reads_config_override_path` sets that key for the
    /// duration of its body. Its own comment says the reset protects "the next
    /// SERIAL test" — but these run in parallel, so while the override is live any
    /// concurrent coverage test looks for `build/cov/report.lcov` instead of
    /// `lcov.info`, finds nothing, and writes 0 rows. That is what failed
    /// `coverage_real_zero_hits_writes_a_real_zero` (expected 1 row, got 0) and
    /// `backfill_runs_command_at_sampled_commits_and_ingests_history`. The
    /// fixtures are already per-test unique — the shared state is the config row,
    /// which is global by design.
    static COVERAGE_CONFIG_LOCK: TestGate = TestGate::new();
    use sqlx_core::query_as::query_as;

    // ── Pure: lcov parser ────────────────────────────────────────────────────

    #[test]
    fn parse_lcov_sums_lh_lf_across_records() {
        // Two file records: (LH 8 / LF 10) + (LH 3 / LF 6) → (11 hit, 16 found).
        let report = "\
TN:
SF:src/a.rs
DA:1,1
LH:8
LF:10
end_of_record
SF:src/b.rs
LH:3
LF:6
end_of_record
";
        assert_eq!(parse_lcov(report), (11, 16), "LH/LF summed across records");
    }

    #[test]
    fn parse_lcov_falls_back_to_da_when_no_lf() {
        // No LF/LH summary lines → derive from DA records: 3 instrumented, 2 hit.
        let report = "\
SF:src/a.rs
DA:1,5
DA:2,0
DA:3,1
end_of_record
";
        assert_eq!(parse_lcov(report), (2, 3), "DA fallback: hit = DA with count>0, found = all DA");
    }

    #[test]
    fn parse_lcov_empty_or_garbage_is_zero() {
        assert_eq!(parse_lcov(""), (0, 0), "empty report → (0,0)");
        assert_eq!(parse_lcov("not an lcov file\nrandom\n"), (0, 0), "no LF/DA → (0,0)");
    }

    // ── Ingest: read the current checkout's report ───────────────────────────

    fn write_lcov(root: &std::path::Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[tokio::test]
    async fn coverage_ingests_current_lcov_and_pools_into_the_default_read() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // A repo with an lcov report (LH 15 / LF 20) → ONE scope=user coverage row
        // (identity NULL), value 0.75, props numerator/denominator = 15/20, keyed on the
        // repository, and surfaced in the DEFAULT scope=user project read (like knowledge).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        write_lcov(repo.path(), "lcov.info", "SF:src/a.rs\nLH:15\nLF:20\nend_of_record\n");

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "one coverage row for the repo with a report");

        let (value, props, scope, repo_id, identity): (
            f64,
            serde_json::Value,
            String,
            Option<uuid::Uuid>,
            Option<String>,
        ) = query_as(
            "SELECT pm.value::float8, pm.props, pm.scope::text, pm.repository_id, pm.identity \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'coverage'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!((value - 0.75).abs() < 1e-9, "coverage = 15 hit / 20 found = 0.75");
        assert_eq!(props["numerator"].as_i64(), Some(15), "numerator = lines hit");
        assert_eq!(props["denominator"].as_i64(), Some(20), "denominator = lines found");
        assert_eq!(scope, "user", "coverage is scope=user (pools into the default read; no author dimension)");
        assert_eq!(repo_id, Some(rid), "keyed on the resolved repository_id");
        assert_eq!(identity, None, "identity NULL — coverage is not author-attributed");
        // It surfaces in the default scope=user project read (the pooled view path).
        let daily = daily_rows(pg, &pid).await;
        let cov = daily.iter().find(|r| r.0 == "coverage").expect("coverage in the scope=user read");
        assert!((cov.1 - 0.75).abs() < 1e-9, "the default read carries the coverage value");

        // Idempotent: re-run upserts in place.
        let again = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(again, 1, "re-run recomputes the same row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 1, "idempotent upsert — still one row");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn coverage_real_zero_hits_writes_a_real_zero() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // A report with real instrumented lines but 0 hit → a REAL 0.0 row (the suite is
        // instrumented but nothing is covered), never suppressed.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        write_lcov(repo.path(), "lcov.info", "SF:src/a.rs\nLH:0\nLF:12\nend_of_record\n");

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 1, "a real 0.0 over a real denominator is written");
        let (value, den): (f64, i64) = query_as(
            "SELECT pm.value::float8, (pm.props->>'denominator')::int8 \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'coverage'",
        )
        .bind(pid)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert!(value.abs() < 1e-9, "value is a real 0.0, not a suppressed row");
        assert_eq!(den, 12, "denominator = 12 instrumented lines");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn coverage_no_report_or_empty_writes_no_row() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // No lcov file at all → honest-empty (no row).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, _repo) = seed_git_project_folder(pg, &uniq).await;

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no report → no coverage row (never a fabricated 0)");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows without a report");

        // An lcov with 0 instrumented lines (LF:0) → also no row (no denominator).
        write_lcov(_repo.path(), "lcov.info", "SF:src/a.rs\nLH:0\nLF:0\nend_of_record\n");
        let w2 = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(w2, 0, "0 instrumented lines → no denominator → no row");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn coverage_historical_as_of_skips() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // Forward-only: a historical as_of writes NO row (historical coverage is the
        // opt-in backfill, not this snapshot path), even with a report present.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        write_lcov(repo.path(), "lcov.info", "SF:src/a.rs\nLH:5\nLF:5\nend_of_record\n");

        let past = chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let written = compute(&ctx, &pid.to_string(), Some(past)).await.unwrap();
        assert_eq!(written, 0, "historical as_of → forward-only skip → no row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 0, "no rows for a historical as_of");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn coverage_reads_config_override_path() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // The metrics.coverage_lcov override points at a non-default location.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        write_lcov(repo.path(), "build/cov/report.lcov", "SF:src/a.rs\nLH:9\nLF:10\nend_of_record\n");
        pg.set_config(LCOV_PATH_KEY, "build/cov/report.lcov").await.unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        let den: Option<i64> = query_as(
            "SELECT (pm.props->>'denominator')::int8 FROM sensei.project_metrics pm \
               JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'coverage'",
        )
        .bind(pid)
        .fetch_optional(pg.pool())
        .await
        .unwrap()
        .map(|(d,): (i64,)| d);

        // RESET the shared-DB global config BEFORE asserting, so a failed assertion
        // can't leak `metrics.coverage_lcov` into the next serial test (which would then
        // read the wrong path and spuriously fail).
        pg.set_config(LCOV_PATH_KEY, "").await.unwrap();
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;

        assert_eq!(written, 1, "the config override path is read");
        assert_eq!(den, Some(10), "read the override report (LF:10)");
    }

    // ── Opt-in backfill (checkout past commits + run the coverage command) ───

    #[tokio::test]
    async fn backfill_runs_command_at_sampled_commits_and_ingests_history() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // With metrics.coverage_command configured, backfill checks out sampled past
        // commits (one per ISO week), runs the command (here: write a fixed lcov), and
        // ingests it → one historical coverage row per sampled commit, commit_sha set.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        // Three commits in three distinct ISO weeks → three sampled anchors.
        for (d, f) in [("2025-05-05", "a.rs"), ("2025-05-12", "b.rs"), ("2025-05-19", "c.rs")] {
            git_commit_on_day(repo.path(), d, &[(f, "x\n")]);
        }
        // The "coverage command" just writes a fixed lcov into the worktree (LH 5/LF 10).
        pg.set_config(COVERAGE_COMMAND_KEY, "printf 'SF:x\\nLH:5\\nLF:10\\nend_of_record\\n' > lcov.info").await.unwrap();

        let written = backfill(pg, &pid.to_string(), None).await.unwrap();

        #[allow(clippy::type_complexity)]
        let rows: Vec<(chrono::NaiveDate, Option<String>, f64)> = query_as(
            "SELECT pm.computed_on, pm.commit_sha, pm.value::float8 \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'coverage' ORDER BY pm.computed_on",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();

        // Reset the shared-DB global config BEFORE asserting (panic-safe leak guard).
        pg.set_config(COVERAGE_COMMAND_KEY, "").await.unwrap();
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;

        assert_eq!(written, 3, "one coverage row per sampled ISO-week commit");
        assert_eq!(rows.len(), 3, "three historical coverage rows");
        assert!(rows.iter().all(|r| r.1.is_some()), "every backfill row carries a commit_sha (commit-cadence historical point)");
        assert!(rows.iter().all(|r| (r.2 - 0.5).abs() < 1e-9), "value = 5 hit / 10 found = 0.5");
        let days: Vec<_> = rows.iter().map(|r| r.0.to_string()).collect();
        assert!(days.contains(&"2025-05-05".to_string()), "stamped on the commit-day, not today");
    }

    #[tokio::test]
    async fn backfill_disabled_without_a_configured_command() {
        let _guard = COVERAGE_CONFIG_LOCK.enter();
        // No metrics.coverage_command → backfill is a no-op (the daemon never runs the
        // project's tests unless explicitly configured).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        git_commit_on_day(repo.path(), "2025-05-05", &[("a.rs", "x\n")]);
        pg.set_config(COVERAGE_COMMAND_KEY, "").await.unwrap(); // ensure unset

        let written = backfill(pg, &pid.to_string(), None).await.unwrap();
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
        assert_eq!(written, 0, "no coverage_command → backfill runs nothing (opt-in)");
    }
}
