//! Repo-level scan logic — which repositories exist, and what is in one.
//!
//! A repository is a directory holding a `.git`, which is a DIRECTORY for a
//! clone and a FILE for a submodule or linked worktree. That is the whole
//! definition; there is no second kind.
//!
//! ## Finding repositories: three steps
//!
//!   1. [`discover`] — from DISK, applying the root's exclusions as the walk
//!      descends. Reads no database, so discovery cannot be wrong because the
//!      database is stale, and a repo created while the daemon runs is found by
//!      the walk rather than missed by a lookup.
//!   2. [`compare`] — the discovered set against the previous run.
//!   3. [`narrow`] — to the repos an event batch actually touched.
//!
//! ## Reading one repository: [`scan`]
//!
//! ONE traversal, classified as it passes. The walk already has to visit every
//! entry for the file set, so testing each name against the manifest registry
//! costs a string comparison; a separate manifest glob would re-walk the tree
//! AND need its own exclusion rules, which is a second place for them to drift.
//!
//! Every function here is PURE of the database. That is what lets the whole
//! stage be proven with a temp directory and no Postgres.
//!
//! The RULES are not re-implemented here. The traversal, exclusion matching and
//! the longest-prefix owner each already have exactly one owner elsewhere,
//! shared with the fs-watcher; a second copy is how the scanner and the watcher
//! last disagreed about what belongs in the index.

// No caller yet: this is the root scanner's new contract, landing ahead of the
// stage-1 wiring so it can be proven on its own. The allow goes when
// `TaskKind::ScanRoot` calls it — it is not a licence for dead code.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::adapters::manifest;
use crate::indexer::facts::Language;
use crate::indexer::incremental::owning_root;
use crate::indexer::lang;
use crate::indexer::search::{EntryKind, Kinds, SearchSpec, Unreadable, search};

/// The always-on prunes, as globs (see [`search`] for why globs).
///
/// `**/.git/**` is the load-bearing one: root discovery must MATCH `.git`, and
/// a match does not stop the walk, so without this the scan descends into every
/// repository's object store.
///
/// The rest are a cost guard, not a correctness one: measured over this
/// machine's 140 checkouts, NONE sits inside one of them.
pub fn default_prunes() -> Vec<String> {
    let mut out: Vec<String> =
        ["**/.git/**", "node_modules", "dist", "build", "target", "__pycache__"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
    out.extend(CREDENTIAL_DIRS.iter().map(|d| (*d).to_string()));
    out
}

/// Directories that hold CREDENTIALS, never descended and never promoted to a
/// repository.
///
/// Unlike the prunes above, these are not a cost guard — they are a hard floor.
/// `discover` runs with `hidden: true` so that it can see `.git`, which also
/// lets it see every other dotfile directory; a user who points a watch root at
/// `$HOME` (or who keeps `~/.ssh` as its own checkout, a documented dotfiles
/// pattern) would otherwise have it promoted to a repository and its
/// non-dotfile children — `id_rsa`, `known_hosts`, `config` — indexed as
/// ordinary files, their paths and hashes landing in `sensei.files` and any
/// language-extension match parsed into `nodes`.
///
/// Deliberately NOT part of the user's exclusion list: an exclusion is a
/// preference the user can remove, and this is not offered as one.
pub const CREDENTIAL_DIRS: &[&str] =
    &[".ssh", ".aws", ".gnupg", ".kube", ".docker", ".config/gcloud", ".azure"];

/// What one disk walk for repositories found, and what it could not see.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Discovered {
    /// Repository roots, absolute, sorted.
    pub repos: Vec<PathBuf>,
    /// Subtrees the walk could not read — almost always a permission. See
    /// [`Unreadable`]; on macOS a TCC-protected directory reads as EMPTY.
    pub unreadable: Vec<Unreadable>,
}

impl Discovered {
    /// Whether this walk SAW the whole tree. The gate on treating absence as
    /// deletion — see [`compare`].
    pub fn is_complete(&self) -> bool {
        self.unreadable.is_empty()
    }
}

/// The repo-level diff: what this walk found against what the last one did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoDiff {
    pub added: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    /// Recorded before and absent now. EMPTY unless the walk was exhaustive —
    /// see [`compare`].
    pub removed: Vec<PathBuf>,
}

/// Step 1 — find every repository root under `root`, from disk.
///
/// `exclusions` are the watch root's own, already resolved to absolute
/// prefixes. They are applied AS THE WALK DESCENDS, not as a filter afterwards.
///
/// `.gitignore` is deliberately NOT honoured here: an outer repository's ignore
/// rules are not a statement about whether an inner checkout is a repository.
/// Hidden entries ARE yielded, because `.git` is one.
pub fn discover(root: &Path, exclusions: &[String]) -> Result<Discovered, String> {
    let prunes = default_prunes();
    let git = [".git".to_string()];
    let result = search(&SearchSpec {
        root,
        patterns: &git,
        // A clone's `.git` is a directory; a submodule's is a FILE. Matching
        // only directories misses every checked-out submodule.
        kinds: Kinds::Both,
        exclusions,
        prunes: &prunes,
        gitignore: false,
        hidden: true,
    })?;

    // A repository is the PARENT of its `.git`. An empty parent means the
    // `.git` sat directly in the search root, so the root IS the repository.
    let mut repos: BTreeSet<PathBuf> = BTreeSet::new();
    for f in &result.found {
        match f.rel_path.parent() {
            Some(p) if p.as_os_str().is_empty() => repos.insert(root.to_path_buf()),
            Some(p) => repos.insert(root.join(p)),
            None => repos.insert(root.to_path_buf()),
        };
    }
    Ok(Discovered { repos: repos.into_iter().collect(), unreadable: result.unreadable })
}

/// Step 2 — diff the discovered set against the previous run.
///
/// `exhaustive` gates REMOVALS and nothing else. Absence is evidence of
/// deletion only when the walk actually looked everywhere: an event-scoped walk
/// saw a handful of paths, and an incomplete one hit a directory it could not
/// read. Deriving removals from either would report every repository it did not
/// visit as deleted — and a removal cascades nodes, edges and files.
pub fn compare(found: &[PathBuf], previous: &BTreeSet<PathBuf>, exhaustive: bool) -> RepoDiff {
    let mut diff = RepoDiff::default();
    let current: BTreeSet<&PathBuf> = found.iter().collect();

    for repo in found {
        if previous.contains(repo) {
            diff.unchanged.push(repo.clone());
        } else {
            diff.added.push(repo.clone());
        }
    }
    if exhaustive {
        diff.removed = previous.iter().filter(|p| !current.contains(p)).cloned().collect();
    }
    diff
}

/// Step 3 — keep only the repos that OWN at least one changed path.
///
/// Ownership is the longest matching prefix, so a file under
/// `repo/submodule/x.rs` narrows to the SUBMODULE and not to its parent.
/// The rule is [`owning_root`]'s, shared with the watch-root resolver, so the
/// two can never disagree about what "longest prefix" means.
pub fn narrow(repos: Vec<PathBuf>, changed: &[PathBuf]) -> Vec<PathBuf> {
    let owners: BTreeSet<&PathBuf> =
        changed.iter().filter_map(|p| owning_root(p, &repos)).collect();
    let keep: BTreeSet<PathBuf> = owners.into_iter().cloned().collect();
    repos.into_iter().filter(|r| keep.contains(r)).collect()
}

// ── Reading one repository ───────────────────────────────────────────────

/// What one entry inside a repository IS.
///
/// The classification is made once, at the walk, and CARRIED — rather than
/// re-derived by each consumer from the path. Three consumers would be three
/// chances to disagree about whether `Cargo.toml` is a manifest or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryClass {
    Folder,
    /// A package manifest, with the ecosystem its adapter claims.
    Manifest {
        ecosystem: &'static str,
    },
    /// A lockfile. Separate from a manifest because what it MEANS differs: a
    /// manifest states a range, a lockfile states what is installed, and one
    /// lockfile serves a whole SUBTREE of manifests.
    Lockfile,
    /// Source in a language this indexer can read.
    Source {
        language: Language,
    },
    /// A file no adapter claims.
    ///
    /// RECORDED, not dropped. It gets a `files` row with
    /// `skip_reason = 'unsupported_format'`, which is what makes the skip stick
    /// (the fingerprint is stored, so the mtime gate does not re-enqueue it
    /// every pass) AND what makes it re-indexable later: add an adapter for the
    /// extension and the existing rows say exactly which files to revisit.
    /// Dropping them instead leaves no record the file was ever seen.
    Unsupported,
}

/// One classified entry, relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoEntry {
    pub rel_path: PathBuf,
    pub class: EntryClass,
}

/// Everything one repository walk found, classified.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepoContents {
    /// Every entry, sorted by path.
    pub entries: Vec<RepoEntry>,
    pub unreadable: Vec<Unreadable>,
}

impl RepoContents {
    pub fn is_complete(&self) -> bool {
        self.unreadable.is_empty()
    }

    fn of<'a>(&'a self, f: impl Fn(&EntryClass) -> bool + 'a) -> Vec<&'a RepoEntry> {
        self.entries.iter().filter(|e| f(&e.class)).collect()
    }

    pub fn folders(&self) -> Vec<&RepoEntry> {
        self.of(|c| matches!(c, EntryClass::Folder))
    }

    pub fn manifests(&self) -> Vec<&RepoEntry> {
        self.of(|c| matches!(c, EntryClass::Manifest { .. }))
    }

    pub fn lockfiles(&self) -> Vec<&RepoEntry> {
        self.of(|c| matches!(c, EntryClass::Lockfile))
    }

    /// Files a language adapter claims — the set that gets a parse task.
    pub fn sources(&self) -> Vec<&RepoEntry> {
        self.of(|c| matches!(c, EntryClass::Source { .. }))
    }

    /// Files no adapter claims. They still get a `files` row.
    pub fn unsupported(&self) -> Vec<&RepoEntry> {
        self.of(|c| matches!(c, EntryClass::Unsupported))
    }
}

/// Walk ONE repository and classify everything in it.
///
/// One traversal produces folders, manifests, lockfiles, source and
/// unsupported files together. `.gitignore` IS honoured here — inside a
/// repository its own rules are exactly the right answer to "does this file
/// belong in the index", and honouring them is what stops the scan and the
/// fs-watcher churning over generated artifacts.
pub fn scan(repo_root: &Path, exclusions: &[String]) -> Result<RepoContents, String> {
    let prunes = default_prunes();
    let result = search(&SearchSpec {
        root: repo_root,
        patterns: &[],
        kinds: Kinds::Both,
        exclusions,
        prunes: &prunes,
        gitignore: true,
        hidden: false,
    })?;

    let entries = result
        .found
        .iter()
        .map(|f| RepoEntry { rel_path: f.rel_path.clone(), class: classify(&f.rel_path, f.kind) })
        .collect();
    Ok(RepoContents { entries, unreadable: result.unreadable })
}

/// What one repo walk found, by class. Emitted once the walk has classified
/// everything and BEFORE anything is written, so the counts describe the work
/// about to happen rather than the work that happened.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub folders: usize,
    /// Every FILE examined — supported and not. Deliberately the larger
    /// number: an unsupported file still gets a row.
    pub files: usize,
    /// Files a language adapter claims. The parse-task count.
    pub supported: usize,
    pub unsupported: usize,
    pub manifests: usize,
    pub lockfiles: usize,
}

impl RepoContents {
    pub fn counts(&self) -> Counts {
        let mut c = Counts::default();
        for e in &self.entries {
            match e.class {
                EntryClass::Folder => c.folders += 1,
                EntryClass::Manifest { .. } => {
                    c.manifests += 1;
                    c.files += 1;
                }
                EntryClass::Lockfile => {
                    c.lockfiles += 1;
                    c.files += 1;
                }
                EntryClass::Source { .. } => {
                    c.supported += 1;
                    c.files += 1;
                }
                EntryClass::Unsupported => {
                    c.unsupported += 1;
                    c.files += 1;
                }
            }
        }
        c
    }
}

/// One folder row to write, with the parent it hangs off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFolder {
    /// Relative to the repository root. EMPTY for the repo root itself.
    pub rel_path: PathBuf,
    /// `None` only for the repo root, whose parent is the watch root and is
    /// therefore not this repository's to name.
    pub parent: Option<PathBuf>,
}

/// The folder tree to write, PARENTS BEFORE CHILDREN.
///
/// Ordered that way because a writer needs the parent's id to insert the child,
/// and a caller that has to sort this itself will one day forget.
///
/// The set is derived from the ANCESTORS of every entry, not only from the
/// directories the walk yielded. Those differ: a directory can be absent from
/// the walk's own output while a file below it is present — `.gitignore`
/// re-includes (`generated/*` plus `!generated/keep.rs`) produce exactly that.
/// Taking only the yielded directories leaves such a file with a parent that
/// was never written, and a file row pointing at a missing folder fails closed
/// on insert.
///
/// The repo root is always first and always present, even for an empty
/// repository: it is the row every other row hangs off.
pub fn folder_tree(contents: &RepoContents) -> Vec<PlannedFolder> {
    let mut paths: BTreeSet<PathBuf> = BTreeSet::new();
    paths.insert(PathBuf::new()); // the repo root

    for e in &contents.entries {
        // A folder contributes itself; a file contributes only its ancestors.
        let start = match e.class {
            EntryClass::Folder => Some(e.rel_path.clone()),
            _ => e.rel_path.parent().map(Path::to_path_buf),
        };
        let mut cur = start;
        while let Some(p) = cur {
            if !paths.insert(p.clone()) {
                break; // this ancestor chain is already recorded
            }
            cur = p.parent().map(Path::to_path_buf);
        }
    }

    // A `BTreeSet<PathBuf>` already orders an ancestor before its descendants:
    // a parent is a strict prefix ending at a separator, so it sorts first.
    paths
        .into_iter()
        .map(|rel_path| {
            let parent = if rel_path.as_os_str().is_empty() {
                None
            } else {
                Some(rel_path.parent().map(Path::to_path_buf).unwrap_or_default())
            };
            PlannedFolder { rel_path, parent }
        })
        .collect()
}

/// The classification rule, pure over a path — testable without a filesystem,
/// and one owner so two callers cannot disagree.
///
/// EVERY DECISION IS THE ADAPTER'S. This asks `accepts_lockfile` / `accepts`
/// rather than testing `manifest_filenames` or a local name list, because the
/// adapters are the only place that knows what this indexer can actually READ.
/// `accepts` also covers the extension-only ecosystems (.NET's `.csproj`,
/// `.sln`), which a `manifest_filenames` test silently misses — the trait
/// documents exactly that and says callers should prefer it.
///
/// A CAPABILITY, NOT A DESCRIPTION. `pnpm-lock.yaml` is a lockfile in English
/// and `Unsupported` here, because no adapter has a reader for it: the npm
/// adapter lists the three formats it can parse and deliberately omits
/// `bun.lockb` on the grounds that "listing it would claim a reader this
/// adapter does not have". Classifying by the broader name list instead
/// promises pins that never arrive. Unsupported is the honest answer AND the
/// useful one — the file still gets a row, so adding a pnpm reader later comes
/// with a list of exactly which files to revisit.
///
/// ORDER MATTERS: a lockfile is tested FIRST. `Cargo.lock` is TOML and
/// `package-lock.json` is JSON, so a manifest or language test reached first
/// would claim them.
pub fn classify(rel_path: &Path, kind: EntryKind) -> EntryClass {
    if kind == EntryKind::Dir {
        return EntryClass::Folder;
    }
    let Some(name) = rel_path.file_name().and_then(|n| n.to_str()) else {
        return EntryClass::Unsupported;
    };

    for adapter in manifest::registered_adapters() {
        if adapter.accepts_lockfile(name) {
            return EntryClass::Lockfile;
        }
    }
    for adapter in manifest::registered_adapters() {
        if adapter.accepts(name) {
            return EntryClass::Manifest { ecosystem: adapter.ecosystem() };
        }
    }

    // Language adapter extensions carry the dot (`.rs`), so build that shape
    // rather than assuming: a mismatch here classifies EVERY file unsupported,
    // silently.
    let ext = rel_path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    match lang::adapter_for_ext(&format!(".{ext}")) {
        Some(a) => EntryClass::Source { language: a.language() },
        None => EntryClass::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_checkout(root: &Path, rel: &str) {
        std::fs::create_dir_all(root.join(rel).join(".git")).unwrap();
    }

    fn mk_gitfile_checkout(root: &Path, rel: &str) {
        let dir = root.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".git"), "gitdir: ../.git/modules/x").unwrap();
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn rels(d: &Discovered, base: &Path) -> Vec<String> {
        d.repos
            .iter()
            .map(|r| {
                r.strip_prefix(base)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| ".".to_string())
            })
            .collect()
    }

    fn paths(v: &[&str]) -> Vec<PathBuf> {
        v.iter().map(PathBuf::from).collect()
    }

    fn names(entries: &[&RepoEntry]) -> Vec<String> {
        entries.iter().map(|e| e.rel_path.to_string_lossy().to_string()).collect()
    }

    // ── discover ────────────────────────────────────────────────────────

    #[test]
    fn discovers_a_git_directory_and_a_git_file() {
        // A submodule checkout has a `.git` FILE. Matching only directories
        // misses every checked-out submodule.
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "plain");
        mk_gitfile_checkout(t.path(), "sub");

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["plain", "sub"]);
        assert!(d.is_complete());
    }

    #[test]
    fn an_excluded_subtree_is_pruned_during_descent() {
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "kept-repo");
        mk_checkout(t.path(), "excluded-path-a/nested-repo");

        let excl = vec![t.path().join("excluded-path-a").to_string_lossy().to_string()];
        let d = discover(t.path(), &excl).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["kept-repo"]);
    }

    #[test]
    fn an_exclusion_nested_below_the_root_is_honoured() {
        // A two-segment exclusion excludes that one subtree and nothing else —
        // the walk still descends its parent to reach the siblings.
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "group-a/excluded-path-b");
        mk_checkout(t.path(), "group-a/kept-repo");

        let excl = vec![t.path().join("group-a/excluded-path-b").to_string_lossy().to_string()];
        let d = discover(t.path(), &excl).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["group-a/kept-repo"]);
    }

    #[test]
    fn a_submodule_inside_a_repo_is_its_own_repo() {
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "outer");
        mk_gitfile_checkout(t.path(), "outer/vendor/inner");

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["outer", "outer/vendor/inner"]);
    }

    #[test]
    fn a_repo_nested_deeper_than_the_old_depth_bound_is_found() {
        // The predecessor bounded this walk at 8 and silently lost anything
        // below. There is no bound now.
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "a/b/c/d/e/f/g/h/i/j/deep-repo");

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["a/b/c/d/e/f/g/h/i/j/deep-repo"]);
    }

    /// A CREDENTIAL DIRECTORY IS NEVER A REPOSITORY, even when it is a real
    /// checkout — the documented "dotfiles as a git repo" pattern.
    ///
    /// `discover` runs with `hidden: true` so it can see `.git`, which also
    /// lets it see `.ssh`. Promoting that to a repository would index
    /// `id_rsa`, `known_hosts` and `config` as ordinary files.
    #[tokio::test]
    async fn a_credential_directory_is_never_promoted_to_a_repository() {
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "ordinary");
        for dir in [".ssh", ".aws", ".gnupg"] {
            mk_checkout(t.path(), dir);
            std::fs::write(t.path().join(dir).join("id_rsa"), "PRIVATE KEY").unwrap();
        }

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(
            rels(&d, t.path()),
            vec!["ordinary"],
            "a credential directory must never become a repository"
        );
    }

    /// THE SECOND HALF OF THE PROTECTION: a repo walk does not descend into
    /// dotfile directories at all.
    ///
    /// The credential floor stops `.ssh` being PROMOTED to a repository. This
    /// is what stops a credential directory that sits INSIDE an ordinary
    /// repository from being read — `repo::scan` runs with `hidden: false`,
    /// unlike `discover`, which needs `hidden: true` to see `.git`.
    ///
    /// DOUBLY PROTECTED, and measured: removing the credential floor alone
    /// leaves this green (the dotfile skip still catches it), and flipping
    /// `hidden` alone leaves it green too (the floor still catches it). Only
    /// removing BOTH turns it red. That is defence in depth rather than a weak
    /// test — but it is worth saying, because a reader who mutates one
    /// mechanism and sees green would otherwise conclude the test is vacuous.
    /// The promotion test above is the one that pins the floor on its own.
    #[tokio::test]
    async fn a_repo_walk_does_not_descend_into_dotfile_directories() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join(".aws")).unwrap();
        std::fs::write(t.path().join(".aws/credentials"), "[default]\naws_secret=hunter2").unwrap();
        std::fs::write(t.path().join("main.rs"), "fn main() {}").unwrap();

        let c = scan(t.path(), &[]).unwrap();

        assert!(
            c.entries.iter().any(|e| e.rel_path.to_string_lossy() == "main.rs"),
            "the ordinary file is still indexed"
        );
        assert!(
            !c.entries.iter().any(|e| e.rel_path.to_string_lossy().contains(".aws")),
            "nothing under a dotfile directory may be recorded, got {:?}",
            c.entries.iter().map(|e| e.rel_path.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_code_directory_with_no_git_is_not_a_repo() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "loose/Cargo.toml", "[package]\nname=\"q\"\n");

        let d = discover(t.path(), &[]).unwrap();

        assert!(d.repos.is_empty(), "no .git, no repo");
    }

    #[test]
    fn the_search_root_can_itself_be_a_repo() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join(".git")).unwrap();

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(d.repos, vec![t.path().to_path_buf()]);
    }

    #[test]
    fn a_symlinked_checkout_is_not_discovered_twice() {
        let t = tempfile::tempdir().unwrap();
        mk_checkout(t.path(), "real");
        #[cfg(unix)]
        std::os::unix::fs::symlink(t.path().join("real"), t.path().join("link")).unwrap();

        let d = discover(t.path(), &[]).unwrap();

        assert_eq!(rels(&d, t.path()), vec!["real"]);
    }

    #[test]
    fn an_empty_tree_yields_no_repos_and_is_complete() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("just/files")).unwrap();

        let d = discover(t.path(), &[]).unwrap();

        assert!(d.repos.is_empty());
        assert!(d.is_complete(), "empty is not the same as unreadable");
    }

    // ── compare ─────────────────────────────────────────────────────────

    #[test]
    fn compare_splits_added_unchanged_and_removed() {
        let previous: BTreeSet<PathBuf> = paths(&["/w/b", "/w/gone"]).into_iter().collect();

        let diff = compare(&paths(&["/w/a", "/w/b"]), &previous, true);

        assert_eq!(diff.added, paths(&["/w/a"]));
        assert_eq!(diff.unchanged, paths(&["/w/b"]));
        assert_eq!(diff.removed, paths(&["/w/gone"]));
    }

    #[test]
    fn a_non_exhaustive_compare_never_reports_a_removal() {
        // An event-scoped walk saw one path. Everything else is UNVISITED, not
        // deleted — and a removal cascades nodes, edges and files.
        let previous: BTreeSet<PathBuf> = paths(&["/w/a", "/w/b", "/w/c"]).into_iter().collect();

        let diff = compare(&paths(&["/w/a"]), &previous, false);

        assert!(diff.removed.is_empty(), "absence is not evidence under a partial walk");
        assert_eq!(diff.unchanged, paths(&["/w/a"]));
    }

    #[test]
    fn a_first_ever_scan_is_all_added_with_no_special_case() {
        let diff = compare(&paths(&["/w/a", "/w/b"]), &BTreeSet::new(), true);

        assert_eq!(diff.added.len(), 2);
        assert!(diff.removed.is_empty());
    }

    // ── narrow ──────────────────────────────────────────────────────────

    #[test]
    fn narrow_keeps_only_repos_owning_a_changed_path() {
        let kept =
            narrow(paths(&["/w/a", "/w/b", "/w/c"]), &paths(&["/w/a/src/x.rs", "/w/c/y.rs"]));

        assert_eq!(kept, paths(&["/w/a", "/w/c"]));
    }

    #[test]
    fn narrow_picks_the_submodule_over_its_parent() {
        let kept = narrow(
            paths(&["/w/outer", "/w/outer/vendor/inner"]),
            &paths(&["/w/outer/vendor/inner/src/x.rs"]),
        );

        assert_eq!(kept, paths(&["/w/outer/vendor/inner"]));
    }

    #[test]
    fn narrow_does_not_match_a_sibling_by_string_prefix() {
        let kept = narrow(paths(&["/w/proj"]), &paths(&["/w/project/src/x.rs"]));

        assert!(kept.is_empty());
    }

    // ── classify ────────────────────────────────────────────────────────

    #[test]
    fn classify_names_manifests_lockfiles_source_and_unsupported() {
        let c = |p: &str| classify(Path::new(p), EntryKind::File);

        assert!(matches!(c("Cargo.toml"), EntryClass::Manifest { .. }));
        assert!(matches!(c("app/package.json"), EntryClass::Manifest { .. }));
        assert_eq!(c("Cargo.lock"), EntryClass::Lockfile);
        assert_eq!(c("package-lock.json"), EntryClass::Lockfile);
        // CAPABILITY, NOT VOCABULARY. `pnpm-lock.yaml` is a lockfile in
        // English, and no adapter has a reader for it — the npm adapter lists
        // the three formats it can parse and omits this one. Calling it a
        // Lockfile would promise pins that never arrive; Unsupported records it
        // so a future pnpm reader knows which files to revisit.
        assert_eq!(c("pnpm-lock.yaml"), EntryClass::Unsupported);
        assert_eq!(c("yarn.lock"), EntryClass::Unsupported);
        assert_eq!(c("src/main.rs"), EntryClass::Source { language: Language::Rust });
        assert_eq!(c("web/a.ts"), EntryClass::Source { language: Language::TypeScript });
        assert_eq!(c("README.md"), EntryClass::Unsupported);
        assert_eq!(c("logo.png"), EntryClass::Unsupported);
        assert_eq!(classify(Path::new("src"), EntryKind::Dir), EntryClass::Folder);
    }

    /// NO GAP, IN EITHER DIRECTION: every filename an adapter claims is
    /// classified as that adapter's manifest, and nothing else is.
    ///
    /// Driven off `registered_adapters()` rather than a list written here, so
    /// adding an adapter extends the test automatically. A hand-written list
    /// would be a second inventory of the same fact and would silently stop
    /// covering the newest adapter — which is exactly how `classify` came to
    /// consult a name constant that overclaimed nine files no adapter can read.
    #[test]
    fn every_adapter_manifest_is_classified_with_that_adapters_ecosystem() {
        for a in manifest::registered_adapters() {
            for name in a.manifest_filenames() {
                assert_eq!(
                    classify(Path::new(name), EntryKind::File),
                    EntryClass::Manifest { ecosystem: a.ecosystem() },
                    "{name} is {}'s manifest but was not classified as one",
                    a.ecosystem()
                );
                // And at depth, not only at the repo root.
                assert!(
                    matches!(
                        classify(&Path::new("packages/one").join(name), EntryKind::File),
                        EntryClass::Manifest { .. }
                    ),
                    "{name} not recognised below the root"
                );
            }
            // Extension-only ecosystems (.NET's `.csproj` / `.sln`) have no
            // fixed filename; `accepts` covers both and a `manifest_filenames`
            // test alone would miss every one of them.
            for ext in a.manifest_extensions() {
                let name = format!("Some.{ext}");
                assert!(
                    matches!(
                        classify(Path::new(&name), EntryKind::File),
                        EntryClass::Manifest { .. }
                    ),
                    "{name} is an extension {} claims but was not classified as a manifest",
                    a.ecosystem()
                );
            }
        }
    }

    /// Every lockfile an adapter can READ is classified as one.
    ///
    /// Capability, not vocabulary: an adapter lists only the formats it has a
    /// parser for, so this cannot pass by naming files we cannot open.
    #[test]
    fn every_adapter_lockfile_is_classified_as_a_lockfile() {
        for a in manifest::registered_adapters() {
            for name in a.lockfile_filenames() {
                assert_eq!(
                    classify(Path::new(name), EntryKind::File),
                    EntryClass::Lockfile,
                    "{name} is a lockfile {} can read but was not classified as one",
                    a.ecosystem()
                );
            }
        }
    }

    /// No filename is claimed by two adapters, and no manifest name is also a
    /// lockfile name.
    ///
    /// `classify` takes the FIRST adapter that accepts, so an overlap would
    /// make the answer depend on registration order — and the lockfile test
    /// runs first, so a manifest that any adapter also called a lockfile would
    /// silently never be parsed as a manifest at all.
    #[test]
    fn no_filename_is_claimed_by_two_adapters() {
        let adapters = manifest::registered_adapters();
        for (i, a) in adapters.iter().enumerate() {
            for name in a.manifest_filenames() {
                for (j, b) in adapters.iter().enumerate() {
                    if i != j {
                        assert!(
                            !b.accepts(name),
                            "{name} is accepted by both {} and {}",
                            a.ecosystem(),
                            b.ecosystem()
                        );
                    }
                    assert!(
                        !b.accepts_lockfile(name),
                        "{name} is a manifest for {} and a lockfile for {}",
                        a.ecosystem(),
                        b.ecosystem()
                    );
                }
            }
        }
    }

    #[test]
    fn a_lockfile_is_never_classified_as_source_or_manifest() {
        // ORDER. `Cargo.lock` is TOML and `package-lock.json` is JSON; a
        // language or manifest test reached first would claim both.
        assert_eq!(classify(Path::new("Cargo.lock"), EntryKind::File), EntryClass::Lockfile);
        assert_eq!(classify(Path::new("package-lock.json"), EntryKind::File), EntryClass::Lockfile);
    }

    #[test]
    fn an_extensionless_file_is_unsupported_not_a_panic() {
        assert_eq!(classify(Path::new("LICENSE"), EntryKind::File), EntryClass::Unsupported);
        assert_eq!(classify(Path::new("Makefile"), EntryKind::File), EntryClass::Unsupported);
    }

    // ── scan ────────────────────────────────────────────────────────────

    #[test]
    fn one_scan_yields_folders_manifests_lockfiles_source_and_unsupported() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "Cargo.toml", "[package]\nname=\"x\"\n");
        write(t.path(), "Cargo.lock", "");
        write(t.path(), "src/main.rs", "fn main() {}");
        write(t.path(), "web/app.ts", "export const a = 1;");
        write(t.path(), "README.md", "# x");

        let c = scan(t.path(), &[]).unwrap();

        assert_eq!(names(&c.folders()), vec!["src", "web"]);
        assert_eq!(names(&c.manifests()), vec!["Cargo.toml"]);
        assert_eq!(names(&c.lockfiles()), vec!["Cargo.lock"]);
        assert_eq!(names(&c.sources()), vec!["src/main.rs", "web/app.ts"]);
        assert_eq!(names(&c.unsupported()), vec!["README.md"]);
        assert!(c.is_complete());
    }

    #[test]
    fn an_unsupported_file_is_recorded_rather_than_dropped() {
        // It gets a `files` row so the skip STICKS (the stored fingerprint
        // stops it being re-enqueued every pass) and so a future adapter knows
        // exactly which files to revisit. Dropping it leaves no record it was
        // ever seen.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "notes.md", "");
        write(t.path(), "data.parquet", "");

        let c = scan(t.path(), &[]).unwrap();

        assert_eq!(names(&c.unsupported()), vec!["data.parquet", "notes.md"]);
        assert!(c.sources().is_empty());
        assert_eq!(c.entries.len(), 2, "every file is present, none silently dropped");
    }

    #[test]
    fn a_repo_scan_honours_gitignore() {
        // Inside a repository its own rules are the right answer to "does this
        // belong in the index" — and honouring them is what stops the scan and
        // the watcher churning over generated artifacts.
        let t = tempfile::tempdir().unwrap();
        write(t.path(), ".gitignore", "generated/\n");
        write(t.path(), "src/a.rs", "");
        write(t.path(), "generated/b.rs", "");

        let c = scan(t.path(), &[]).unwrap();

        assert_eq!(names(&c.sources()), vec!["src/a.rs"]);
    }

    // ── counts ──────────────────────────────────────────────────────────

    #[test]
    fn counts_separate_supported_from_every_file_examined() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "Cargo.toml", "");
        write(t.path(), "Cargo.lock", "");
        write(t.path(), "src/main.rs", "");
        write(t.path(), "src/lib.rs", "");
        write(t.path(), "README.md", "");

        let c = scan(t.path(), &[]).unwrap().counts();

        assert_eq!(c.folders, 1, "src");
        assert_eq!(c.supported, 2, "the parse-task count");
        assert_eq!(c.unsupported, 1);
        assert_eq!(c.manifests, 1);
        assert_eq!(c.lockfiles, 1);
        // Every file is examined, so `files` is the sum of the four file
        // classes — not just the supported ones.
        assert_eq!(c.files, 5);
        assert_eq!(c.files, c.supported + c.unsupported + c.manifests + c.lockfiles);
    }

    // ── folder_tree ─────────────────────────────────────────────────────

    fn tree_of(pairs: &[(&str, EntryClass)]) -> Vec<PlannedFolder> {
        let entries = pairs
            .iter()
            .map(|(p, c)| RepoEntry { rel_path: PathBuf::from(p), class: c.clone() })
            .collect();
        folder_tree(&RepoContents { entries, unreadable: vec![] })
    }

    fn shown(tree: &[PlannedFolder]) -> Vec<(String, Option<String>)> {
        tree.iter()
            .map(|f| {
                (
                    f.rel_path.to_string_lossy().to_string(),
                    f.parent.as_ref().map(|p| p.to_string_lossy().to_string()),
                )
            })
            .collect()
    }

    #[test]
    fn the_folder_tree_links_each_folder_to_its_parent() {
        let tree = tree_of(&[
            ("src", EntryClass::Folder),
            ("src/deep", EntryClass::Folder),
            ("src/deep/a.rs", EntryClass::Source { language: Language::Rust }),
        ]);

        assert_eq!(
            shown(&tree),
            vec![
                ("".into(), None),
                ("src".into(), Some("".into())),
                ("src/deep".into(), Some("src".into())),
            ]
        );
    }

    #[test]
    fn the_repo_root_is_first_and_is_the_only_parentless_row() {
        let tree = tree_of(&[("a/b/c", EntryClass::Folder)]);

        assert_eq!(tree[0].rel_path, PathBuf::new(), "the root is first");
        assert_eq!(tree.iter().filter(|f| f.parent.is_none()).count(), 1);
    }

    #[test]
    fn parents_always_precede_their_children() {
        // A writer needs the parent's id to insert the child.
        let tree = tree_of(&[
            ("z/deep/leaf", EntryClass::Folder),
            ("a", EntryClass::Folder),
            ("z", EntryClass::Folder),
            ("z/deep", EntryClass::Folder),
        ]);

        let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
        for f in &tree {
            if let Some(p) = &f.parent {
                assert!(seen.contains(p), "{:?} written before its parent {p:?}", f.rel_path);
            }
            seen.insert(f.rel_path.clone());
        }
    }

    #[test]
    fn an_ancestor_the_walk_did_not_yield_is_still_created() {
        // THE HOLE this closes: a `.gitignore` that excludes a directory's
        // contents and re-includes one file (`generated/*` + `!generated/keep`)
        // yields the FILE without the DIRECTORY. Taking only yielded folders
        // leaves that file pointing at a folder row that was never written.
        let tree =
            tree_of(&[("generated/keep.rs", EntryClass::Source { language: Language::Rust })]);

        assert_eq!(shown(&tree), vec![("".into(), None), ("generated".into(), Some("".into()))]);
    }

    #[test]
    fn an_empty_repository_still_gets_its_root_row() {
        // Every file and folder row hangs off it.
        let tree = tree_of(&[]);

        assert_eq!(shown(&tree), vec![("".into(), None)]);
    }

    #[test]
    fn a_repo_scan_does_not_walk_the_object_store() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), ".git/objects/ab/cdef", "");
        write(t.path(), "src/a.rs", "");

        let c = scan(t.path(), &[]).unwrap();

        assert!(
            !c.entries.iter().any(|e| e.rel_path.to_string_lossy().contains(".git")),
            "got {:?}",
            c.entries.iter().map(|e| e.rel_path.clone()).collect::<Vec<_>>()
        );
    }
}
