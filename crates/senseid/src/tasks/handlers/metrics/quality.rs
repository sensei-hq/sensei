//! `quality` metric group computer (Phase 8, repo-grain).
//!
//! Supersedes the former own-graph `duplication` computer: instead of a snapshot
//! over the current symbol embeddings, the code-quality family is now sourced from
//! **`qlty` scans of a git worktree checked out to a PAST commit**, so it backfills
//! over real history. It follows the `churn` template (git-sourced, per-day, honest-
//! empty on non-git/missing-tool) but keys every row on the **repository** grain: it
//! iterates the repositories a project spans ([`PgStore::repository_roots_for_project`],
//! D2 — a project is a GROUP of repositories) and scans EACH checkout root, so a
//! multi-repo project no longer collapses to a single root.
//!
//! Two registry keys (`task_name = "quality"`), stamped `computed_on = the sampled
//! commit-day` and `commit_sha = the sampled commit`:
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
//! ## Dual scope (D7/D8 + P-B) — repo twin + local-user attribution
//! Each repository/commit yields TWO rows per metric:
//! - `scope = repo` (whole-tree): the qlty scan over EVERY file; `identity = NULL`;
//!   the shared, author-agnostic quality of the repository.
//! - `scope = user` (author-attributed, P-B): the SAME qlty findings post-filtered to
//!   the files the LOCAL git author has touched — both `numerator` and `denominator`
//!   measured on that touched surface; `identity = the local git author email`
//!   ([`crate::git_identity::read_git_user`]). A single local user, so `identity` is
//!   the email (NULL only for the whole-tree twin). When git resolves no `user.email`
//!   for the checkout, OR the user has touched no file that still exists (0
//!   denominator), NO `scope = user` row is written (honest-empty) — the whole-tree
//!   twin still is; never a fabricated author or a 0/0.
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
//! duplicates/smells writes a real `0.0` (row written, never suppressed). A ratio
//! with a 0 denominator (empty touched surface) writes NO row.

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
/// `sensei.metric_scope` text values — the whole-tree twin and the local-user row.
const SCOPE_REPO: &str = "repo";
const SCOPE_USER: &str = "user";

/// The registry `key`s this computer produces.
const KEY_DUPLICATION_RATIO: &str = "duplication_ratio";
const KEY_MODULE_QUALITY: &str = "module_quality";

/// Human-readable cadence label for the sampling `log()` — one qlty scan per ISO
/// week (the week's first commit-day). Kept as a const so the cadence is named, not
/// a magic rule buried in [`sample_commit_days`].
const SAMPLE_CADENCE: &str = "weekly (first commit-day per ISO week)";

/// One qlty scan's per-file signals for a single commit. Per-file granularity so the
/// dual scope is derivable from ONE scan: the whole-tree (`scope = repo`) aggregate is
/// the sum across every file, and the local-user (`scope = user`) aggregate is the sum
/// restricted to the files the author touched. Pure data so [`compute_with_scanner`]
/// can be exercised with a fake scanner (no `git worktree` / `qlty` subprocess) while
/// the parsers below are unit-tested on captured real qlty output.
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) struct QualityScan {
    /// Whole-tree total physical source lines (`qlty metrics --all` TOTAL row) — the
    /// `scope = repo` denominator.
    pub total_lines: i64,
    /// Per-file physical source lines (`qlty metrics --all` per-file rows) — the parts
    /// the `scope = user` denominator sums over the touched surface.
    pub per_file_lines: HashMap<String, i64>,
    /// Per-file DISTINCT duplicated-line count (from `qlty smells --sarif`).
    pub per_file_dup_lines: HashMap<String, i64>,
    /// Per-file NON-duplication (maintainability) finding count (from `qlty smells`).
    pub per_file_maintainability: HashMap<String, i64>,
}

impl QualityScan {
    /// Whole-tree distinct duplicated lines — the `scope = repo` duplication numerator.
    fn whole_tree_dup_lines(&self) -> i64 {
        self.per_file_dup_lines.values().sum()
    }
    /// Whole-tree maintainability findings — the `scope = repo` module-quality numerator.
    fn whole_tree_maintainability(&self) -> i64 {
        self.per_file_maintainability.values().sum()
    }
    /// Total source lines over the `touched` files that still exist in the scan — the
    /// `scope = user` denominator (files the user touched that are gone from the current
    /// tree simply have no entry, so they drop out).
    fn touched_lines(&self, touched: &HashSet<String>) -> i64 {
        touched.iter().filter_map(|f| self.per_file_lines.get(f)).sum()
    }
    /// Distinct duplicated lines over the `touched` files — the `scope = user`
    /// duplication numerator.
    fn touched_dup_lines(&self, touched: &HashSet<String>) -> i64 {
        touched.iter().filter_map(|f| self.per_file_dup_lines.get(f)).sum()
    }
    /// Maintainability findings over the `touched` files — the `scope = user`
    /// module-quality numerator.
    fn touched_maintainability(&self, touched: &HashSet<String>) -> i64 {
        touched.iter().filter_map(|f| self.per_file_maintainability.get(f)).sum()
    }
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
    // Resolve `qlty` through the SHARED bootstrap resolver — NOT a bare
    // `Command::new("qlty")`. qlty installs at `~/.qlty/bin` (or `$QLTY_INSTALL/bin`),
    // which the daemon's launchd PATH does not include; a bare spawn misses it and
    // silently empties the whole quality family. `command_for` scans PATH + those
    // user-local dirs (its doc: callers MUST use it when the binary may live outside
    // the process PATH).
    let mut cmd = match sensei_bootstrap::util::command_for("qlty") {
        Ok(cmd) => cmd,
        Err(e) => {
            // qlty is a SOFT prereq (absent → honest-empty, daemon runs fine), but a
            // fully-unresolvable CLI would otherwise seal the watermark as if measured.
            // Surface it (no silent errors) instead of a silent empty.
            tracing::warn!(
                error = %e,
                "quality: qlty CLI not resolved — quality metrics stay honest-empty until it is installed (~/.qlty/bin, $QLTY_INSTALL/bin, or on PATH)",
            );
            return None;
        }
    };
    let out = cmd.args(args).current_dir(dir).output().ok()?;
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

/// Parse `qlty metrics --all`' table into `(whole-tree TOTAL lines, per-file lines)`.
/// Locates the `name` + `lines` columns by their header names (robust to column
/// reordering) after [`strip_ansi`], then reads every table row: the `TOTAL` row
/// yields the whole-tree denominator, and each file row yields that file's source
/// lines (the parts the `scope = user` denominator sums). The TOTAL is `None` when
/// there is no parseable TOTAL row (an empty scan) — the caller then writes no row
/// (honest-empty). Pure over captured qlty output.
fn parse_metrics(stdout: &str) -> (Option<i64>, HashMap<String, i64>) {
    let clean = strip_ansi(stdout);
    let mut lines_col: Option<usize> = None;
    let mut name_col: usize = 0;
    let mut total: Option<i64> = None;
    let mut per_file: HashMap<String, i64> = HashMap::new();
    for line in clean.lines() {
        if !line.contains('|') {
            continue; // skip the `---+---` separator + any non-table chrome
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        // The first `|`-bearing row is the header — find the `lines` (+ `name`) columns.
        let Some(lcol) = lines_col else {
            if let Some(i) = cells.iter().position(|c| *c == "lines") {
                lines_col = Some(i);
                name_col = cells.iter().position(|c| *c == "name").unwrap_or(0);
            }
            continue;
        };
        let name = cells.get(name_col).copied().unwrap_or("");
        let val = cells.get(lcol).and_then(|v| v.parse::<i64>().ok());
        if name == "TOTAL" {
            total = val;
        } else if let (Some(v), false) = (val, name.is_empty()) {
            per_file.insert(name.to_string(), v);
        }
    }
    (total, per_file)
}

/// The duplication + maintainability signals parsed from `qlty smells --all --sarif`,
/// bucketed PER FILE so the dual scope is derivable from one scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SmellCounts {
    /// Per-file count of DISTINCT physical source lines flagged as duplicated (union
    /// of the line ranges across every `identical-code`/`similar-code` finding,
    /// de-duplicated per file so the ratio stays in `[0, 1]`).
    per_file_dup_lines: HashMap<String, i64>,
    /// Per-file count of NON-duplication smell findings (complexity / nesting /
    /// parameters / …) — the maintainability burden. A finding with no locatable file
    /// is bucketed under the empty key so it counts toward the whole-tree total yet is
    /// never attributed to a user's touched surface.
    per_file_maintainability: HashMap<String, i64>,
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

/// The artifact uri of a result's PRIMARY location (its first `locations` entry) — the
/// file a maintainability finding is attributed to. `None` when the finding carries no
/// locatable file (the caller then buckets it under the whole-tree total only).
fn primary_uri(result: &serde_json::Value) -> Option<String> {
    result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"]
        .as_str()
        .map(str::to_string)
}

/// Parse `qlty smells --all --sarif` into per-file [`SmellCounts`]. A missing
/// `results` array is an honest zero (a clean scan), not an error; malformed JSON
/// propagates `Err` (a scan that ran but produced garbage is a genuine failure, not
/// honest-empty). Pure over captured qlty SARIF.
fn parse_smells(json: &str) -> Result<SmellCounts, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("quality: bad qlty SARIF: {e}"))?;
    let Some(results) = v["runs"][0]["results"].as_array() else {
        return Ok(SmellCounts::default()); // no run/results → clean scan (0/0)
    };
    let mut dup_per_file: HashMap<String, HashSet<i64>> = HashMap::new();
    let mut maint_per_file: HashMap<String, i64> = HashMap::new();
    for result in results {
        if is_duplication(result) {
            collect_dup_lines(result, &mut dup_per_file);
        } else {
            // A maintainability finding with no locatable file still counts toward the
            // whole-tree total (bucketed under "") but can't be attributed to a user.
            let uri = primary_uri(result).unwrap_or_default();
            *maint_per_file.entry(uri).or_insert(0) += 1;
        }
    }
    let per_file_dup_lines = dup_per_file
        .into_iter()
        .map(|(uri, lines)| (uri, lines.len() as i64))
        .collect();
    Ok(SmellCounts { per_file_dup_lines, per_file_maintainability: maint_per_file })
}

/// The FIXED `qlty` ruler pinned into every scanned worktree (P-B, spec §7). Every
/// sampled commit is measured against THIS config — never its in-tree `.qlty`, which
/// is absent on pre-config history and can drift between commits — so the trend is
/// comparable ("the same ruler"). Deliberately plugin-LESS: `qlty metrics`/`smells`
/// are built-in tree-sitter analyses, while the linter `[[plugin]]`s are for `qlty
/// check` and would force a per-scan network source fetch. A STABLE snapshot —
/// changing it re-bases the ruler, so it is versioned here deliberately rather than
/// tracked to the live repo `.qlty`.
const PINNED_QLTY_CONFIG: &str = r#"config_version = "0"
exclude_patterns = [
  "*.min.*", "*-min.*", "*_min.*",
  "**/.yarn/**", "**/*.d.ts", "**/assets/**", "**/build/**", "**/cache/**",
  "**/dist/**", "**/generated/**", "**/node_modules/**", "**/target/**",
  "**/testdata/**", "**/vendor/**",
]
test_patterns = [
  "**/test/**", "**/spec/**", "**/*.test.*", "**/*.spec.*",
  "**/*_test.*", "**/*_spec.*", "**/test_*.*",
]
[smells]
mode = "comment"
[[source]]
name = "default"
default = true
"#;

/// Write [`PINNED_QLTY_CONFIG`] to `wt/.qlty/qlty.toml` (creating `.qlty/`), so the
/// scan measures against the fixed ruler regardless of what config the checkout
/// carried (config-pinning, P-B). Propagates the IO error — a scan is never silently
/// left unpinned. qlty's content cache is global (`~/.qlty/cache`), so scans across
/// worktrees already share it — no per-scan cache flag is needed.
fn pin_qlty_config(wt: &Path) -> Result<(), String> {
    let dir = wt.join(".qlty");
    std::fs::create_dir_all(&dir).map_err(|e| format!("quality: pin .qlty dir: {e}"))?;
    std::fs::write(dir.join("qlty.toml"), PINNED_QLTY_CONFIG)
        .map_err(|e| format!("quality: pin qlty.toml: {e}"))
}

/// Run the two `qlty` scans in the worktree `wt` and assemble a [`QualityScan`].
/// `qlty` absent / no `.qlty` config / no parseable TOTAL → `Ok(None)` (honest-empty);
/// a scan that ran but produced unparseable SARIF → `Err` (genuine failure → retry).
fn run_qlty_scan(wt: &Path) -> Result<Option<QualityScan>, String> {
    // Config-pinning (P-B, spec §7): write the fixed ruler into the worktree BEFORE
    // scanning, so this commit is measured against the SAME config as every other
    // (its in-tree `.qlty` — absent on pre-config history, drifting between commits —
    // is ignored). With the config always present, a `qlty metrics` miss below now
    // means qlty is ABSENT (honest-empty), never "unconfigured".
    pin_qlty_config(wt)?;
    let Some(metrics) = run_qlty(wt, &["metrics", "--all", "--quiet", "--no-upgrade-check"]) else {
        return Ok(None); // qlty CLI absent → honest-empty
    };
    let (total, per_file_lines) = parse_metrics(&metrics);
    let Some(total_lines) = total else {
        return Ok(None); // no TOTAL row (empty scan) → no denominator → honest-empty
    };
    let Some(sarif) = run_qlty(wt, &["smells", "--all", "--sarif", "--no-upgrade-check"]) else {
        return Ok(None);
    };
    let smells = parse_smells(&sarif)?;
    Ok(Some(QualityScan {
        total_lines,
        per_file_lines,
        per_file_dup_lines: smells.per_file_dup_lines,
        per_file_maintainability: smells.per_file_maintainability,
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
pub(super) fn scan_at_commit<T, F>(root: &str, sha: &str, scan: F) -> Result<Option<T>, String>
where
    F: FnOnce(&Path) -> Result<Option<T>, String>,
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
pub(super) fn resolve_commit_as_of(root: &str, day: NaiveDate) -> Option<String> {
    let until = format!("{} 23:59:59", day.format("%Y-%m-%d"));
    let out = super::churn::run_git(
        root,
        &["rev-list", "-1", "--first-parent", "--before", &until, "HEAD"],
    )?;
    let sha = out.trim();
    (!sha.is_empty()).then(|| sha.to_string())
}

/// The distinct set of repo-relative file paths the git author `email` touched across
/// history up to `sha` (`git log <sha> --author=<email> --name-only`). Intersected
/// with the scan's present files by the caller, this is the LOCAL user's touched
/// surface — the files the `scope = user` quality row measures over. An empty set (git
/// miss, or the author touched nothing) yields no `scope = user` row (honest-empty),
/// never a fabricated attribution. Paths are repo-root-relative, matching qlty's
/// per-file names and SARIF artifact uris.
fn git_authored_files(root: &str, sha: &str, email: &str) -> HashSet<String> {
    let author = format!("--author={email}");
    let Some(out) = super::churn::run_git(
        root,
        &["log", "--no-merges", "--name-only", "--pretty=format:", &author, sha],
    ) else {
        return HashSet::new();
    };
    out.lines()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect()
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
/// sampled past commits, keyed on the REPOSITORY grain. `project_raw` is the project
/// uuid carried in `task.folder_path`. `as_of`:
/// - `Some(D)` — scan ONLY the sampled commit-day `D` (`computed_on = D`, the
///   backfill/gap-fill + trailing-window path). A `D` that is not a sampled anchor is
///   a cheap no-op (no worktree/scan) so the planner's trailing-window refresh does
///   not force a scan on every calendar day.
/// - `None` — the sampled anchors within the trailing [`metrics.window_days`] window.
///
/// Returns the number of `project_metrics` rows written (`0` = honest-empty: no
/// repository-linked checkout, non-git repos, absent qlty, no commit as-of the day, a
/// scan miss, or none of the group's metrics active). Idempotent — re-running
/// backfills in place via the upsert identity. A past day already covered (per
/// repository) by every active quality metric is skipped (a historical commit's code
/// is immutable) — only `today` is always re-scanned.
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
/// `qlty` scanner; tests pass a fake so the sampling/as_of/dual-scope/upsert logic is
/// exercised without a qlty subprocess (git history + author resolution still run for
/// real against the seeded repo). `scan` receives `(repository_root_abs_path, sha)`.
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

    // The repositories this project spans (D2) — each scanned in its own checkout
    // root. A project with no repository-linked checkout (a repo-less quasi-repo, or a
    // repository that cannot be resolved) writes NO row: honest-empty, never a
    // fabricated repository (I-E).
    let repos = pg.repository_roots_for_project(&project_id).await?;
    if repos.is_empty() {
        return Ok(0);
    }
    let today = super::today(pg).await?;
    let window_days = crate::tasks::metrics_scheduler::window_days(pg).await;
    // Pre-AI baseline policy (spec D17) — same rule as churn.
    let baseline = crate::tasks::metrics_scheduler::baseline_history(pg).await;

    let mut written = 0u32;
    for (repository_id, abs_path) in repos {
        // Baseline floor (spec D17): by default quality starts at this repository's
        // first AI-transcript day; a repo with no captured AI activity is skipped.
        let floor = super::churn::repo_history_floor(pg, &repository_id, &abs_path, baseline).await?;
        if matches!(floor, super::churn::RepoFloor::Skip) {
            continue;
        }
        // The LOCAL git author for this checkout — the identity the `scope = user`
        // rows carry (I-C). `None` when git resolves no `user.email` → NO `scope =
        // user` rows for this repo (the whole-tree twin is still written); never a
        // fabricated author.
        let local_email = crate::git_identity::read_git_user(Path::new(&abs_path)).email;

        // The sampled anchor set (one commit-day per ISO week) for THIS repo — the
        // SAME rule the planner uses, so a planned day is a real anchor.
        let sampled = sample_commit_days(&super::churn::git_commit_days(&abs_path));
        let sampled_set: HashSet<NaiveDate> = sampled.iter().copied().collect();

        // The target day-set: the single `as_of` day, else the sampled anchors inside
        // the trailing window (the today-incremental path).
        let target_days: Vec<NaiveDate> = match as_of {
            Some(d) => vec![d],
            None => {
                let lo = today - chrono::Duration::days((window_days.max(1) - 1) as i64);
                sampled.iter().copied().filter(|d| *d >= lo && *d <= today).collect()
            }
        };

        tracing::info!(
            project = %project_raw,
            repository = %repository_id,
            cadence = SAMPLE_CADENCE,
            sampled_anchors = sampled.len(),
            targets = target_days.len(),
            "quality: sampling git history for qlty scans",
        );

        for day in target_days {
            // Only sampled anchors are scanned; a non-anchor day (e.g. a trailing-window
            // calendar day with no anchor) is a cheap no-op — never a scan.
            if !sampled_set.contains(&day) {
                continue;
            }
            // Baseline floor (spec D17): skip a sampled day before this repository's
            // history floor — pre-AI history is opt-in.
            if let super::churn::RepoFloor::From(f) = floor
                && day < f
            {
                continue;
            }
            // A past day already covered (per repository) by EVERY active quality metric
            // is settled: a historical commit's code is immutable, so re-scanning can't
            // change it. Only `today` (whose HEAD may advance) is always re-scanned. The
            // repository is GLOBAL, so a day another project already scanned for this
            // repo is skipped here too (the shared code is identical).
            if day < today && repo_day_fully_covered(pg, &active_ids, &repository_id, day).await? {
                continue;
            }
            let Some(sha) = resolve_commit_as_of(&abs_path, day) else {
                continue; // no commit on/before this day → honest-empty
            };
            let Some(qs) = scan(&abs_path, &sha)? else {
                continue; // qlty absent / no config / scan miss → honest-empty
            };
            if qs.total_lines <= 0 {
                continue; // no source lines → no denominator → never a fabricated 0/0
            }

            // scope = repo: the whole-tree twin (identity = NULL, D7/D8).
            written += write_ratio_rows(
                pg, &repository_id, SCOPE_REPO, None, &sha, day, dup_id, mq_id,
                qs.whole_tree_dup_lines(), qs.whole_tree_maintainability(), qs.total_lines,
            )
            .await?;

            // scope = user: the SAME findings restricted to the local author's touched
            // files (P-B). Skipped when git has no local email OR the touched surface is
            // empty (0 denominator → no row); never a fabricated author or 0/0.
            if let Some(email) = local_email.as_deref() {
                let touched = git_authored_files(&abs_path, &sha, email);
                let user_total = qs.touched_lines(&touched);
                written += write_ratio_rows(
                    pg, &repository_id, SCOPE_USER, Some(email), &sha, day, dup_id,
                    mq_id, qs.touched_dup_lines(&touched), qs.touched_maintainability(&touched),
                    user_total,
                )
                .await?;
            }
        }
    }

    Ok(written)
}

/// Write the `duplication_ratio` + `module_quality` rows for one repository/commit at
/// one scope. `denominator <= 0` → NO row (never a fabricated 0/0, I-E). Each ratio
/// carries `props.numerator` + `props.denominator` (I-F) plus the sampled `commit` for
/// the explainer; the sampled sha also rides the first-class `commit_sha` column (I-D).
/// `identity` is the local author email for `scope = user`, `None` for the whole-tree
/// `scope = repo` twin (I-C). Every row is repository-keyed with `folder_id`/
/// `session_id` NULL and `grain = daily` (I-A). Returns the rows written (0, 1, or 2).
#[allow(clippy::too_many_arguments)]
async fn write_ratio_rows(
    pg: &PgStore,
    repository_id: &uuid::Uuid,
    scope: &str,
    identity: Option<&str>,
    sha: &str,
    day: NaiveDate,
    dup_id: Option<uuid::Uuid>,
    mq_id: Option<uuid::Uuid>,
    dup_numerator: i64,
    maintainability_numerator: i64,
    denominator: i64,
) -> Result<u32, String> {
    if denominator <= 0 {
        return Ok(0); // no denominator → NO row (never a fabricated 0/0)
    }
    let mut written = 0u32;
    // duplication_ratio: distinct duplicated lines ÷ total source lines.
    if let Some(mid) = dup_id {
        let value = dup_numerator as f64 / denominator as f64;
        let props = serde_json::json!({
            "numerator": dup_numerator,
            "denominator": denominator,
            "commit": sha,
        });
        pg.upsert_project_metric_repo(
            &mid, repository_id, scope, identity, Some(sha), day,
            GRAIN_DAILY, value, &props, SOURCE_MEASURED)
        .await?;
        written += 1;
    }
    // module_quality (maintainability): non-duplication smells ÷ total source lines.
    if let Some(mid) = mq_id {
        let value = maintainability_numerator as f64 / denominator as f64;
        let props = serde_json::json!({
            "numerator": maintainability_numerator,
            "denominator": denominator,
            "commit": sha,
        });
        pg.upsert_project_metric_repo(
            &mid, repository_id, scope, identity, Some(sha), day,
            GRAIN_DAILY, value, &props, SOURCE_MEASURED)
        .await?;
        written += 1;
    }
    Ok(written)
}

/// Whether `day` already has a `scope = repo` daily row for EVERY id in `metric_ids`
/// ON `repository_id` — so a settled past day is skipped only when the repository's
/// whole-tree twin is fully captured (a metric newly activated after an earlier scan
/// still backfills). The whole-tree twin is always written on a successful scan, so it
/// is the authoritative coverage signal (a `scope = user` row can legitimately be
/// absent when git resolves no local author). Empty `metric_ids` → trivially not
/// covered. Propagates the read error; never masks it.
async fn repo_day_fully_covered(
    pg: &PgStore,
    metric_ids: &[uuid::Uuid],
    repository_id: &uuid::Uuid,
    day: NaiveDate,
) -> Result<bool, String> {
    if metric_ids.is_empty() {
        return Ok(false);
    }
    let (present,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT count(DISTINCT metric_id)
           FROM sensei.project_metrics
          WHERE repository_id = $1
            AND metric_id = ANY($2)
            AND scope = 'repo'
            AND grain = 'daily'
            AND computed_on = $3",
    )
    .bind(repository_id)
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
        make_ctx, repository_for_folder, seed_git_project_folder, seed_metrics_project_folder,
    };
    use sqlx_core::query_as::query_as;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    // ── Pure: qlty metrics parser (ANSI-stripped, TOTAL + per-file) ───────────

    #[test]
    fn parse_metrics_reads_total_and_per_file_lines() {
        // A captured-shape `qlty metrics` table: SGR `\x1b[0m` resets around cells
        // (qlty emits them even with NO_COLOR), a `---+---` separator, one file row,
        // and the TOTAL row. The parser strips ANSI, finds the `lines` column by its
        // header name, and reads it off TOTAL (374, NOT LOC=331) plus the per-file row
        // (`lib.rs` = 81) — the parts the scope=user denominator sums.
        let e = "\u{1b}[0m";
        let sample = format!(
            "{e} name {e}|{e} classes | funcs | fields | cyclo | complex | LCOM |{e} lines {e}| LOC \n\
             ------+------+------+------+------+------+------+------+------\n\
             {e} lib.rs {e}| 0 | 7 | 0 | 2 | 1 | 0 | 81 | 72 \n\
             {e} TOTAL {e}| 5 | 26 | 15 | 29 | 14 | 1 |{e} 374 {e}| 331 \n"
        );
        let (total, per_file) = parse_metrics(&sample);
        assert_eq!(total, Some(374), "total physical `lines` (col 7) off TOTAL, ANSI stripped — not LOC");
        assert_eq!(per_file.get("lib.rs"), Some(&81), "per-file `lines` for lib.rs (the scope=user parts)");
        assert_eq!(per_file.len(), 1, "only the one file row (TOTAL is not a per-file entry)");
        // No TOTAL row → None (an empty scan writes no row, never a fabricated 0).
        let (none_total, empty) = parse_metrics("no table here");
        assert_eq!(none_total, None);
        assert!(empty.is_empty());
    }

    // ── Pure: qlty smells SARIF parser (per-file) ─────────────────────────────

    #[test]
    fn parse_smells_counts_distinct_dup_lines_and_maintainability_findings_per_file() {
        // Captured-shape SARIF: ONE duplication finding spanning three 23-line blocks
        // in three DIFFERENT files (523..=545, 312..=334, 360..=382 → 3×23 = 69
        // distinct duplicated lines, 23 per file), plus TWO non-duplication
        // (maintainability) findings both located in a.rs.
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
        assert_eq!(counts.per_file_dup_lines.get("a.rs"), Some(&23), "23 distinct dup lines in a.rs");
        assert_eq!(counts.per_file_dup_lines.get("b.rs"), Some(&23), "23 in b.rs");
        assert_eq!(counts.per_file_dup_lines.get("c.rs"), Some(&23), "23 in c.rs");
        let dup_total: i64 = counts.per_file_dup_lines.values().sum();
        assert_eq!(dup_total, 69, "3 distinct 23-line blocks across 3 files = 69 whole-tree");
        assert_eq!(counts.per_file_maintainability.get("a.rs"), Some(&2), "both maintainability findings in a.rs");
        let maint_total: i64 = counts.per_file_maintainability.values().sum();
        assert_eq!(maint_total, 2, "the 2 non-duplication findings");
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
        assert_eq!(counts.per_file_dup_lines.get("x.rs"), Some(&14), "union of 1..=10 and 5..=14 = 14 distinct lines (deduped)");
        assert!(counts.per_file_maintainability.is_empty());
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
    async fn quality_single_day_as_of_upserts_both_scopes_stamped_on_the_commit_day() {
        // THE repo-grain history test: a commit on a historical day D (a sampled
        // anchor) authored by the fixture's local identity (test@sensei.test) touching
        // a.rs → a qlty scan (injected: per-file a.rs = 200 lines, 40 dup lines, 20
        // maintainability findings). Because the local user touched exactly a.rs, the
        // scope=user surface equals the whole tree here, so BOTH scopes read
        // duplication_ratio 40/200=0.2 + module_quality 20/200=0.1, stamped
        // computed_on=D, commit_sha=the sampled sha, repository_id=the repo. The qlty
        // scan is faked; the git history + author resolution run for real.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;

        let dday = (chrono::Utc::now() - chrono::Duration::days(60)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);

        let fake = |_root: &str, _sha: &str| {
            Ok(Some(QualityScan {
                total_lines: 200,
                per_file_lines: HashMap::from([("a.rs".to_string(), 200)]),
                per_file_dup_lines: HashMap::from([("a.rs".to_string(), 40)]),
                per_file_maintainability: HashMap::from([("a.rs".to_string(), 20)]),
            }))
        };

        // Incremental (as_of=None): D is 60 days ago, outside the rolling window → NO
        // rows (honest-empty for the recent window, never a fabricated backfill).
        let incr = compute_with_scanner(&ctx, &pid.to_string(), None, fake).await.unwrap();
        assert_eq!(incr, 0, "the 60-day-old anchor is outside the rolling window → no incremental rows");

        // Backfill (as_of=Some(D)): 2 metrics × 2 scopes (repo + user) = 4 rows.
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), fake).await.unwrap();
        assert_eq!(written, 4, "duplication_ratio + module_quality, each at scope=repo AND scope=user");

        // scope=user rows (daily_rows now filters scope='user') — author-attributed.
        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("user duplication_ratio row");
        assert!((dr.1 - 0.2).abs() < 1e-9, "duplication_ratio = 40/200 = 0.2 (user touched a.rs)");
        assert_eq!(dr.2["numerator"].as_i64(), Some(40), "numerator = distinct duplicated lines on the touched surface");
        assert_eq!(dr.2["denominator"].as_i64(), Some(200), "denominator = touched source lines");
        let mq = daily.iter().find(|r| r.0 == "module_quality").expect("user module_quality row");
        assert!((mq.1 - 0.1).abs() < 1e-9, "module_quality = 20/200 = 0.1");
        assert_eq!(mq.2["numerator"].as_i64(), Some(20), "numerator = maintainability smells on the touched surface");

        // scope=repo whole-tree twin: identity NULL, repository_id set, commit_sha set.
        let sha = resolve_commit_as_of(&repo.path().to_string_lossy(), dday).expect("a commit on D");
        let repo_row: (f64, i64, i64, Option<String>, Option<String>) = query_as(
            "SELECT pm.value::float8, (pm.props->>'numerator')::int8, (pm.props->>'denominator')::int8, \
                    pm.identity, pm.commit_sha \
               FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.repository_id = $1 AND pm.scope = 'repo' AND m.key = 'duplication_ratio' \
                AND pm.computed_on = $2",
        )
        .bind(rid).bind(dday).fetch_one(pg.pool()).await.unwrap();
        assert!((repo_row.0 - 0.2).abs() < 1e-9, "whole-tree duplication_ratio = 40/200 = 0.2");
        assert_eq!(repo_row.1, 40, "whole-tree numerator");
        assert_eq!(repo_row.2, 200, "whole-tree denominator");
        assert_eq!(repo_row.3, None, "scope=repo identity is NULL (I-C)");
        assert_eq!(repo_row.4.as_deref(), Some(sha.as_str()), "commit_sha = the sampled commit (I-D)");

        // computed_on stamped to the true commit day D; the user row carries the author.
        let (on_d,): (i64,) = query_as(
            "SELECT count(*) FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.repository_id = $1 AND pm.scope = 'user' AND pm.identity = 'test@sensei.test' \
                AND m.key = 'duplication_ratio' AND pm.computed_on = $2",
        )
        .bind(rid).bind(dday).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(on_d, 1, "the scope=user duplication_ratio row is stamped computed_on=D, identity=the local author");

        // Re-running the now-covered PAST day is a bounded no-op: the covered-skip
        // fires (a historical commit is immutable), so nothing is re-scanned or
        // re-written and the rows are unchanged — never duplicated.
        let again = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), fake).await.unwrap();
        assert_eq!(again, 0, "a covered past day is skipped on re-run (immutable history)");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE repository_id = $1")
            .bind(rid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total, 4, "still exactly 4 rows — the re-run wrote nothing, never a duplicate");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_real_zero_writes_rows() {
        // A real scan with 0 duplicates AND 0 smells over real source lines → BOTH
        // scopes write BOTH rows with value 0.0 (a real zero over a real denominator),
        // never suppressed. 2 metrics × 2 scopes = 4 rows.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(30)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n")]);

        let clean = |_r: &str, _s: &str| {
            Ok(Some(QualityScan {
                total_lines: 120,
                per_file_lines: HashMap::from([("a.rs".to_string(), 120)]),
                per_file_dup_lines: HashMap::new(),
                per_file_maintainability: HashMap::new(),
            }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), clean).await.unwrap();
        assert_eq!(written, 4, "a real 0.0 over a real denominator is still written (repo + user)");
        let daily = daily_rows(pg, &pid).await;
        let dr = daily.iter().find(|r| r.0 == "duplication_ratio").expect("user duplication_ratio row");
        assert!(dr.1.abs() < 1e-9, "value is a real 0.0, not a suppressed row");
        assert_eq!(dr.2["denominator"].as_i64(), Some(120), "real denominator → row written");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_zero_total_lines_writes_no_row() {
        // Never-fabricate: a scan reporting 0 total source lines has no denominator →
        // NO row at either scope (a 0/0 would be a fabricated zero).
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(20)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n")]);

        let empty = |_r: &str, _s: &str| {
            Ok(Some(QualityScan {
                total_lines: 0,
                per_file_lines: HashMap::new(),
                per_file_dup_lines: HashMap::new(),
                per_file_maintainability: HashMap::new(),
            }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), empty).await.unwrap();
        assert_eq!(written, 0, "0 total lines → no denominator → no row");
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE repository_id = $1")
            .bind(rid).fetch_one(pg.pool()).await.unwrap();
        assert_eq!(total, 0, "no rows for a zero-line scan (never fabricated)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    #[tokio::test]
    async fn quality_covered_past_day_is_skipped_but_scanner_untouched() {
        // Bounded work: a PAST day already covered (per repository) by BOTH active
        // quality metrics at scope=repo is settled (a historical commit is immutable) →
        // the scanner is NOT applied and the stored value is untouched. A scanner that
        // returns a DIFFERENT value proves the skip: the pre-covered 0.5 must remain.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(45)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);
        let sha = resolve_commit_as_of(&repo.path().to_string_lossy(), dday).expect("a commit on D");

        // Pre-cover D with BOTH metrics at scope=repo, value 0.5.
        let ids = pg.active_metric_ids("quality").await.unwrap();
        for key in ["duplication_ratio", "module_quality"] {
            let mid = *ids.get(key).expect("active quality metric");
            pg.upsert_project_metric_repo(
                &mid, &rid, "repo", None, Some(&sha), dday, "daily", 0.5,
                &serde_json::json!({"numerator": 1, "denominator": 2, "commit": sha}), "measured")
            .await
            .unwrap();
        }

        let different = |_r: &str, _s: &str| {
            Ok(Some(QualityScan {
                total_lines: 100,
                per_file_lines: HashMap::from([("a.rs".to_string(), 100)]),
                per_file_dup_lines: HashMap::from([("a.rs".to_string(), 90)]),
                per_file_maintainability: HashMap::from([("a.rs".to_string(), 90)]),
            }))
        };
        let written = compute_with_scanner(&ctx, &pid.to_string(), Some(dday), different).await.unwrap();
        assert_eq!(written, 0, "the covered past day is skipped (immutable history) — scanner not applied");

        let (value,): (f64,) = query_as(
            "SELECT pm.value::float8 FROM sensei.project_metrics pm JOIN sensei.metrics m ON m.id = pm.metric_id \
              WHERE pm.repository_id = $1 AND pm.scope = 'repo' AND m.key = 'duplication_ratio' AND pm.computed_on = $2",
        )
        .bind(rid).bind(dday).fetch_one(pg.pool()).await.unwrap();
        assert!((value - 0.5).abs() < 1e-9, "the pre-covered 0.5 remains — the skip fired (not the scanner's 0.9)");

        cleanup_metrics_fixture(pg, &pid, Some(&fid), &[]).await;
    }

    // ── Honest-empty: non-git / real qlty path with no config ────────────────

    #[tokio::test]
    async fn quality_non_git_project_writes_no_rows() {
        // A project whose repo-root folder is NOT a git repo (the synthetic
        // `/_test/metrics-*` path, still assigned a repository row) → no git commit
        // history → NO rows (honest-empty). The scanner is never called because the
        // sampled-anchor set is empty for a non-git root.
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
    async fn quality_real_scan_pins_config_and_measures_pre_config_history() {
        // End-to-end REAL path (worktree + real `run_qlty_scan` incl. config-pinning):
        // the seeded temp repo has NO committed `.qlty`, but `pin_qlty_config` writes the
        // fixed ruler into the scanned worktree — so a repo with real code IS measured
        // (P-B: "pre-config history is measurable, same ruler"). qlty on PATH → real
        // repo-grain rows; qlty absent → honest-empty (never a fabricated score). Also
        // exercises worktree hygiene: no dangling worktree is left behind.
        let ctx = make_ctx().await;
        let pg = ctx.pg();
        let uniq = uuid::Uuid::new_v4();
        let (pid, fid, repo) = seed_git_project_folder(pg, &uniq).await;
        let rid = repository_for_folder(pg, &fid).await;
        let dday = (chrono::Utc::now() - chrono::Duration::days(10)).date_naive();
        git_commit_on_day(repo.path(), &dday.format("%Y-%m-%d").to_string(), &[("a.rs", "1\n2\n3\n")]);

        let written = compute(&ctx, &pid.to_string(), Some(dday)).await.unwrap();
        let (total,): (i64,) = query_as("SELECT count(*) FROM sensei.project_metrics WHERE repository_id = $1")
            .bind(rid).fetch_one(pg.pool()).await.unwrap();

        // qlty is optional (SOFT prereq): gate the assertion on its presence so this
        // passes both locally (qlty installed → measured) and on a bare CI (absent →
        // honest-empty). Config-pinning removed the "no committed .qlty" miss cause, so
        // a miss now means only the CLI is absent. Resolve it THE SAME WAY production
        // does (the shared resolver, incl. ~/.qlty/bin) — NOT a bare `Command::new`,
        // whose PATH-only view is exactly what silently emptied quality in the daemon.
        let qlty_present = sensei_bootstrap::util::which_binary("qlty").is_some();
        if qlty_present {
            assert!(written > 0, "config-pinning lets a repo with no committed .qlty be measured (qlty present)");
            assert_eq!(total as u32, written, "every written quality row is repository-attributed");
        } else {
            assert_eq!(written, 0, "qlty CLI absent → honest-empty (never a fabricated score)");
            assert_eq!(total, 0, "no rows without the qlty CLI");
        }

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
