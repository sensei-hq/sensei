//! `scripts/deploy-dojo-db.sh` — the dōjō schema deploy, ONE script for the
//! release (production) and for CI (a fresh Supabase before the access-rule
//! tests), so every schema change rehearses the exact path production takes.
//!
//! Driven with a fake `dbd` that logs each call and can be told to report a
//! parse error or a remaining difference.

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/deploy-dojo-db.sh")
}

struct Run {
    code: Option<i32>,
    stderr: String,
    calls: Vec<String>,
}

/// `released` writes `released: true` into the fake design; `fake` sets the
/// fake dbd's behaviour (`parse_error`, `drift`).
fn run(name: &str, url: Option<&str>, released: bool, fake: &[&str]) -> Run {
    let dir = std::env::temp_dir().join(format!("deploy-dojo-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let db = dir.join("database");
    std::fs::create_dir_all(&db).unwrap();
    std::fs::write(
        db.join("design.yaml"),
        format!("project:\n  name: t\n{}", if released { "  released: true\n" } else { "" }),
    )
    .unwrap();
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let log = dir.join("dbd.log");
    let dbd = bin.join("dbd");
    std::fs::write(
        &dbd,
        format!(
            "#!/bin/sh\necho \"$*\" >> \"{log}\"\n\
             case \"$1\" in\n\
               inspect) [ -n \"$FAKE_PARSE_ERROR\" ] && echo 'Parse error in ddl/x.ddl'; exit 0 ;;\n\
               diff) case \"$*\" in *--exit-code*) [ -n \"$FAKE_DRIFT\" ] && exit 2 ;; esac; exit 0 ;;\n\
             esac\nexit 0\n",
            log = log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&dbd, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();

    let mut cmd = Command::new(script());
    cmd.current_dir(&db)
        .env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()))
        .env_remove("DATABASE_URL")
        .env_remove("FAKE_PARSE_ERROR")
        .env_remove("FAKE_DRIFT");
    if let Some(u) = url {
        cmd.env("DATABASE_URL", u);
    }
    for f in fake {
        cmd.env(format!("FAKE_{}", f.to_uppercase()), "1");
    }
    let out = cmd.output().unwrap();
    let calls =
        std::fs::read_to_string(&log).unwrap_or_default().lines().map(String::from).collect();
    std::fs::remove_dir_all(&dir).unwrap();
    Run { code: out.status.code(), stderr: String::from_utf8_lossy(&out.stderr).into(), calls }
}

const URL: Option<&str> = Some("postgresql://fake/dojo");

#[test]
fn a_pre_release_deploy_reconciles_then_deploys_then_verifies() {
    let r = run("pre", URL, false, &[]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert_eq!(
        r.calls,
        [
            "inspect",
            "diff --scope dojo",
            "reconcile --scope dojo --allow-destructive",
            "deploy --scope dojo",
            "diff --scope dojo --exit-code",
        ]
    );
}

#[test]
fn a_released_project_migrates_and_never_reconciles() {
    let r = run("released", URL, true, &[]);
    assert_eq!(r.code, Some(0), "{}", r.stderr);
    assert!(r.calls.contains(&"migrate --status".to_string()), "{:?}", r.calls);
    assert!(!r.calls.iter().any(|c| c.starts_with("reconcile")), "{:?}", r.calls);
    assert!(r.calls.contains(&"deploy --scope dojo".to_string()));
}

#[test]
fn a_parse_error_stops_it_before_anything_is_applied() {
    // `dbd inspect` exits 0 on a file it cannot parse and drops it silently.
    let r = run("parse", URL, false, &["parse_error"]);
    assert_ne!(r.code, Some(0));
    assert_eq!(r.calls, ["inspect"], "nothing after the failed inspect");
}

#[test]
fn a_difference_left_after_the_deploy_fails_it() {
    let r = run("drift", URL, false, &["drift"]);
    assert_ne!(r.code, Some(0), "a deploy that left the schema unlike the design is no success");
}

#[test]
fn without_a_target_it_refuses_rather_than_succeed() {
    let r = run("nourl", None, false, &[]);
    assert_ne!(r.code, Some(0));
    assert!(r.stderr.contains("DATABASE_URL"), "{}", r.stderr);
    assert!(r.calls.is_empty());
}
