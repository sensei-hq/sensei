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

/// Complexity of every file at `root`'s HEAD, measured in a temporary worktree
/// against the pinned ruler — never in the working copy, which may carry no qlty
/// config and must not be written to. `Ok(None)`: qlty absent or the worktree
/// could not be made (honest-empty).
pub(crate) fn measure_head(
    root: &str,
) -> Result<Option<std::collections::HashMap<String, FileComplexity>>, String> {
    scan_at_commit(root, "HEAD", |wt| {
        pin_qlty_config(wt)?;
        Ok(run_qlty(wt, &["metrics", "--all", "--functions", "--quiet", "--no-upgrade-check"])
            .map(|out| parse_function_metrics(&out)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
