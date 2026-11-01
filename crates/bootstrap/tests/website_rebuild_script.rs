//! `scripts/rebuild-website.sh`, driven with a fake `curl` (#346).
//!
//! The website is its own repository now; its Sensei page reads the version this repo
//! released. A release rebuilds it through a Cloudflare deploy hook and must see the new
//! version live — a hook that was never configured, or a site still showing the old
//! version, fails the release job rather than passing quietly.

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/rebuild-website.sh")
}

/// Fake `curl`: a POST (the hook) is recorded; a GET of the raw VERSION answers `$RAW`;
/// any other GET answers with a page footer showing `v$SERVED`. Every call is appended
/// to `$CALLS`.
fn run(dir: &Path, hook: Option<&str>, served: &str) -> (std::process::Output, String) {
    run_with(dir, hook, served, "0.22.0")
}

fn run_with(dir: &Path, hook: Option<&str>, served: &str, raw: &str) -> (std::process::Output, String) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let curl = bin.join("curl");
    std::fs::write(
        &curl,
        "#!/bin/sh\necho \"curl $*\" >> \"$CALLS\"\ncase \"$*\" in\n  *-X\\ POST*) echo '{\"success\":true}' ;;\n  *VERSION*) echo \"$RAW\" ;;\n  *) echo \"<div class=\\\"mono footer-version\\\">v$SERVED</div>\" ;;\nesac\n",
    )
    .unwrap();
    Command::new("chmod").arg("+x").arg(&curl).status().unwrap();
    let calls = dir.join("calls.log");
    let mut cmd = Command::new("bash");
    cmd.arg(script())
        .arg("0.22.0")
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("SERVED", served)
        .env("RAW", raw)
        .env("CALLS", &calls)
        .env("WEBSITE_POLLS", "2")
        .env("WEBSITE_POLL_SECONDS", "0")
        .env_remove("WEBSITE_DEPLOY_HOOK");
    if let Some(h) = hook {
        cmd.env("WEBSITE_DEPLOY_HOOK", h);
    }
    let out = cmd.output().unwrap();
    (out, std::fs::read_to_string(&calls).unwrap_or_default())
}

#[test]
fn a_missing_hook_fails_without_calling_anything() {
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run(t.path(), None, "0.22.0");
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("WEBSITE_DEPLOY_HOOK"));
    assert_eq!(calls, "");
}

#[test]
fn the_hook_is_posted_and_the_new_version_seen_live() {
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run(t.path(), Some("https://hooks.example/abc"), "0.22.0");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(calls.contains("-X POST https://hooks.example/abc"), "{calls}");
    assert!(calls.contains("https://sensei-hq.com/sensei"), "{calls}");
}

#[test]
fn a_site_still_on_the_old_version_fails_the_job() {
    let t = tempfile::tempdir().unwrap();
    let (o, _) = run(t.path(), Some("https://hooks.example/abc"), "0.21.1");
    assert!(!o.status.success());
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("0.21.1") && err.contains("0.22.0"), "{err}");
}

/// GitHub's raw CDN can serve main's old VERSION for minutes after a release; a hook
/// fired then would rebuild the site with the old version. No hook until it is new.
#[test]
fn the_hook_waits_for_main_to_show_the_new_version() {
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run_with(t.path(), Some("https://hooks.example/abc"), "0.22.0", "0.21.1");
    assert!(!o.status.success());
    assert!(!calls.contains("-X POST"), "the hook fired before main showed 0.22.0: {calls}");
}
