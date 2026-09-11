//! Stage 3 — structure write: the barrier.
//!
//! Spec: `docs/spec/indexer/03-structure-write.md`.
//!
//! Every folder row and every file row is written BEFORE any parse task is
//! enqueued. The gap between "all structure exists" and "work begins" is the
//! barrier, and three problems do not arise because of it (R14):
//!
//! 1. No race. Parse tasks run concurrently; if each did get-or-create on its
//!    file row, two tasks touching one file would race to insert it. Creating
//!    the rows upfront, single-threaded, removes the race rather than locking
//!    around it.
//! 2. The denominator is free — at the barrier the complete post-filter file
//!    set is known, which IS `folders.props.expected_files`.
//! 3. A stalled parse is visible: a file row with no outcome is a task that
//!    never ran, which today is indistinguishable from a file that does not
//!    exist.
//!
//! [`plan_structure`] is PURE. It is also the SAME classification stage 9's
//! incremental path needs (09 S4) — one implementation, so a full scan and an
//! incremental update cannot disagree about what changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// What the walk observed about one file. `hash` is the content fingerprint
/// that decides re-parsing (R14/S6) — `mtime` alone is not enough, because a
/// touched-but-identical file must NOT be re-parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    pub mtime: i64,
    pub hash: String,
}

/// How one file changed between the previous scan and this one.
///
/// Derived from OLD **plus** NEW — the pairing is what makes a change
/// classifiable rather than guessable (R14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// New file. Create the row, parse it.
    Added,
    /// Content changed. Touch mtime + hash, parse it.
    ContentChanged,
    /// Seen before, byte-identical, only the timestamp moved. Refresh mtime
    /// and do NOT parse — re-parsing on mtime alone is how a `touch` or a
    /// checkout turns into a full re-index.
    TouchedOnly,
    /// Unchanged in every respect.
    Unchanged,
    /// Gone from disk. Handed to reconcile (R10.8), NEVER a prefix DELETE.
    Removed,
}

impl ChangeKind {
    /// Whether this change requires re-parsing the file. Content decides —
    /// never the timestamp.
    pub fn needs_parse(self) -> bool {
        matches!(self, ChangeKind::Added | ChangeKind::ContentChanged)
    }
}

/// The classification of one scan against the previous state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StructurePlan {
    /// Every observed file with its verdict, ordered by path so a plan is
    /// comparable and a test can assert on it directly.
    pub files: Vec<(String, ChangeKind)>,
    /// Paths present before and absent now. These go to reconcile.
    pub removed: Vec<String>,
}

impl StructurePlan {
    /// The barrier's denominator (S3): every file this scan will account for.
    ///
    /// Counted at the barrier from the plan itself, never recomputed later by
    /// a second query — a progress bar whose denominator is re-derived can
    /// disagree with the work actually queued.
    pub fn expected_files(&self) -> usize {
        self.files.len()
    }

    /// The subset that needs a parse task. Everything else already has a
    /// current row.
    pub fn to_parse(&self) -> Vec<&str> {
        self.files.iter().filter(|(_, k)| k.needs_parse()).map(|(p, _)| p.as_str()).collect()
    }

    pub fn count(&self, kind: ChangeKind) -> usize {
        self.files.iter().filter(|(_, k)| *k == kind).count()
    }
}

/// One folder row to write, and what it hangs off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFolder {
    pub abs_path: PathBuf,
    /// `None` only for the repo root, whose parent is the watch root.
    pub parent: Option<PathBuf>,
    /// A `sensei.folder_kind` value: `git` for the repo root,
    /// `workspace_member` for a manifest directory an ancestor DECLARES, and
    /// `package` for one nothing declares.
    pub kind: &'static str,
}

/// One file row to write: which folder owns it, and its path relative to that
/// folder — the `(folder_id, file_path)` pair `files` is keyed on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub abs_path: PathBuf,
    pub folder: PathBuf,
    pub rel_path: String,
}

/// The folder and file rows one repo needs, with ownership resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FolderPlan {
    /// Parents before children — `apply_structure` sets `parent_id` at INSERT.
    pub folders: Vec<PlannedFolder>,
    pub files: Vec<PlannedFile>,
}

impl FolderPlan {
    /// The files each folder owns, keyed by folder, for the per-folder
    /// [`plan_structure`] pass. A folder with no files still appears — that is
    /// `expected_files = 0`, a real state, not a missing one.
    pub fn by_folder(&self) -> BTreeMap<&Path, Vec<&PlannedFile>> {
        let mut m: BTreeMap<&Path, Vec<&PlannedFile>> =
            self.folders.iter().map(|f| (f.abs_path.as_path(), Vec::new())).collect();
        for f in &self.files {
            m.entry(f.folder.as_path()).or_default().push(f);
        }
        m
    }
}

/// Resolve one repo's folder rows and file ownership (S1). PURE.
///
/// **A folder row is a structural unit, not a directory.** `folders` carries
/// the repo root plus each manifest-bearing directory — measured at 58 folders
/// across 36 repositories (D11), not one row per directory on disk. Keying
/// commands and dependencies at the repository instead collapses 22 folders'
/// command sets and loses which directory each `build` runs in.
///
/// Every file is owned by the DEEPEST folder containing it, and its path is
/// relative to that folder. That is what makes the recursive rollup in
/// `sensei.folder_completeness` mean something: a workspace member's progress
/// is its own, and the repo's is the sum.
pub fn plan_folders(
    repo_root: &Path,
    manifest_dirs: &[PathBuf],
    declared_members: &BTreeSet<String>,
    files: &[PathBuf],
) -> FolderPlan {
    // The repo root always has a row; a manifest sitting AT the root must not
    // produce a second one (folders.abs_path is UNIQUE, and two rows for one
    // directory would split its files across two owners).
    let mut dirs: Vec<PathBuf> = vec![repo_root.to_path_buf()];
    for d in manifest_dirs {
        if d != repo_root && d.starts_with(repo_root) && !dirs.contains(d) {
            dirs.push(d.clone());
        }
    }
    // Shallowest first, then lexicographic — parents precede children, and the
    // walk's yield order cannot change the result (R6).
    dirs.sort_by(|a, b| a.components().count().cmp(&b.components().count()).then(a.cmp(b)));

    let folders: Vec<PlannedFolder> = dirs
        .iter()
        .map(|d| PlannedFolder {
            abs_path: d.clone(),
            parent: if d == repo_root { None } else { deepest_owner(d, &dirs, true) },
            kind: folder_kind(repo_root, d, declared_members),
        })
        .collect();

    let mut files: Vec<PlannedFile> = files
        .iter()
        .filter_map(|f| {
            let folder = deepest_owner(f, &dirs, false)?;
            let rel = f.strip_prefix(&folder).ok()?.to_string_lossy().to_string();
            Some(PlannedFile { abs_path: f.clone(), folder, rel_path: rel })
        })
        .collect();
    files.sort_by(|a, b| a.abs_path.cmp(&b.abs_path));

    FolderPlan { folders, files }
}

/// Classify one folder row against the repo's DECLARED workspace members.
///
/// The repo root is the repository itself. Every other row here holds a
/// manifest, so the only question is whether something declares it:
///
/// - **`workspace_member`** — its repo-relative path is in the member list an
///   ancestor manifest publishes (`package.json` `workspaces`, `Cargo.toml`
///   `[workspace] members`).
/// - **`package`** — it has a manifest and nothing declares it. A real build
///   unit that belongs to no workspace; `marketplace/` in this repo is one.
///
/// Matching is on PATH, never on the directory's name or its position in the
/// tree. `detect_workspace_members` reports repo-relative paths, and matching
/// on the last segment would make any directory called `one` read as the
/// declared `packages/one`. Asserting membership nothing declared is the R4
/// fabrication — a plausible relationship a caller cannot tell from a real one.
fn folder_kind(repo_root: &Path, dir: &Path, declared_members: &BTreeSet<String>) -> &'static str {
    if dir == repo_root {
        return "git";
    }
    let Ok(rel) = dir.strip_prefix(repo_root) else { return "package" };
    let rel = rel.to_string_lossy();
    if declared_members.contains(rel.as_ref()) { "workspace_member" } else { "package" }
}

/// The deepest directory in `dirs` that contains `path`.
///
/// `strict` excludes `path` itself, which is what a folder's PARENT needs — a
/// folder is not its own parent. A file wants `strict = false` so a file
/// sitting directly in a manifest directory is owned by it.
fn deepest_owner(path: &Path, dirs: &[PathBuf], strict: bool) -> Option<PathBuf> {
    dirs.iter()
        .filter(|d| path.starts_with(d) && !(strict && *d == path))
        .max_by_key(|d| d.components().count())
        .cloned()
}

/// Classify this scan against the previous one (S4). PURE.
///
/// `current` is what the walk just observed; `previous` is what the `files`
/// table holds. Both are keyed by folder-relative path.
pub fn plan_structure(
    current: &BTreeMap<String, FileFacts>,
    previous: &BTreeMap<String, FileFacts>,
) -> StructurePlan {
    let mut plan = StructurePlan::default();

    for (path, now) in current {
        let kind = match previous.get(path) {
            None => ChangeKind::Added,
            Some(before) if before.hash != now.hash => ChangeKind::ContentChanged,
            Some(before) if before.mtime != now.mtime => ChangeKind::TouchedOnly,
            Some(_) => ChangeKind::Unchanged,
        };
        plan.files.push((path.clone(), kind));
    }

    // A path in the previous set and not the current one is REMOVED. Absence
    // here is observed, not inferred — the walk enumerated the directory — so
    // it is safe to act on, unlike the "parsed zero symbols" case R10.3 brakes.
    for path in previous.keys() {
        if !current.contains_key(path) {
            plan.removed.push(path.clone());
        }
    }

    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(mtime: i64, hash: &str) -> FileFacts {
        FileFacts { mtime, hash: hash.to_string() }
    }

    fn map(v: &[(&str, i64, &str)]) -> BTreeMap<String, FileFacts> {
        v.iter().map(|(p, m, h)| ((*p).to_string(), facts(*m, h))).collect()
    }

    #[test]
    fn a_new_file_is_added_and_parses() {
        let plan = plan_structure(&map(&[("a.rs", 1, "h1")]), &map(&[]));
        assert_eq!(plan.files, vec![("a.rs".into(), ChangeKind::Added)]);
        assert_eq!(plan.to_parse(), vec!["a.rs"]);
    }

    #[test]
    fn changed_content_parses_but_a_bare_touch_does_not() {
        // The distinction the whole `content_hash` column exists for. A
        // checkout or a `touch` moves every mtime; re-parsing on that turns a
        // no-op into a full re-index.
        let previous = map(&[("same.rs", 100, "h1"), ("edited.rs", 100, "h1")]);
        let current = map(&[("same.rs", 999, "h1"), ("edited.rs", 999, "h2")]);

        let plan = plan_structure(&current, &previous);

        assert_eq!(plan.count(ChangeKind::TouchedOnly), 1);
        assert_eq!(plan.count(ChangeKind::ContentChanged), 1);
        assert_eq!(plan.to_parse(), vec!["edited.rs"], "only the edited file re-parses");
    }

    #[test]
    fn an_identical_file_is_unchanged_and_does_not_parse() {
        let m = map(&[("a.rs", 1, "h1")]);
        let plan = plan_structure(&m, &m);
        assert_eq!(plan.files, vec![("a.rs".into(), ChangeKind::Unchanged)]);
        assert!(plan.to_parse().is_empty());
    }

    #[test]
    fn a_vanished_file_is_removed_and_is_not_in_the_file_set() {
        // Removed files go to reconcile (R10.8), so they must NOT appear in
        // `files` — a plan that both keeps and removes a path is incoherent.
        let plan = plan_structure(
            &map(&[("kept.rs", 1, "h1")]),
            &map(&[("kept.rs", 1, "h1"), ("gone.rs", 1, "h2")]),
        );

        assert_eq!(plan.removed, vec!["gone.rs"]);
        assert!(!plan.files.iter().any(|(p, _)| p == "gone.rs"));
        assert!(plan.to_parse().is_empty());
    }

    #[test]
    fn the_denominator_counts_every_observed_file_not_just_the_parsed_ones() {
        // S3. expected_files is the BARRIER's count — what the scan will
        // account for — not the work queue. A progress bar keyed on the
        // parse list reads 100% while unchanged files are unaccounted for.
        let previous = map(&[("a.rs", 1, "h1"), ("b.rs", 1, "h2")]);
        let current = map(&[("a.rs", 1, "h1"), ("b.rs", 2, "CHANGED"), ("c.rs", 1, "h3")]);

        let plan = plan_structure(&current, &previous);

        assert_eq!(plan.expected_files(), 3, "every observed file");
        assert_eq!(plan.to_parse().len(), 2, "only two need work");
    }

    #[test]
    fn a_rename_is_an_add_plus_a_remove_with_no_special_case() {
        // R10.5: a rename needs no case of its own. In Rust and TS the module
        // path IS the file path, so every fqn in the file changes — it is not
        // "the same symbol at a new path".
        let plan = plan_structure(&map(&[("b.rs", 1, "same")]), &map(&[("a.rs", 1, "same")]));

        assert_eq!(plan.files, vec![("b.rs".into(), ChangeKind::Added)]);
        assert_eq!(plan.removed, vec!["a.rs"]);
    }

    #[test]
    fn an_empty_scan_of_a_populated_folder_removes_everything_it_observed_gone() {
        // A repo whose files were all deleted. Observed, not inferred — the
        // walk enumerated the directory and found nothing, which is different
        // from a parse returning zero symbols (R10.3's brake).
        let plan = plan_structure(&map(&[]), &map(&[("a.rs", 1, "h1"), ("b.rs", 1, "h2")]));

        assert_eq!(plan.removed, vec!["a.rs", "b.rs"]);
        assert_eq!(plan.expected_files(), 0);
    }

    #[test]
    fn the_plan_is_order_independent() {
        // R6/A6: the same inputs must produce the same plan regardless of the
        // order the walk happened to yield them in. BTreeMap makes this true
        // by construction; the test is what stops someone swapping in a HashMap.
        let a = map(&[("z.rs", 1, "h"), ("a.rs", 1, "h"), ("m.rs", 1, "h")]);
        let b = map(&[("a.rs", 1, "h"), ("m.rs", 1, "h"), ("z.rs", 1, "h")]);
        assert_eq!(plan_structure(&a, &map(&[])), plan_structure(&b, &map(&[])));
    }
}

#[cfg(test)]
mod folder_plan_tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    fn folder_names(plan: &FolderPlan) -> Vec<(String, Option<String>)> {
        plan.folders
            .iter()
            .map(|f| {
                (
                    f.abs_path.to_string_lossy().to_string(),
                    f.parent.as_ref().map(|p| p.to_string_lossy().to_string()),
                )
            })
            .collect()
    }

    /// Every file, as (owning folder, folder-relative path).
    fn owners(plan: &FolderPlan) -> Vec<(String, String)> {
        plan.files
            .iter()
            .map(|f| (f.folder.to_string_lossy().to_string(), f.rel_path.clone()))
            .collect()
    }

    #[test]
    fn a_repo_with_no_manifests_is_one_folder_owning_every_file() {
        let plan =
            plan_folders(Path::new("/r"), &[], &BTreeSet::new(), &[p("/r/a.rs"), p("/r/src/b.rs")]);

        assert_eq!(folder_names(&plan), vec![("/r".to_string(), None)]);
        assert_eq!(
            owners(&plan),
            vec![("/r".into(), "a.rs".into()), ("/r".into(), "src/b.rs".into())]
        );
    }

    #[test]
    fn a_workspace_member_gets_its_own_folder_and_owns_its_files() {
        // D11: 572 commands span 58 folders but only 36 repositories. A member
        // that shares the repo's folder loses which directory its `build` runs
        // in — the exact collapse the folder grain exists to prevent.
        let plan = plan_folders(
            Path::new("/r"),
            &[p("/r/crates/one")],
            &BTreeSet::new(),
            &[p("/r/README.md"), p("/r/crates/one/src/lib.rs")],
        );

        assert_eq!(
            folder_names(&plan),
            vec![("/r".to_string(), None), ("/r/crates/one".to_string(), Some("/r".to_string()))]
        );
        assert_eq!(
            owners(&plan),
            vec![("/r".into(), "README.md".into()), ("/r/crates/one".into(), "src/lib.rs".into()),],
            "the member's file is relative to the MEMBER, not the repo"
        );
    }

    #[test]
    fn a_nested_manifest_parents_to_the_nearer_one_not_the_repo_root() {
        // The recursive rollup in `folder_completeness` walks parent_id. Parent
        // every member at the repo root and a nested package's files are
        // counted twice at the top and never at the package above it.
        let plan = plan_folders(
            Path::new("/r"),
            &[p("/r/app"), p("/r/app/plugin")],
            &BTreeSet::new(),
            &[p("/r/app/x.ts"), p("/r/app/plugin/y.ts")],
        );

        assert_eq!(
            folder_names(&plan),
            vec![
                ("/r".to_string(), None),
                ("/r/app".to_string(), Some("/r".to_string())),
                ("/r/app/plugin".to_string(), Some("/r/app".to_string())),
            ]
        );
        // Ordered by absolute path, which `PathBuf` compares component-wise:
        // "plugin" sorts before "x.ts", so the nested file comes first.
        assert_eq!(
            owners(&plan),
            vec![("/r/app/plugin".into(), "y.ts".into()), ("/r/app".into(), "x.ts".into())],
            "the deepest enclosing folder owns the file"
        );
    }

    #[test]
    fn a_manifest_at_the_repo_root_does_not_create_a_second_folder() {
        // `/r/Cargo.toml` puts the repo root in manifest_dirs. It already has a
        // folder row; a second one would violate folders.abs_path UNIQUE and,
        // worse, split one directory's files across two owners.
        let plan = plan_folders(Path::new("/r"), &[p("/r")], &BTreeSet::new(), &[p("/r/a.rs")]);

        assert_eq!(folder_names(&plan), vec![("/r".to_string(), None)]);
        assert_eq!(owners(&plan), vec![("/r".into(), "a.rs".into())]);
    }

    #[test]
    fn a_declared_member_is_a_workspace_member_and_an_undeclared_one_is_a_package() {
        // The distinction the `package` enum value exists for. Both hold a
        // manifest; only one is DECLARED by an ancestor workspace root.
        // Calling the undeclared one a member asserts a relationship no
        // manifest states (R4) — `marketplace/` in this repo is exactly that.
        let declared: BTreeSet<String> = ["crates/one".to_string()].into_iter().collect();
        let plan = plan_folders(
            Path::new("/r"),
            &[p("/r/crates/one"), p("/r/marketplace")],
            &declared,
            &[],
        );

        let kinds: Vec<(String, &str)> = plan
            .folders
            .iter()
            .map(|f| (f.abs_path.to_string_lossy().to_string(), f.kind))
            .collect();
        // Shallowest first, so `marketplace` precedes `crates/one`.
        assert_eq!(
            kinds,
            vec![
                ("/r".to_string(), "git"),
                ("/r/marketplace".to_string(), "package"),
                ("/r/crates/one".to_string(), "workspace_member"),
            ]
        );
    }

    #[test]
    fn membership_is_matched_on_PATH_not_on_the_directory_name() {
        // `detect_workspace_members` reports repo-relative PATHS. Matching on
        // the last segment would make any directory called `one` anywhere in
        // the tree read as the declared member.
        let declared: BTreeSet<String> = ["packages/one".to_string()].into_iter().collect();
        let plan = plan_folders(Path::new("/r"), &[p("/r/vendor/one")], &declared, &[]);

        assert_eq!(plan.folders[1].kind, "package", "same name, different path");
    }

    #[test]
    fn with_no_declared_members_every_manifest_dir_is_a_package() {
        // A repo whose root manifest declares no workspace. Nothing is a
        // member, and saying so is the honest answer.
        let plan = plan_folders(Path::new("/r"), &[p("/r/a"), p("/r/b")], &BTreeSet::new(), &[]);
        assert!(plan.folders[1..].iter().all(|f| f.kind == "package"));
    }

    #[test]
    fn the_repo_root_is_always_git_even_when_it_declares_itself() {
        // A root Cargo.toml that is both the workspace root and a package puts
        // "" or "." in the member set. The root's row is the REPOSITORY.
        let declared: BTreeSet<String> = ["".to_string(), ".".to_string()].into_iter().collect();
        let plan = plan_folders(Path::new("/r"), &[p("/r")], &declared, &[]);
        assert_eq!(plan.folders.len(), 1);
        assert_eq!(plan.folders[0].kind, "git");
    }

    #[test]
    fn folders_are_emitted_parents_before_children() {
        // apply_structure sets parent_id at INSERT, so a child written before
        // its parent has no id to point at. Emission order is the guarantee.
        let plan = plan_folders(
            Path::new("/r"),
            &[p("/r/a/b/c"), p("/r/a"), p("/r/a/b")],
            &BTreeSet::new(),
            &[],
        );

        let depths: Vec<usize> =
            plan.folders.iter().map(|f| f.abs_path.components().count()).collect();
        let mut sorted = depths.clone();
        sorted.sort();
        assert_eq!(depths, sorted, "shallower folders come first");
    }

    #[test]
    fn a_file_outside_the_repo_root_is_dropped_not_misattributed() {
        // R4: a wrong owner is worse than a missing row. A path that does not
        // live under the root cannot be made relative to any folder here.
        let plan = plan_folders(
            Path::new("/r"),
            &[],
            &BTreeSet::new(),
            &[p("/elsewhere/a.rs"), p("/r/b.rs")],
        );

        assert_eq!(owners(&plan), vec![("/r".into(), "b.rs".into())]);
    }

    #[test]
    fn the_plan_is_order_independent() {
        // R6/A6. The walk's yield order must not change the structure.
        let a = plan_folders(
            Path::new("/r"),
            &[p("/r/z"), p("/r/a")],
            &BTreeSet::new(),
            &[p("/r/z/1.rs"), p("/r/a/2.rs")],
        );
        let b = plan_folders(
            Path::new("/r"),
            &[p("/r/a"), p("/r/z")],
            &BTreeSet::new(),
            &[p("/r/a/2.rs"), p("/r/z/1.rs")],
        );
        assert_eq!(folder_names(&a), folder_names(&b));
        assert_eq!(owners(&a), owners(&b));
    }
}
