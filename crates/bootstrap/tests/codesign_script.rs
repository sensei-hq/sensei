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

/// Fakes: `security find-identity` lists `sensei-dev` when `$HAS_DEV_ID=1`;
/// `codesign` appends its arguments to `$CODESIGN_LOG`.
fn run(dir: &Path, has_dev_id: bool, extra: &[(&str, &str)]) -> (std::process::Output, String) {
    run_on(dir, "senseid", has_dev_id, extra)
}

/// As [`run`], signing `target` (a file, or a `.app` directory when it ends so).
fn run_on(
    dir: &Path,
    target_name: &str,
    has_dev_id: bool,
    extra: &[(&str, &str)],
) -> (std::process::Output, String) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, body) in [
        (
            "security",
            // Like the real tool, more output follows the match: under
            // `pipefail` a `| grep -q` that exits at the match SIGPIPEs it, and
            // the pipeline read as "no identity" — the flake seen under the hook.
            "#!/bin/sh\n[ \"$HAS_DEV_ID\" = 1 ] && echo '  1) ABCDEF \"sensei-dev\"'\n[ \"$HAS_DEVELOPER_ID\" = 1 ] && echo '  2) 123456 \"Developer ID Application: Example Co (TEAM123456)\"'\ni=0; while [ $i -lt 20000 ]; do echo \"  $i) 0000 \\\"other\\\"\"; i=$((i+1)); done\necho '     1 valid identities found'\n",
        ),
        ("codesign", "#!/bin/sh\necho \"$@\" >> \"$CODESIGN_LOG\"\n"),
    ] {
        let p = bin.join(name);
        std::fs::write(&p, body).unwrap();
        Command::new("chmod").arg("+x").arg(&p).status().unwrap();
    }
    let target = dir.join(target_name);
    if target_name.ends_with(".app") {
        std::fs::create_dir_all(target.join("Contents/MacOS")).unwrap();
    } else {
        std::fs::write(&target, "x").unwrap();
    }
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

/// #284: the e2e app bundle is signed too, so its Screen Recording grant (which
/// the inspect harness's window capture needs) survives a rebuild. A bundle's
/// identifier is `dev.sensei.app.<name>`: `dev.sensei.sensei` is the CLI's, and
/// grants are keyed on identity.
#[test]
fn an_app_bundle_gets_its_own_stable_identifier() {
    let t = tempfile::tempdir().unwrap();
    let (o, log) = run_on(t.path(), "Sensei.app", true, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(log.contains("--identifier dev.sensei.app.sensei"), "{log}");
    assert!(log.contains("Sensei.app"), "signs the bundle itself: {log}");
}

/// #282: the company's Developer ID, once its certificate is in the keychain, is the
/// identity users' grants are keyed on — it beats the self-signed `sensei-dev`, and
/// it is signed with a secure timestamp, which notarization requires.
#[test]
fn a_developer_id_in_the_keychain_wins_and_is_timestamped() {
    let t = tempfile::tempdir().unwrap();
    let (o, log) = run(t.path(), true, &[("HAS_DEVELOPER_ID", "1")]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(log.contains("--sign Developer ID Application: Example Co (TEAM123456)"), "{log}");
    assert!(log.contains("--timestamp"), "{log}");
    assert!(log.contains("--options runtime"), "{log}");
}

/// The self-signed dev identity is not timestamped: a local install must not need the
/// network to sign.
#[test]
fn the_dev_identity_is_not_timestamped() {
    let t = tempfile::tempdir().unwrap();
    let (_, log) = run(t.path(), true, &[]);
    assert!(!log.contains("--timestamp"), "{log}");
}

/// CI names the identity explicitly (`SENSEI_SIGN_IDENTITY`, from the release secrets);
/// a Developer ID named that way is timestamped too.
#[test]
fn an_explicit_developer_id_is_timestamped() {
    let t = tempfile::tempdir().unwrap();
    let (_, log) =
        run(t.path(), false, &[("SENSEI_SIGN_IDENTITY", "Developer ID Application: X (T)")]);
    assert!(log.contains("--timestamp"), "{log}");
}
