//! Stage 2 — scan repo: submodules, subtrees, manifests, lockfiles, files.
//!
//! Spec: `docs/spec/indexer/02-scan-repo.md`.
//!
//! Given ONE repo root, discover everything structural inside it. It produces
//! structure; it parses no source.
//!
//! Four of the five entry points are PURE and take TEXT or PATHS rather than a
//! repo, so their suites run on literals. That is deliberate: stages 1-3 were
//! ordered first precisely because they can be proven without a database, and
//! a function that needs one to be tested has lost that property.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ── S1: submodules, declared in .gitmodules ──────────────────────────────

/// One `[submodule "…"]` stanza.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleDecl {
    /// The stanza's name — `[submodule "NAME"]`. Not necessarily the path.
    pub name: String,
    /// Repo-relative path the submodule is checked out at.
    pub path: String,
    pub url: Option<String>,
}

/// Parse `.gitmodules` CONTENT (S1).
///
/// Takes the text, not a path, so the whole test suite is string literals.
///
/// Tolerant by design: a malformed stanza is skipped and the rest still
/// parse. One bad entry must not cost a repo every other submodule — the
/// failure mode the spec's table calls out.
pub fn find_submodules(gitmodules: &str) -> Vec<SubmoduleDecl> {
    let mut out: Vec<SubmoduleDecl> = Vec::new();
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut url: Option<String> = None;

    // A stanza is complete when the NEXT header starts or the text ends.
    fn flush(
        out: &mut Vec<SubmoduleDecl>,
        name: &mut Option<String>,
        path: &mut Option<String>,
        url: &mut Option<String>,
    ) {
        if let (Some(n), Some(p)) = (name.take(), path.take()) {
            // A stanza with a name but no path declares nothing locatable, so
            // it is dropped rather than recorded as a phantom submodule.
            out.push(SubmoduleDecl { name: n, path: p, url: url.take() });
        } else {
            *url = None;
        }
    }

    for raw in gitmodules.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("[submodule ") {
            flush(&mut out, &mut name, &mut path, &mut url);
            name = rest.trim_end_matches(']').trim().trim_matches('"').to_string().into();
            continue;
        }
        if line.starts_with('[') {
            // Some other section — end the current stanza, ignore the section.
            flush(&mut out, &mut name, &mut path, &mut url);
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        match k.trim() {
            "path" => path = Some(v.trim().to_string()),
            "url" => url = Some(v.trim().to_string()),
            _ => {}
        }
    }
    flush(&mut out, &mut name, &mut path, &mut url);
    out
}

// ── S6c: which lockfile serves a manifest ────────────────────────────────

/// Resolve the lockfile serving `manifest_dir`: the NEAREST one at or above
/// it, stopping at `repo_root` (S6c).
///
/// Not "same folder" and not "repo root" — measured on this repository, all
/// three rules give different answers:
///
/// ```text
/// crates/senseid/Cargo.toml      -> /Cargo.lock          (walks up)
/// app/src-tauri/Cargo.toml       -> app/src-tauri/Cargo.lock  (its OWN wins)
/// marketplace/package.json       -> None
/// ```
///
/// A "use the repo root's lockfile" rule gets two of those three wrong.
///
/// PURE: `candidates` is the set of lockfile paths already discovered by the
/// walk, so this does no IO and is testable on literals.
pub fn nearest_lockfile(
    manifest_dir: &Path,
    repo_root: &Path,
    filenames: &[&str],
    candidates: &[PathBuf],
) -> Option<PathBuf> {
    let mut dir = manifest_dir;
    loop {
        for name in filenames {
            let want = dir.join(name);
            if candidates.iter().any(|c| c == &want) {
                return Some(want);
            }
        }
        if dir == repo_root {
            return None;
        }
        match dir.parent() {
            // Never climb out of the repo: a sibling checkout's lockfile is
            // not this repo's, and following the filesystem past the root
            // would silently borrow one.
            Some(p) if p.starts_with(repo_root) || p == repo_root => dir = p,
            _ => return None,
        }
    }
}

/// Invert [`nearest_lockfile`] — every manifest a lockfile serves (09 S10).
///
/// The incremental path needs this to fan a lockfile change out to the right
/// manifests, and it MUST be the same resolution inverted rather than a
/// second implementation: two copies of "which manifests does this lock
/// serve" will disagree, and the disagreement is silent in the direction that
/// leaves stale pins.
pub fn manifests_served_by(
    lockfile: &Path,
    repo_root: &Path,
    filenames: &[&str],
    lockfiles: &[PathBuf],
    manifest_dirs: &[PathBuf],
) -> Vec<PathBuf> {
    manifest_dirs
        .iter()
        .filter(|d| {
            nearest_lockfile(d, repo_root, filenames, lockfiles).as_deref() == Some(lockfile)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── S1 ───────────────────────────────────────────────────────────────

    #[test]
    fn parses_two_submodule_stanzas() {
        let s = r#"
[submodule "homebrew"]
	path = homebrew
	url = git@github.com:sensei-hq/homebrew-tap.git
[submodule "marketplace"]
	path = marketplace
	url = https://github.com/sensei-hq/marketplace
"#;
        let subs = find_submodules(s);
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].name, "homebrew");
        assert_eq!(subs[0].path, "homebrew");
        assert_eq!(subs[0].url.as_deref(), Some("git@github.com:sensei-hq/homebrew-tap.git"));
        assert_eq!(subs[1].path, "marketplace");
    }

    #[test]
    fn a_malformed_stanza_does_not_cost_the_others() {
        // The spec's failure mode: parse what is valid, drop the rest. One bad
        // entry must not lose a repo every other submodule.
        let s = r#"
[submodule "broken"]
	url = git@example.com:x.git
[submodule "good"]
	path = vendor/good
	url = git@example.com:good.git
"#;
        let subs = find_submodules(s);
        assert_eq!(subs.len(), 1, "the path-less stanza declares nothing locatable");
        assert_eq!(subs[0].name, "good");
        assert_eq!(subs[0].url.as_deref(), Some("git@example.com:good.git"));
    }

    #[test]
    fn a_url_from_a_dropped_stanza_does_not_leak_into_the_next() {
        let s = "[submodule \"a\"]\n\turl = git@example.com:a.git\n\
                 [submodule \"b\"]\n\tpath = b\n";
        let subs = find_submodules(s);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].path, "b");
        assert_eq!(subs[0].url, None, "a's url must not attach to b");
    }

    #[test]
    fn no_gitmodules_content_is_zero_submodules_not_an_error() {
        assert!(find_submodules("").is_empty());
        assert!(find_submodules("# nothing here\n").is_empty());
    }

    #[test]
    fn a_non_submodule_section_ends_the_stanza() {
        let s = "[submodule \"a\"]\n\tpath = a\n[core]\n\tpath = not-a-submodule\n";
        let subs = find_submodules(s);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].path, "a");
    }

    // ── S6c ──────────────────────────────────────────────────────────────

    const CARGO: &[&str] = &["Cargo.lock"];

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn a_manifest_walks_up_to_the_root_lockfile() {
        // crates/senseid/Cargo.toml has no sibling lock -> /Cargo.lock
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock")];
        assert_eq!(
            nearest_lockfile(&p("/repo/crates/senseid"), &root, CARGO, &locks),
            Some(p("/repo/Cargo.lock"))
        );
    }

    #[test]
    fn a_nearer_lockfile_wins_over_the_root() {
        // app/src-tauri has its OWN Cargo.lock — a "use the repo root" rule
        // would hand it another workspace's pins.
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock"), p("/repo/app/src-tauri/Cargo.lock")];
        assert_eq!(
            nearest_lockfile(&p("/repo/app/src-tauri"), &root, CARGO, &locks),
            Some(p("/repo/app/src-tauri/Cargo.lock")),
            "nearest wins, not root"
        );
    }

    #[test]
    fn no_lockfile_anywhere_above_is_none_not_a_guess() {
        // marketplace/package.json has none. S6d: the version stays a RANGE.
        let root = p("/repo");
        let locks = vec![p("/repo/app/bun.lock")];
        assert_eq!(nearest_lockfile(&p("/repo/marketplace"), &root, &["bun.lock"], &locks), None);
    }

    #[test]
    fn resolution_never_climbs_out_of_the_repo() {
        // A sibling checkout's lockfile is not this repo's.
        let root = p("/repo");
        let locks = vec![p("/Cargo.lock")];
        assert_eq!(nearest_lockfile(&p("/repo/crates/x"), &root, CARGO, &locks), None);
    }

    #[test]
    fn the_root_manifest_finds_the_root_lockfile() {
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock")];
        assert_eq!(nearest_lockfile(&root, &root, CARGO, &locks), Some(p("/repo/Cargo.lock")));
    }

    // ── 09 S10: the inverse, for the incremental fan-out ─────────────────

    #[test]
    fn a_root_lockfile_serves_every_manifest_with_no_nearer_one() {
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock"), p("/repo/app/src-tauri/Cargo.lock")];
        let manifests = vec![
            p("/repo"),
            p("/repo/crates/senseid"),
            p("/repo/crates/cli"),
            p("/repo/app/src-tauri"),
        ];

        let served = manifests_served_by(&p("/repo/Cargo.lock"), &root, CARGO, &locks, &manifests);
        assert_eq!(served, vec![p("/repo"), p("/repo/crates/senseid"), p("/repo/crates/cli")]);

        let own = manifests_served_by(
            &p("/repo/app/src-tauri/Cargo.lock"),
            &root,
            CARGO,
            &locks,
            &manifests,
        );
        assert_eq!(own, vec![p("/repo/app/src-tauri")], "the nearer lock serves only itself");
    }

    #[test]
    fn the_fan_out_is_the_resolver_inverted_not_a_second_rule() {
        // Every manifest the lockfile claims must independently resolve BACK
        // to it. Two implementations of this would disagree silently, and the
        // disagreement leaves stale pins.
        let root = p("/repo");
        let locks = vec![p("/repo/Cargo.lock"), p("/repo/tools/x/Cargo.lock")];
        let manifests =
            vec![p("/repo"), p("/repo/a"), p("/repo/b/c"), p("/repo/tools/x"), p("/repo/tools/y")];

        for lock in &locks {
            for m in manifests_served_by(lock, &root, CARGO, &locks, &manifests) {
                assert_eq!(
                    nearest_lockfile(&m, &root, CARGO, &locks).as_ref(),
                    Some(lock),
                    "{} was claimed by {} but resolves elsewhere",
                    m.display(),
                    lock.display()
                );
            }
        }
    }
}

// ── S4/S5/S6b: ONE walk that yields files, manifests and lockfiles ───────

/// Lockfile names, per ecosystem. Stage 2 S6b.
///
/// These live here rather than on `ManifestAdapter` for now because the trait
/// has no lockfile method yet; when it gains `lockfile_filenames()` this
/// constant is DELETED and the set becomes registry-derived, exactly as the
/// manifest set already is. Kept in one place so that swap is one edit.
pub const LOCKFILE_NAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "poetry.lock",
    "uv.lock",
    "Gemfile.lock",
    "composer.lock",
    "go.sum",
    "Package.resolved",
];

/// Everything one repo walk found. Paths are ABSOLUTE.
#[derive(Debug, Clone, Default)]
pub struct RepoScan {
    pub files: Vec<PathBuf>,
    pub folders: Vec<PathBuf>,
    /// Manifests, with the adapter's ecosystem already resolved.
    pub manifests: Vec<(PathBuf, &'static str)>,
    pub lockfiles: Vec<PathBuf>,
    /// Paths the ignore rules excluded, counted so the delta against the old
    /// scan is explainable rather than mysterious.
    pub ignored: usize,
}

impl RepoScan {
    /// The distinct directories holding a manifest — the input to S6c and the
    /// grain manifest facts are stored at (D11).
    pub fn manifest_dirs(&self) -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> =
            self.manifests.iter().filter_map(|(p, _)| p.parent().map(Path::to_path_buf)).collect();
        v.sort();
        v.dedup();
        v
    }
}

/// Walk one repo root ONCE, collecting the file set and, as each entry passes,
/// testing it against the manifest registry and the lockfile names (S4, S5,
/// S6b).
///
/// ONE traversal, not three. `scan_repo` already has to visit every file for
/// the file set, so testing each name costs a string comparison; a separate
/// manifest glob would re-walk the tree to find ~200 entries AND need its own
/// exclusion rules, which is a second place for them to drift.
///
/// Ignore rules come from the SHARED `build_walker` (S4): `.gitignore`,
/// `.ignore`, the global gitignore, `.git/info/exclude`, and
/// `require_git(false)` so they still apply in a non-git directory. Reusing it
/// is what stops the fs-watcher and the scan disagreeing about what belongs in
/// the index — a disagreement that previously caused a permanent add/prune
/// churn loop.
pub fn scan_repo_files(repo_root: &Path) -> RepoScan {
    let mut scan = RepoScan::default();
    for entry in crate::tasks::handlers::helpers::build_walker(repo_root).build() {
        let Ok(entry) = entry else {
            scan.ignored += 1;
            continue;
        };
        let path = entry.path();
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        if is_dir {
            scan.folders.push(path.to_path_buf());
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };

        // Tested as the entry passes — no second traversal.
        if let Some(a) = crate::adapters::manifest::manifest_adapter_for_filename(name) {
            scan.manifests.push((path.to_path_buf(), a.ecosystem()));
        } else if LOCKFILE_NAMES.contains(&name) {
            scan.lockfiles.push(path.to_path_buf());
        }
        scan.files.push(path.to_path_buf());
    }
    scan.files.sort();
    scan.folders.sort();
    scan.manifests.sort();
    scan.lockfiles.sort();
    scan
}

#[cfg(test)]
mod walk_tests {
    use super::*;

    fn touch(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn rels(v: &[PathBuf], base: &Path) -> Vec<String> {
        v.iter()
            .filter_map(|p| p.strip_prefix(base).ok())
            .map(|p| p.to_string_lossy().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    #[test]
    fn one_walk_yields_files_manifests_and_lockfiles() {
        // S5/S6b: manifests and lockfiles are recognised AS the file set is
        // built, not by a second glob.
        let t = tempfile::tempdir().unwrap();
        touch(t.path(), "Cargo.toml", "[package]\nname='x'\n");
        touch(t.path(), "Cargo.lock", "# lock\n");
        touch(t.path(), "src/main.rs", "fn main() {}");
        touch(t.path(), "app/package.json", "{\"name\":\"a\"}");

        let scan = scan_repo_files(t.path());

        assert!(rels(&scan.files, t.path()).contains(&"src/main.rs".to_string()));
        let manifests =
            rels(&scan.manifests.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(), t.path());
        assert!(manifests.contains(&"Cargo.toml".to_string()));
        assert!(manifests.contains(&"app/package.json".to_string()));
        assert_eq!(rels(&scan.lockfiles, t.path()), vec!["Cargo.lock"]);
    }

    #[test]
    fn a_gitignored_file_is_not_in_the_set() {
        // S4. The shared walker's rules apply, including in a NON-git
        // directory (require_git(false)) — which is the case a plain WalkDir
        // gets wrong and the watcher/scan churn loop came from.
        let t = tempfile::tempdir().unwrap();
        touch(t.path(), ".gitignore", "secret.rs\nbuilt/\n");
        touch(t.path(), "keep.rs", "");
        touch(t.path(), "secret.rs", "");
        touch(t.path(), "built/out.rs", "");

        let files = rels(&scan_repo_files(t.path()).files, t.path());

        assert!(files.contains(&"keep.rs".to_string()));
        assert!(!files.contains(&"secret.rs".to_string()), "gitignored file leaked in");
        assert!(!files.contains(&"built/out.rs".to_string()), "gitignored dir leaked in");
    }

    #[test]
    fn manifest_dirs_are_the_grain_facts_are_stored_at() {
        // D11: a manifest sits AT a folder, so that folder is the grain. Two
        // manifests in one directory are ONE directory.
        let t = tempfile::tempdir().unwrap();
        touch(t.path(), "Cargo.toml", "");
        touch(t.path(), "package.json", "{}");
        touch(t.path(), "app/package.json", "{}");

        let dirs = rels(&scan_repo_files(t.path()).manifest_dirs(), t.path());

        assert_eq!(
            dirs,
            vec!["app"],
            "the root dedupes to an empty relative path, app is distinct"
        );
    }

    #[test]
    fn the_walk_and_the_lockfile_resolver_compose() {
        // The two halves of S6: the walk finds the candidates, the resolver
        // picks per manifest. Proven together rather than separately.
        let t = tempfile::tempdir().unwrap();
        touch(t.path(), "Cargo.toml", "");
        touch(t.path(), "Cargo.lock", "");
        touch(t.path(), "crates/a/Cargo.toml", "");
        touch(t.path(), "tools/x/Cargo.toml", "");
        touch(t.path(), "tools/x/Cargo.lock", "");

        let scan = scan_repo_files(t.path());
        let locks = scan.lockfiles.clone();

        assert_eq!(
            nearest_lockfile(&t.path().join("crates/a"), t.path(), &["Cargo.lock"], &locks),
            Some(t.path().join("Cargo.lock")),
            "a member with no sibling lock walks up"
        );
        assert_eq!(
            nearest_lockfile(&t.path().join("tools/x"), t.path(), &["Cargo.lock"], &locks),
            Some(t.path().join("tools/x/Cargo.lock")),
            "its own lock wins over the root's"
        );
    }
}

/// Every pin in one lockfile, indexed by package name (02 S6b, 02b S11).
///
/// The ecosystem picks the adapter and the FILENAME picks the reader within
/// it — npm alone has several lockfile grammars. An ecosystem with no
/// registered reader yields an empty map, which is a named gap: the caller
/// falls back to the manifest range and nothing is fabricated.
pub fn pins_by_name(ecosystem: &str, filename: &str, content: &str) -> BTreeMap<String, String> {
    crate::adapters::manifest::registered_adapters()
        .iter()
        .filter(|a| a.ecosystem() == ecosystem && a.accepts_lockfile(filename))
        .flat_map(|a| a.parse_lockfile(filename, content))
        .map(|p| (p.name, p.version))
        .collect()
}

/// The version to record for one DIRECT dependency.
///
/// The lockfile pin when it has one, else the manifest's own version. Looked
/// up BY NAME — the caller never enumerates the lockfile, which is what keeps
/// the transitive tree out while still getting the resolved pin (02b S11).
///
/// A miss returns the manifest version unchanged rather than nothing: the
/// project really does depend on that package, and dropping it would lose a
/// real dependency to a missing lockfile entry.
pub fn resolve_pin(pins: &BTreeMap<String, String>, name: &str, manifest_version: &str) -> String {
    pins.get(name).cloned().unwrap_or_else(|| manifest_version.to_string())
}

#[cfg(test)]
mod pin_tests {
    use super::*;

    #[test]
    fn a_direct_dep_takes_the_lockfile_pin_over_the_manifest_floor() {
        // The whole point of reading a lockfile. `^2.60.1` cleans to `2.60.1`,
        // which is a range FLOOR indistinguishable from a pin; the lockfile
        // holds what is actually installed.
        let lock = r#"{"packages": {"@sveltejs/kit": ["@sveltejs/kit@2.69.2", "", {}, "sha"]}}"#;
        let pins = pins_by_name("npm", "bun.lock", lock);

        assert_eq!(resolve_pin(&pins, "@sveltejs/kit", "2.60.1"), "2.69.2");
    }

    #[test]
    fn a_dep_absent_from_the_lockfile_keeps_the_manifest_version() {
        // Honest fallback: no lockfile entry means no better answer exists.
        let pins = pins_by_name("npm", "bun.lock", r#"{"packages": {}}"#);
        assert_eq!(resolve_pin(&pins, "left-pad", "1.0.0"), "1.0.0");
    }

    #[test]
    fn looking_up_by_name_does_not_drag_in_the_transitive_tree() {
        // A lockfile lists EVERY package. Reading it must not turn 2 direct
        // deps into 4 libraries — the manifest selects, the lockfile supplies
        // the version (02b S11).
        let lock = r#"{"packages": {
            "direct": ["direct@1.0.0", "", {}, "s"],
            "transitive-a": ["transitive-a@9.9.9", "", {}, "s"],
            "transitive-b": ["transitive-b@8.8.8", "", {}, "s"]
        }}"#;
        let pins = pins_by_name("npm", "bun.lock", lock);
        assert_eq!(pins.len(), 3, "the reader sees all three");

        // ...but the caller only ever asks about what the manifest declared.
        let direct = ["direct"];
        let resolved: Vec<String> = direct.iter().map(|d| resolve_pin(&pins, d, "1.0.0")).collect();
        assert_eq!(resolved, vec!["1.0.0"], "one dep in, one version out");
    }

    #[test]
    fn an_unknown_ecosystem_yields_no_pins_rather_than_guessing() {
        assert!(pins_by_name("elvish", "Elv.lock", "whatever").is_empty());
    }
}

#[cfg(test)]
mod corpus_check {
    use super::*;

    /// Run the walk over THIS repository and print what it found.
    ///
    /// `#[ignore]` because it depends on the working tree, not because it is
    /// unimportant: the spec's numbers (six manifest-bearing folders, the
    /// three Cargo.lock positions) were measured here, and this is how they
    /// stay honest. `cargo test -p senseid corpus_check -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn walk_this_repo() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf();
        let scan = scan_repo_files(&root);
        println!("root            {}", root.display());
        println!("files           {}", scan.files.len());
        println!("folders         {}", scan.folders.len());
        println!("manifests       {}", scan.manifests.len());
        println!("manifest dirs   {}", scan.manifest_dirs().len());
        println!("lockfiles       {}", scan.lockfiles.len());
        for (p, eco) in &scan.manifests {
            println!("  manifest {:8} {}", eco, p.strip_prefix(&root).unwrap().display());
        }
        for l in &scan.lockfiles {
            println!("  lock          {}", l.strip_prefix(&root).unwrap().display());
        }
    }

    /// Resolve every manifest in THIS repo to its nearest lockfile, read the
    /// pins, and report where a pin differs from the manifest's own version.
    ///
    /// `#[ignore]`: it reads the working tree. This is the measurement behind
    /// 02b S11's claim that a cleaned manifest version is a range FLOOR, not a
    /// pin — run it after a `bun install` and the numbers move.
    #[test]
    #[ignore]
    fn pins_in_this_repo() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf();
        let scan = scan_repo_files(&root);
        let dirs = scan.manifest_dirs();

        let (mut served, mut unserved, mut differing, mut total) = (0, 0, 0, 0);
        for (manifest, eco) in &scan.manifests {
            let Some(dir) = manifest.parent() else { continue };
            let Some(adapter) = crate::adapters::manifest::registered_adapters()
                .iter()
                .find(|a| a.ecosystem() == *eco)
            else {
                continue;
            };
            let lock = nearest_lockfile(dir, &root, adapter.lockfile_filenames(), &scan.lockfiles);
            let rel = manifest.strip_prefix(&root).unwrap().display().to_string();

            let Some(lock) = lock else {
                unserved += 1;
                println!("  {rel:52} NO LOCKFILE (versions stay ranges)");
                continue;
            };
            served += 1;
            let Ok(content) = std::fs::read_to_string(&lock) else { continue };
            let name = lock.file_name().unwrap().to_str().unwrap();
            let pins = pins_by_name(eco, name, &content);

            let Ok(mtext) = std::fs::read_to_string(manifest) else { continue };
            let mut moved = Vec::new();
            for dep in adapter.parse_dependencies(&mtext) {
                if dep.local_source.is_some() {
                    continue; // a workspace sibling is first-party, not a library
                }
                total += 1;
                let pin = resolve_pin(&pins, &dep.lib_name, &dep.version);
                if pin != dep.version {
                    differing += 1;
                    moved.push(format!("{} {} -> {}", dep.lib_name, dep.version, pin));
                }
            }
            println!(
                "  {rel:52} lock={:28} pins={:4} moved={}",
                lock.strip_prefix(&root).unwrap().display(),
                pins.len(),
                moved.len()
            );
            for m in moved.iter().take(4) {
                println!("      {m}");
            }
        }
        println!(
            "\nmanifests {} | manifest dirs {} | served by a lockfile {} | unserved {}",
            scan.manifests.len(),
            dirs.len(),
            served,
            unserved
        );
        println!("direct external deps {total} | version corrected by the lockfile {differing}");
    }
}
