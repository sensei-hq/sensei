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
