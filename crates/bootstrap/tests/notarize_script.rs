//! `scripts/ci-notarize.sh`, driven with a fake `xcrun` and `ditto` (#282).
//!
//! `notarytool submit --wait` can finish on a submission Apple REJECTED; the release
//! must fail there and show Apple's log, not ship a binary Gatekeeper will refuse.
//! Unix only: the script and its fakes are shell scripts.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/ci-notarize.sh")
}

/// Fakes: `xcrun notarytool submit` prints `{"id":"sub-1","status":"$STATUS"}`;
/// `xcrun notarytool log` prints a fixed rejection log; `ditto` writes the zip.
/// Every call is appended to `$CALLS`. Like the real `ditto -c -k`, the fake refuses
/// more than one source.
fn run(dir: &Path, status: &str) -> (std::process::Output, String) {
    run_with(dir, status, &["senseid"])
}

fn run_with(dir: &Path, status: &str, binaries: &[&str]) -> (std::process::Output, String) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, body) in [
        (
            "xcrun",
            "#!/bin/sh\necho \"xcrun $*\" >> \"$CALLS\"\ncase \"$2\" in\n  submit) echo \"{\\\"id\\\":\\\"sub-1\\\",\\\"status\\\":\\\"$STATUS\\\"}\" ;;\n  log) echo 'The binary is not signed with a valid Developer ID certificate.' ;;\nesac\n",
        ),
        (
            "ditto",
            "#!/bin/sh\necho \"ditto $*\" >> \"$CALLS\"\nn=0; for a; do case \"$a\" in -*) ;; *) n=$((n+1)) ;; esac; done\n[ \"$n\" -eq 2 ] || { echo \"ditto: Can't archive multiple sources\" >&2; exit 1; }\nfor last; do :; done\ntouch \"$last\"\n",
        ),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        Command::new("chmod").arg("+x").arg(&p).status().unwrap();
    }
    let targets: Vec<PathBuf> = binaries
        .iter()
        .map(|b| {
            let p = dir.join(b);
            std::fs::write(&p, "x").unwrap();
            p
        })
        .collect();
    let calls = dir.join("calls.log");
    let out = Command::new("bash")
        .arg(script())
        .args(&targets)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("RUNNER_TEMP", dir)
        .env("STATUS", status)
        .env("CALLS", &calls)
        .env("APPLE_API_KEY", "KEY123")
        .env("APPLE_API_ISSUER", "issuer-uuid")
        .env("APPLE_API_KEY_P8", "-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----")
        .output()
        .unwrap();
    (out, std::fs::read_to_string(&calls).unwrap_or_default())
}

#[test]
fn an_accepted_submission_passes_and_uses_the_api_key() {
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run(t.path(), "Accepted");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(calls.contains("notarytool submit"), "{calls}");
    assert!(calls.contains("--key-id KEY123"), "{calls}");
    assert!(calls.contains("--issuer issuer-uuid"), "{calls}");
    assert!(calls.contains("--wait"), "{calls}");
}

#[test]
fn a_rejected_submission_fails_and_shows_apples_log() {
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run(t.path(), "Invalid");
    assert!(!o.status.success(), "a rejected submission must fail the release");
    assert!(calls.contains("notarytool log sub-1"), "{calls}");
    let said =
        format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(said.contains("not signed with a valid Developer ID"), "{said}");
}

#[test]
fn the_private_key_is_written_only_for_the_call_and_removed() {
    let t = tempfile::tempdir().unwrap();
    let _ = run(t.path(), "Accepted");
    let left: Vec<_> = std::fs::read_dir(t.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "p8"))
        .collect();
    assert!(left.is_empty(), "the .p8 must not outlive the job step");
}

#[test]
fn the_release_s_three_binaries_go_up_in_one_archive() {
    // release.yml passes senseid, sensei and sensei-mcp; `ditto -c -k` archives ONE
    // source, so v0.22.0's notarize step failed with "Can't archive multiple sources".
    let t = tempfile::tempdir().unwrap();
    let (o, calls) = run_with(t.path(), "Accepted", &["senseid", "sensei", "sensei-mcp"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(calls.matches("ditto ").count(), 1, "{calls}");
    assert_eq!(calls.matches("notarytool submit").count(), 1, "{calls}");
}
