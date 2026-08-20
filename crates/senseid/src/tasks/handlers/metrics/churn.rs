//! `churn` metric group computer.
//!
//! Follows the `session_outcomes` template (resolve `key → metric_id` via the
//! active registry, write daily rows to `sensei.project_metrics`) but at the
//! REPOSITORY grain (spec 2026-08-18): metrics key on the GLOBAL `repository_id`,
//! not the project. Its two git-sourced churn metrics are sourced from **git**, not
//! the file-indexing feed:
//!
//! - `churn_rate` (count, per-repository daily): the number of DISTINCT source
//!   files touched by that day's commits — the real files-changed-over-time signal
//!   (GitClear's churn *definition*, measured first-party from `git log`). One row
//!   per (repository × scope × commit-day).
//! - `churn_concentration` (pct, per-repository daily): the share of the day's
//!   line-churn absorbed by the busiest 20% of files (Pareto) — `numerator` =
//!   line-churn of the top-20% files, `denominator` = total line-churn. A day with
//!   commits but zero line-churn (only binary/mode changes) has no denominator ⇒ NO
//!   concentration row (never a fabricated 0/0), though `churn_rate` still counts
//!   the touched files.
//! - `rework_density` (ratio, PROJECT-level, attributed to the project's PRIMARY
//!   repository): a forward-only snapshot of `inference.detected_patterns`
//!   (`name LIKE 'rework: %'`) over `sensei.nodes` project files. It has no natural
//!   per-repository git grain, so it is written ONCE per project against
//!   [`PgStore::primary_repository_for_project`] (`scope = 'user'`, `folder_id =
//!   NULL`). The former per-module (`folder_id`-set) rows are RETIRED — under the
//!   repo-grain identity (`project_metrics_identity`, which does NOT include
//!   `folder_id`) they would collide. It keeps its DB source and its
//!   historical-`as_of`-skip behavior.
//!
//! ## Dual derivation (I-B) — the whole-tree twin + the local-user value
//! For each repository the churn metrics are derived TWICE:
//! - `scope = 'repo'`: the whole-tree `git log` (ALL authors), `identity = NULL` —
//!   the repository-wide churn twin.
//! - `scope = 'user'`: `git log --author=<email>` per local git identity,
//!   `identity = <email>` — the local user's OWN churn. This is the value the
//!   default project read pools (the project view filters `scope = 'user'`).
//!
//! Local identities are the checkout's effective git author email (git's own
//! local→global precedence via [`crate::git_identity::read_git_user`]); sensei
//! models a single local user, so this is normally one email. A checkout with no
//! resolvable git identity yields NO `scope = 'user'` rows (honest-empty, never a
//! fabricated author).
//!
//! ## Git sourcing (why + how)
//! The two churn metrics were previously counted from `activity.task_executions`
//! (the file-INDEXING feed), so a re-index spiked `churn_rate` and only recent
//! history existed. Real churn is files-changed-over-time from GIT, so they now
//! read `git log` for EACH of the project's repository checkout roots — resolved
//! via [`PgStore::repository_roots_for_project`] (one `(repository_id, abs_path)`
//! per distinct repository, shallowest checkout wins). A project with no
//! repository-linked checkout, a root that is not a git repo, git being absent, or
//! a repo with no commits on the selected day all produce NO row: an honest "no git
//! churn data", never a fabricated value. This mirrors the git discipline already
//! used by `indexer/cross_repo` and `tasks/handlers/scan` (shell out to git,
//! tolerate its absence).
//!
//! Commits bucket on the COMMITTER date (`%cd --date=short`); both the planner's
//! per-day discovery ([`git_commit_days`]) and this computer read that same field,
//! so a planned commit-day maps to the day the compute buckets it under. Merges
//! are excluded (`--no-merges`).
//!
//! ## `as_of` (per-day, mirrors `session_outcomes`/`autonomy`)
//! `churn` is a PER-DAY planned group: the planner enqueues one
//! `ComputeGroupMetrics{as_of=Some(D)}` per git-commit-day plus the trailing window, so
//! churn backfills over real git history.
//! - `Some(D)` — compute ONLY day `D` from that day's commits, `computed_on = D`.
//! - `None` — the rolling window: every commit-day in the trailing
//!   [`metrics.window_days`] window through today.
//!
//! ## Capture-scope split (governance, #3)
//! Churn is GIT-derived, so it must NOT authorize the activity pruner's
//! capture-before-reclaim: a git-churn row for a day must never green-light
//! reclaiming that day's sessions before their session-anchored metrics (ftr /
//! throughput) are captured. The planner backfills churn per-day, but churn's
//! registry `capture_source` is `git` (not `session`), so the pruner's guard —
//! which authorizes reclaim only on `capture_source = 'session'` — excludes it.
//!
//! Never-fabricate: every DB call propagates `Err`; a git failure/miss produces NO
//! row (honest-empty). A ratio/pct with denominator 0 writes NO row. A repository
//! that cannot be resolved is SKIPPED, never a made-up repository (I-E).

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::db::pg_store::PgStore;
use crate::tasks::executor::TaskContext;

use super::MetricGroup;

/// `sensei.metric_grain` text value (churn writes daily rows only — I-A).
const GRAIN_DAILY: &str = "daily";
/// `sensei.metric_source` text value — churn is measured, not estimated.
const SOURCE_MEASURED: &str = "measured";
/// `sensei.metric_scope` text values (I-B): the local-user value + the whole-tree twin.
const SCOPE_USER: &str = "user";
const SCOPE_REPO: &str = "repo";

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

/// Run `git` in `root` with `args` and return stdout, or `None` when git is
/// unavailable, `root` is not a git repo / does not exist, the repo has no commits,
/// or the command otherwise fails. A `None` is an honest "no git data" (the caller
/// writes no row), never a fabricated value — matching the git discipline in
/// `indexer/cross_repo` and `tasks/handlers/scan` (shell out, tolerate absence).
/// Shared with the `quality` group (worktree/commit resolution) so the git shell-out
/// helper lives in exactly one place.
pub(super) fn run_git(root: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The local git author identities to attribute the `scope = 'user'` churn twin to.
/// sensei models a SINGLE local user, so this is the checkout's effective git author
/// email (git's own local→global precedence via
/// [`crate::git_identity::read_git_user`]), returned as a `Vec` so the caller loops
/// uniformly. A checkout with no resolvable git identity yields an EMPTY set → no
/// `scope = 'user'` rows for that repository (honest-empty, never a fabricated
/// author — I-E). Reuses the shared identity helper rather than re-reading git config.
fn local_identities(root: &str) -> Vec<String> {
    crate::git_identity::read_git_user(std::path::Path::new(root))
        .email
        .into_iter()
        .collect()
}

/// The pre-AI history floor for one repository's git-cadence metrics — how far back a
/// commit-day may be computed. Shared by `churn` + `quality` so the baseline rule
/// lives in ONE place.
#[derive(Clone, Copy)]
pub(super) enum RepoFloor {
    /// No captured AI activity for this repository under the `off` policy → compute
    /// NOTHING (the git-cadence metrics measure the user+AI interaction).
    Skip,
    /// No floor — the repository's entire git history (`baseline = full`).
    All,
    /// Compute only commit-days on/after this floor day.
    From(NaiveDate),
}

/// Resolve the [`RepoFloor`] for `repository_id`/`root` under the `baseline` policy
/// (spec D17): by DEFAULT (`off`) the git-cadence metrics start at the repository's
/// first AI-transcript day ([`PgStore::repo_ai_start`]) — not its whole pre-AI git
/// history. `full` lifts the floor entirely; `N` extends it back N commits before the
/// first AI commit (a before/after baseline). A repository with no AI activity is
/// `Skip` under `off`/`N` (no reference point) and `All` under `full`. Never
/// fabricates a date; propagates the read error.
pub(super) async fn repo_history_floor(
    pg: &PgStore,
    repository_id: &uuid::Uuid,
    root: &str,
    baseline: crate::tasks::metrics_scheduler::BaselineHistory,
) -> Result<RepoFloor, String> {
    use crate::tasks::metrics_scheduler::BaselineHistory as B;
    if baseline == B::Full {
        return Ok(RepoFloor::All);
    }
    // `off` / `N` both anchor on the first AI transcript; no AI activity → no anchor.
    let Some(ai_start) = pg.repo_ai_start(repository_id).await? else {
        return Ok(RepoFloor::Skip);
    };
    Ok(match baseline {
        B::Off => RepoFloor::From(ai_start),
        // Extend the floor back N commits before the first AI commit; fewer than N
        // pre-AI commits (or git absent) → floor at the AI-start (no fabricated day).
        B::Commits(n) => RepoFloor::From(nth_commit_day_before(root, ai_start, n).unwrap_or(ai_start)),
        B::Full => RepoFloor::All, // handled above; kept exhaustive.
    })
}

/// The committer-day of the Nth commit strictly before `day` (first-parent,
/// no-merges) — the pre-AI baseline floor for `baseline = N`. `None` when git is
/// unavailable, `root` is not a git repo, or there are fewer than 1 commit before
/// `day` (the caller then floors at `day`, never a fabricated earlier date).
fn nth_commit_day_before(root: &str, day: NaiveDate, n: u32) -> Option<NaiveDate> {
    let before = format!("{} 00:00:00", day.format(GIT_DATE_FMT));
    let out = run_git(
        root,
        &[
            "log", "--first-parent", "--no-merges", "--before", &before, "-n",
            &n.to_string(), "--date=short", "--pretty=format:%cd",
        ],
    )?;
    // The last line is the oldest of the (up to) N commits before `day` — the floor.
    out.lines()
        .last()
        .and_then(|l| NaiveDate::parse_from_str(l.trim(), GIT_DATE_FMT).ok())
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
/// `window_days` window through `today`. `author` scopes the walk: `Some(email)`
/// adds `--author=<email>` (the `scope = 'user'` derivation — the local user's own
/// commits); `None` counts ALL authors (the `scope = 'repo'` whole-tree twin). The
/// scan is bounded with `--since`/`--until` (a ±2-day buffer that comfortably
/// absorbs a commit's timezone offset, which is < 1 day) and then filtered to the
/// EXACT `[lo, hi]` committer-day range on the parsed `%cd` date — so the bucketing
/// matches [`git_commit_days`] regardless of git's date-boundary interpretation.
/// Empty on a non-git root / absent git / no commits on the range (honest-empty).
fn git_day_file_churn(
    root: &str,
    window_days: u32,
    today: NaiveDate,
    as_of: Option<NaiveDate>,
    author: Option<&str>,
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
    let author_flag = author.map(|email| format!("--author={email}"));
    let mut args: Vec<&str> = vec![
        "log", "--no-merges", "--numstat", "--date=short", &pretty, "--since", &since,
        "--until", &until,
    ];
    if let Some(ref a) = author_flag {
        args.push(a);
    }
    let Some(out) = run_git(root, &args) else {
        return HashMap::new();
    };
    let mut by_day = parse_numstat_log(&out);
    by_day.retain(|d, _| *d >= lo && *d <= hi);
    by_day
}

/// `# project files` — `kind = 'file'` nodes across the project's folders (the
/// `rework_density` denominator). One scalar count; the former per-folder breakdown
/// is gone with the per-module rows.
async fn project_file_count(pg: &PgStore, project_id: &uuid::Uuid) -> Result<i64, String> {
    let (total,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(*)::int8
           FROM sensei.nodes   n
           JOIN sensei.folders f ON f.id = n.folder_id
          WHERE f.project_id  = $1
            AND n.kind        = 'file'::sensei.node_kind",
    )
    .bind(project_id)
    .fetch_one(pg.pool())
    .await
    .map_err(|e| e.to_string())?;
    Ok(total)
}

/// `# rework-flagged files` — `inference.detected_patterns` rows the analyzer writes
/// as `name = "rework: <file>"` (`is_anti_pattern`). One row per file (the table's
/// uniqueness is `(project_id, name, is_anti_pattern)`), so a row count IS a
/// distinct-file count — the `rework_density` numerator.
async fn rework_count(pg: &PgStore, project_id: &uuid::Uuid) -> Result<i64, String> {
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
    Ok(total)
}

/// Compute the `churn` group for one project. `project_raw` is the project uuid
/// carried in `task.folder_path`. `as_of` is the target `computed_on` day:
/// - `churn_rate` / `churn_concentration` (git-sourced, per-repository): `Some(D)`
///   computes ONLY day `D`'s commits (`computed_on = D`, the backfill/gap-fill
///   path); `None` computes every commit-day in the trailing window.
/// - `rework_density` (forward-only snapshot): a historical `as_of` (`Some(D)`,
///   `D != today`) skips it (see [`super::is_historical`]).
///
/// Returns the number of `project_metrics` rows written (`0` = honest-empty: no git
/// churn on the day-set, no rework/file data, no resolvable repository, or none of
/// the group's metrics active). Idempotent — re-running backfills in place via the
/// upsert identity (`project_metrics_identity`).
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

    // ── churn_rate + churn_concentration (git-sourced, per-repository, per-day) ──
    if churn_rate_id.is_some() || concentration_id.is_some() {
        // Iterate the project's repository roots (D2: a project is a GROUP of
        // repositories). Each root is the shallowest checkout dir for a repository —
        // the working tree we run the git-log numstat walk in. A project with no
        // repository-linked checkout yields no roots → honest-empty (no rows). A
        // repository is NEVER fabricated (I-E).
        let today = super::today(pg).await?;
        // Pre-AI baseline policy (spec D17): by DEFAULT the git-cadence metrics start
        // at each repository's first AI-transcript day — not its whole (years of,
        // all-authors) git history. `full`/`N` opt into pre-AI history.
        let baseline = crate::tasks::metrics_scheduler::baseline_history(pg).await;
        for (repository_id, root) in pg.repository_roots_for_project(&project_id).await? {
            // Resolve this repository's history floor. A repo with no captured AI
            // activity is SKIPPED under the default (nothing to measure).
            let floor = repo_history_floor(pg, &repository_id, &root, baseline).await?;
            if matches!(floor, RepoFloor::Skip) {
                continue;
            }
            // Dual derivation (I-B): the whole-tree twin (scope=repo, ALL authors,
            // identity=NULL) then one author-filtered row per local git identity
            // (scope=user, identity=that email — the value the project view pools).
            // A checkout with no git identity contributes only the scope=repo twin.
            let mut scopes: Vec<(&str, Option<String>)> = vec![(SCOPE_REPO, None)];
            for email in local_identities(&root) {
                scopes.push((SCOPE_USER, Some(email)));
            }
            for (scope, author_email) in scopes {
                let author = author_email.as_deref();
                // identity mirrors the author on the scope=user twin, NULL on
                // scope=repo (I-C). commit_sha is NULL: a day-bucketed aggregate
                // spans many commits, so no single sha is meaningful (I-D; see the
                // contract_decisions note). folder_id/session_id are always NULL and
                // grain is always daily (I-A).
                for (day, files) in git_day_file_churn(&root, window_days, today, as_of, author) {
                    // Baseline floor (spec D17): skip a commit-day before this
                    // repository's history floor — pre-AI history is opt-in.
                    if let RepoFloor::From(f) = floor
                        && day < f
                    {
                        continue;
                    }
                    // churn_rate (count): # distinct files touched that day. A
                    // returned day always carries ≥1 file (the parser records a day
                    // only for a real numstat line), so this is ≥ 1 — never a
                    // fabricated 0. A count metric carries no numerator/denominator.
                    if let Some(mid) = churn_rate_id {
                        let no_props = serde_json::json!({});
                        pg.upsert_project_metric_repo(
                            &mid, &project_id, Some(&repository_id), scope, author, None,
                            None, None, day, GRAIN_DAILY, files.len() as f64, &no_props,
                            SOURCE_MEASURED,
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
                        // Rank files by churn desc, path asc (deterministic ties),
                        // take the busiest ceil(20%) — ≥1 whenever there is ≥1 file.
                        let mut ranked: Vec<(&String, i64)> =
                            files.iter().map(|(p, &c)| (p, c)).collect();
                        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
                        let top_k =
                            ((ranked.len() as f64) * CONCENTRATION_TOP_FRACTION).ceil() as usize;
                        let top_k = top_k.max(1);
                        let numerator: i64 = ranked.iter().take(top_k).map(|(_, c)| *c).sum();
                        let value = numerator as f64 / total as f64;
                        // I-F: pct rows carry props.numerator + props.denominator.
                        let props =
                            serde_json::json!({ "numerator": numerator, "denominator": total });
                        pg.upsert_project_metric_repo(
                            &mid, &project_id, Some(&repository_id), scope, author, None,
                            None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                        )
                        .await?;
                        written += 1;
                    }
                }
            }
        }
    }

    // ── rework_density (ratio): PROJECT-level snapshot, primary-repository grain ──
    // Forward-only: a rework snapshot reflects the CURRENT signal vs current project
    // files and cannot be reconstructed for a past day, so a historical `as_of`
    // (Some(D), D != today) skips it entirely — no fabricated historical snapshot.
    // This is the LAST block, so a `return` here is equivalent to skipping it.
    if let Some(mid) = rework_id {
        if super::is_historical(pg, as_of).await? {
            return Ok(written);
        }
        // Attribute the PROJECT-level rework snapshot to the project's PRIMARY
        // repository (the repo-grain identity). rework_density has no per-author or
        // per-repository git derivation — it is a whole-project count — so it is
        // written ONCE (scope=user, identity=NULL: attributing a project-wide value
        // to a single author would be a fabricated attribution, I-C/I-E). No
        // repository → honest-empty (no row). The former per-module (folder_id-set)
        // rows are retired (they collide under project_metrics_identity).
        if let Some(repository_id) = pg.primary_repository_for_project(&project_id).await? {
            let project_files = project_file_count(pg, &project_id).await?;
            // 0 project files → no denominator → NO row (a real denominator of 0
            // would be a fabricated 0/0). 0 rework over real files → a real 0.0.
            if project_files > 0 {
                let rework_total = rework_count(pg, &project_id).await?;
                let day = super::today(pg).await?;
                let value = rework_total as f64 / project_files as f64;
                // I-F: ratio rows carry props.numerator + props.denominator.
                let props =
                    serde_json::json!({ "numerator": rework_total, "denominator": project_files });
                pg.upsert_project_metric_repo(
                    &mid, &project_id, Some(&repository_id), SCOPE_USER, None, None,
                    None, None, day, GRAIN_DAILY, value, &props, SOURCE_MEASURED,
                )
                .await?;
                written += 1;
            }
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::metrics_scheduler::BaselineHistory;
    use crate::tasks::test_support::{
        cleanup_metrics_fixture, daily_project_metric_rows as daily_rows, git_commit_on_day,
        make_ctx, repository_for_folder, seed_detected_pattern, seed_file_node,
        seed_git_project_folder, seed_metrics_project_folder, seed_repo_ai_start,
    };
    use sqlx_core::query_as::query_as;

    /// Delete the fixture's default (2000-01-01) AI-start session so a floor test can
    /// install its OWN `repo_ai_start` — otherwise `least(...)` pins the floor to 2000.
    async fn clear_ai_sessions(pg: &PgStore, fid: &uuid::Uuid) {
        sqlx_core::query::query("DELETE FROM activity.sessions WHERE repo_folder_id = $1")
            .bind(fid)
            .execute(pg.pool())
            .await
            .unwrap();
    }

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
        // for the project's repository root, computed for a SINGLE historical day D
        // via as_of=Some(D), stamped computed_on=D (mirrors session_outcomes/autonomy).
        // Under repo grain the fixture repo (author test@sensei.test) yields a DUAL
        // derivation: the scope=repo whole-tree twin AND the scope=user author twin.
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
        // (honest-empty for the recent window, never a fabricated backfill). No file
        // nodes seeded → no rework_density row either.
        let incr = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old commit-day is outside the rolling window → no incremental churn rows");

        // Backfill (as_of=Some(D)): computes exactly that day's churn, DUAL scope —
        // scope=repo (rate+conc) + scope=user (rate+conc) = 4 rows. rework_density is
        // forward-only, so a historical backfill day skips it.
        let written = compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();
        assert_eq!(written, 4, "churn_rate + churn_concentration × {{repo, user}} scope for day D");

        // The project read pools scope=user (daily_rows filters scope='user').
        let daily = daily_rows(pg, &pid).await;
        let rate = daily.iter().find(|r| r.0 == "churn_rate").expect("churn_rate scope=user row");
        assert!((rate.1 - 2.0).abs() < 1e-9, "churn_rate = 2 distinct files changed (a.rs, b.rs)");
        let conc = daily.iter().find(|r| r.0 == "churn_concentration").expect("churn_concentration row");
        // top ceil(20% of 2) = 1 busiest file = a.rs (4) / total line-churn 6.
        assert!((conc.1 - 4.0 / 6.0).abs() < 1e-9, "concentration = top-file line-churn 4 / total 6");
        assert_eq!(conc.2["numerator"].as_i64(), Some(4), "concentration numerator = busiest-20% line-churn");
        assert_eq!(conc.2["denominator"].as_i64(), Some(6), "concentration denominator = total line-churn");

        // computed_on is stamped to the true commit day D (60 days ago); the row is
        // keyed to the resolved repository at scope=user (I-A/I-B).
        let (rate_on_d,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND pm.grain = 'daily' AND pm.scope = 'user' \
                AND m.key = 'churn_rate' AND pm.computed_on = $2",
        )
        .bind(pid)
        .bind(d)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(rate_on_d, 1, "one scope=user churn_rate row stamped computed_on = the commit's day D");

        // Idempotent: re-backfilling the same day upserts in place (still 4 rows).
        let again = compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();
        assert_eq!(again, 4, "re-running the same day backfills in place");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1")
            .bind(pid)
            .fetch_one(pg.pool())
            .await
            .unwrap();
        assert_eq!(total, 4, "idempotent upsert — still 4 rows (2 scope=repo + 2 scope=user) after a second backfill");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_writes_dual_scope_repo_grain_rows() {
        // Pin the repo-grain identity contract (I-A..I-D) on the churn_rate rows: for
        // a single-author repository the compute writes EXACTLY two churn_rate rows —
        // the scope=repo whole-tree twin (identity NULL) and the scope=user author
        // twin (identity = the checkout's git email) — BOTH keyed on the resolved
        // repository_id, with folder_id/session_id/commit_sha NULL and grain=daily.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;

        let d = (chrono::Utc::now() - chrono::Duration::days(45)).date_naive();
        let day = d.format("%Y-%m-%d").to_string();
        git_commit_on_day(repo.path(), &day, &[("a.rs", "1\n2\n3\n")]);

        compute(&ctx, &pid.to_string(), Some(d)).await.unwrap();

        // (scope, identity, repository_id, folder_id, session_id, commit_sha, grain)
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String, Option<String>, Option<uuid::Uuid>, Option<uuid::Uuid>,
            Option<uuid::Uuid>, Option<String>, String,
        )> = query_as(
            "SELECT pm.scope::text, pm.identity, pm.repository_id, pm.folder_id, \
                    pm.session_id, pm.commit_sha, pm.grain::text \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.project_id = $1 AND m.key = 'churn_rate' \
              ORDER BY pm.scope",
        )
        .bind(pid)
        .fetch_all(pg.pool())
        .await
        .unwrap();

        assert_eq!(rows.len(), 2, "one scope=repo twin + one scope=user twin for the single repository");

        // scope=repo: whole-tree, all authors, identity NULL (sorts first).
        let (scope_r, ident_r, repo_r, folder_r, sess_r, sha_r, grain_r) = &rows[0];
        assert_eq!(scope_r, "repo", "the whole-tree twin is scope=repo");
        assert_eq!(*ident_r, None, "scope=repo identity is NULL (I-C)");
        assert_eq!(*repo_r, Some(rid), "keyed on the resolved repository_id (I-A)");
        assert_eq!(*folder_r, None, "folder_id is NULL (I-A)");
        assert_eq!(*sess_r, None, "session_id is NULL (I-A)");
        assert_eq!(*sha_r, None, "commit_sha is NULL for a day-bucketed aggregate (I-D)");
        assert_eq!(grain_r, "daily", "grain is daily (I-A)");

        // scope=user: author-filtered, identity = the checkout's git email.
        let (scope_u, ident_u, repo_u, folder_u, sess_u, sha_u, grain_u) = &rows[1];
        assert_eq!(scope_u, "user", "the local-user value is scope=user");
        assert_eq!(ident_u.as_deref(), Some("test@sensei.test"), "scope=user identity = the checkout git email (I-C)");
        assert_eq!(*repo_u, Some(rid), "keyed on the resolved repository_id (I-A)");
        assert_eq!(*folder_u, None, "folder_id is NULL (I-A)");
        assert_eq!(*sess_u, None, "session_id is NULL (I-A)");
        assert_eq!(*sha_u, None, "commit_sha is NULL for a day-bucketed aggregate (I-D)");
        assert_eq!(grain_u, "daily", "grain is daily (I-A)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_concentration_top_20_percent_rounds_up_over_git() {
        // Pin the ceil(20%) Pareto rule with N=6 files, where ceil(6×0.2)=2 diverges
        // from a floor-then-max(1) mutation (=1). One commit on day D with per-file
        // line-churn 4,3,1,1,1,1 (total 11); the top TWO busiest files (4+3=7) are
        // the numerator, NOT the top-1 (4). Read the pooled scope=user row.
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
        // A project whose repository checkout folder is NOT a real git repo (the
        // synthetic `/_test/metrics-*` path, still repository-linked by the fixture) →
        // the git churn source misses → NO churn_rate / churn_concentration rows
        // (honest-empty, never a fabricated 0). rework_density is DB-sourced and
        // unaffected — it still computes its real 0.0 over real files, attributed to
        // the project's primary repository.
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

    // ── rework_density (project-level, primary-repository grain) ─────────────

    #[tokio::test]
    async fn churn_no_data_writes_zero_rows() {
        // Never-fabricate: a project with no repository-linked folder, no patterns,
        // and no files writes NO rows (not a defaulted 0).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let pid = pg
            .create_project(&format!("_test:churn-empty:{uniq}"), None, None)
            .await
            .unwrap();

        let written = compute(&ctx, &pid.to_string(), None).await.unwrap();
        assert_eq!(written, 0, "no repository / rework / files → zero rows written");

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
        // props.numerator/denominator, written PROJECT-LEVEL ONLY (attributed to the
        // primary repository, scope=user) — no per-module rows. The 0-project-files
        // case writes NO row.
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
        assert_eq!(written_a, 1, "one PROJECT-level rework_density row (non-git root → no churn rows; per-module rows retired)");

        let daily_a = daily_rows(pg, &pid_a).await;
        let rd = daily_a.iter().find(|r| r.0 == "rework_density").expect("rework_density project row");
        assert!((rd.1 - 0.5).abs() < 1e-9, "rework_density = 2 rework files / 4 project files = 0.5");
        assert_eq!(rd.2["numerator"].as_i64(), Some(2), "numerator = # rework-flagged files (correction-prone excluded)");
        assert_eq!(rd.2["denominator"].as_i64(), Some(4), "denominator = # project files");

        // Repo grain retires the per-module rows: NOTHING is written with folder_id set.
        let (module_rows,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics WHERE project_id = $1 AND folder_id IS NOT NULL",
        )
        .bind(pid_a)
        .fetch_one(pg.pool())
        .await
        .unwrap();
        assert_eq!(module_rows, 0, "no per-module (folder_id-set) rows under repo grain");

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
        assert_eq!(written, 1, "one PROJECT-level rework_density row (real 0.0); no churn rows on a non-git root");

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

    // ── Pre-AI baseline floor (spec D17) ──────────────────────────────────────

    #[tokio::test]
    async fn repo_ai_start_resolves_earliest_ai_day_and_none_when_absent() {
        // repo_ai_start = the EARLIEST captured AI day for the repository (least of the
        // session + assistant-event mins), or None when the repo has no AI activity.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, _repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;

        // The fixture seeds one AI-start session at 2000-01-01.
        let epoch = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
        assert_eq!(
            pg.repo_ai_start(&rid).await.unwrap(),
            Some(epoch),
            "the fixture's default AI-start day is resolved",
        );

        // A LATER session never moves the floor forward (least = earliest).
        seed_repo_ai_start(pg, &fid, &pid, "2025-06-15T12:00:00Z").await;
        assert_eq!(
            pg.repo_ai_start(&rid).await.unwrap(),
            Some(epoch),
            "repo_ai_start = the EARLIEST captured AI day, not the latest",
        );

        // Every AI session removed → None (honest-empty; no reference point).
        clear_ai_sessions(pg, &fid).await;
        assert_eq!(pg.repo_ai_start(&rid).await.unwrap(), None, "no AI activity → no AI-start day");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn repo_history_floor_off_full_commits_and_skip() {
        // The floor resolver: off → first AI day; full → no floor; N → N commits before
        // the first AI day (clamped to the oldest commit); no AI activity → Skip (off/N)
        // but still All under full.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        let root = repo.path().to_string_lossy().to_string();

        // Three commits BEFORE a known AI-start day (2025-03-10), newest-last.
        clear_ai_sessions(pg, &fid).await;
        for (d, f) in [("2025-03-01", "f0.rs"), ("2025-03-03", "f1.rs"), ("2025-03-05", "f2.rs")] {
            git_commit_on_day(repo.path(), d, &[(f, "x\n")]);
        }
        seed_repo_ai_start(pg, &fid, &pid, "2025-03-10T00:00:00Z").await;
        let ai_day = NaiveDate::from_ymd_opt(2025, 3, 10).unwrap();

        // off → floor at the first AI day.
        match repo_history_floor(pg, &rid, &root, BaselineHistory::Off).await.unwrap() {
            RepoFloor::From(d) => assert_eq!(d, ai_day, "off floors at the first AI transcript day"),
            _ => panic!("off → From(ai_start)"),
        }
        // full → no floor.
        assert!(
            matches!(repo_history_floor(pg, &rid, &root, BaselineHistory::Full).await.unwrap(), RepoFloor::All),
            "full → All (no floor)",
        );
        // Commits(2) → the 2nd-newest commit before the AI day (2025-03-03).
        match repo_history_floor(pg, &rid, &root, BaselineHistory::Commits(2)).await.unwrap() {
            RepoFloor::From(d) => assert_eq!(
                d, NaiveDate::from_ymd_opt(2025, 3, 3).unwrap(),
                "N=2 → 2 commits before the AI day",
            ),
            _ => panic!("Commits(2) → From(nth-before)"),
        }
        // Commits(50), only 3 pre-AI commits → clamp to the OLDEST commit (never fabricated earlier).
        match repo_history_floor(pg, &rid, &root, BaselineHistory::Commits(50)).await.unwrap() {
            RepoFloor::From(d) => assert_eq!(
                d, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                "N beyond history → the oldest commit day",
            ),
            _ => panic!("Commits(50) → oldest commit"),
        }

        // No AI activity: off/N → Skip, full → All.
        clear_ai_sessions(pg, &fid).await;
        assert!(
            matches!(repo_history_floor(pg, &rid, &root, BaselineHistory::Off).await.unwrap(), RepoFloor::Skip),
            "no AI + off → Skip",
        );
        assert!(
            matches!(repo_history_floor(pg, &rid, &root, BaselineHistory::Commits(3)).await.unwrap(), RepoFloor::Skip),
            "no AI + N → Skip",
        );
        assert!(
            matches!(repo_history_floor(pg, &rid, &root, BaselineHistory::Full).await.unwrap(), RepoFloor::All),
            "no AI + full → All",
        );

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn churn_default_floors_pre_ai_commit_days() {
        // End-to-end: with the default (`off`) policy, backfilling a commit-day BEFORE
        // the repository's first AI transcript writes NOTHING, while a day on/after the
        // AI-start computes churn normally. (Same 60-day commit the re-sourcing test
        // computes when its AI-start is 2000 — here the floor is later, so it's excluded.)
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;

        // Replace the fixture's 2000 AI-start with a MID anchor 40 days ago.
        clear_ai_sessions(pg, &fid).await;
        let ai_day = (chrono::Utc::now() - chrono::Duration::days(40)).date_naive();
        seed_repo_ai_start(pg, &fid, &pid, &format!("{}T00:00:00Z", ai_day.format("%Y-%m-%d"))).await;

        // A commit BEFORE the AI-start (60 days ago) and one AFTER (20 days ago).
        let pre = (chrono::Utc::now() - chrono::Duration::days(60)).date_naive();
        let post = (chrono::Utc::now() - chrono::Duration::days(20)).date_naive();
        git_commit_on_day(repo.path(), &pre.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n")]);
        git_commit_on_day(repo.path(), &post.format("%Y-%m-%d").to_string(), &[("b.rs", "1\n2\n3\n")]);

        // Default (off): the PRE-AI day is floored out → nothing written.
        let pre_written = compute(&ctx, &pid.to_string(), Some(pre)).await.unwrap();
        assert_eq!(pre_written, 0, "a commit-day before the first AI transcript is floored out by default");

        // The POST-AI day computes normally (dual scope repo + user = 4 rows).
        let post_written = compute(&ctx, &pid.to_string(), Some(post)).await.unwrap();
        assert_eq!(post_written, 4, "a commit-day on/after the AI-start computes churn (repo + user scope)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── assistant_events enrichment (base-insert + post-update worker) ───────

    #[tokio::test]
    async fn enrich_assistant_events_derives_attrs_from_own_payload() {
        // The EnrichAssistantEvents worker derives repository_id/plugin/method/
        // tool_kind/call_info from each raw event's own tool_name + payload->tool_input
        // + cwd. Insert raw events (enriched_at NULL) then enrich in one batch.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        let root = repo.path().to_string_lossy().to_string();
        let sid = format!("_test:enrich:{uniq}");

        // (tool_name, cwd, payload) — an MCP plugin call in the repo, a Bash call, and a builtin.
        let events: [(&str, Option<&str>, serde_json::Value); 3] = [
            ("mcp__plugin_sensei_sensei__get_project_conventions", Some(root.as_str()),
             serde_json::json!({"tool_input": {"repoId": "sensei"}})),
            ("Bash", Some(root.as_str()),
             serde_json::json!({"tool_input": {"command": "cargo build", "description": "build"}})),
            ("Read", None, serde_json::json!({"tool_input": {"file_path": "/x"}})),
        ];
        for (tool_name, cwd, payload) in &events {
            sqlx_core::query::query(
                "INSERT INTO activity.assistant_events (session_id, event_type, tool_name, cwd, ts, payload) \
                 VALUES ($1, 'PreToolUse', $2, $3, 0, $4)",
            )
            .bind(&sid)
            .bind(tool_name)
            .bind(cwd)
            .bind(payload)
            .execute(pg.pool())
            .await
            .unwrap();
        }

        // Drain the whole un-enriched backlog (the shared test DB may hold other
        // tests' events with higher priority in id order; the worker enriches
        // oldest-first, so drain until empty to reach our freshly-inserted rows).
        let mut total = 0u64;
        loop {
            let n = pg.enrich_assistant_events(500).await.unwrap();
            total += n;
            if n == 0 {
                break;
            }
        }
        assert!(total >= 3, "enriched at least the 3 seeded events (got {total})");

        #[allow(clippy::type_complexity)]
        let rows: Vec<(String, Option<uuid::Uuid>, Option<String>, Option<String>, Option<String>, Option<String>)> =
            query_as(
                "SELECT tool_name, repository_id, plugin, method, tool_kind, call_info \
                   FROM activity.assistant_events WHERE session_id = $1 ORDER BY tool_name",
            )
            .bind(&sid)
            .fetch_all(pg.pool())
            .await
            .unwrap();
        let by = |t: &str| rows.iter().find(|r| r.0 == t).unwrap_or_else(|| panic!("row for {t}")).clone();

        let mcp = by("mcp__plugin_sensei_sensei__get_project_conventions");
        assert_eq!(mcp.2.as_deref(), Some("sensei"), "plugin parsed from mcp__plugin_<plugin>_");
        assert_eq!(mcp.3.as_deref(), Some("get_project_conventions"), "method = segment after final __");
        assert_eq!(mcp.4.as_deref(), Some("mcp"), "tool_kind = mcp");
        assert_eq!(mcp.1, Some(rid), "repository_id resolved from cwd via repo_anchor_for");

        let bash = by("Bash");
        assert_eq!(bash.4.as_deref(), Some("bash"), "tool_kind = bash");
        assert_eq!(bash.5.as_deref(), Some("cargo build"), "call_info = the shell command");
        assert_eq!(bash.2, None, "no plugin for a builtin");

        let read = by("Read");
        assert_eq!(read.4.as_deref(), Some("builtin"), "tool_kind = builtin");
        assert_eq!(read.1, None, "no cwd → no repository_id (honest-empty)");

        // Idempotent: a second pass enriches nothing (all enriched_at set).
        assert_eq!(pg.enrich_assistant_events(100).await.unwrap(), 0, "re-run enriches nothing (enriched_at set)");

        sqlx_core::query::query("DELETE FROM activity.assistant_events WHERE session_id = $1")
            .bind(&sid).execute(pg.pool()).await.unwrap();
        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }
}
