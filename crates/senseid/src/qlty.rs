//! `qlty`, run locally: the subprocess, its table output, and a pinned scan of
//! a git worktree. Shared by the quality metric and the Code map's complexity
//! (user, 2026-10-27: rely on local qlty rather than re-deriving what it
//! measures). qlty is a SOFT prerequisite: absent, every caller is honest-empty.

use std::path::Path;

/// Run `qlty` with `args` in `dir` and return stdout, or `None` when `qlty` is not
/// on `PATH`, the directory has no `.qlty` config, or the command otherwise fails —
/// an honest "no qlty data" (the caller writes no row), never a fabricated value.
/// `qlty` is an OPTIONAL tool (kept SOFT — not a hard bootstrap prereq): absent → the
/// metric is honest-empty and the daemon runs fine. Mirrors the shell-out-and-
/// tolerate-absence discipline of [`crate::tasks::handlers::metrics::churn::run_git`]. Note `qlty smells`/`qlty
/// metrics` exit `0` even when they find smells, so a non-success status is a genuine
/// failure (missing config / missing tool), not "found findings".
pub(crate) fn run_qlty(dir: &Path, args: &[&str]) -> Option<String> {
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
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Strip ANSI/SGR escape sequences from `qlty metrics`' colored table so the TOTAL
/// row is parseable. `qlty` emits `\x1b[…m` resets around every cell regardless of
/// `NO_COLOR`, so a strip is required; each sequence runs from `ESC` to its final
/// letter byte. Pure.
pub(crate) fn strip_ansi(s: &str) -> String {
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

/// The FIXED `qlty` ruler pinned into every scanned worktree (P-B, spec §7). Every
/// sampled commit is measured against THIS config — never its in-tree `.qlty`, which
/// is absent on pre-config history and can drift between commits — so the trend is
/// comparable ("the same ruler"). Deliberately plugin-LESS: `qlty metrics`/`smells`
/// are built-in tree-sitter analyses, while the linter `[[plugin]]`s are for `qlty
/// check` and would force a per-scan network source fetch. A STABLE snapshot —
/// changing it re-bases the ruler, so it is versioned here deliberately rather than
/// tracked to the live repo `.qlty`.
pub(crate) const PINNED_QLTY_CONFIG: &str = r#"config_version = "0"
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
pub(crate) fn pin_qlty_config(wt: &Path) -> Result<(), String> {
    let dir = wt.join(".qlty");
    std::fs::create_dir_all(&dir).map_err(|e| format!("quality: pin .qlty dir: {e}"))?;
    std::fs::write(dir.join("qlty.toml"), PINNED_QLTY_CONFIG)
        .map_err(|e| format!("quality: pin qlty.toml: {e}"))
}

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
        if crate::tasks::handlers::metrics::churn::run_git(
            &self.root,
            &["worktree", "remove", "--force", &self.path],
        )
        .is_none()
        {
            tracing::warn!(path = %self.path, "quality: git worktree remove failed — pruning");
            let _ =
                crate::tasks::handlers::metrics::churn::run_git(&self.root, &["worktree", "prune"]);
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
pub(crate) fn scan_at_commit<T, F>(root: &str, sha: &str, scan: F) -> Result<Option<T>, String>
where
    F: FnOnce(&Path) -> Result<Option<T>, String>,
{
    let base = std::env::temp_dir().join(format!("sensei-qlty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&base).map_err(|e| format!("quality: worktree base dir: {e}"))?;
    let wt = base.join("wt");
    let wt_str = wt.to_string_lossy().into_owned();
    if crate::tasks::handlers::metrics::churn::run_git(
        root,
        &["worktree", "add", "--detach", "-q", &wt_str, sha],
    )
    .is_none()
    {
        tracing::warn!(root, sha, "quality: git worktree add failed — no quality row");
        let _ = std::fs::remove_dir_all(&base);
        return Ok(None);
    }
    let _guard = WorktreeGuard { root: root.to_string(), path: wt_str, base };
    scan(&wt)
    // `_guard` drops here → `worktree remove --force` + temp parent dir removed.
}

/// A file's functions as `qlty metrics --functions` counts them: how many, how
/// many exceed [`COMPLEX_CYCLO`] cyclomatic complexity, and the largest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileComplexity {
    pub functions: i32,
    pub complex: i32,
    pub max_cyclo: i32,
}

/// A function over this cyclomatic complexity counts as complex (M17: the
/// Complexity lens tints by the share of functions over 10).
pub(crate) const COMPLEX_CYCLO: i32 = 10;

/// Per-file complexity from `qlty metrics --all --functions`: a path line, then
/// that file's function table. Columns are found by their header names. Pure.
pub(crate) fn parse_function_metrics(
    stdout: &str,
) -> std::collections::HashMap<String, FileComplexity> {
    let clean = strip_ansi(stdout);
    let mut out = std::collections::HashMap::new();
    let mut file: Option<String> = None;
    let mut cyclo_col: Option<usize> = None;
    for line in clean.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('-') {
            continue;
        }
        if !line.contains('|') {
            // A path line opens the next file's table.
            file = Some(trimmed.to_string());
            cyclo_col = None;
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let Some(col) = cyclo_col else {
            cyclo_col = cells.iter().position(|c| *c == "cyclo");
            continue;
        };
        let (Some(path), Some(cyclo)) =
            (file.as_ref(), cells.get(col).and_then(|v| v.parse::<i32>().ok()))
        else {
            continue;
        };
        let f: &mut FileComplexity = out.entry(path.clone()).or_default();
        f.functions += 1;
        f.complex += i32::from(cyclo > COMPLEX_CYCLO);
        f.max_cyclo = f.max_cyclo.max(cyclo);
    }
    out
}

/// Whether a SARIF result from `qlty smells` is a duplication finding — by its
/// `duplication` taxon, with a `ruleId` substring fallback.
pub(crate) fn is_duplication(result: &serde_json::Value) -> bool {
    let by_taxon = result["taxa"]
        .as_array()
        .is_some_and(|taxa| taxa.iter().any(|t| t["id"].as_str() == Some("duplication")));
    by_taxon
        || result["ruleId"].as_str().is_some_and(|r| {
            r.contains("identical") || r.contains("similar") || r.contains("duplicat")
        })
}

/// One copy of duplicated code: a file and its line range.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Span {
    pub path: String,
    pub start: i64,
    pub end: i64,
}

/// Code qlty found more than once: its copies, how big (qlty's `mass`), how
/// many lines, and whether the copies are identical or only similar.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub identical: bool,
    pub mass: f64,
    pub lines: i64,
    pub copies: Vec<Span>,
}

/// The duplicate groups in `qlty smells --sarif` output, largest first. qlty
/// reports a duplicate once per copy, each naming the others, so findings are
/// merged by their structural hash and every copy is kept once. Pure.
pub(crate) fn parse_duplicates(sarif: &str) -> Result<Vec<DuplicateGroup>, String> {
    let v: serde_json::Value =
        serde_json::from_str(sarif).map_err(|e| format!("qlty: bad SARIF: {e}"))?;
    let mut groups: std::collections::BTreeMap<String, DuplicateGroup> = Default::default();
    let span = |loc: &serde_json::Value| {
        let pl = &loc["physicalLocation"];
        Some(Span {
            path: pl["artifactLocation"]["uri"].as_str()?.to_string(),
            start: pl["region"]["startLine"].as_i64()?,
            end: pl["region"]["endLine"].as_i64()?,
        })
    };
    let results = v["runs"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|r| r["results"].as_array().into_iter().flatten());
    for result in results.filter(|r| is_duplication(r)) {
        let props = &result["properties"];
        let Some(hash) = props["structural_hash"].as_str() else { continue };
        let group = groups.entry(hash.to_string()).or_insert_with(|| DuplicateGroup {
            hash: hash.to_string(),
            identical: props["identical"].as_bool().unwrap_or(false),
            mass: props["mass"].as_f64().unwrap_or(0.0),
            lines: 0,
            copies: Vec::new(),
        });
        let locs = result["locations"].as_array().into_iter().flatten();
        let related = result["relatedLocations"].as_array().into_iter().flatten();
        for copy in locs.chain(related).filter_map(span) {
            group.lines = group.lines.max(copy.end - copy.start + 1);
            if !group.copies.contains(&copy) {
                group.copies.push(copy);
            }
        }
    }
    let mut out: Vec<DuplicateGroup> = groups
        .into_values()
        .filter(|g| g.copies.len() > 1)
        .map(|mut g| {
            g.copies.sort();
            g
        })
        .collect();
    out.sort_by(|a, b| b.mass.total_cmp(&a.mass).then_with(|| a.hash.cmp(&b.hash)));
    Ok(out)
}

/// What one qlty run at a repository's HEAD measures: each file's complexity,
/// and the code it finds more than once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeadMeasure {
    pub files: std::collections::HashMap<String, FileComplexity>,
    pub duplicates: Vec<DuplicateGroup>,
}

/// Complexity and duplication at `root`'s HEAD, measured in a temporary
/// worktree against the pinned ruler — never in the working copy, which may
/// carry no qlty config and must not be written to. `Ok(None)`: qlty absent or
/// the worktree could not be made (honest-empty).
pub(crate) fn measure_head(root: &str) -> Result<Option<HeadMeasure>, String> {
    scan_at_commit(root, "HEAD", |wt| {
        pin_qlty_config(wt)?;
        let Some(metrics) =
            run_qlty(wt, &["metrics", "--all", "--functions", "--quiet", "--no-upgrade-check"])
        else {
            return Ok(None);
        };
        let Some(sarif) = run_qlty(wt, &["smells", "--all", "--sarif", "--no-upgrade-check"])
        else {
            return Ok(None);
        };
        Ok(Some(HeadMeasure {
            files: parse_function_metrics(&metrics),
            duplicates: parse_duplicates(&sarif)?,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(
        rule: &str,
        hash: &str,
        mass: f64,
        at: (&str, i64, i64),
        also: &[(&str, i64, i64)],
    ) -> serde_json::Value {
        let loc = |(uri, s, e): (&str, i64, i64)| serde_json::json!({"physicalLocation": {"artifactLocation": {"uri": uri}, "region": {"startLine": s, "endLine": e}}});
        serde_json::json!({
            "ruleId": rule,
            "taxa": [{"id": "duplication"}],
            "locations": [loc(at)],
            "relatedLocations": also.iter().map(|l| loc(*l)).collect::<Vec<_>>(),
            "properties": {"mass": mass, "structural_hash": hash, "identical": rule.ends_with("identical-code")},
        })
    }

    /// qlty reports a duplicate once per copy, each naming the others: one
    /// group per structural hash, every copy once, largest first. A finding
    /// that is not duplication is not a group.
    #[test]
    fn duplicate_findings_become_groups_of_their_copies() {
        let sarif = serde_json::json!({"runs": [{"results": [
            finding("qlty:similar-code", "aa", 87.0, ("src/a.rs", 10, 25), &[("src/b.rs", 40, 55)]),
            finding("qlty:similar-code", "aa", 87.0, ("src/b.rs", 40, 55), &[("src/a.rs", 10, 25)]),
            finding("qlty:identical-code", "bb", 120.0, ("src/c.rs", 1, 30), &[("src/c.rs", 100, 129), ("src/d.rs", 5, 34)]),
            {"ruleId": "qlty:function-complexity", "locations": [], "properties": {}},
        ]}]})
        .to_string();
        let groups = parse_duplicates(&sarif).unwrap();
        assert_eq!(
            groups
                .iter()
                .map(|g| (g.hash.as_str(), g.identical, g.copies.len(), g.lines))
                .collect::<Vec<_>>(),
            [("bb", true, 3, 30), ("aa", false, 2, 16)]
        );
        assert_eq!(groups[1].copies[0], Span { path: "src/a.rs".into(), start: 10, end: 25 });
        assert!(parse_duplicates("not json").is_err());
        assert!(parse_duplicates(r#"{"runs":[{}]}"#).unwrap().is_empty());
    }

    #[test]
    fn function_tables_become_per_file_counts() {
        let out = "\
crates/x/a.rs

\u{1b}[0m function \u{1b}[0m| fields | cyclo | cognitive | lines | loc
----------+--------+-------+-----------+-------+-----
 f        |      0 |    12 |         3 |    10 |  10
 g        |      0 |     2 |         0 |     3 |   3
 h        |      0 |    10 |         0 |     3 |   3

src/b.ts

 function | fields | cyclo | cognitive | lines | loc
----------+--------+-------+-----------+-------+-----
 k        |      0 |    31 |        20 |    90 |  80
";
        let got = parse_function_metrics(out);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got["crates/x/a.rs"],
            FileComplexity { functions: 3, complex: 1, max_cyclo: 12 }
        );
        assert_eq!(got["src/b.ts"], FileComplexity { functions: 1, complex: 1, max_cyclo: 31 });
    }
}
