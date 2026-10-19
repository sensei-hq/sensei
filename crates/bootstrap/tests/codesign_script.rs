//! `scripts/codesign-local.sh`, driven with a fake `security` and `codesign`.
//!
//! macOS ties a keychain item's "Always Allow", and folder-access (TCC) grants,
//! to the program's code signature. Local installs were ad-hoc signed, whose
//! identity — and default identifier, `senseid-<hash>` — changes every build, so
//! every rebuild looked like a new program and the keychain asked again.

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/codesign-local.sh")
}

/// Serialises the runs in this file. Each one writes fake tools and then execs
/// them; another test thread forking at that moment can hold the new file open,
/// and the exec then fails — which the script reads as "tool absent". Seen once
/// under the pre-commit hook.
static RUN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Fakes: `security find-identity` lists `sensei-dev` when `$HAS_DEV_ID=1`;
/// `codesign` appends its arguments to `$CODESIGN_LOG`.
fn run(dir: &Path, has_dev_id: bool, extra: &[(&str, &str)]) -> (std::process::Output, String) {
    let _one_at_a_time = RUN.lock().unwrap_or_else(|e| e.into_inner());
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, body) in [
        (
            "security",
            "#!/bin/sh\n[ \"$HAS_DEV_ID\" = 1 ] && echo '  1) ABCDEF \"sensei-dev\"'\necho '     1 valid identities found'\n",
        ),
        ("codesign", "#!/bin/sh\necho \"$@\" >> \"$CODESIGN_LOG\"\n"),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        Command::new("chmod").arg("+x").arg(&p).status().unwrap();
    }
    let target = dir.join("senseid");
    std::fs::write(&target, "x").unwrap();
    let log = dir.join("codesign.log");
    let mut c = Command::new("bash");
    c.arg(script())
        .arg(&target)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("HAS_DEV_ID", if has_dev_id { "1" } else { "0" })
        .env("CODESIGN_LOG", &log)
        .env_remove("SENSEI_SIGN_IDENTITY");
    for (k, v) in extra {
        c.env(k, v);
    }
    let out = c.output().unwrap();
    (out, std::fs::read_to_string(&log).unwrap_or_default())
}

#[test]
fn a_dev_identity_in_the_keychain_signs_with_a_stable_identifier() {
    let t = tempfile::tempdir().unwrap();
    let (o, log) = run(t.path(), true, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(log.contains("--sign sensei-dev"), "{log}");
    assert!(log.contains("--identifier dev.sensei.senseid"), "{log}");
}

#[test]
fn without_one_it_signs_ad_hoc_says_so_and_still_fixes_the_identifier() {
    let t = tempfile::tempdir().unwrap();
    let (o, log) = run(t.path(), false, &[]);
    assert!(o.status.success());
    assert!(log.contains("--sign -"), "{log}");
    assert!(log.contains("--identifier dev.sensei.senseid"), "{log}");
    assert!(
        String::from_utf8_lossy(&o.stdout).contains("make dev-signing-cert"),
        "says how to fix it"
    );
}

#[test]
fn an_explicit_identity_wins() {
    let t = tempfile::tempdir().unwrap();
    let (_, log) = run(t.path(), true, &[("SENSEI_SIGN_IDENTITY", "Developer ID Application: X")]);
    assert!(log.contains("--sign Developer ID Application: X"), "{log}");
}
