//! Stage 9 — incremental: changed files back to repos.
//!
//! Spec: `docs/spec/indexer/09-incremental.md`. Whole-system: R14 (incremental
//! mode), R15 (§7g, the root set), R6, R10.5.
//!
//! This is the INVERSE of stages 1–3. Those walk DOWN from a root to files;
//! this walks UP from a file to its root. A change arrives as a set of paths;
//! the job is to turn it into the smallest correct amount of work.
//!
//! Everything here is PURE and takes the root set as data. That is what makes
//! the longest-prefix rule testable on literals with no filesystem and no
//! database — and it is also what makes it correct, because the root set has to
//! come from the DATABASE (S2) and a function that fetched its own would be
//! reading it at the wrong moment.
//
// These stages have no caller on purpose: the shipped indexer under
// `crate::languages` keeps producing the graph until cutover
// (`docs/spec/indexer/10-cutover.md`), and wiring them in early would put two
// producers with different rules on one set of tables. The allow goes when the
// cutover gives them callers — it is not a licence for genuinely dead code.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What a batch of changed paths resolved to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchResult {
    /// Paths grouped by the repo root that owns them.
    ///
    /// GROUPED, not a flat list of `(path, root)` pairs: a branch switch
    /// touches thousands of files at once, and one task per event is how the
    /// queue got 350-file bursts every five minutes. The grouping is the batch.
    ///
    /// A `BTreeMap` so the order a caller sees is the roots' own order rather
    /// than a hash seed's — R6 applies to work scheduling too.
    pub by_repo: BTreeMap<PathBuf, Vec<PathBuf>>,
    /// Paths under no known root (S3).
    ///
    /// These ESCALATE to `scan_root`. They are NOT attached to the nearest
    /// ancestor: a path under no known root means a repo appeared that the
    /// scan has not seen, and guessing a parent for it files every one of its
    /// files under a repo they do not belong to — silently, and with the wrong
    /// `folder_id` on every node that follows.
    pub escalate: Vec<PathBuf>,
}

/// The repo root that owns `path`: the LONGEST matching prefix (S1).
///
/// Longest and not first-match, because roots nest. A file under
/// `repo/submodule/x.rs` belongs to the SUBMODULE, and a first-match rule
/// returns whichever of the two the caller happened to list first — so the
/// answer would depend on argument order, which R6 rules out.
///
/// This is the same rule
/// [`watcher::root_watcher::watch_root_for_path`](crate::watcher::root_watcher)
/// applies to WATCH roots. One rule, two root sets: that function answers
/// "which watch root", this one "which repository", and the sets differ because
/// a watch root holds many repos. They share this implementation so the two can
/// never disagree about what "longest prefix" means.
pub fn owning_root<'a>(path: &Path, roots: &'a [PathBuf]) -> Option<&'a PathBuf> {
    roots.iter().filter(|r| path.starts_with(r)).max_by_key(|r| r.as_os_str().len())
}

/// Group changed paths by the repo that owns each, escalating the rest.
///
/// `known_roots` MUST come from the database (S2). Submodules are discovered by
/// `scan_repo` (stage 2 S2), so the complete root set exists only after the
/// scan-repo wave drains and has been persisted. Reading it from a scan-root
/// run's in-memory output would miss every submodule and quietly attach
/// submodule files to their parent repo — the exact failure S1 exists to
/// prevent, arriving through the back door.
pub fn match_repos(paths: &[PathBuf], known_roots: &[PathBuf]) -> MatchResult {
    let mut out = MatchResult::default();
    for path in paths {
        match owning_root(path, known_roots) {
            Some(root) => out.by_repo.entry(root.clone()).or_default().push(path.clone()),
            None => out.escalate.push(path.clone()),
        }
    }
    out
}

/// What one changed path retriggers (S9/S10/S11).
///
/// The variants are the whole of S11: **neither manifest arm can carry a source
/// file**, so "a dependency change does not re-parse 48,646 files" is a property
/// of the type rather than a rule someone has to remember. R14's structure/work
/// split is exactly this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Retrigger {
    /// A source file changed. The file-parse path, one file.
    Parse(PathBuf),
    /// A manifest or lockfile changed. Re-run the manifest pass for these
    /// manifest DIRECTORIES and nothing else.
    ///
    /// One directory for a manifest; potentially many for a lockfile — see
    /// [`retrigger_for`] for why that asymmetry is the part to get right.
    ManifestPass(Vec<PathBuf>),
}

/// Turn one changed path into the work it actually implies.
///
/// **The fan-out is ASYMMETRIC (S10), and this is the part to get right.** A
/// manifest serves its own folder. A lockfile serves a SUBTREE — every manifest
/// below it with no nearer lock — so changing one changes the resolved pins of
/// all of them. Measured on this repo: `/Cargo.lock` serves the root plus the
/// workspace members under `crates/`, while `app/src-tauri/Cargo.lock` serves
/// exactly one.
///
/// Getting it wrong is silent in the direction that matters: retrigger only the
/// lockfile's own folder and the other crates keep stale pins, with nothing
/// counting them.
///
/// The served set comes from [`scan_repo::manifests_served_by`], which is
/// stage 2's own resolver INVERTED rather than a second implementation. Two
/// implementations of "which manifests does this lockfile serve" will disagree,
/// and the disagreement leaves stale pins rather than erroring.
///
/// A manifest change does NOT re-parse source files (S11). The dependency set
/// changed; the code did not. The exception — a package RENAME, which changes
/// every fqn in the package — is detected by the manifest reader comparing
/// `name` against what the graph was built with, not here.
pub fn retrigger_for(
    changed: &Path,
    repo_root: &Path,
    manifest_dirs: &[PathBuf],
    lockfiles: &[PathBuf],
    manifest_names: &[&str],
    lockfile_names: &[&str],
) -> Retrigger {
    let name = changed.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if lockfile_names.contains(&name) {
        return Retrigger::ManifestPass(crate::indexer::scan_repo::manifests_served_by(
            changed,
            repo_root,
            lockfile_names,
            lockfiles,
            manifest_dirs,
        ));
    }
    if manifest_names.contains(&name) {
        // THAT folder only. A manifest declares its own package; a sibling's
        // pins are not affected by it.
        let dir = changed.parent().map(Path::to_path_buf).unwrap_or_default();
        return Retrigger::ManifestPass(vec![dir]);
    }
    Retrigger::Parse(changed.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// S1. A file under a submodule belongs to the SUBMODULE.
    ///
    /// The roots are listed PARENT FIRST, which is the order that breaks a
    /// first-match rule. A test that listed them the other way round would pass
    /// against both implementations and prove nothing.
    #[test]
    fn a_file_under_a_submodule_belongs_to_the_submodule() {
        let roots = vec![p("/w/repo"), p("/w/repo/vendor/sub")];
        let got = match_repos(&[p("/w/repo/vendor/sub/src/x.rs")], &roots);

        assert_eq!(
            got.by_repo,
            BTreeMap::from([(p("/w/repo/vendor/sub"), vec![p("/w/repo/vendor/sub/src/x.rs")])]),
            "the deeper root owns the file"
        );
        assert!(got.escalate.is_empty());
    }

    /// And the answer does not depend on the order the roots arrive in.
    #[test]
    fn the_owning_root_is_the_same_whichever_order_the_roots_are_listed() {
        let file = p("/w/repo/vendor/sub/src/x.rs");
        let parent_first = vec![p("/w/repo"), p("/w/repo/vendor/sub")];
        let child_first = vec![p("/w/repo/vendor/sub"), p("/w/repo")];
        assert_eq!(
            owning_root(&file, &parent_first),
            owning_root(&file, &child_first),
            "argument order is not evidence (R6)"
        );
        assert_eq!(owning_root(&file, &parent_first), Some(&p("/w/repo/vendor/sub")));
    }

    /// S3. A path under NO known root escalates rather than being adopted.
    #[test]
    fn a_path_under_no_known_root_escalates() {
        let roots = vec![p("/w/repo")];
        let got = match_repos(&[p("/w/elsewhere/new-repo/src/x.rs")], &roots);

        assert!(
            got.by_repo.is_empty(),
            "no repo owns it, and the nearest ancestor is not an answer — attaching it would \
             file every one of that repo's files under the wrong folder"
        );
        assert_eq!(got.escalate, vec![p("/w/elsewhere/new-repo/src/x.rs")]);
    }

    /// A prefix match is on PATH COMPONENTS, not on characters.
    ///
    /// `/w/repo-2` starts with the string `/w/repo` and is a different repo.
    /// A `starts_with` on strings would hand every file in it to the wrong one;
    /// `Path::starts_with` compares components, which is why it is used here
    /// rather than `str::starts_with`.
    #[test]
    fn a_sibling_whose_name_extends_anothers_is_not_inside_it() {
        let roots = vec![p("/w/repo"), p("/w/repo-2")];
        assert_eq!(owning_root(&p("/w/repo-2/src/x.rs"), &roots), Some(&p("/w/repo-2")));
        assert_eq!(owning_root(&p("/w/repo/src/x.rs"), &roots), Some(&p("/w/repo")));
    }

    /// The batch is GROUPED BY REPO, because a burst is normal.
    ///
    /// A branch switch touches thousands of files. One task per event is how
    /// the queue got 350-file bursts every five minutes; the grouping is what
    /// makes it one unit of work per repo instead.
    #[test]
    fn paths_are_batched_by_repo_not_emitted_one_by_one() {
        let roots = vec![p("/w/a"), p("/w/b")];
        let got =
            match_repos(&[p("/w/a/1.rs"), p("/w/b/1.rs"), p("/w/a/2.rs"), p("/w/c/1.rs")], &roots);

        assert_eq!(got.by_repo.len(), 2, "two repos, two batches — not four events");
        assert_eq!(got.by_repo[&p("/w/a")], vec![p("/w/a/1.rs"), p("/w/a/2.rs")]);
        assert_eq!(got.by_repo[&p("/w/b")], vec![p("/w/b/1.rs")]);
        assert_eq!(got.escalate, vec![p("/w/c/1.rs")]);
    }

    /// An empty root set escalates everything rather than matching nothing
    /// silently. Before the first scan drains, EVERY path is a new repo.
    #[test]
    fn with_no_known_roots_every_path_escalates() {
        let got = match_repos(&[p("/w/a/1.rs"), p("/w/b/1.rs")], &[]);
        assert!(got.by_repo.is_empty());
        assert_eq!(got.escalate.len(), 2, "nothing is silently dropped");
    }

    const CARGO_LOCK: &[&str] = &["Cargo.lock"];
    const CARGO_TOML: &[&str] = &["Cargo.toml"];

    /// This repo's real shape, as literals: one root lock and one nested lock,
    /// with workspace members between them.
    fn workspace() -> (PathBuf, Vec<PathBuf>, Vec<PathBuf>) {
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock"), p("/repo/app/src-tauri/Cargo.lock")];
        let manifests = vec![
            p("/repo"),
            p("/repo/crates/senseid"),
            p("/repo/crates/cli"),
            p("/repo/crates/mcp"),
            p("/repo/app/src-tauri"),
        ];
        (root, locks, manifests)
    }

    /// S10. A LOCKFILE change fans out to every manifest it serves.
    ///
    /// The failure this catches is silent in the direction that matters:
    /// retrigger the lockfile's own folder alone and three crates keep stale
    /// pins with nothing counting them.
    #[test]
    fn a_root_lockfile_change_retriggers_every_manifest_it_serves() {
        let (root, locks, manifests) = workspace();
        let got = retrigger_for(
            &p("/repo/Cargo.lock"),
            &root,
            &manifests,
            &locks,
            CARGO_TOML,
            CARGO_LOCK,
        );
        assert_eq!(
            got,
            Retrigger::ManifestPass(vec![
                p("/repo"),
                p("/repo/crates/senseid"),
                p("/repo/crates/cli"),
                p("/repo/crates/mcp"),
            ]),
            "the root lock serves the root and every member with no nearer lock — and NOT \
             app/src-tauri, which has its own"
        );
    }

    /// The other half of the asymmetry: a NESTED lockfile serves only itself,
    /// and the root's manifests are untouched.
    #[test]
    fn a_nested_lockfile_change_retriggers_only_its_own_manifest() {
        let (root, locks, manifests) = workspace();
        let got = retrigger_for(
            &p("/repo/app/src-tauri/Cargo.lock"),
            &root,
            &manifests,
            &locks,
            CARGO_TOML,
            CARGO_LOCK,
        );
        assert_eq!(got, Retrigger::ManifestPass(vec![p("/repo/app/src-tauri")]));
    }

    /// S10. A MANIFEST change retriggers its own folder and no other.
    #[test]
    fn a_manifest_change_retriggers_that_folder_only() {
        let (root, locks, manifests) = workspace();
        let got = retrigger_for(
            &p("/repo/crates/cli/Cargo.toml"),
            &root,
            &manifests,
            &locks,
            CARGO_TOML,
            CARGO_LOCK,
        );
        assert_eq!(
            got,
            Retrigger::ManifestPass(vec![p("/repo/crates/cli")]),
            "a manifest declares its own package; a sibling's pins do not move with it"
        );
    }

    /// S11. A manifest or lockfile change re-parses ZERO source files.
    ///
    /// Asserted through the TYPE: neither manifest arm can carry a source file,
    /// so this cannot regress into "re-index the repo on a version bump" —
    /// the waste R14's structure/work split exists to avoid.
    #[test]
    fn a_dependency_change_reparses_no_source_files() {
        let (root, locks, manifests) = workspace();
        for changed in ["/repo/Cargo.lock", "/repo/crates/cli/Cargo.toml"] {
            let got = retrigger_for(&p(changed), &root, &manifests, &locks, CARGO_TOML, CARGO_LOCK);
            assert!(
                matches!(got, Retrigger::ManifestPass(_)),
                "{changed} is a dependency change, not a code change"
            );
        }
    }

    /// And a source file takes the parse path, not the manifest one.
    #[test]
    fn a_source_change_takes_the_parse_path() {
        let (root, locks, manifests) = workspace();
        let got = retrigger_for(
            &p("/repo/crates/cli/src/main.rs"),
            &root,
            &manifests,
            &locks,
            CARGO_TOML,
            CARGO_LOCK,
        );
        assert_eq!(got, Retrigger::Parse(p("/repo/crates/cli/src/main.rs")));
    }

    /// A root itself is under itself. The repo directory changing (a
    /// `.gitignore` edit, a manifest) is a change IN that repo, not a new one.
    #[test]
    fn a_root_path_is_owned_by_itself() {
        let roots = vec![p("/w/repo")];
        assert_eq!(owning_root(&p("/w/repo"), &roots), Some(&p("/w/repo")));
    }
}
