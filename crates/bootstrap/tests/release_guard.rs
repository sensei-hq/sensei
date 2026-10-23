//! `scripts/release-guard.sh` (wave 0b, user 2026-10-22). Two steps, because
//! `main` is protected (a push needs its status checks): the version BUMP is a
//! commit on a clean develop that reaches main through the PR; the RELEASE tags
//! main's head — clean, exactly `origin/main`, its VERSION not yet tagged — and
//! pushes only the tag. Driven against real throwaway git repositories.

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

fn guard(dir: &Path, mode: &str) -> std::process::Output {
    isolated("bash", dir).arg(script()).arg(mode).output().unwrap()
}

fn err(o: &std::process::Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

// ── the release: tag main ────────────────────────────────────────────────────

#[test]
fn a_clean_main_that_matches_origin_may_release() {
    let (_t, work) = repo();
    let o = guard(&work, "release");
    assert!(o.status.success(), "{}", err(&o));
}

#[test]
fn a_release_from_develop_is_refused() {
    let (_t, work) = repo();
    git(&work, &["checkout", "develop"]);
    let o = guard(&work, "release");
    assert!(!o.status.success());
    assert!(err(&o).contains("main"), "says to release from main");
}

#[test]
fn uncommitted_changes_refuse_a_release() {
    let (_t, work) = repo();
    std::fs::write(work.join("VERSION"), "0.2.0\n").unwrap();
    let o = guard(&work, "release");
    assert!(!o.status.success());
    assert!(err(&o).contains("clean"));
}

#[test]
fn a_main_that_is_not_what_origin_holds_refuses_a_release() {
    let (_t, work) = repo();
    std::fs::write(work.join("NOTE"), "x\n").unwrap();
    git(&work, &["add", "NOTE"]);
    git(&work, &["commit", "-m", "unpushed"]);
    let o = guard(&work, "release");
    assert!(!o.status.success());
    assert!(err(&o).contains("origin/main"));
}

#[test]
fn a_version_already_tagged_is_not_released_again() {
    let (_t, work) = repo();
    git(&work, &["tag", "v0.1.0"]);
    git(&work, &["push", "origin", "v0.1.0"]);
    let o = guard(&work, "release");
    assert!(!o.status.success(), "one tag per version");
    assert!(err(&o).contains("v0.1.0"), "names the tag: {}", err(&o));
}

// ── the bump: a commit on develop ────────────────────────────────────────────

#[test]
fn a_clean_develop_that_matches_origin_may_bump() {
    let (_t, work) = repo();
    git(&work, &["checkout", "develop"]);
    git(&work, &["push", "-u", "origin", "develop"]);
    let o = guard(&work, "bump");
    assert!(o.status.success(), "{}", err(&o));
}

#[test]
fn a_bump_on_main_is_refused() {
    let (_t, work) = repo();
    let o = guard(&work, "bump");
    assert!(!o.status.success());
    assert!(err(&o).contains("develop"), "says to bump on develop");
}

#[test]
fn an_unknown_step_is_refused() {
    let (_t, work) = repo();
    assert!(!guard(&work, "ship").status.success());
}
