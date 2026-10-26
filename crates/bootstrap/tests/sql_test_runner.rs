//! `database/tests/run.sh` — the SQL assertions against a real Postgres.
//!
//! The dōjō's access rules (58 of its 61 files) ran only on a developer's
//! machine: the runner demanded BOTH databases, and a CI job that stands up the
//! dōjō has no daemon database. `run.sh <scope>` runs one scope's directory and
//! asks only for that scope's database.
//!
//! Driven with a fake `psql` that answers for the dōjō's URL, refuses the
//! daemon's, and logs every file it is asked to run — so no Postgres is needed.

use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../database/tests/run.sh")
}

const DOJO: &str = "postgresql://fake/dojo";
const SENSEI: &str = "postgresql://fake/sensei";

/// Runs `run.sh args` with the fake `psql` first on PATH; returns the output
/// and the files the fake was asked to run.
fn run(dir: &Path, args: &[&str]) -> (std::process::Output, Vec<String>) {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let log = dir.join("psql.log");
    let psql = bin.join("psql");
    std::fs::write(
        &psql,
        format!(
            "#!/bin/sh\n\
             # $1 is the URL. Only the dōjō's answers.\n\
             [ \"$1\" = \"{DOJO}\" ] || exit 2\n\
             while [ $# -gt 0 ]; do [ \"$1\" = -f ] && echo \"$2\" >> \"{log}\"; shift; done\n\
             exit 0\n",
            log = log.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&psql, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let out = Command::new(script())
        .args(args)
        .env("PATH", path)
        .env("DATABASE_URL", DOJO)
        .env("SENSEI_DATABASE_URL", SENSEI)
        .output()
        .unwrap();
    let ran = std::fs::read_to_string(&log).unwrap_or_default().lines().map(String::from).collect();
    (out, ran)
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("sql-runner-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn a_scope_runs_its_own_files_against_its_own_database_only() {
    let d = tmp("scope");
    let (out, ran) = run(&d, &["dojo"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(0), "the daemon's database is not needed: {err}");
    assert!(ran.len() > 50, "the dōjō's files ran: {}", ran.len());
    assert!(ran.iter().all(|f| f.starts_with("./dojo/")), "nothing outside dojo/ ran: {ran:?}");
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn with_no_scope_every_database_is_asked_for() {
    let d = tmp("all");
    let (out, ran) = run(&d, &[]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(69), "{err}");
    assert!(err.contains(SENSEI), "the daemon's database is asked for: {err}");
    assert!(ran.is_empty(), "nothing runs until every database answers");
    std::fs::remove_dir_all(&d).unwrap();
}

#[test]
fn an_unknown_scope_is_refused_not_run_as_nothing() {
    // A typo must not run zero files and pass.
    let d = tmp("typo");
    let (out, ran) = run(&d, &["dojoo"]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{err}");
    assert!(err.contains("dojoo"), "{err}");
    assert!(ran.is_empty());
    std::fs::remove_dir_all(&d).unwrap();
}
