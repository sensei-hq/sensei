//! Stage 1 — scan root: which repositories exist.
//!
//! Spec: `docs/spec/indexer/01-scan-root.md`.
//!
//! Finds repository roots under a scanned directory and nothing else. This
//! stage knows which repos exist; it knows NOTHING about files. That division
//! is the point of the two-stage split — the moment scan root opens a repo's
//! contents it has become scan repo, and the exclusion rules have two homes.
//!
//! [`find_git_roots`] is PURE of the database by construction: it takes a path
//! and returns roots. `save_repo` (the IO half) is separate so discovery is
//! testable with no database at all — the property the whole stage ordering
//! was chosen for.
//!
//! NOTE ON THE WALKER. `tasks::handlers::helpers::build_walker` is the shared
//! ignore-aware walker and is deliberately NOT used here: it sets
//! `hidden(true)`, so it never yields `.git`. Root discovery is the one walk
//! that must see hidden entries, which is why it does its own traversal
//! instead of reusing that one.

use std::path::{Path, PathBuf};

/// How a repository root's `.git` presents on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitKind {
    /// An ordinary checkout: `.git` is a directory.
    Directory,
    /// A SUBMODULE checkout: `.git` is a FILE pointing into the parent's
    /// `.git/modules/`. Globbing only for directories misses every one of
    /// these, which is why [`GitKind`] exists rather than a bare path list.
    File,
}

/// A discovered repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRoot {
    pub abs_path: PathBuf,
    pub git_kind: GitKind,
}

/// Directory names pruned DURING traversal (S2).
///
/// Applied as the walk descends, not as a filter afterwards: descending into
/// `node_modules` to discover you should not have is the cost this stage
/// exists to avoid.
#[derive(Debug, Clone)]
pub struct RootExclusions {
    names: Vec<String>,
}

impl RootExclusions {
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { names: names.into_iter().map(Into::into).collect() }
    }

    /// The directories a root scan should never descend into. Deliberately
    /// short: this is not the file-level exclusion list (that is stage 2's,
    /// and it comes from `.gitignore` plus `DEFAULT_EXCLUDE_GLOBS`). A root
    /// scan only needs to avoid the trees that are large AND cannot contain a
    /// repository worth indexing.
    pub fn defaults() -> Self {
        Self::new(["node_modules", "target", "dist", "build", ".next", ".svelte-kit", "vendor"])
    }

    fn excludes(&self, dir_name: &str) -> bool {
        self.names.iter().any(|n| n == dir_name)
    }
}

impl Default for RootExclusions {
    fn default() -> Self {
        Self::defaults()
    }
}

/// Why a directory could not be searched. Recorded rather than swallowed: a
/// tree we could not read is a different fact from a tree with no repos in it
/// (R6 — absence is not evidence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreadableDir {
    pub path: PathBuf,
    pub reason: String,
}

/// What one root scan found.
#[derive(Debug, Clone, Default)]
pub struct RootScan {
    pub roots: Vec<RepoRoot>,
    /// Directories that could not be read. NOT an error for the scan as a
    /// whole — one unreadable subtree must not cost every other repo.
    pub unreadable: Vec<UnreadableDir>,
    /// Directories skipped by [`RootExclusions`], counted so the delta against
    /// a previous scan is explainable rather than mysterious.
    pub excluded: usize,
}

/// Find repository roots under `dir` (S1–S3). Filesystem reads only — no
/// database, by design.
///
/// - A `.git` entry counts whether it is a DIRECTORY or a FILE (S1).
/// - Exclusions are applied while descending (S2).
/// - Once a root is found the walk does NOT descend into it (S3): whatever is
///   inside belongs to stage 2, including any nested repo, which stage 2
///   discovers and enqueues itself.
pub fn find_git_roots(dir: &Path, exclusions: &RootExclusions) -> RootScan {
    let mut scan = RootScan::default();
    walk(dir, exclusions, &mut scan);
    scan.roots.sort_by(|a, b| a.abs_path.cmp(&b.abs_path));
    scan
}

fn walk(dir: &Path, exclusions: &RootExclusions, scan: &mut RootScan) {
    // A root announces itself by its `.git` entry. Check before descending so
    // S3's prune is structural rather than a later filter.
    let git = dir.join(".git");
    match git.metadata() {
        Ok(md) if md.is_dir() => {
            scan.roots.push(RepoRoot { abs_path: dir.to_path_buf(), git_kind: GitKind::Directory });
            return;
        }
        Ok(_) => {
            // A `.git` FILE — a submodule checkout. Still a root (S1).
            scan.roots.push(RepoRoot { abs_path: dir.to_path_buf(), git_kind: GitKind::File });
            return;
        }
        Err(_) => {}
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            scan.unreadable
                .push(UnreadableDir { path: dir.to_path_buf(), reason: e.to_string() });
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // `file_type()` does NOT follow symlinks, so a symlink loop cannot
        // recurse: a link to an ancestor is not a directory here.
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        if exclusions.excludes(&name) {
            scan.excluded += 1;
            continue;
        }
        walk(&path, exclusions, scan);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_repo(root: &Path, rel: &str, kind: GitKind) {
        let dir = root.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        match kind {
            GitKind::Directory => std::fs::create_dir_all(dir.join(".git")).unwrap(),
            GitKind::File => std::fs::write(dir.join(".git"), "gitdir: ../.git/modules/x").unwrap(),
        }
    }

    fn paths(scan: &RootScan, base: &Path) -> Vec<String> {
        scan.roots
            .iter()
            .map(|r| r.abs_path.strip_prefix(base).unwrap().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn finds_a_git_directory_and_a_git_file() {
        // S1. A submodule checkout has a `.git` FILE, not a directory. Globbing
        // only for directories misses every checked-out submodule.
        let t = tempfile::tempdir().unwrap();
        mk_repo(t.path(), "plain", GitKind::Directory);
        mk_repo(t.path(), "sub", GitKind::File);

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert_eq!(paths(&scan, t.path()), vec!["plain", "sub"]);
        assert_eq!(scan.roots[0].git_kind, GitKind::Directory);
        assert_eq!(scan.roots[1].git_kind, GitKind::File, "a .git FILE is still a root");
    }

    #[test]
    fn a_repo_inside_an_excluded_directory_is_not_returned() {
        // S2. Exclusions apply DURING traversal. A vendored dependency with its
        // own .git must not become a tracked repository.
        let t = tempfile::tempdir().unwrap();
        mk_repo(t.path(), "app", GitKind::Directory);
        mk_repo(t.path(), "node_modules/some-dep", GitKind::Directory);

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert_eq!(paths(&scan, t.path()), vec!["app"]);
        assert_eq!(scan.excluded, 1, "the skip is counted, not silent");
    }

    #[test]
    fn does_not_descend_below_a_found_root() {
        // S3. A nested repo inside a root belongs to stage 2, which discovers
        // and enqueues it. Returning it here would give two owners of the same
        // subtree.
        let t = tempfile::tempdir().unwrap();
        mk_repo(t.path(), "outer", GitKind::Directory);
        mk_repo(t.path(), "outer/nested", GitKind::Directory);

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert_eq!(paths(&scan, t.path()), vec!["outer"], "nested root is stage 2's");
    }

    #[test]
    fn a_directory_with_no_repos_yields_none_and_is_not_an_error() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("just/files")).unwrap();
        std::fs::write(t.path().join("just/files/a.txt"), "x").unwrap();

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert!(scan.roots.is_empty());
        assert!(scan.unreadable.is_empty(), "empty is not the same as unreadable");
    }

    #[test]
    fn a_symlink_to_an_ancestor_does_not_recurse() {
        // Bounded traversal. `file_type()` does not follow symlinks, so a loop
        // is simply not a directory to descend into.
        let t = tempfile::tempdir().unwrap();
        mk_repo(t.path(), "app", GitKind::Directory);
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.path(), t.path().join("loop")).unwrap();

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert_eq!(paths(&scan, t.path()), vec!["app"]);
    }

    #[test]
    fn the_scanned_directory_can_itself_be_a_root() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join(".git")).unwrap();

        let scan = find_git_roots(t.path(), &RootExclusions::defaults());

        assert_eq!(scan.roots.len(), 1);
        assert_eq!(scan.roots[0].abs_path, t.path());
    }
}
