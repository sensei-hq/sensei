//! `scripts/release-guard.sh` (wave 0b, user 2026-10-22): a release is cut only
//! from `main` — clean, and exactly what `origin/main` holds — so a version tag
//! never lands on develop or on an unpushed commit. Driven against real
//! throwaway git repositories, not fakes.

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/release-guard.sh")
}

/// A command that sees only `dir`'s repository. Run from a git hook (the
/// pre-commit runs `make test-fast`), the environment carries `GIT_DIR`,
/// `GIT_INDEX_FILE` and friends for the repository being committed; a child
/// `git` that inherits them writes THAT repository's config and index instead
/// of the throwaway one — `user.email = t@example.com` landed in the real
/// `.git/config` that way. Every `GIT_*` variable is dropped.
fn isolated(program: &str, dir: &Path) -> Command {
    let mut c = Command::new(program);
    c.current_dir(dir);
    for (k, _) in std::env::vars_os() {
        if k.to_string_lossy().starts_with("GIT_") {
            c.env_remove(k);
        }
    }
    c
}

fn git(dir: &Path, args: &[&str]) {
    let o = isolated("git", dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&o.stderr));
}

/// A clone of a bare `origin` with one commit on `main` and a `develop` branch.
fn repo() -> (tempfile::TempDir, PathBuf) {
    let t = tempfile::tempdir().unwrap();
    let origin = t.path().join("origin.git");
    let work = t.path().join("work");
    git(t.path(), &["init", "--bare", "-b", "main", origin.to_str().unwrap()]);
    git(t.path(), &["clone", origin.to_str().unwrap(), work.to_str().unwrap()]);
    for (k, v) in [("user.email", "t@example.com"), ("user.name", "t"), ("commit.gpgsign", "false")]
    {
        git(&work, &["config", k, v]);
    }
    std::fs::write(work.join("VERSION"), "0.1.0\n").unwrap();
    git(&work, &["add", "VERSION"]);
    git(&work, &["commit", "-m", "init"]);
    git(&work, &["push", "-u", "origin", "main"]);
    git(&work, &["branch", "develop"]);
    (t, work)
}

fn guard(dir: &Path) -> std::process::Output {
    isolated("bash", dir).arg(script()).output().unwrap()
}

#[test]
fn a_clean_main_that_matches_origin_may_release() {
    let (_t, work) = repo();
    let o = guard(&work);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
}

#[test]
fn a_release_from_develop_is_refused() {
    let (_t, work) = repo();
    git(&work, &["checkout", "develop"]);
    let o = guard(&work);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("main"), "says to release from main");
}

#[test]
fn uncommitted_changes_refuse_a_release() {
    let (_t, work) = repo();
    std::fs::write(work.join("VERSION"), "0.2.0\n").unwrap();
    let o = guard(&work);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("clean"));
}

#[test]
fn a_main_that_is_not_what_origin_holds_refuses_a_release() {
    let (_t, work) = repo();
    std::fs::write(work.join("NOTE"), "x\n").unwrap();
    git(&work, &["add", "NOTE"]);
    git(&work, &["commit", "-m", "unpushed"]);
    let o = guard(&work);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("origin/main"));
}
