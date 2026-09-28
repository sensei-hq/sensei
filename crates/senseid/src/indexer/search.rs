//! ONE filesystem search, for every purpose that needs one.
//!
//! Root discovery wants `.git` entries and must SEE hidden ones. A repo walk
//! wants every source file and must honour `.gitignore`. A manifest pass wants
//! `Cargo.toml` and `package.json`. Those are the same traversal with different
//! parameters, and the alternative — a hand-rolled walk per caller — is how the
//! scanner and the fs-watcher last disagreed about what belongs in the index.
//!
//! Built on `ignore::WalkBuilder` (ripgrep's walker), which is already a
//! dependency and already implements the parts that are easy to get wrong.
//!
//! ## A MATCH DOES NOT STOP THE WALK
//!
//! Matching and descent are separate concerns in this walker, and conflating
//! them is the mistake this note exists to prevent. `patterns` decide what is
//! YIELDED; only the skip rules below decide what is ENTERED. Searching for
//! `.git` therefore does not stop at `.git` — without a skip rule the walk
//! would descend into every repository's object store. `filter_entry` cannot
//! express "yield this directory but do not enter it": returning `false`
//! suppresses the entry as well. The caller says so with a PRUNE instead —
//! `**/.git/**` skips the contents and leaves `.git` matchable.
//!
//! ## `.gitignore` is a FLAG, never an exclusion entry
//!
//! It is tempting to flatten `.gitignore` into the exclusion list and have one
//! input. It does not survive contact with what `.gitignore` actually is:
//! hierarchical (every directory may carry one), scoped to the directory it
//! sits in, and NEGATABLE (`!keep-this` re-includes a path an earlier rule
//! excluded). A flat list of prefixes can express none of that, so flattening
//! silently drops negations — re-including exactly the files somebody took the
//! trouble to un-ignore. `ignore` implements the real semantics, so this is a
//! `bool` that turns those semantics on.
//!
//! ## THERE IS NO DEPTH BOUND
//!
//! Deliberately. A depth guard does not make a walk safe, it makes it WRONG:
//! everything past the bound is silently missing, and missing reads exactly
//! like absent. The predecessor bounded root discovery at 8, so a repository
//! nested deeper simply did not exist as far as the index was concerned.
//! Cost is controlled by pruning subtrees that cannot contain what we want,
//! which is honest, rather than by truncating the ones that can.
//!
//! ## Three ways a directory is skipped, and they are not the same
//!
//! * `exclusions` — the watch root's own list. Never descended, never matched.
//!   The user said this subtree is not ours; nothing overrules that.
//! * `prunes` — a cost guard (`node_modules`, `**/.git/**`), as GLOBS. Never
//!   descended. Globs because `**/.git/**` must skip an object store while
//!   leaving the `.git` entry itself matchable — a segment rule cannot.
//! * `gitignore` — the repo's own rules, with full hierarchy and negation.

// No caller yet: this is the root scanner's new contract, landing ahead of the
// stage-1 wiring so it can be proven on its own. The allow goes when
// `TaskKind::ScanRoot` calls it — it is not a licence for dead code.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::tasks::handlers::scan_logic::is_excluded;

/// Whether a match is a file or a directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryKind {
    File,
    Dir,
}

/// Which kinds a search returns. The traversal is identical either way — this
/// filters what comes back, so a caller that wants both pays for one walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kinds {
    Files,
    Dirs,
    Both,
}

impl Kinds {
    fn wants(self, kind: EntryKind) -> bool {
        match self {
            Kinds::Both => true,
            Kinds::Files => kind == EntryKind::File,
            Kinds::Dirs => kind == EntryKind::Dir,
        }
    }
}

/// One match, relative to the search root.
///
/// RELATIVE because that is the grain `sensei.files` is keyed by and the grain
/// a caller compares against a previous run. An absolute path here would make
/// every caller strip the prefix, and one of them would forget.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Found {
    pub rel_path: PathBuf,
    pub kind: EntryKind,
}

impl Found {
    /// The absolute path, for a caller that needs to open the entry.
    pub fn abs(&self, root: &Path) -> PathBuf {
        root.join(&self.rel_path)
    }
}

/// A directory the walk could not read.
///
/// The overwhelmingly common cause is ACCESS RESTRICTION, and on macOS that is
/// routine rather than exotic: `~/Documents`, `~/Desktop` and `~/Downloads` are
/// TCC-protected, so a daemon without Full Disk Access gets `Operation not
/// permitted` from `read_dir` and sees an EMPTY directory. Unmounted network
/// volumes and root-owned trees do the same.
///
/// It is recorded rather than swallowed because the two readings are opposite
/// and indistinguishable from the result alone: "this tree holds nothing" and
/// "this tree could not be opened". A scan that silently returned the first for
/// the second would report every repository under it as REMOVED, and a removal
/// cascades. See [`SearchResult::is_complete`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// `None` when the walker reported an error it could not attribute to a
    /// path. Not defaulted to the root: an invented path is worse than an
    /// absent one.
    pub path: Option<PathBuf>,
    pub reason: String,
}

/// What one search found, and what it could not see.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResult {
    /// Matches, sorted, so two runs over an unchanged tree are comparable.
    pub found: Vec<Found>,
    pub unreadable: Vec<Unreadable>,
}

impl SearchResult {
    /// Whether this search SAW the whole tree it was pointed at.
    ///
    /// The gate on treating absence as deletion. A caller that diffs against a
    /// previous run must not derive removals from an incomplete search — a
    /// revoked permission would otherwise read as "every repository under here
    /// was deleted".
    pub fn is_complete(&self) -> bool {
        self.unreadable.is_empty()
    }
}

/// What to search for. See the module docs for why `gitignore` is a flag,
/// why `exclusions` and `prunes` are separate, and why there is no depth bound.
#[derive(Debug, Clone)]
pub struct SearchSpec<'a> {
    pub root: &'a Path,
    /// Globs an entry must match, tested against BOTH its file name and its
    /// path relative to the root, so `.git` and `src/**/*.rs` both work.
    /// EMPTY MEANS EVERYTHING — an empty pattern list is not an empty result.
    pub patterns: &'a [String],
    pub kinds: Kinds,
    /// The watch root's own exclusions. Never descended, never matched.
    pub exclusions: &'a [String],
    /// Cost guards, as GLOBS (matched like `patterns`). Never descended.
    /// Glob rather than segment form because the rule that matters cannot be
    /// said any other way: `**/.git/**` skips an object store while leaving the
    /// `.git` entry itself matchable.
    pub prunes: &'a [String],
    /// Honour `.gitignore`, `.ignore`, the global gitignore and
    /// `.git/info/exclude`, with their real hierarchy and negation.
    pub gitignore: bool,
    /// Yield dotfile entries. Root discovery needs this to see `.git`.
    pub hidden: bool,
}

impl<'a> SearchSpec<'a> {
    /// Everything under `root`, honouring `.gitignore` — the repo-walk default.
    pub fn under(root: &'a Path) -> Self {
        Self {
            root,
            patterns: &[],
            kinds: Kinds::Both,
            exclusions: &[],
            prunes: &[],
            gitignore: true,
            hidden: false,
        }
    }
}

/// Run one search.
///
/// A malformed glob is an ERROR rather than a silent empty result: a pattern
/// that never matches and a pattern that could not be compiled are different
/// facts, and returning `[]` for the second reads as "this tree is empty".
/// A directory that could not be READ is not an error — the rest of the tree is
/// still real — so it lands in [`SearchResult::unreadable`] instead.
pub fn search(spec: &SearchSpec) -> Result<SearchResult, String> {
    let globs = compile(spec.patterns)?;

    let mut builder = ignore::WalkBuilder::new(spec.root);
    builder
        // `ignore`'s flag means "SKIP hidden", so it is the inverse of ours.
        .hidden(!spec.hidden)
        .git_ignore(spec.gitignore)
        .git_global(spec.gitignore)
        .git_exclude(spec.gitignore)
        // Still apply ignore files in a directory that is not a git repo.
        .require_git(false)
        // A symlinked tree is already reached by its real path; following the
        // link indexes the same files twice.
        .follow_links(false);

    // Cloned into the closure: `filter_entry` outlives this call.
    let exclusions: Arc<Vec<String>> = Arc::new(spec.exclusions.to_vec());
    let prunes = Arc::new(compile(spec.prunes)?);
    let root = spec.root.to_path_buf();
    builder.filter_entry(move |entry| {
        let path = entry.path();
        // The root itself is never filtered out — filtering it yields nothing
        // at all, which reads as an empty tree.
        if path == root {
            return true;
        }
        if is_excluded(path, &exclusions) {
            return false;
        }
        // Prunes are GLOBS, so `**/.git/**` skips an object store while leaving
        // the `.git` entry itself matchable — the thing a segment rule cannot
        // say.
        //
        // NOT `matches`, whose empty case means MATCH EVERYTHING. That reading
        // is right for `patterns` and exactly inverted here: an empty prune
        // list must prune NOTHING. Sharing the helper pruned every tree that
        // passed no prunes.
        let Some(globs) = prunes.as_ref() else { return true };
        // A path that cannot be made relative is outside the search and is not
        // ours to prune.
        match path.strip_prefix(&root) {
            Ok(rel) => !globs.is_match(rel) && !path.file_name().is_some_and(|n| globs.is_match(n)),
            Err(_) => true,
        }
    });

    let mut found = BTreeSet::new();
    let mut unreadable = Vec::new();
    for entry in builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                unreadable.push(Unreadable { path: error_path(&e), reason: e.to_string() });
                continue;
            }
        };
        let path = entry.path();
        if path == spec.root {
            continue;
        }
        let kind = match entry.file_type() {
            Some(t) if t.is_dir() => EntryKind::Dir,
            Some(t) if t.is_file() => EntryKind::File,
            // Neither a file nor a directory (a socket, a symlink we do not
            // follow) is not something any caller here can open.
            _ => continue,
        };
        if !spec.kinds.wants(kind) {
            continue;
        }
        let Ok(rel) = path.strip_prefix(spec.root) else { continue };
        if !matches(&globs, rel, path) {
            continue;
        }
        found.insert(Found { rel_path: rel.to_path_buf(), kind });
    }
    Ok(SearchResult { found: found.into_iter().collect(), unreadable })
}

/// The path an `ignore` error is about, unwrapping the variants that nest one.
/// `None` rather than a guess when the walker could not attribute it.
fn error_path(e: &ignore::Error) -> Option<PathBuf> {
    match e {
        ignore::Error::WithPath { path, .. } => Some(path.clone()),
        ignore::Error::WithDepth { err, .. } | ignore::Error::WithLineNumber { err, .. } => {
            error_path(err)
        }
        ignore::Error::Partial(errs) => errs.iter().find_map(error_path),
        ignore::Error::Loop { child, .. } => Some(child.clone()),
        _ => None,
    }
}

/// `None` when no patterns were given — which means match everything, and is
/// distinct from a globset that matches nothing.
fn compile(patterns: &[String]) -> Result<Option<GlobSet>, String> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        b.add(Glob::new(p).map_err(|e| format!("bad glob {p:?}: {e}"))?);
    }
    b.build().map(Some).map_err(|e| e.to_string())
}

/// Matched against the file NAME and the RELATIVE path, so a caller can write
/// either `.git` or `crates/*/Cargo.toml` and get what they meant.
fn matches(globs: &Option<GlobSet>, rel: &Path, full: &Path) -> bool {
    let Some(globs) = globs else { return true };
    if globs.is_match(rel) {
        return true;
    }
    full.file_name().is_some_and(|n| globs.is_match(Path::new(n)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn rels(r: &SearchResult) -> Vec<String> {
        r.found.iter().map(|f| f.rel_path.to_string_lossy().to_string()).collect()
    }

    fn spec<'a>(root: &'a Path, patterns: &'a [String]) -> SearchSpec<'a> {
        SearchSpec { patterns, gitignore: false, ..SearchSpec::under(root) }
    }

    fn pats(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn an_empty_pattern_list_matches_everything() {
        // Empty means EVERYTHING. A search that read it as "match nothing"
        // would return [] for a tree full of files.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "a.rs", "");
        write(t.path(), "dir/b.txt", "");

        let r = search(&spec(t.path(), &[])).unwrap();

        assert_eq!(rels(&r), vec!["a.rs", "dir", "dir/b.txt"]);
    }

    #[test]
    fn a_bare_name_pattern_matches_at_any_depth() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "Cargo.toml", "");
        write(t.path(), "crates/one/Cargo.toml", "");
        write(t.path(), "crates/one/src/lib.rs", "");

        let p = pats(&["Cargo.toml"]);
        let r = search(&spec(t.path(), &p)).unwrap();

        assert_eq!(rels(&r), vec!["Cargo.toml", "crates/one/Cargo.toml"]);
    }

    #[test]
    fn a_path_glob_matches_the_relative_path() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "src/a.rs", "");
        write(t.path(), "src/deep/b.rs", "");
        write(t.path(), "other/c.rs", "");

        let p = pats(&["src/**/*.rs"]);
        let r = search(&spec(t.path(), &p)).unwrap();

        assert_eq!(rels(&r), vec!["src/a.rs", "src/deep/b.rs"]);
    }

    #[test]
    fn kinds_filters_without_changing_the_traversal() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "dir/a.rs", "");

        let files = search(&SearchSpec { kinds: Kinds::Files, ..spec(t.path(), &[]) }).unwrap();
        let dirs = search(&SearchSpec { kinds: Kinds::Dirs, ..spec(t.path(), &[]) }).unwrap();

        assert_eq!(rels(&files), vec!["dir/a.rs"]);
        assert_eq!(rels(&dirs), vec!["dir"]);
        assert!(files.found.iter().all(|f| f.kind == EntryKind::File));
        assert!(dirs.found.iter().all(|f| f.kind == EntryKind::Dir));
    }

    #[test]
    fn hidden_entries_are_visible_only_when_asked_for() {
        // Root discovery needs `.git`; a repo walk must not see it.
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("repo/.git")).unwrap();
        write(t.path(), "repo/src.rs", "");

        let p = pats(&[".git"]);
        let without = search(&spec(t.path(), &p)).unwrap();
        let with = search(&SearchSpec { hidden: true, ..spec(t.path(), &p) }).unwrap();

        assert!(without.found.is_empty(), "hidden entries stay hidden by default");
        assert_eq!(rels(&with), vec!["repo/.git"]);
    }

    #[test]
    fn matching_dot_git_does_not_walk_its_contents() {
        // THE RULE. A match does not stop descent, so without the git-internals
        // skip this walk enters every repository's object store. The `.git`
        // entry is still returned — only its contents are unreachable.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "repo/.git/objects/ab/deadbeef", "");
        write(t.path(), "repo/.git/HEAD", "ref: refs/heads/main");
        write(t.path(), "repo/src.rs", "");

        let prunes = pats(&["**/.git/**"]);
        let all =
            search(&SearchSpec { hidden: true, prunes: &prunes, ..spec(t.path(), &[]) }).unwrap();

        let names = rels(&all);
        assert!(names.contains(&"repo/.git".to_string()), "the .git entry itself is matchable");
        assert!(
            !names.iter().any(|n| n.contains(".git/")),
            "nothing inside .git is walked, got {names:?}"
        );
    }

    #[test]
    fn a_worktree_dot_git_file_is_matched_like_a_directory() {
        // A submodule or linked worktree stores `.git` as a FILE. It has no
        // contents to skip and must still be found.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "sub/.git", "gitdir: ../.git/modules/sub");

        let p = pats(&[".git"]);
        let r = search(&SearchSpec { hidden: true, ..spec(t.path(), &p) }).unwrap();

        assert_eq!(rels(&r), vec!["sub/.git"]);
        assert_eq!(r.found[0].kind, EntryKind::File);
    }

    #[test]
    fn there_is_no_depth_bound() {
        // A depth guard does not make a walk safe, it makes it silently wrong.
        // The predecessor bounded root discovery at 8.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "a/b/c/d/e/f/g/h/i/j/k/deep.rs", "");

        let r = search(&SearchSpec { kinds: Kinds::Files, ..spec(t.path(), &[]) }).unwrap();

        assert_eq!(rels(&r), vec!["a/b/c/d/e/f/g/h/i/j/k/deep.rs"]);
    }

    #[test]
    fn exclusions_prune_the_subtree_and_are_not_matched() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "kept/a.rs", "");
        write(t.path(), "excluded-path-a/b.rs", "");

        let excl = vec![t.path().join("excluded-path-a").to_string_lossy().to_string()];
        let r = search(&SearchSpec { exclusions: &excl, ..spec(t.path(), &[]) }).unwrap();

        assert_eq!(rels(&r), vec!["kept", "kept/a.rs"]);
    }

    #[test]
    fn a_prune_glob_skips_a_subtree() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "src/a.rs", "");
        write(t.path(), "node_modules/dep/b.rs", "");

        let prunes = pats(&["node_modules"]);
        let r = search(&SearchSpec { prunes: &prunes, ..spec(t.path(), &[]) }).unwrap();

        assert_eq!(rels(&r), vec!["src", "src/a.rs"]);
    }

    #[test]
    fn a_prune_is_boundary_safe_and_does_not_match_a_longer_name() {
        // `build` must not prune `build-tools`.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "build/out.js", "");
        write(t.path(), "build-tools/a.rs", "");

        let prunes = pats(&["build"]);
        let r = search(&SearchSpec { prunes: &prunes, ..spec(t.path(), &[]) }).unwrap();

        assert_eq!(rels(&r), vec!["build-tools", "build-tools/a.rs"]);
    }

    #[test]
    fn gitignore_is_honoured_with_its_negations_when_the_flag_is_on() {
        // THE REASON gitignore is a flag and not an exclusion entry: a flat
        // prefix list cannot express `!keep-this`, so flattening would silently
        // drop the re-include.
        //
        // `generated/*` and not `generated/`: git cannot re-include a file whose
        // PARENT DIRECTORY is excluded, so excluding the directory itself would
        // drop `keep-this.rs` too — correctly, and without testing negation at
        // all. Excluding the CONTENTS is the form where a negation bites.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), ".gitignore", "generated/*\n!generated/keep-this.rs\n");
        write(t.path(), "src/a.rs", "");
        write(t.path(), "generated/drop-me.rs", "");
        write(t.path(), "generated/keep-this.rs", "");

        let on = search(&SearchSpec {
            gitignore: true,
            kinds: Kinds::Files,
            ..SearchSpec::under(t.path())
        })
        .unwrap();

        let names = rels(&on);
        assert!(names.contains(&"src/a.rs".to_string()));
        assert!(!names.contains(&"generated/drop-me.rs".to_string()), "ignored file excluded");
        assert!(
            names.contains(&"generated/keep-this.rs".to_string()),
            "a negation re-includes the file — the semantics a flat list would lose"
        );
    }

    #[test]
    fn a_nested_gitignore_applies_to_its_own_directory() {
        // Hierarchy, the other half a flat list cannot express.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "pkg/.gitignore", "local-only.rs\n");
        write(t.path(), "pkg/local-only.rs", "");
        write(t.path(), "pkg/shared.rs", "");
        write(t.path(), "other/local-only.rs", "");

        let r = search(&SearchSpec {
            gitignore: true,
            kinds: Kinds::Files,
            ..SearchSpec::under(t.path())
        })
        .unwrap();

        let names = rels(&r);
        assert!(!names.contains(&"pkg/local-only.rs".to_string()), "ignored in its own directory");
        assert!(names.contains(&"pkg/shared.rs".to_string()));
        assert!(
            names.contains(&"other/local-only.rs".to_string()),
            "a nested .gitignore does not reach a sibling directory"
        );
    }

    #[test]
    fn gitignore_off_sees_the_ignored_files() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), ".gitignore", "generated/\n");
        write(t.path(), "generated/drop-me.rs", "");

        let off = search(&SearchSpec { kinds: Kinds::Files, ..spec(t.path(), &[]) }).unwrap();

        assert!(rels(&off).contains(&"generated/drop-me.rs".to_string()));
    }

    #[test]
    fn results_are_sorted_so_two_runs_are_comparable() {
        let t = tempfile::tempdir().unwrap();
        for n in ["z.rs", "a.rs", "m.rs"] {
            write(t.path(), n, "");
        }

        let once = search(&spec(t.path(), &[])).unwrap();
        let twice = search(&spec(t.path(), &[])).unwrap();

        assert_eq!(once, twice);
        assert_eq!(rels(&once), vec!["a.rs", "m.rs", "z.rs"]);
    }

    #[test]
    fn a_malformed_glob_is_an_error_not_an_empty_result() {
        // An empty result would read as "this tree has nothing in it".
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "a.rs", "");

        let p = pats(&["["]);
        let err = search(&spec(t.path(), &p)).unwrap_err();

        assert!(err.contains("bad glob"), "got {err}");
    }

    #[test]
    fn an_absent_root_yields_no_matches() {
        let t = tempfile::tempdir().unwrap();
        let gone = t.path().join("never-created");

        let r = search(&spec(&gone, &[])).unwrap();

        assert!(r.found.is_empty());
    }

    #[test]
    fn a_complete_search_reports_itself_complete() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "a.rs", "");

        let r = search(&spec(t.path(), &[])).unwrap();

        assert!(r.is_complete());
        assert!(r.unreadable.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_directory_is_reported_and_not_read_as_empty() {
        // THE MACOS CASE. A TCC-protected directory (~/Documents without Full
        // Disk Access) returns "Operation not permitted" from read_dir. If that
        // were swallowed, an exhaustive scan would report every repository
        // under it as REMOVED — and a removal cascades.
        use std::os::unix::fs::PermissionsExt;

        let t = tempfile::tempdir().unwrap();
        write(t.path(), "visible.rs", "");
        let locked = t.path().join("locked");
        std::fs::create_dir_all(locked.join("hidden-repo")).unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Running as root defeats the permission bits; skip rather than assert
        // something the environment cannot produce.
        let enforced = std::fs::read_dir(&locked).is_err();

        let r = search(&spec(t.path(), &[])).unwrap();

        // Restore before any assertion so the tempdir can always be cleaned up.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();

        if !enforced {
            return;
        }
        assert!(!r.is_complete(), "an unreadable subtree makes the search incomplete");
        assert_eq!(r.unreadable.len(), 1, "got {:?}", r.unreadable);
        assert_eq!(r.unreadable[0].path.as_deref(), Some(locked.as_path()));
        assert!(rels(&r).contains(&"visible.rs".to_string()), "the rest of the tree still counts");
    }
}
